//! Instant d'affichage des images, transcrit de
//! `stable/presentation-time/presentation-time.xml` (wayland-protocols).
use crate::platform::linux::ffi::wayland::{Interface, Message, interface, message};

pub(in crate::platform::linux) static WP_PRESENTATION: Interface = interface(
    c"wp_presentation",
    1,
    &WP_PRESENTATION_REQUESTS,
    &WP_PRESENTATION_EVENTS,
);
static WP_PRESENTATION_REQUESTS: [Message; 2] =
    [message(c"destroy", c""), message(c"feedback", c"on")];
static WP_PRESENTATION_EVENTS: [Message; 1] = [message(c"clock_id", c"u")];
/// Numéros de `wp_presentation` : requêtes, puis événements (`EV_`).
pub(in crate::platform::linux) mod wp_presentation {
    pub const DESTROY: u32 = 0;
    pub const FEEDBACK: u32 = 1;
    pub const EV_CLOCK_ID: u32 = 0;
}
pub(in crate::platform::linux) static WP_PRESENTATION_FEEDBACK: Interface = interface(
    c"wp_presentation_feedback",
    1,
    &WP_PRESENTATION_FEEDBACK_REQUESTS,
    &WP_PRESENTATION_FEEDBACK_EVENTS,
);
static WP_PRESENTATION_FEEDBACK_REQUESTS: [Message; 0] = [];
static WP_PRESENTATION_FEEDBACK_EVENTS: [Message; 3] = [
    message(c"sync_output", c"o"),
    message(c"presented", c"uuuuuuu"),
    message(c"discarded", c""),
];
/// Numéros de `wp_presentation_feedback` (événements seulement).
pub(in crate::platform::linux) mod wp_presentation_feedback {
    pub const EV_SYNC_OUTPUT: u32 = 0;
    pub const EV_PRESENTED: u32 = 1;
    pub const EV_DISCARDED: u32 = 2;
}
