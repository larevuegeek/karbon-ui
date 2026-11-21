//! La vue d'un arbre : une liste virtualisée des lignes visibles, chacune décalée selon
//! sa profondeur, avec sa flèche, sa sélection, le curseur et la marque de dépôt.
use super::{
    rows::TreeRow,
    state::{DropPlace, TreeMsg, TreeState},
};
use crate::ui::{
    editor::EditorPalette,
    element::{Align, Element, ElementId, Insets, PaintContext, SizeF, TextMeasurer, Widget},
    runtime::{PointerButton, PointerEvent, PointerPhase},
    scene::{DrawCommand, PointF, RectF, Scene},
    virtual_list::VirtualList,
};
use std::{collections::HashSet, rc::Rc};

/// Hauteur d'une ligne par défaut, en pixels logiques.
pub(super) const ROW_HEIGHT: f32 = 22.;
/// Décalage d'un niveau de profondeur, qui est aussi la largeur de la flèche.
pub(super) const INDENT: f32 = 14.;

/// What the content of a row is built from ([`TreeView::new`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TreeItem {
    /// Key of the node.
    pub key: u64,
    /// Nesting level, `0` for a top-level node.
    pub depth: u16,
    /// The row is selected.
    pub selected: bool,
    /// The node has children.
    pub has_children: bool,
    /// The node shows its children.
    pub expanded: bool,
    /// Identifier of the row: derive the identifiers of the content from it
    /// (`item.scope.child("name")`), so that they stay unique in the window.
    pub scope: ElementId,
}

/// A tree of nodes in a virtualized list: only the visible rows are built, so a tree of
/// 100,000 nodes scrolls like a short one. Arrows fold and unfold the branches, a click
/// selects (Ctrl adds or removes a row, Shift selects a range), a double click
/// activates, dragging rows onto another one moves them under it. The application keeps
/// a [`TreeState`], forwards the [`TreeMsg`]s of the view to
/// [`TreeState::update`] and the keys to [`TreeState::key`] while the tree is its active
/// panel. The mouse gestures use [`Element::on_pointer`], delivered on Windows only for
/// now.
///
/// ```
/// use karbon_ui::ui::{
///     editor::{EditorPalette, TreeMsg, TreeSource, TreeState, TreeView, label},
///     prelude::*,
/// };
///
/// #[derive(Clone)]
/// enum Msg {
///     Tree(TreeMsg),
/// }
/// struct Objects {
///     roots: Vec<u64>,
/// }
/// impl TreeSource for Objects {
///     fn children(&self, parent: Option<u64>) -> &[u64] {
///         if parent.is_none() { &self.roots } else { &[] }
///     }
/// }
/// struct Outline {
///     objects: Objects,
///     tree: TreeState,
/// }
/// impl Application for Outline {
///     type Message = Msg;
///     fn view(&self, _: &ViewContext) -> Element {
///         TreeView::new("outline", &self.tree, Msg::Tree, |item| {
///             let name = format!("Objet {}", item.key);
///             label(item.scope.child("name"), name, &EditorPalette::DARK)
///         })
///         .element()
///     }
///     fn update(&mut self, message: Msg) -> bool {
///         let Msg::Tree(message) = message;
///         self.tree.update(message, &self.objects).rebuild
///     }
/// }
/// ```
pub struct TreeView<M> {
    id: ElementId,
    rows: Rc<Vec<TreeRow>>,
    selected: Rc<HashSet<u64>>,
    cursor: Option<u64>,
    reveal: Option<usize>,
    dragging: bool,
    drop: Option<(usize, DropPlace)>,
    row_height: f32,
    focused: bool,
    palette: EditorPalette,
    map: Rc<dyn Fn(TreeMsg) -> M>,
    content: Rc<dyn Fn(&TreeItem) -> Element>,
}

impl<M: 'static> TreeView<M> {
    /// The view of `state`: `map` wraps its gestures in the application's message,
    /// `content` builds what follows the arrow on each visible row (an icon, a name).
    pub fn new(
        id: impl Into<ElementId>,
        state: &TreeState,
        map: impl Fn(TreeMsg) -> M + 'static,
        content: impl Fn(&TreeItem) -> Element + 'static,
    ) -> Self {
        let reveal = state.reveal.and_then(|key| state.index_of(key));
        Self {
            id: id.into(),
            rows: Rc::clone(&state.rows),
            selected: Rc::clone(&state.selected),
            cursor: state.cursor,
            reveal,
            dragging: state.drag.is_some(),
            drop: state.drag.and_then(|drag| drag.target),
            row_height: state.row_height,
            focused: false,
            palette: EditorPalette::DARK,
            map: Rc::new(map),
            content: Rc::new(content),
        }
    }

    /// Colours of the rows ([`EditorPalette::DARK`] by default).
    pub fn palette(mut self, palette: EditorPalette) -> Self {
        self.palette = palette;
        self
    }

    /// Show the cursor row outlined: the tree is the panel the keys go to.
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// The element: a virtualized list filling its parent's height.
    pub fn element(self) -> Element {
        let Self {
            id,
            rows,
            selected,
            cursor,
            reveal,
            dragging,
            drop,
            row_height,
            focused,
            palette,
            map,
            content,
        } = self;
        let count = rows.len();
        let scroll_map = Rc::clone(&map);
        let build = move |index: usize| {
            let row = rows[index];
            let item = TreeItem {
                key: row.key,
                depth: row.depth,
                selected: selected.contains(&row.key),
                has_children: row.has_children,
                expanded: row.expanded,
                scope: id.item(row.key),
            };
            let inset = f32::from(row.depth) * INDENT + INDENT + 4.;
            let painter = RowPainter {
                item,
                inset,
                cursor: focused && cursor == Some(row.key),
                drop: drop.filter(|d| d.0 == index).map(|d| d.1),
                palette,
            };
            let map = Rc::clone(&map);
            let key = row.key;
            Element::row(item.scope.0, vec![content(&item)])
                .insets(Insets {
                    left: inset,
                    right: RIGHT,
                    ..Insets::default()
                })
                .align(Align::Center)
                .painter(painter)
                .on_pointer(move |event: PointerEvent| {
                    wanted(event, dragging).then(|| map(TreeMsg::Pointer { key, event }))
                })
        };
        let mut list = VirtualList::new(count, row_height, build).overscan(2);
        if let Some(index) = reveal {
            list = list.reveal(index);
        }
        Element::virtual_list(id.0, list)
            .on_scroll(move |metrics| scroll_map(TreeMsg::Scrolled(metrics.viewport_size())))
            .surface(palette.panel, 0.)
    }
}

/// Événements que les lignes prennent : les appuis des boutons gauche et droit, puis,
/// pendant un glisser, les mouvements et le relâchement.
fn wanted(event: PointerEvent, dragging: bool) -> bool {
    match event.phase {
        PointerPhase::Down => matches!(
            event.button,
            Some(PointerButton::Primary | PointerButton::Secondary)
        ),
        PointerPhase::Move | PointerPhase::Up => dragging,
        _ => false,
    }
}

/// Marge à droite du contenu d'une ligne.
const RIGHT: f32 = 4.;

/// Fond d'une ligne : sélection, flèche, curseur et marque de dépôt.
struct RowPainter {
    item: TreeItem,
    /// Marge à gauche du contenu : le peintre reçoit la boîte du contenu, la ligne
    /// commence `inset` plus à gauche.
    inset: f32,
    cursor: bool,
    drop: Option<DropPlace>,
    palette: EditorPalette,
}

impl Widget for RowPainter {
    fn measure(&self, _: SizeF, _: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
        Ok(SizeF::default())
    }

    fn paint(&self, context: &PaintContext, scene: &mut Scene) {
        let content = context.rect;
        let r = RectF {
            left: content.left - self.inset,
            right: content.right + RIGHT,
            ..content
        };
        let p = &self.palette;
        if self.item.selected {
            scene.fill(r, 0., p.selection);
        } else if context.state.hovered {
            scene.fill(r, 0., p.hover);
        }
        if self.item.has_children {
            let x = r.left + f32::from(self.item.depth) * INDENT + 4.;
            let y = (r.top + r.bottom) / 2.;
            let points = if self.item.expanded {
                vec![
                    pt(x - 0.5, y - 2.5),
                    pt(x + 7.5, y - 2.5),
                    pt(x + 3.5, y + 2.5),
                ]
            } else {
                vec![pt(x + 1., y - 4.), pt(x + 6., y), pt(x + 1., y + 4.)]
            };
            scene.commands.push(DrawCommand::Path {
                points,
                closed: true,
                color: p.muted,
            });
        }
        if self.cursor {
            let inner = RectF {
                left: r.left + 0.5,
                top: r.top + 0.5,
                right: r.right - 0.5,
                bottom: r.bottom - 0.5,
            };
            scene.commands.push(DrawCommand::Stroke {
                rect: inner,
                radius: 0.,
                width: 1.,
                color: p.accent,
            });
        }
        let line = |y: f32| DrawCommand::Line {
            start: pt(r.left + f32::from(self.item.depth) * INDENT, y),
            end: pt(r.right, y),
            width: 2.,
            color: p.accent,
        };
        match self.drop {
            Some(DropPlace::Before) => scene.commands.push(line(r.top + 1.)),
            Some(DropPlace::After) => scene.commands.push(line(r.bottom - 1.)),
            Some(DropPlace::Into) => scene.commands.push(DrawCommand::Stroke {
                rect: r,
                radius: 2.,
                width: 2.,
                color: p.accent,
            }),
            None => {}
        }
    }
}

fn pt(x: f32, y: f32) -> PointF {
    PointF { x, y }
}
