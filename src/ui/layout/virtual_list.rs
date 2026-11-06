//! Vertical collection: logical data kept separate from materialised rows.
use crate::application::Action;
use crate::ui::{element::*, layout, scene::RectF, scroll};
use std::{collections::BTreeMap, ops::Range};

const MAX_REALIZED: usize = 2048;
const MAX_EXTENT: f64 = 16_777_216.;

/// Vertical list of `count` fixed-height rows that only builds the rows near the viewport.
///
/// Attach it with [`Element::virtual_list`](crate::ui::element::Element::virtual_list). Rows
/// must be at least 1 logical pixel high, overscan is limited to 64 rows and the total height
/// to 16,777,216 logical pixels.
pub struct VirtualList {
    count: usize,
    row_height: f32,
    overscan: usize,
    build: Box<dyn Fn(usize) -> Element>,
    pub(crate) indices: Vec<usize>,
    pub(crate) active: Option<usize>,
    pub(crate) pinned: Option<usize>,
    range: Range<usize>,
    last_pin: Option<usize>,
    builds: u64,
}
/// Snapshot of a [`VirtualList`]'s state, for diagnostics and tests.
#[derive(Debug, Clone)]
pub struct VirtualListMetrics {
    /// Number of logical items.
    pub item_count: usize,
    /// Index range built around the actually visible part (overscan included).
    pub realized_range: Range<usize>,
    /// Number of rows currently built; includes at most one active row kept outside the
    /// viewport for keyboard navigation.
    pub realized_rows: usize,
    /// Index of the keyboard-active row, if any.
    pub active_index: Option<usize>,
    /// Calls to the row builder since this view was created.
    pub builds: u64,
}
impl VirtualList {
    /// Creates a list of `count` rows of `row_height` logical pixels; `build(index)` creates a row.
    ///
    /// `build` must be deterministic and side-effect free, with stable and unique IDs.
    pub fn new(count: usize, row_height: f32, build: impl Fn(usize) -> Element + 'static) -> Self {
        Self {
            count,
            row_height,
            overscan: 2,
            build: Box::new(build),
            indices: Vec::new(),
            active: (count > 0).then_some(0),
            pinned: None,
            range: 0..0,
            last_pin: None,
            builds: 0,
        }
    }
    /// Number of extra rows built above and below the viewport (default 2, at most 64).
    pub fn overscan(mut self, rows: usize) -> Self {
        self.overscan = rows;
        self
    }
    /// Returns a snapshot of the list's current state.
    pub fn metrics(&self) -> VirtualListMetrics {
        VirtualListMetrics {
            item_count: self.count,
            realized_range: self.range.clone(),
            realized_rows: self.indices.len(),
            active_index: self.active,
            builds: self.builds,
        }
    }
    pub(crate) fn stride(&self, gap: f32) -> f32 {
        self.row_height + gap
    }
    pub(crate) fn last_index(&self) -> Option<usize> {
        self.count.checked_sub(1)
    }
    pub(crate) fn row_height(&self) -> f32 {
        self.row_height
    }
    pub(crate) fn extent(&self, gap: f32) -> f32 {
        if self.count == 0 {
            0.
        } else {
            (self.count as f64 * self.row_height as f64
                + self.count.saturating_sub(1) as f64 * gap as f64) as f32
        }
    }
    pub(crate) fn validate(&self, gap: f32) -> crate::Result<()> {
        let extent = self.count as f64 * self.row_height as f64
            + self.count.saturating_sub(1) as f64 * gap as f64;
        if !self.row_height.is_finite()
            || self.row_height < 1.
            || self.overscan > 64
            || !extent.is_finite()
            || extent > MAX_EXTENT
        {
            return Err(invalid(
                "invalid virtual list row height, overscan or extent",
            ));
        }
        Ok(())
    }
}
fn invalid(message: &'static str) -> crate::Error {
    crate::Error::invalid_input(message)
}

pub(crate) fn arrange(node: &mut Element, text: &mut dyn TextMeasurer) -> crate::Result<()> {
    let list = node
        .virtual_list
        .as_ref()
        .ok_or_else(|| invalid("virtual list without an item builder"))?;
    list.validate(node.style.vertical_gap())?;
    let s = scroll::metrics(
        layout::content_rect(node),
        Axis::Vertical,
        list.extent(node.style.vertical_gap()),
        node.scroll_offset,
        node.rules
            .as_ref()
            .map_or(scroll::BAR_SIZE, |r| r.scrollbar.thickness()),
    );
    node.scroll = Some(s);
    node.scroll_offset = s.offset;
    refresh(node, text, true)?;
    Ok(())
}

fn range(list: &VirtualList, viewport: RectF, clip: RectF, offset: f32, gap: f32) -> Range<usize> {
    let visible = layout::intersect(viewport, clip);
    if layout::width(visible) <= 0. || layout::height(visible) <= 0. || list.count == 0 {
        return 0..0;
    }
    let start = ((offset + visible.top - viewport.top).max(0.) / list.stride(gap)).floor() as usize;
    let end = ((offset + visible.bottom - viewport.top).max(0.) / list.stride(gap)).ceil() as usize;
    start.saturating_sub(list.overscan).min(list.count)
        ..end.saturating_add(list.overscan).min(list.count)
}
fn validate_row(node: &Element) -> crate::Result<()> {
    fn fixed(node: &Element) -> bool {
        layout::fixed(node) || node.children.iter().any(fixed)
    }
    if fixed(node) {
        return Err(invalid(
            "position: fixed elements must be placed outside virtual list rows",
        ));
    }
    layout::validate(node)?;
    fn has_virtual(node: &Element) -> bool {
        matches!(node.layout, Layout::VirtualList) || node.children.iter().any(has_virtual)
    }
    if has_virtual(node) {
        return Err(invalid("nested virtual lists are not supported yet"));
    }
    fn has_input(node: &Element) -> bool {
        node.widget
            .as_ref()
            .is_some_and(|w| w.text_input().is_some())
            || node.children.iter().any(has_input)
    }
    if has_input(node) {
        return Err(invalid(
            "text inputs inside virtual list rows are not supported yet",
        ));
    }
    Ok(())
}
fn refresh(node: &mut Element, text: &mut dyn TextMeasurer, force: bool) -> crate::Result<bool> {
    let s = node.scroll.unwrap();
    let context = node
        .rules
        .as_ref()
        .map_or(Default::default(), |r| r.unit_context);
    let inherited = node
        .styles
        .as_ref()
        .map_or(Default::default(), |s| s.computed.text);
    let list = node.virtual_list.as_mut().unwrap();
    let range = range(
        list,
        s.viewport,
        node.clip,
        s.offset,
        node.style.vertical_gap(),
    );
    if !force && range == list.range && list.pinned == list.last_pin {
        return Ok(false);
    }
    let extra = usize::from(
        list.pinned
            .is_some_and(|pin| pin < list.count && !range.contains(&pin)),
    );
    if range.len().saturating_add(extra) > MAX_REALIZED {
        return Err(invalid(
            "virtual list viewport too large: more than 2048 realized rows",
        ));
    }
    let mut indices: Vec<_> = range.clone().collect();
    if let Some(pin) = list.pinned.filter(|&i| i < list.count)
        && !range.contains(&pin)
    {
        indices.push(pin);
        indices.sort_unstable();
    }
    let mut old: BTreeMap<_, _> = std::mem::take(&mut list.indices)
        .into_iter()
        .zip(std::mem::take(&mut node.children))
        .collect();
    // Libérer les lignes sorties AVANT de construire les nouvelles : pas d'accumulation.
    old.retain(|index, _| indices.binary_search(index).is_ok());
    let clip = layout::intersect(s.viewport, node.clip);
    let mut children = Vec::with_capacity(indices.len());
    for &index in &indices {
        let retained = old.remove(&index);
        let new = retained.is_none();
        let mut row = retained.unwrap_or_else(|| {
            list.builds += 1;
            (list.build)(index)
        });
        if new {
            validate_row(&row)?;
        }
        if new || force {
            crate::ui::styling::resolve::resolve_inherited(
                &mut row,
                inherited,
                crate::ui::styling::StyleEnvironment {
                    viewport: SizeF {
                        width: context.viewport_width,
                        height: context.viewport_height,
                    },
                    root_font: Some(context.root_font),
                    pixel_scale: context.pixel_scale,
                    ..Default::default()
                },
            )?;
            layout::arrange(
                &mut row,
                RectF::new(
                    s.viewport.left,
                    s.viewport.top + index as f32 * list.stride(node.style.vertical_gap())
                        - s.offset,
                    layout::width(s.viewport),
                    list.row_height,
                ),
                clip,
                text,
            )?;
        }
        children.push(row);
    }
    node.children = children;
    list.indices = indices;
    list.range = range;
    list.last_pin = list.pinned;
    layout::update_paint_order(node);
    Ok(true)
}

/// Après translation d'un parent, recalculer aussi les listes devenues visibles/invisibles.
pub(crate) fn refresh_all(
    node: &mut Element,
    focused: Option<ElementId>,
    text: &mut dyn TextMeasurer,
) -> crate::Result<bool> {
    let mut changed = false;
    if let Some(list) = &mut node.virtual_list {
        list.pinned = if focused == Some(node.id) {
            list.active
        } else {
            None
        };
        changed = refresh(node, text, false)?;
    }
    for child in &mut node.children {
        changed |= refresh_all(child, focused, text)?;
    }
    Ok(changed)
}
pub(crate) fn containing_row(root: &Element, target: ElementId) -> Option<(ElementId, usize)> {
    if let Some(list) = &root.virtual_list {
        return root
            .children
            .iter()
            .zip(&list.indices)
            .find_map(|(row, &index)| row.find(target).map(|_| (root.id, index)));
    }
    root.children
        .iter()
        .find_map(|child| containing_row(child, target))
}
pub(crate) fn first_action(node: &Element) -> Option<(ElementId, Action)> {
    node.widget
        .as_ref()
        .and_then(|w| w.action())
        .map(|action| (node.id, action))
        .or_else(|| node.children.iter().find_map(first_action))
}
