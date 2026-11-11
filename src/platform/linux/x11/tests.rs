//! Sonde de la boucle X11 (`shell/probe.rs`) sur une vraie connexion au serveur X (XWayland
//! compris). Sans serveur (`DISPLAY` absent : intégration continue), elle est sautée en
//! l'écrivant sur la sortie d'erreur ; `KARBON_REQUIRE_X11=1` en fait un échec.
#[test]
fn x11_loop_paints_wakes_for_tasks_and_runs_window_commands() {
    crate::platform::linux::shell::probe::probe(
        super::run_windows,
        "X11",
        std::env::var_os("DISPLAY").is_some(),
        "KARBON_REQUIRE_X11",
    );
}
/// La disposition est lue du serveur (extension XKB), pas recompilée d'après ses noms :
/// sous XWayland, ces noms restent `us` quelle que soit la disposition du bureau.
#[test]
fn the_keyboard_comes_from_the_server_through_xkb() {
    if std::env::var_os("DISPLAY").is_none()
        || crate::platform::linux::ffi::xcb::xkb_x11().is_none()
    {
        eprintln!("karbon-ui: no X server or libxkbcommon-x11, test skipped");
        return;
    }
    let connection = super::connection::Connection::connect().expect("X server");
    let locale = crate::platform::linux::shell::keyboard::locale();
    let keyboard = connection.server_keyboard(&locale).expect("XKB keymap");
    // Touche à droite de Tab (AD01, code XKB 24) : une lettre dans toute disposition.
    let sym = keyboard.keysym(24);
    eprintln!("karbon-ui: AD01 = {sym:#x}");
    assert!(sym != 0);
}
#[test]
fn desktop_scale_comes_from_xft_dpi() {
    use super::connection::dpi_scale;
    assert_eq!(dpi_scale("Xft.antialias:\t1\nXft.dpi:\t192\n"), Some(2.));
    assert_eq!(dpi_scale("Xft.dpi: 144"), Some(1.5));
    assert_eq!(dpi_scale("Xft.dpi:\t72"), Some(1.));
    assert_eq!(dpi_scale("Xcursor.size: 24"), None);
}
/// Écrit `value` à `at` dans `bytes` (ordre de l'hôte, celui du serveur local).
fn put(bytes: &mut [u8], at: usize, value: &[u8]) {
    bytes[at..at + value.len()].copy_from_slice(value);
}
#[test]
fn xinput_motion_events_decode_positions_modifiers_and_scroll_valuators() {
    use super::xinput::device_event;
    // En-tête de 84 octets (`full_sequence` compris), un mot de boutons, un mot de masque
    // (axes 2 et 3), deux valeurs FP3232.
    let mut e = vec![0u8; 84 + 4 + 4 + 16];
    e[0] = 35;
    let words = ((e.len() - 36) / 4) as u32;
    put(&mut e, 4, &words.to_ne_bytes());
    put(&mut e, 8, &6u16.to_ne_bytes()); // XI_Motion
    put(&mut e, 10, &2u16.to_ne_bytes()); // pointeur maître
    put(&mut e, 12, &1234u32.to_ne_bytes());
    put(&mut e, 24, &0x2a_0000u32.to_ne_bytes()); // fenêtre
    put(&mut e, 44, &((10 << 16) | 0x8000u32).to_ne_bytes()); // x = 10,5
    put(&mut e, 48, &(20u32 << 16).to_ne_bytes()); // y = 20
    put(&mut e, 52, &1u16.to_ne_bytes()); // buttons_len
    put(&mut e, 54, &1u16.to_ne_bytes()); // valuators_len
    put(&mut e, 76, &5u32.to_ne_bytes()); // Maj + Ctrl
    e[83] = 1; // groupe
    put(&mut e, 88, &0b1100u32.to_ne_bytes());
    put(&mut e, 92, &(-3i32).to_ne_bytes());
    put(&mut e, 96, &0x8000_0000u32.to_ne_bytes()); // -3 + 0,5
    put(&mut e, 100, &45i32.to_ne_bytes());
    let d = device_event(&e).unwrap();
    assert_eq!(
        (d.kind, d.device, d.time, d.window),
        (6, 2, 1234, 0x2a_0000)
    );
    assert_eq!(d.position, (10.5, 20.));
    assert_eq!((d.mods, d.group), (5, 1));
    assert_eq!(d.valuators, vec![(2, -2.5), (3, 45.)]);
    // Tronqué : refusé plutôt que lu au-delà.
    assert!(device_event(&e[..96]).is_none());
}
#[test]
fn xinput_device_queries_list_scroll_axes() {
    use super::xinput::scroll_axes;
    // Un périphérique (id 11, nom de 5 octets), une classe de boutons puis une classe de
    // défilement vertical sur l'axe 3, incrément 15.
    let mut r = vec![0u8; 32 + 12 + 8 + 8 + 24];
    put(&mut r, 8, &1u16.to_ne_bytes());
    put(&mut r, 32, &11u16.to_ne_bytes());
    put(&mut r, 38, &2u16.to_ne_bytes()); // classes
    put(&mut r, 40, &5u16.to_ne_bytes()); // nom
    let class = 32 + 12 + 8;
    put(&mut r, class, &0u16.to_ne_bytes()); // boutons
    put(&mut r, class + 2, &2u16.to_ne_bytes());
    let scroll = class + 8;
    put(&mut r, scroll, &3u16.to_ne_bytes());
    put(&mut r, scroll + 2, &6u16.to_ne_bytes());
    put(&mut r, scroll + 6, &3u16.to_ne_bytes()); // axe
    put(&mut r, scroll + 8, &1u16.to_ne_bytes()); // vertical
    put(&mut r, scroll + 16, &15i32.to_ne_bytes());
    let devices = scroll_axes(&r);
    let axis = devices[&11][&3];
    assert!(axis.vertical && axis.increment == 15. && axis.last.is_none());
}
