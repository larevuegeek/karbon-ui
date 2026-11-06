//! Anchored temporary layers and menu actions, without native calls.
use crate::application::Action;
use crate::ui::{
    element::*,
    layout,
    scene::{DrawCommand, PointF, RectF, Scene},
    theme::Color,
};
use std::sync::Arc;

/// What a [`Popover`] is positioned against.
#[derive(Clone, Copy, Debug)]
pub enum Anchor {
    /// A visible, enabled element of the page (not inside the popover); the
    /// popover is dismissed if it disappears.
    Element(ElementId),
    /// A point in logical window coordinates, for example a context-menu position.
    Point(PointF),
}
/// Request to open a context menu, passed to the handler registered with
/// [`Element::context_menu`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContextMenuRequest {
    /// Element that received the request.
    pub target: ElementId,
    /// Suggested menu position in logical window coordinates (pointer position,
    /// or the bottom-left of the focused element when opened from the keyboard).
    pub position: PointF,
    /// Whether the request came from the keyboard (Shift+F10 or the menu key).
    pub keyboard: bool,
}
/// Behaviour of a [`Popover`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PopoverKind {
    /// Free-form panel with normal Tab navigation.
    Panel,
    /// Menu: only menu items are focusable, navigated with the arrow, Home and End keys.
    Menu,
    /// List box: only options ([`Role::ListItem`]) are focusable, navigated like a menu;
    /// choosing an option closes it.
    Listbox,
    /// Suggestions under a text field (combobox): a list box that is not modal. The
    /// focus stays in the field, which keeps receiving typed text and keys (its
    /// `on_key` drives the highlighted option); the page stays interactive. A click
    /// on an option chooses it and closes the list; Escape, Tab, a click outside the
    /// list and its anchor, the wheel outside the list or the window losing the focus
    /// close it. Its anchor is the text field.
    Combobox,
}
impl PopoverKind {
    /// Role of the focusable items of a menu or list box (`None` for a panel).
    pub fn item_role(self) -> Option<Role> {
        match self {
            Self::Panel => None,
            Self::Menu => Some(Role::MenuItem),
            Self::Listbox | Self::Combobox => Some(Role::ListItem),
        }
    }
    /// Roles allowed on the focusable content of a menu or list box.
    pub(crate) fn allows(self, role: Role) -> bool {
        match self {
            Self::Panel => true,
            Self::Menu => matches!(role, Role::Menu | Role::MenuItem),
            Self::Listbox | Self::Combobox => matches!(role, Role::List | Role::ListItem),
        }
    }
}
/// The popover node is a combobox list, which leaves the page interactive.
pub(crate) fn is_combobox(node: &Element) -> bool {
    config(node).is_some_and(|p| p.kind == PopoverKind::Combobox)
}
/// Side of the anchor a [`Popover`] opens on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Side {
    /// Above the anchor.
    Top,
    /// Right of the anchor.
    Right,
    /// Below the anchor (default).
    #[default]
    Bottom,
    /// Left of the anchor.
    Left,
}
/// Alignment of a [`Popover`] along the side of its anchor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SideAlign {
    /// Left (or top) edges aligned (default).
    #[default]
    Start,
    /// Centred on the anchor.
    Center,
    /// Right (or bottom) edges aligned.
    End,
}
/// Where a [`Popover`] is placed relative to its anchor. It flips to the opposite
/// side when the preferred one lacks room and the other has more, then shifts to stay
/// inside the window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PopoverPlacement {
    /// Preferred side.
    pub side: Side,
    /// Alignment along that side.
    pub align: SideAlign,
    /// Distance between the anchor and the popover, in logical pixels.
    pub offset: f32,
    /// Make the popover at least as wide (or tall, for left/right) as its anchor.
    pub match_anchor: bool,
    /// Shift along the anchor side after alignment, in logical pixels: to the right
    /// for `Top`/`Bottom`, downwards for `Left`/`Right` (negative values go the other
    /// way). A menu that overhangs its button by 16 px uses `End` and `16`.
    pub cross_offset: f32,
}
impl Default for PopoverPlacement {
    fn default() -> Self {
        Self {
            side: Side::Bottom,
            align: SideAlign::Start,
            offset: 4.,
            match_anchor: false,
            cross_offset: 0.,
        }
    }
}
/// Temporary layer anchored to an element or a point and shown above the page.
/// The application opens it by adding it to the view and closes it when
/// `on_dismiss` is received.
#[derive(Clone)]
pub struct Popover {
    /// Accessible name; must not be blank.
    pub label: Arc<str>,
    /// Position reference.
    pub anchor: Anchor,
    /// Message emitted when the popover must close.
    pub on_dismiss: Action,
    /// Element focused when the popover opens; must be inside it.
    pub initial_focus: Option<ElementId>,
    /// Panel or menu behaviour.
    pub kind: PopoverKind,
    /// Side, alignment and offset relative to the anchor.
    pub placement: PopoverPlacement,
}
impl Popover {
    /// Popover emitting a clone of `on_dismiss` when it must close: Escape, click
    /// outside, a chosen menu item, or an anchor that disappeared.
    pub fn new<M: Clone + 'static>(
        label: impl Into<Arc<str>>,
        anchor: Anchor,
        on_dismiss: M,
    ) -> Self {
        Self {
            label: label.into(),
            anchor,
            on_dismiss: Action::new(on_dismiss),
            initial_focus: None,
            kind: PopoverKind::Panel,
            placement: PopoverPlacement::default(),
        }
    }
    /// Give the popover list box behaviour (see [`PopoverKind::Listbox`]); its options
    /// are elements with the [`Role::ListItem`] role.
    pub fn listbox(mut self) -> Self {
        self.kind = PopoverKind::Listbox;
        self
    }
    /// Give the popover combobox behaviour (see [`PopoverKind::Combobox`]): a list of
    /// suggestions under its anchor, a text field that keeps the focus.
    pub fn combobox(mut self) -> Self {
        self.kind = PopoverKind::Combobox;
        self
    }
    /// Place the popover relative to its anchor.
    pub fn placement(mut self, placement: PopoverPlacement) -> Self {
        self.placement = placement;
        self
    }
    /// Give the popover menu behaviour (see [`PopoverKind::Menu`]); its content
    /// should be [`MenuItem`]s and [`MenuSeparator`]s.
    pub fn menu(mut self) -> Self {
        self.kind = PopoverKind::Menu;
        self
    }
    /// Focus the element `id` (inside the popover) when it opens.
    pub fn initial_focus(mut self, id: impl Into<ElementId>) -> Self {
        self.initial_focus = Some(id.into());
        self
    }
}
impl Widget for Popover {
    fn measure(&self, _: SizeF, _: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
        Ok(SizeF::default())
    }
    fn paint(&self, _: &PaintContext, _: &mut Scene) {}
    fn popover(&self) -> Option<&Popover> {
        Some(self)
    }
    fn focusable(&self) -> bool {
        // Une liste de suggestions laisse le focus dans son champ.
        self.kind != PopoverKind::Combobox
    }
    fn semantics(&self) -> Semantics<'_> {
        Semantics {
            role: match self.kind {
                PopoverKind::Menu => Role::Menu,
                PopoverKind::Listbox | PopoverKind::Combobox => Role::List,
                PopoverKind::Panel => Role::Popover,
            },
            name: &self.label,
            enabled: true,
        }
    }
}
pub(crate) fn config(node: &Element) -> Option<&Popover> {
    node.widget.as_ref()?.popover()
}
pub(crate) fn top(root: &Element) -> Option<&Element> {
    matches!(root.layout, Layout::Layers)
        .then(|| root.children.last())
        .flatten()
        .filter(|n| config(n).is_some())
}
pub(crate) fn anchor(root: &Element, popup: &Element) -> Option<RectF> {
    match config(popup)?.anchor {
        Anchor::Point(p) => Some(RectF::new(p.x, p.y, 0., 0.)),
        Anchor::Element(id) => {
            let node = root.children.iter().rev().nth(1)?.find(id)?;
            if node.widget.as_ref().is_some_and(|w| !w.semantics().enabled) {
                return None;
            }
            let rect = layout::intersect(node.rect, node.clip);
            (layout::width(rect) > 0. && layout::height(rect) > 0.).then_some(rect)
        }
    }
}

mod menu;
pub use menu::{MenuItem, MenuSeparator};
