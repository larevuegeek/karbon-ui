//! La grille de propriétés : des sections qui se plient, chacune faite de lignes
//! « libellé, contrôle », avec une bulle d'aide par ligne.
use crate::{
    application::Action,
    ui::{
        components::Text,
        editor::EditorPalette,
        element::{
            Align, Axis, Element, ElementId, Insets, Length, PaintContext, Role, Semantics, SizeF,
            TextMeasurer, Widget,
        },
        scene::{PointF, RectF, Scene},
        tooltip::Tooltip,
    },
};
use std::sync::Arc;

/// A row of a [`PropertyGrid`]: a label and the control that edits the value (a
/// [`NumberField`](super::super::NumberField), a [`checkbox`](super::checkbox), a
/// [`choice`](super::choice)…), with an optional tooltip on the whole row.
pub struct Property {
    label: Arc<str>,
    editor: Element,
    tooltip: Option<Arc<str>>,
}

impl Property {
    /// The row labelled `label`, edited by `editor`.
    pub fn new(label: impl Into<Arc<str>>, editor: Element) -> Self {
        Self {
            label: label.into(),
            editor,
            tooltip: None,
        }
    }

    /// Show `text` when the pointer rests on the row or the keyboard reaches it: what
    /// the property does, in a sentence.
    pub fn tooltip(mut self, text: impl Into<Arc<str>>) -> Self {
        self.tooltip = Some(text.into());
        self
    }
}

/// A titled group of rows that folds: « Transformation », « Lumière ». The application
/// keeps which sections are open.
pub struct PropertySection {
    key: u64,
    title: Arc<str>,
    open: bool,
    rows: Vec<Property>,
}

impl PropertySection {
    /// The section of `key` titled `title`, showing its rows when `open`.
    pub fn new(key: u64, title: impl Into<Arc<str>>, open: bool) -> Self {
        Self {
            key,
            title: title.into(),
            open,
            rows: Vec::new(),
        }
    }

    /// Add a row.
    pub fn row(mut self, property: Property) -> Self {
        self.rows.push(property);
        self
    }
}

/// The inspector panel of an editor: sections that fold, made of « label, control »
/// rows, in a panel that scrolls. The controls are ordinary elements, so their
/// messages go to the application as usual; a click on a section title sends
/// `on_toggle` with the key of the section.
///
/// ```
/// use karbon_ui::ui::{
///     editor::{EditorPalette, Property, PropertyGrid, PropertySection, checkbox, read_only},
///     prelude::*,
/// };
///
/// #[derive(Clone)]
/// enum Msg {
///     Fold(u64),
///     Visible(bool),
/// }
/// let p = EditorPalette::DARK;
/// let grid = PropertyGrid::new("inspector", Msg::Fold)
///     .section(
///         PropertySection::new(1, "Objet", true)
///             .row(Property::new("Nom", read_only("name", "Caméra", &p)))
///             .row(
///                 Property::new("Visible", checkbox("visible", Some(true), Msg::Visible, &p))
///                     .tooltip("Montre l'objet dans la vue et dans le jeu."),
///             ),
///     )
///     .element();
/// # let _ = grid;
/// ```
pub struct PropertyGrid<M> {
    id: ElementId,
    on_toggle: std::rc::Rc<dyn Fn(u64) -> M>,
    sections: Vec<PropertySection>,
    label_width: f32,
    palette: EditorPalette,
}

impl<M: Clone + 'static> PropertyGrid<M> {
    /// An empty grid; `on_toggle` turns the key of a section clicked into a message.
    pub fn new(id: impl Into<ElementId>, on_toggle: impl Fn(u64) -> M + 'static) -> Self {
        Self {
            id: id.into(),
            on_toggle: std::rc::Rc::new(on_toggle),
            sections: Vec::new(),
            label_width: 110.,
            palette: EditorPalette::DARK,
        }
    }

    /// Add a section.
    pub fn section(mut self, section: PropertySection) -> Self {
        self.sections.push(section);
        self
    }

    /// Width of the label column, in logical pixels (110 by default).
    pub fn label_width(mut self, width: f32) -> Self {
        self.label_width = width.max(0.);
        self
    }

    /// Colours of the grid ([`EditorPalette::DARK`] by default).
    pub fn palette(mut self, palette: EditorPalette) -> Self {
        self.palette = palette;
        self
    }

    /// The element: a vertical scroll panel filling its parent.
    pub fn element(self) -> Element {
        let p = self.palette;
        let mut children = Vec::new();
        for section in self.sections {
            let scope = self.id.item(section.key);
            let header = SectionHeader {
                title: Arc::clone(&section.title),
                open: section.open,
                action: Action::new((self.on_toggle)(section.key)),
                palette: p,
            };
            children.push(
                Element::custom(scope.child("header").0, header).height(Length::Px(p.row_height)),
            );
            if !section.open {
                continue;
            }
            for (index, property) in section.rows.into_iter().enumerate() {
                children.push(row(
                    scope.item(index as u64),
                    property,
                    self.label_width,
                    &p,
                ));
            }
        }
        let content = Element::column(self.id.child("rows").0, children)
            .gap(2.)
            .padding(4.);
        Element::scroll(self.id.0, Axis::Vertical, content).surface(p.panel, 0.)
    }
}

/// Une ligne « libellé, contrôle ».
fn row(id: ElementId, property: Property, label_width: f32, p: &EditorPalette) -> Element {
    let label = Element::custom(
        id.child("label").0,
        Text::new(property.label).size(p.font_size).color(p.muted),
    )
    .width(Length::Px(label_width));
    let editor = property.editor.width(Length::Fill(1.));
    let line = Element::row(id.0, vec![label, editor])
        .gap(6.)
        .align(Align::Center)
        .min_height(p.row_height)
        .insets(Insets {
            left: 14.,
            right: 2.,
            ..Insets::default()
        });
    match property.tooltip {
        Some(text) => line.tooltip_with(Tooltip::new(text).style(p.tooltip())),
        None => line,
    }
}

/// Titre d'une section : flèche, titre en gras ; un clic, Entrée ou Espace la plie ou
/// la déplie.
struct SectionHeader {
    title: Arc<str>,
    open: bool,
    action: Action,
    palette: EditorPalette,
}

impl Widget for SectionHeader {
    fn measure(&self, _: SizeF, _: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
        Ok(SizeF {
            width: 0.,
            height: self.palette.row_height,
        })
    }

    fn paint(&self, context: &PaintContext, scene: &mut Scene) {
        let p = &self.palette;
        let r = context.rect;
        let hovered = context.state.hovered || context.state.focused;
        scene.fill(r, 3., if hovered { p.hover } else { p.field });
        let (x, y) = (r.left + 6., (r.top + r.bottom) / 2.);
        let points = if self.open {
            vec![
                pt(x - 0.5, y - 2.5),
                pt(x + 7.5, y - 2.5),
                pt(x + 3.5, y + 2.5),
            ]
        } else {
            vec![pt(x + 1., y - 4.), pt(x + 6., y), pt(x + 1., y + 4.)]
        };
        scene.commands.push(crate::ui::scene::DrawCommand::Path {
            points,
            closed: true,
            color: p.muted,
        });
        let height = r.bottom - r.top;
        let top = r.top + ((height - p.font_size * 1.35) / 2.).max(0.);
        scene.text_single_line(
            RectF::new(
                r.left + 20.,
                top,
                (r.right - r.left - 24.).max(0.),
                p.font_size * 1.4,
            ),
            Arc::clone(&self.title),
            p.font_size,
            true,
            p.text,
        );
    }

    fn action(&self) -> Option<Action> {
        Some(self.action.clone())
    }

    fn semantics(&self) -> Semantics<'_> {
        Semantics {
            role: Role::Heading,
            name: &self.title,
            enabled: true,
        }
    }
}

fn pt(x: f32, y: f32) -> PointF {
    PointF { x, y }
}
