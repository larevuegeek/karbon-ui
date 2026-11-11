//! Layout: page, container, stacks, grid, spacer, divider.
use super::{css, space::Space};
use crate::ui::{
    element::Element,
    layout::{GridTrack, TrackBreadth},
    template::{Component, Slots, TemplateContext},
};

macro_rules! flex_box {
    ($(#[$doc:meta])* $name:ident, $row:expr, $gap:expr) => {
        $(#[$doc])*
        #[derive(Default)]
        pub struct $name {
            /// Distance between children.
            pub gap: Space,
            /// Inner margin.
            pub padding: Space,
            /// Centre children on the cross axis.
            pub center: bool,
            /// Align children to the end of the cross axis.
            pub end: bool,
            /// Stretch children on the cross axis.
            pub stretch: bool,
            /// Spread children along the main axis.
            pub between: bool,
            /// Centre children along the main axis.
            pub main_center: bool,
            /// Push children to the end of the main axis.
            pub main_end: bool,
            /// Let children wrap on several lines.
            pub wrap: bool,
            /// Take the whole available width.
            pub full: bool,
            /// Take the remaining space of the parent.
            pub grow: bool,
            /// Stay within the space given by the parent instead of growing with the children.
            pub clip: bool,
        }
        impl<R> Component<R> for $name {
            fn render(self, cx: &TemplateContext<'_, R>, mut slots: Slots) -> Element {
                let row: bool = $row;
                let gap = self.gap.or($gap);
                let padding = self.padding.or(0.);
                kit_ui!(cx, slots;
                    <div class="k-flex" class:k-row={row} class:k-center={self.center}
                        class:k-end={self.end} class:k-stretch={self.stretch}
                        class:k-between={self.between}
                        class:k-main-center={self.main_center} class:k-main-end={self.main_end}
                        class:k-wrap={self.wrap} class:k-full={self.full} class:k-grow={self.grow} class:k-clip={self.clip}
                        style:gap={gap} style:padding={padding}>
                        <slot />
                    </div>
                )
            }
        }
    };
}
flex_box!(
    /// Vertical pile of children.
    Stack, false, 12.
);
flex_box!(
    /// Horizontal row of children, centred on the cross axis.
    Row, true, 8.
);

/// Root of a page: background, text colour and scrolling of the theme.
#[derive(Default)]
pub struct Page {
    /// The page does not scroll: its content manages its own scrolling areas.
    pub fixed: bool,
    /// Inner margin.
    pub padding: Space,
    /// Distance between children.
    pub gap: Space,
}
impl<R> Component<R> for Page {
    fn render(self, cx: &TemplateContext<'_, R>, mut slots: Slots) -> Element {
        let (padding, gap) = (self.padding.or(0.), self.gap.or(0.));
        kit_ui!(cx, slots; <main class="k-page" class:k-page-fixed={self.fixed} style:padding={padding} style:gap={gap}><slot /></main>)
    }
}

/// Centred column of limited width.
#[derive(Default)]
pub struct Container {
    /// Largest width in pixels, 1120 by default.
    pub max: Option<f32>,
    /// Inner margin.
    pub padding: Space,
    /// Distance between children.
    pub gap: Space,
}
impl<R> Component<R> for Container {
    fn render(self, cx: &TemplateContext<'_, R>, mut slots: Slots) -> Element {
        let (padding, gap) = (self.padding.or(0.), self.gap.or(0.));
        let max = self.max.unwrap_or(1120.);
        kit_ui!(cx, slots;
            <div class="k-container" style:max-width={max} style:padding={padding} style:gap={gap}><slot /></div>
        )
    }
}

/// Grid of equal columns: `<Grid cols={3} gap="md">`, or as many columns of at least
/// `min` pixels as the width allows, `<Grid min={240.} gap="md">`: the grid adapts to a
/// narrow window, down to one column (`repeat(auto-fit, minmax(240px, 1fr))`).
#[derive(Default)]
pub struct Grid {
    /// Number of columns, 2 by default; ignored with `min`.
    pub cols: Option<f32>,
    /// Smallest column width: the number of columns follows the width of the grid.
    pub min: Option<f32>,
    /// Distance between cells.
    pub gap: Space,
}
impl<R> Component<R> for Grid {
    fn render(self, cx: &TemplateContext<'_, R>, mut slots: Slots) -> Element {
        let cols = self.cols.unwrap_or(2.).max(1.) as usize;
        let gap = self.gap.or(12.);
        let mut grid = kit_ui!(cx, slots;
            <div class="k-grid" style:columns={cols} style:gap={gap}><slot /></div>
        );
        if let Some(min) = self.min.filter(|m| m.is_finite() && *m > 0.) {
            let rules = grid.layout_rules();
            rules.columns = Some(std::sync::Arc::from([GridTrack {
                min: TrackBreadth::Px(min),
                max: TrackBreadth::Fr(1.),
            }]));
            rules.grid_auto_repeat = Some(true);
        }
        grid
    }
}

/// Pushes its neighbours apart in a [`Row`] or a [`Stack`].
#[derive(Default)]
pub struct Spacer;
impl<R> Component<R> for Spacer {
    fn render(self, cx: &TemplateContext<'_, R>, _: Slots) -> Element {
        kit_ui!(cx; <div class="k-spacer"></div>)
    }
}

/// Thin separating line; `vertical` for a separator inside a [`Row`].
#[derive(Default)]
pub struct Divider {
    /// Draw a vertical line.
    pub vertical: bool,
    /// Share the width of a [`Row`] with its neighbours.
    pub grow: bool,
}
impl<R> Component<R> for Divider {
    fn render(self, cx: &TemplateContext<'_, R>, _: Slots) -> Element {
        kit_ui!(cx;
            <div class:k-divider={!self.vertical} class:k-divider-grow={self.grow} class:k-divider-v={self.vertical}></div>
        )
    }
}
