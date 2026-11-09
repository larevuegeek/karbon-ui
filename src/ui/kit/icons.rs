//! Built-in icons, drawn on a 24 × 24 grid with a stroke of 2.
use super::css;
use super::space::Space;
use crate::ui::{
    element::Element,
    template::{Component, Slots, TemplateContext},
};

/// Built-in icon: `<Icon name="plus" size={20} />`, drawn with the current text colour.
#[derive(Default)]
pub struct Icon {
    /// Icon name, see [`ICON_NAMES`].
    pub name: &'static str,
    /// Side in logical pixels, 16 by default.
    pub size: Space,
}

macro_rules! icons {
    ($($key:literal => $d:literal,)*) => {
        /// Names accepted by [`Icon`].
        pub const ICON_NAMES: &[&str] = &[$($key),*];
        fn draw<R>(cx: &TemplateContext<'_, R>, name: &str, size: f32) -> Element {
            match name {
                $($key => kit_ui!(cx;
                    <svg class="k-icon" style:width={size} style:height={size} viewBox="0 0 24 24">
                        <g fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                            <path d=$d />
                        </g>
                    </svg>
                ),)*
                other => {
                    debug_assert!(false, "karbon-ui: unknown icon `{other}`, see ICON_NAMES");
                    kit_ui!(cx; <div class="k-icon" style:width={size} style:height={size}></div>)
                }
            }
        }
    };
}
icons!(
    "plus" => "M12 5v14M5 12h14",
    "loader" => "M12 3a9 9 0 1 0 9 9",
    "minus" => "M5 12h14",
    "check" => "M5 13l4 4L19 7",
    "x" => "M6 6l12 12M18 6L6 18",
    "search" => "M4 11a7 7 0 1 0 14 0a7 7 0 1 0-14 0M20 20l-4-4",
    "chevron-left" => "M15 6l-6 6 6 6",
    "chevron-right" => "M9 6l6 6-6 6",
    "chevron-up" => "M6 15l6-6 6 6",
    "chevron-down" => "M6 9l6 6 6-6",
    "arrow-left" => "M19 12H5M11 6l-6 6 6 6",
    "arrow-right" => "M5 12h14M13 6l6 6-6 6",
    "arrow-up" => "M12 19V5M6 11l6-6 6 6",
    "arrow-down" => "M12 5v14M6 13l6 6 6-6",
    "menu" => "M4 6h16M4 12h16M4 18h16",
    "more-horizontal" => "M5 12h.01M12 12h.01M19 12h.01",
    "more-vertical" => "M12 5v.01M12 12v.01M12 19v.01",
    "settings" => "M4 7h9M17 7h3M4 17h3M11 17h9M13 7a2 2 0 1 0 4 0a2 2 0 1 0-4 0M7 17a2 2 0 1 0 4 0a2 2 0 1 0-4 0",
    "user" => "M8 8a4 4 0 1 0 8 0a4 4 0 1 0-8 0M4 21c0-4 4-6 8-6s8 2 8 6",
    "users" => "M5 8a4 4 0 1 0 8 0a4 4 0 1 0-8 0M2 21c0-4 3-6 7-6s7 2 7 6M16 4.5a4 4 0 0 1 0 7M18 15.5c2.500 .7 4 2.700 4 5.500",
    "home" => "M3 11l9-8 9 8M5 9.500V21h14V9.500M10 21v-6h4v6",
    "heart" => "M12 20s-8-5-8-11a4.500 4.500 0 0 1 8-2.800A4.500 4.500 0 0 1 20 9c0 6-8 11-8 11z",
    "star" => "M12 3l2.700 5.800 6.300 .8-4.600 4.300 1.200 6.300L12 17.100l-5.600 3.100 1.200-6.300L3 9.600l6.300-.8z",
    "trash" => "M4 7h16M9 7V4h6v3M6 7l1 14h10l1-14M10 11v6M14 11v6",
    "edit" => "M4 20l1-4L16 5l3 3L8 19zM14 7l3 3",
    "copy" => "M9 9h11v11H9zM5 15H4V4h11v1",
    "download" => "M12 4v12M7 11l5 5 5-5M4 20h16",
    "upload" => "M12 16V4M7 9l5-5 5 5M4 20h16",
    "mail" => "M3 6h18v12H3zM3 7l9 6 9-6",
    "bell" => "M6 17V11a6 6 0 1 1 12 0v6l2 2H4zM10 21h4",
    "info" => "M3 12a9 9 0 1 0 18 0a9 9 0 1 0-18 0M12 11v5M12 8v.01",
    "alert-triangle" => "M12 4L2 20h20zM12 10v5M12 17.500v.01",
    "alert-circle" => "M3 12a9 9 0 1 0 18 0a9 9 0 1 0-18 0M12 8v5M12 16v.01",
    "check-circle" => "M3 12a9 9 0 1 0 18 0a9 9 0 1 0-18 0M8 12l3 3 5-6",
    "x-circle" => "M3 12a9 9 0 1 0 18 0a9 9 0 1 0-18 0M9 9l6 6M15 9l-6 6",
    "calendar" => "M4 6h16v15H4zM4 10h16M8 3v5M16 3v5",
    "clock" => "M3 12a9 9 0 1 0 18 0a9 9 0 1 0-18 0M12 7v5l3 2",
    "folder" => "M3 6h6l2 3h10v10H3z",
    "file" => "M6 3h8l5 5v13H6zM14 3v5h5",
    "image" => "M3 5h18v14H3zM3 16l5-5 4 4 3-3 6 6M15 9h.01",
    "link" => "M10 14a4 4 0 0 0 6 0l3-3a4 4 0 0 0-6-6l-1 1M14 10a4 4 0 0 0-6 0l-3 3a4 4 0 0 0 6 6l1-1",
    "external-link" => "M14 4h6v6M20 4l-9 9M18 14v6H4V6h6",
    "eye" => "M2 12s4-7 10-7 10 7 10 7-4 7-10 7-10-7-10-7zM9 12a3 3 0 1 0 6 0a3 3 0 1 0-6 0",
    "eye-off" => "M4 4l16 16M10 5.300A10 10 0 0 1 12 5c6 0 10 7 10 7a18 18 0 0 1-3 3.800M6.500 6.600C3.700 8.400 2 12 2 12s4 7 10 7a10 10 0 0 0 4.500-1",
    "lock" => "M5 11h14v10H5zM8 11V7a4 4 0 0 1 8 0v4",
    "log-out" => "M15 4h5v16h-5M10 8l-4 4 4 4M6 12h10",
    "play" => "M7 4l13 8-13 8z",
    "pause" => "M8 5v14M16 5v14",
    "filter" => "M3 5h18l-7 8v6l-4 2v-8z",
    "refresh" => "M20 8A8 8 0 0 0 5 7M4 16a8 8 0 0 0 15 1M5 3v4h4M19 21v-4h-4",
    "sun" => "M8 12a4 4 0 1 0 8 0a4 4 0 1 0-8 0M12 2v2M12 20v2M2 12h2M20 12h2M5 5l1.500 1.500M17.500 17.500L19 19M5 19l1.500-1.500M17.500 6.500L19 5",
    "moon" => "M20 14A8 8 0 1 1 10 4a7 7 0 0 0 10 10z",
    "music" => "M9 18V6l11-2v12M5 18a2.500 2.500 0 1 0 5 0a2.500 2.500 0 1 0-5 0M15 16a2.500 2.500 0 1 0 5 0a2.500 2.500 0 1 0-5 0",
    "globe" => "M3 12a9 9 0 1 0 18 0a9 9 0 1 0-18 0M3 12h18M12 3c3 3 3 15 0 18M12 3c-3 3-3 15 0 18",
);

impl<R> Component<R> for Icon {
    fn render(self, cx: &TemplateContext<'_, R>, _: Slots) -> Element {
        draw(cx, self.name, self.size.or(16.))
    }
}
