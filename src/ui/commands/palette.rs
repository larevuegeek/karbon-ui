//! La palette de commandes (Ctrl+Maj+P) : un champ de recherche et la liste des
//! commandes qui y répondent, les meilleures d'abord, avec leur raccourci ; flèches,
//! Entrée, Échap ou la souris.
use super::search::fuzzy_score;
use crate::{
    application::Action,
    ui::{
        editor::EditorPalette,
        element::{
            Element, ElementId, Length, PaintContext, Role, Semantics, SizeF, TextMeasurer, Widget,
        },
        popover::{Anchor, Popover},
        runtime::Key,
        scene::{PointF, RectF, Scene},
        widgets::TextInput,
    },
};
use std::{rc::Rc, sync::Arc};

/// A line of the [`CommandPalette`]: a command (or anything the application wants to
/// find by name: an object, a page).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PaletteItem {
    /// What the application receives when the line is chosen.
    pub id: String,
    /// The name searched and shown.
    pub name: String,
    /// Shown on the right, dimmed: the shortcut, a kind.
    pub detail: String,
}

/// State of a [`CommandPalette`], kept by the application: open or not, the text typed,
/// the highlighted line.
#[derive(Clone, Debug, Default)]
pub struct PaletteState {
    open: bool,
    query: String,
    selected: usize,
}

/// A gesture in the palette, to give back to [`PaletteState::update`].
#[derive(Clone, Debug, PartialEq)]
pub struct PaletteMsg(Inner);

#[derive(Clone, Debug, PartialEq)]
enum Inner {
    Query(String),
    Key(Key),
    Submit,
    Choose(usize),
    Close,
}

/// What a gesture did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PaletteReply {
    /// The identifier of the line chosen (Enter or a click): the palette is closed.
    pub chosen: Option<String>,
    /// Rebuild the view.
    pub rebuild: bool,
}

/// Lignes montrées au plus.
const SHOWN: usize = 12;

impl PaletteState {
    /// Open the palette with an empty search.
    pub fn open(&mut self) {
        *self = Self {
            open: true,
            ..Self::default()
        };
    }

    /// Close the palette.
    pub fn close(&mut self) {
        self.open = false;
    }

    /// Whether the palette is open.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// The text typed.
    pub fn query(&self) -> &str {
        &self.query
    }

    /// The indices of `items` shown for the text typed, the best first, at most 12.
    pub fn matches(&self, items: &[PaletteItem]) -> Vec<usize> {
        let mut found: Vec<(i32, usize)> = items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| {
                let by_name = fuzzy_score(&self.query, &item.name);
                let by_id = || fuzzy_score(&self.query, &item.id).map(|s| s - 20);
                by_name.or_else(by_id).map(|score| (score, index))
            })
            .collect();
        found.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        found.truncate(SHOWN);
        found.into_iter().map(|(_, index)| index).collect()
    }

    /// The highlighted line among the matches.
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// Apply a gesture of the palette showing `items`.
    pub fn update(&mut self, message: PaletteMsg, items: &[PaletteItem]) -> PaletteReply {
        let shown = self.matches(items);
        let mut reply = PaletteReply {
            rebuild: true,
            ..PaletteReply::default()
        };
        match message.0 {
            Inner::Query(text) => {
                self.query = text;
                self.selected = 0;
            }
            Inner::Key(Key::Down) => {
                self.selected = (self.selected + 1).min(shown.len().saturating_sub(1))
            }
            Inner::Key(Key::Up) => self.selected = self.selected.saturating_sub(1),
            Inner::Key(Key::PageDown) => self.selected = shown.len().saturating_sub(1),
            Inner::Key(Key::PageUp) => self.selected = 0,
            Inner::Key(Key::Escape) | Inner::Close => self.close(),
            Inner::Key(_) => reply.rebuild = false,
            Inner::Submit => reply.chosen = self.choose(&shown, self.selected, items),
            Inner::Choose(line) => reply.chosen = self.choose(&shown, line, items),
        }
        reply
    }

    fn choose(&mut self, shown: &[usize], line: usize, items: &[PaletteItem]) -> Option<String> {
        let index = *shown.get(line)?;
        self.close();
        Some(items[index].id.clone())
    }
}

/// The command palette: a search field and, under it, the commands whose name matches
/// what is typed (letters in order, capitals and accents ignored), the best first, with
/// their shortcut. Up and Down move the highlight, Enter or a click chooses, Escape or a
/// click outside closes. Add [`build`](Self::build)'s element to the overlays of the root
/// `layers` while [`PaletteState::is_open`].
///
/// ```
/// use karbon_ui::ui::{commands::{CommandPalette, PaletteItem, PaletteMsg, PaletteState}, prelude::*, scene::PointF};
///
/// #[derive(Clone)]
/// enum Msg {
///     Palette(PaletteMsg),
/// }
/// let items = vec![PaletteItem { id: "file.save".into(), name: "Enregistrer".into(), detail: "Ctrl+S".into() }];
/// let mut state = PaletteState::default();
/// state.open();
/// let overlay = CommandPalette::new("palette", &state, &items, Msg::Palette)
///     .at(PointF { x: 400., y: 60. })
///     .build();
/// # let _ = overlay;
/// ```
pub struct CommandPalette<'a, M> {
    id: ElementId,
    state: &'a PaletteState,
    items: &'a [PaletteItem],
    map: Rc<dyn Fn(PaletteMsg) -> M>,
    at: PointF,
    width: f32,
    placeholder: Arc<str>,
    palette: EditorPalette,
}

impl<'a, M: Clone + 'static> CommandPalette<'a, M> {
    /// The palette of `items` in the state `state`; `map` wraps its gestures in the
    /// application's message.
    pub fn new(
        id: impl Into<ElementId>,
        state: &'a PaletteState,
        items: &'a [PaletteItem],
        map: impl Fn(PaletteMsg) -> M + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            state,
            items,
            map: Rc::new(map),
            at: PointF { x: 0., y: 0. },
            width: 560.,
            placeholder: "Commande…".into(),
            palette: EditorPalette::DARK,
        }
    }

    /// Where the top left corner of the palette goes, in window coordinates.
    pub fn at(mut self, at: PointF) -> Self {
        self.at = at;
        self
    }

    /// Width of the palette (560 by default).
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Text shown in the empty search field (« Commande… » by default).
    pub fn placeholder(mut self, text: impl Into<Arc<str>>) -> Self {
        self.placeholder = text.into();
        self
    }

    /// Colours ([`EditorPalette::DARK`] by default).
    pub fn palette(mut self, palette: EditorPalette) -> Self {
        self.palette = palette;
        self
    }

    /// The popover of the palette.
    pub fn build(self) -> Element {
        let map = &self.map;
        let p = self.palette;
        let typed = Rc::clone(map);
        let keyed = Rc::clone(map);
        let field = TextInput::new(self.state.query.clone(), move |text| {
            typed(PaletteMsg(Inner::Query(text)))
        })
        .on_key(move |key| keyed(PaletteMsg(Inner::Key(key))))
        .on_submit(map(PaletteMsg(Inner::Submit)))
        .placeholder(Arc::clone(&self.placeholder))
        .label("Rechercher une commande")
        .autofocus();
        let mut children = vec![
            Element::custom(self.id.child("search").0, field).height(Length::Px(p.row_height + 6.)),
        ];
        let shown = self.state.matches(self.items);
        for (line, &index) in shown.iter().enumerate() {
            let item = &self.items[index];
            let row = Row {
                name: item.name.as_str().into(),
                detail: item.detail.as_str().into(),
                selected: line == self.state.selected,
                action: Action::new(map(PaletteMsg(Inner::Choose(line)))),
                palette: p,
            };
            children.push(
                Element::custom(self.id.child("line").item(line as u64).0, row)
                    .height(Length::Px(p.row_height)),
            );
        }
        if shown.is_empty() {
            children.push(crate::ui::editor::label(
                self.id.child("none"),
                "Aucune commande ne correspond.",
                &p,
            ));
        }
        let popover = Popover::new(
            "Palette de commandes",
            Anchor::Point(self.at),
            map(PaletteMsg(Inner::Close)),
        )
        .initial_focus(self.id.child("search"));
        Element::popover(
            self.id.0,
            popover,
            Element::column(self.id.child("lines").0, children),
        )
        .width(Length::Px(self.width))
        .padding(4.)
        .surface(p.field, 6.)
    }
}

/// Une ligne de la palette : le nom, le détail à droite, surlignée si choisie.
struct Row {
    name: Arc<str>,
    detail: Arc<str>,
    selected: bool,
    action: Action,
    palette: EditorPalette,
}

impl Widget for Row {
    fn measure(&self, available: SizeF, _: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
        Ok(SizeF {
            width: available.width,
            height: self.palette.row_height,
        })
    }

    fn paint(&self, context: &PaintContext, scene: &mut Scene) {
        let p = &self.palette;
        let r = context.rect;
        if self.selected {
            scene.fill(r, 3., p.selection);
        } else if context.state.hovered {
            scene.fill(r, 3., p.hover);
        }
        let height = p.font_size * 1.4;
        let top = r.top + ((r.bottom - r.top - height) / 2.).max(0.);
        let width = (r.right - r.left - 16.).max(0.);
        scene.text_single_line(
            RectF::new(r.left + 8., top, width * 0.7, height),
            Arc::clone(&self.name),
            p.font_size,
            false,
            p.text,
        );
        if !self.detail.is_empty() {
            let x = r.left + 8. + width * 0.7;
            scene.text_single_line(
                RectF::new(x, top, width * 0.3, height),
                Arc::clone(&self.detail),
                p.font_size,
                false,
                p.muted,
            );
        }
    }

    fn action(&self) -> Option<Action> {
        Some(self.action.clone())
    }

    fn semantics(&self) -> Semantics<'_> {
        Semantics {
            role: Role::ListItem,
            name: &self.name,
            enabled: true,
        }
    }
}
