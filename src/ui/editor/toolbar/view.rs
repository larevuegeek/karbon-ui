//! La barre d'outils à l'écran : les éléments qui tiennent dans la largeur donnée, puis
//! le bouton « » » qui ouvre les autres dans un menu.
use super::model::{Kind, ToolItem};
use crate::{
    application::Action,
    ui::{
        editor::{EditorPalette, MenuEntry, menu_bar::anchored_menu, tip},
        element::{
            Element, ElementId, Length, PaintContext, Role, Semantics, SizeF, TextMeasurer, Widget,
        },
        popover::Anchor,
        scene::{DrawCommand, RectF, Scene},
        vector::VectorIcon,
    },
};
use std::{rc::Rc, sync::Arc};

/// State of a [`Toolbar`], kept by the application: whether its overflow menu is open.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ToolbarState {
    overflow: bool,
}

impl ToolbarState {
    /// Whether the overflow menu is open.
    pub fn overflow_open(&self) -> bool {
        self.overflow
    }

    /// Apply a gesture of the bar; returns whether to rebuild the view.
    pub fn update(&mut self, message: ToolbarMsg) -> bool {
        self.overflow = match message.0 {
            Inner::Open => !self.overflow,
            Inner::Close => false,
        };
        true
    }
}

/// A gesture of the bar itself (its overflow menu), for [`ToolbarState::update`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToolbarMsg(Inner);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Inner {
    Open,
    Close,
}

/// Écart entre deux éléments, et largeur d'un séparateur.
const GAP: f32 = 2.;
const SEPARATOR: f32 = 9.;

/// A toolbar: icon buttons, options turned on or off, groups of exclusive choices
/// (move, rotate, scale), separators. Each button has its tooltip (name, shortcut,
/// description). When `width` is too small for all the items, the last ones go, in
/// order, to a menu opened by a « » » button at the end of the bar.
///
/// ```
/// use karbon_ui::ui::{editor::{ToolItem, Toolbar, ToolbarMsg, ToolbarState}, prelude::*};
///
/// #[derive(Clone)]
/// enum Msg {
///     Bar(ToolbarMsg),
///     Move,
///     Rotate,
///     Grid,
/// }
/// let state = ToolbarState::default();
/// let items = vec![
///     ToolItem::choice("Déplacer", true, Msg::Move).text("G").shortcut("G"),
///     ToolItem::choice("Tourner", false, Msg::Rotate).text("R").shortcut("R"),
///     ToolItem::separator(),
///     ToolItem::toggle("Grille", false, Msg::Grid).text("#"),
/// ];
/// let (bar, overlays) = Toolbar::new("tools", &state, Msg::Bar, items).build(400.);
/// # let _ = (bar, overlays);
/// ```
pub struct Toolbar<M> {
    id: ElementId,
    open: bool,
    map: Rc<dyn Fn(ToolbarMsg) -> M>,
    items: Vec<ToolItem<M>>,
    palette: EditorPalette,
}

impl<M: Clone + 'static> Toolbar<M> {
    /// The bar of `items` in the state `state`; `map` wraps its own gestures (the
    /// overflow menu) in the application's message.
    pub fn new(
        id: impl Into<ElementId>,
        state: &ToolbarState,
        map: impl Fn(ToolbarMsg) -> M + 'static,
        items: Vec<ToolItem<M>>,
    ) -> Self {
        Self {
            id: id.into(),
            open: state.overflow,
            map: Rc::new(map),
            items,
            palette: EditorPalette::DARK,
        }
    }

    /// Colours and sizes ([`EditorPalette::DARK`] by default).
    pub fn palette(mut self, palette: EditorPalette) -> Self {
        self.palette = palette;
        self
    }

    /// The bar, at most `width` wide, and the overlays to add to the root `layers` (its
    /// overflow menu, when open).
    pub fn build(self, width: f32) -> (Element, Vec<Element>) {
        let p = self.palette;
        let widths: Vec<f32> = self.items.iter().map(|i| item_width(i, &p)).collect();
        let shown = fitting(&widths, width, button_size(&p));
        let mut children = Vec::new();
        let mut hidden = Vec::new();
        for (index, item) in self.items.into_iter().enumerate() {
            if index < shown {
                children.push(item_element(
                    self.id.child("item").item(index as u64),
                    item,
                    widths[index],
                    &p,
                ));
            } else if item.kind != Kind::Separator || !hidden.is_empty() {
                hidden.push(item);
            }
        }
        let mut overlays = Vec::new();
        if !hidden.is_empty() {
            let more = self.id.child("more");
            let button = More {
                action: Action::new((self.map)(ToolbarMsg(Inner::Open))),
                palette: p,
            };
            let element = Element::custom(more.0, button)
                .width(Length::Px(button_size(&p)))
                .height(Length::Px(button_size(&p)))
                .tooltip_with(tip("Plus d'outils", "", "", &p));
            children.push(element);
            if self.open {
                overlays.push(overflow_menu(self.id, hidden, &self.map, p));
            }
        }
        let bar = Element::row(self.id.0, children)
            .gap(GAP)
            .padding(2.)
            .width(Length::Auto)
            .height(Length::Px(button_size(&p) + 4.))
            .surface(p.panel, 4.);
        (bar, overlays)
    }
}

/// Côté d'un bouton carré.
fn button_size(p: &EditorPalette) -> f32 {
    p.row_height + 6.
}

/// La largeur d'un élément dans la barre.
fn item_width<M>(item: &ToolItem<M>, p: &EditorPalette) -> f32 {
    match (item.kind, item.width, &item.icon) {
        (Kind::Separator, _, _) => SEPARATOR,
        (_, Some(width), _) => width,
        (_, None, Some(_)) => button_size(p),
        (_, None, None) => {
            let chars = item.text.chars().count() as f32;
            (16. + chars * 0.6 * p.font_size)
                .round()
                .max(button_size(p))
        }
    }
}

/// Combien d'éléments tiennent dans `width` : tous, ou les premiers avec la place du
/// bouton « » » au bout (un séparateur ne finit jamais la partie montrée).
pub(super) fn fitting(widths: &[f32], width: f32, more: f32) -> usize {
    let inner = width - 4.;
    let total: f32 = widths.iter().sum::<f32>() + GAP * widths.len().saturating_sub(1) as f32;
    if total <= inner {
        return widths.len();
    }
    let mut used = more;
    let mut count = 0;
    for &w in widths {
        if used + w + GAP > inner {
            break;
        }
        used += w + GAP;
        count += 1;
    }
    count
}

fn item_element<M: Clone + 'static>(
    id: ElementId,
    item: ToolItem<M>,
    width: f32,
    p: &EditorPalette,
) -> Element {
    let size = button_size(p);
    if item.kind == Kind::Separator {
        return Element::custom(id.0, Line { palette: *p })
            .width(Length::Px(width))
            .height(Length::Px(size));
    }
    let help = tip(&item.name, &item.description, &item.shortcut, p);
    let lit = matches!(item.kind, Kind::Toggle(true) | Kind::Choice(true));
    let action = item.message.filter(|_| item.enabled).map(Action::new);
    let widget = Button {
        name: item.name,
        text: item.text,
        icon: item.icon,
        lit,
        action,
        palette: *p,
    };
    Element::custom(id.0, widget)
        .width(Length::Px(width))
        .height(Length::Px(size))
        .tooltip_with(help)
}

/// Le menu des éléments qui ne tiennent pas : leur nom, leur marque, leur raccourci.
fn overflow_menu<M: Clone + 'static>(
    bar: ElementId,
    items: Vec<ToolItem<M>>,
    map: &Rc<dyn Fn(ToolbarMsg) -> M>,
    p: EditorPalette,
) -> Element {
    let entries = items
        .into_iter()
        .map(|item| match (item.kind, item.message) {
            (Kind::Separator, _) | (_, None) => MenuEntry::separator(),
            (kind, Some(message)) => {
                let entry = MenuEntry::new(item.name, message)
                    .shortcut(item.shortcut)
                    .enabled(item.enabled)
                    .tip(item.description);
                match kind {
                    Kind::Toggle(on) => entry.checked(on),
                    Kind::Choice(selected) => entry.radio(selected),
                    _ => entry,
                }
            }
        })
        .collect();
    let anchor = Anchor::Element(bar.child("more"));
    let close = map(ToolbarMsg(Inner::Close));
    anchored_menu(bar.child("overflow"), anchor, entries, close, p)
}

/// Un bouton de la barre : fond allumé (option, choix fait), survolé ; icône ou texte.
struct Button {
    name: Arc<str>,
    text: Arc<str>,
    icon: Option<Arc<VectorIcon>>,
    lit: bool,
    action: Option<Action>,
    palette: EditorPalette,
}

impl Widget for Button {
    fn measure(&self, available: SizeF, _: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
        Ok(available)
    }

    fn paint(&self, context: &PaintContext, scene: &mut Scene) {
        let p = &self.palette;
        let r = context.rect;
        let enabled = self.action.is_some();
        if self.lit {
            scene.fill(r, 4., p.selection);
        } else if enabled && (context.state.hovered || context.state.pressed) {
            scene.fill(r, 4., p.hover);
        }
        if context.state.focused {
            scene.fill(
                RectF::new(r.left, r.bottom - 2., r.right - r.left, 2.),
                1.,
                p.accent,
            );
        }
        let color = if enabled { p.text } else { p.disabled };
        match &self.icon {
            Some(icon) => {
                let side = (r.bottom - r.top - 10.).max(4.);
                let rect = RectF::new(
                    (r.left + r.right - side) / 2.,
                    (r.top + r.bottom - side) / 2.,
                    side,
                    side,
                );
                scene.commands.push(DrawCommand::Vector {
                    rect,
                    icon: Arc::clone(icon),
                    color,
                    outline: 0.,
                });
            }
            None => {
                let height = p.font_size * 1.4;
                let top = r.top + ((r.bottom - r.top - height) / 2.).max(0.);
                let text_rect =
                    RectF::new(r.left + 8., top, (r.right - r.left - 16.).max(0.), height);
                scene.text_single_line(
                    text_rect,
                    Arc::clone(&self.text),
                    p.font_size,
                    false,
                    color,
                );
            }
        }
    }

    fn action(&self) -> Option<Action> {
        self.action.clone()
    }

    fn semantics(&self) -> Semantics<'_> {
        Semantics {
            role: Role::Button,
            name: &self.name,
            enabled: self.action.is_some(),
        }
    }
}

/// Le bouton « » » du débordement.
struct More {
    action: Action,
    palette: EditorPalette,
}

impl Widget for More {
    fn measure(&self, available: SizeF, _: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
        Ok(available)
    }

    fn paint(&self, context: &PaintContext, scene: &mut Scene) {
        let p = &self.palette;
        let r = context.rect;
        if context.state.hovered || context.state.pressed {
            scene.fill(r, 4., p.hover);
        }
        let height = p.font_size * 1.4;
        let top = r.top + ((r.bottom - r.top - height) / 2.).max(0.);
        let rect = RectF::new(r.left + (r.right - r.left) / 2. - 5., top, 14., height);
        scene.text_single_line(rect, Arc::<str>::from("»"), p.font_size, false, p.text);
    }

    fn action(&self) -> Option<Action> {
        Some(self.action.clone())
    }

    fn semantics(&self) -> Semantics<'_> {
        Semantics {
            role: Role::Button,
            name: "Plus d'outils",
            enabled: true,
        }
    }
}

/// Le trait vertical d'un séparateur.
struct Line {
    palette: EditorPalette,
}

impl Widget for Line {
    fn measure(&self, available: SizeF, _: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
        Ok(available)
    }

    fn paint(&self, context: &PaintContext, scene: &mut Scene) {
        let r = context.rect;
        let x = ((r.left + r.right) / 2.).floor();
        scene.fill(
            RectF::new(x, r.top + 5., 1., (r.bottom - r.top - 10.).max(0.)),
            0.,
            self.palette.border,
        );
    }

    fn semantics(&self) -> Semantics<'_> {
        Semantics {
            role: Role::Separator,
            name: "",
            enabled: true,
        }
    }
}
