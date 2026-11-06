//! Sonde de la boucle Wayland (`shell/probe.rs`) sur une vraie connexion au compositeur.
//! Sans compositeur (`WAYLAND_DISPLAY` absent : intégration continue), elle est sautée en
//! l'écrivant sur la sortie d'erreur ; `KARBON_REQUIRE_WAYLAND=1` en fait un échec.
#[test]
fn wayland_loop_paints_wakes_for_tasks_and_runs_window_commands() {
    crate::platform::linux::shell::probe::probe(
        super::run_windows,
        "Wayland",
        std::env::var_os("WAYLAND_DISPLAY").is_some(),
        "KARBON_REQUIRE_WAYLAND",
    );
}
