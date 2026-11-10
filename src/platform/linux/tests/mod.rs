//! Suite minimale du backend Linux (services hors écran), sur le modèle de
//! `platform/macos/tests/`. Les boucles de fenêtres sont sondées par `shell/probe.rs`
//! (`wayland/tests.rs`, `x11/tests.rs`).
mod desktop_settings;
mod images;
mod offscreen;
