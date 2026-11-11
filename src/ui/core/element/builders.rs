use super::*;
impl Element {
    pub(super) fn new(
        id: u64,
        layout: Layout,
        children: Vec<Self>,
        widget: Option<Box<dyn Widget>>,
    ) -> Self {
        Self {
            rules: None,
            styles: None,
            gradient: None,
            backgrounds: None,
            motion: None,
            id: ElementId(id),
            layout,
            style: Style::default(),
            children,
            widget,
            rect: RectF::new(0., 0., 0., 0.),
            clip: RectF::new(0., 0., 0., 0.),
            intrinsic: SizeF::default(),
            divider: None,
            scroll: None,
            scroll_offset: 0.,
            virtual_list: None,
            context_menu: None,
            on_scroll: None,
            on_wheel: None,
            listeners: None,
            memo: None,
            layout_cache: None,
            layout_bits: 0,
            content_skipped: false,
            skipped_shift: (0., 0.),
            tooltip: None,
            surface_style: None,
            drag_source: None,
            drop_target: None,
            skip_tab: false,
            semantics_hidden: false,
            drag_region: None,
            resize_region: None,
            expanded: None,
            aria: Default::default(),
        }
    }
    /// Declare whether the popup this element opens is shown, like HTML `aria-expanded`;
    /// UI Automation exposes it with the ExpandCollapse pattern, whose Expand/Collapse
    /// requests activate the element when the state differs. `None` removes it.
    pub fn expanded(mut self, expanded: Option<bool>) -> Self {
        self.expanded = expanded;
        self
    }
    /// Hide (`true`) this element and its subtree from assistive technologies, like HTML
    /// `aria-hidden="true"`; painting, clicks and keyboard focus are unchanged.
    pub fn accessibility_hidden(mut self, hidden: bool) -> Self {
        self.semantics_hidden = hidden;
        self
    }
    /// Let the user move the window by dragging this element (`true`), like Tauri's
    /// `data-tauri-drag-region`: pressing anywhere in its subtree that is not a button,
    /// field, slider or other interactive target starts a native move, and a double
    /// click maximizes or restores the window. Used by custom title bars.
    pub fn drag_region(mut self, enabled: bool) -> Self {
        if enabled {
            self.drag_region = Some(true);
        } else if self.drag_region == Some(true) {
            self.drag_region = None;
        }
        self
    }
    /// Exclude this element and its subtree from an enclosing
    /// [`drag_region`](Self::drag_region) (`true`), like `data-no-drag` in templates or
    /// `-webkit-app-region: no-drag`: pressing there does not move the window, even on
    /// a non-interactive area. A drag region nested inside it moves the window again.
    pub fn no_drag(mut self, enabled: bool) -> Self {
        if enabled {
            self.drag_region = Some(false);
        } else if self.drag_region == Some(false) {
            self.drag_region = None;
        }
        self
    }
    /// Let the user resize the window from this element, as from the window's `edge`
    /// (`data-resize-region="bottom-right"` in templates): pressing anywhere in its
    /// subtree that is not an interactive target starts a resize gesture, even when the
    /// window is not [`Resizable`](crate::window::WindowChange::Resizable). The window
    /// follows the pointer within its
    /// [`MinSize`](crate::window::WindowChange::MinSize),
    /// [`MaxSize`](crate::window::WindowChange::MaxSize) and
    /// [`AspectRatio`](crate::window::WindowChange::AspectRatio) (a corner follows the
    /// larger of the two pointer moves); it takes precedence over an enclosing
    /// [`drag_region`](Self::drag_region). Used for the corner grip of a frameless
    /// window; give it its own cursor (`cursor: nwse-resize`).
    pub fn resize_region(mut self, edge: crate::window::ResizeEdge) -> Self {
        self.resize_region = Some(edge);
        self
    }
    /// Keep (`true`, the default) or remove (`false`) this element from keyboard
    /// navigation with Tab, like HTML `tabindex="-1"`.
    pub fn tab_stop(mut self, enabled: bool) -> Self {
        self.skip_tab = !enabled;
        self
    }

    /// Horizontal container with a raw numeric `id`; see [`crate::ui::view::row`] for the typed form.
    pub fn row(id: u64, children: Vec<Self>) -> Self {
        Self::new(id, Layout::Row, children, None)
    }

    /// Vertical container with a raw numeric `id`; see [`crate::ui::view::column`].
    pub fn column(id: u64, children: Vec<Self>) -> Self {
        Self::new(id, Layout::Column, children, None)
    }

    /// Root layer stack: `page` first, then dialogs and popovers drawn above it.
    pub fn layers(id: u64, page: Self, overlays: Vec<Self>) -> Self {
        let mut children = vec![page];
        children.extend(overlays);
        Self::new(id, Layout::Layers, children, None).height(Length::Fill(1.))
    }

    /// Modal dialog wrapping `content` (480 px wide, 24 px padding, 14 px radius by default).
    pub fn modal(id: u64, options: crate::ui::modal::Modal, content: Self) -> Self {
        let mut node = Self::new(id, Layout::Column, vec![content], Some(Box::new(options)))
            .width(Length::Px(480.))
            .padding(24.);
        node.style.radius = 14.;
        node
    }

    /// Popover wrapping `content` (280 px wide, 8 px padding, 10 px radius by default).
    pub fn popover(id: u64, options: crate::ui::popover::Popover, content: Self) -> Self {
        let mut node = Self::new(id, Layout::Column, vec![content], Some(Box::new(options)))
            .width(Length::Px(280.))
            .padding(8.);
        node.style.radius = 10.;
        node
    }

    /// Ask the application to open a context menu (right click, Shift+F10 or the
    /// context-menu key): `handler` turns the request into a message.
    pub fn context_menu<M: 'static>(
        mut self,
        handler: impl Fn(crate::ui::popover::ContextMenuRequest) -> M + 'static,
    ) -> Self {
        self.context_menu = Some(Box::new(Handler::new(handler)));
        self
    }

    /// Report every scroll of this container (`overflow: auto | scroll`): `handler`
    /// receives the new geometry and turns it into a message, like the DOM `scroll`
    /// event. Typical use: load the next page when the offset nears `max_offset()`.
    pub fn on_scroll<M: 'static>(
        mut self,
        handler: impl Fn(crate::ui::scroll::ScrollMetrics) -> M + 'static,
    ) -> Self {
        self.on_scroll = Some(Box::new(Handler::new(handler)));
        self
    }

    /// Drag source, carried by a button or a widget that has an action.
    pub fn draggable(mut self, source: crate::ui::drag_drop::DragSource) -> Self {
        self.drag_source = Some(Box::new(source));
        self
    }

    /// Accept drops described by `target` on this element.
    pub fn drop_target(mut self, target: crate::ui::drag_drop::DropTarget) -> Self {
        self.drop_target = Some(Box::new(target));
        self
    }

    /// Show a plain-text tooltip when the element is hovered or focused.
    pub fn tooltip(self, text: impl Into<std::sync::Arc<str>>) -> Self {
        self.tooltip_with(crate::ui::tooltip::Tooltip::new(text))
    }

    /// Attach a fully configured [`Tooltip`](crate::ui::tooltip::Tooltip).
    pub fn tooltip_with(mut self, tooltip: crate::ui::tooltip::Tooltip) -> Self {
        self.tooltip = Some(Box::new(tooltip));
        self
    }

    /// Apply a complete recipe; methods called afterwards can refine it.
    pub fn styled(
        mut self,
        style: impl Into<std::sync::Arc<crate::ui::style::SurfaceStyle>>,
    ) -> Self {
        let style = style.into();
        self.style.padding = style.padding;
        self.style.gap = style.gap;
        self.style.row_gap = None;
        self.style.column_gap = None;
        self.style.radius = style.radius;
        self.style.background = None;
        self.surface_style = Some(style);
        self
    }

    /// Grid container with `columns` equal columns.
    pub fn grid(id: u64, columns: usize, children: Vec<Self>) -> Self {
        Self::new(id, Layout::Grid { columns }, children, None)
    }

    /// Two-pane container; see [`Split`].
    pub fn split(id: u64, options: Split, first: Self, second: Self) -> Self {
        Self::new(id, Layout::Split(options), vec![first, second], None)
    }

    /// Leaf element driven by a custom [`Widget`].
    pub fn custom(id: u64, widget: impl Widget + 'static) -> Self {
        Self::new(id, Layout::Leaf, vec![], Some(Box::new(widget)))
    }

    /// Auto content keeps its natural size; Fill content uses the size of the viewport.
    pub fn scroll(id: u64, axis: Axis, content: Self) -> Self {
        Self::new(id, Layout::Scroll(axis), vec![content], None).height(Length::Fill(1.))
    }

    /// Scroll metrics of a scroll container after layout, or `None` for other elements.
    pub fn scroll_metrics(&self) -> Option<crate::ui::scroll::ScrollMetrics> {
        self.scroll
    }

    /// Virtualised vertical list whose rows are built on demand.
    pub fn virtual_list(id: u64, list: crate::ui::virtual_list::VirtualList) -> Self {
        let mut node = Self::new(id, Layout::VirtualList, vec![], None).height(Length::Fill(1.));
        node.virtual_list = Some(Box::new(list));
        node
    }

    /// Metrics of a virtual list after layout, or `None` for other elements.
    pub fn virtual_metrics(&self) -> Option<crate::ui::virtual_list::VirtualListMetrics> {
        self.virtual_list.as_ref().map(|list| list.metrics())
    }

    /// Preferred width (pixels, percentage, fraction or computed length).
    pub fn width(mut self, value: impl Into<Length>) -> Self {
        self.style.width = value.into();
        self
    }

    /// Preferred height (pixels, percentage, fraction or computed length).
    pub fn height(mut self, value: impl Into<Length>) -> Self {
        self.style.height = value.into();
        self
    }

    /// Minimum width, in pixels.
    pub fn min_width(mut self, value: f32) -> Self {
        self.style.min_width = value;
        self
    }

    /// Minimum height, in pixels.
    pub fn min_height(mut self, value: f32) -> Self {
        self.style.min_height = value;
        self
    }

    /// Maximum width, in pixels.
    pub fn max_width(mut self, value: f32) -> Self {
        self.style.max_width = value;
        self
    }

    /// Maximum height, in pixels.
    pub fn max_height(mut self, value: f32) -> Self {
        self.style.max_height = value;
        self
    }

    /// Inner spacing; accepts a number, `[vertical, horizontal]`, `[top, horizontal, bottom]` or `[top, right, bottom, left]`.
    pub fn padding(mut self, value: impl Into<Insets>) -> Self {
        self.style.padding = value.into();
        self
    }

    /// Inner spacing from explicit [`Insets`].
    pub fn insets(mut self, value: Insets) -> Self {
        self.style.padding = value;
        self
    }

    /// Spacing between children on both axes; clears `row_gap` and `column_gap`.
    pub fn gap(mut self, value: f32) -> Self {
        self.style.gap = value;
        self.style.row_gap = None;
        self.style.column_gap = None;
        self
    }
    /// Outer spacing; also clears any `auto` margins.
    pub fn margin(mut self, value: impl Into<Insets>) -> Self {
        self.style.margin = value.into();
        if let Some(rules) = &mut self.rules {
            rules.margin_auto = Default::default();
        }
        self
    }
    /// Vertical spacing between rows.
    pub fn row_gap(mut self, value: f32) -> Self {
        self.style.row_gap = Some(value);
        self
    }
    /// Horizontal spacing between columns.
    pub fn column_gap(mut self, value: f32) -> Self {
        self.style.column_gap = Some(value);
        self
    }

    /// Cross-axis alignment of children.
    pub fn align(mut self, value: Align) -> Self {
        self.style.align = value;
        self
    }

    /// Main-axis distribution of children.
    pub fn justify(mut self, value: Justify) -> Self {
        self.style.justify = value;
        self
    }

    /// Solid background with a corner radius; replaces any gradient.
    pub fn surface(mut self, color: Color, radius: f32) -> Self {
        self.gradient = None;
        self.style.background = Some(color);
        self.style.radius = radius;
        self
    }

    /// Corner radius, in pixels.
    pub fn radius(mut self, radius: f32) -> Self {
        self.style.radius = radius;
        self
    }

    /// Number of grid columns spanned by this element.
    pub fn span(mut self, columns: usize) -> Self {
        self.style.span = columns;
        self
    }

    /// Explicit grid position; row and column are counted from 1.
    pub fn at(mut self, row: usize, column: usize) -> Self {
        self.style.row = Some(row);
        self.style.column = Some(column);
        self
    }

    /// Explicit grid column, counted from 1.
    pub fn column_start(mut self, column: usize) -> Self {
        self.style.column = Some(column);
        self
    }
}
