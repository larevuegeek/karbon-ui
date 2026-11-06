//! Méthodes d'entrée par D-Bus pour les fenêtres X11 (sous Wayland, le compositeur les
//! relaie par `zwp_text_input_v3`) : IBus (`ibus.rs`), le défaut de GNOME et de la plupart
//! des distributions. Le client ne dépend pas du système de fenêtres : la boucle lui passe
//! les touches, le focus du champ et la position du caret, et applique ses demandes.
mod ibus;

pub(super) use ibus::Ibus;

/// Ce que la méthode d'entrée demande à l'application.
#[derive(Debug, PartialEq)]
pub(super) enum Request {
    /// Texte validé.
    Commit(String),
    /// Texte en cours de composition et position du curseur (en caractères) ; vide : la
    /// composition est retirée.
    Preedit(String, usize),
    /// Touche rendue à l'application : symbole, code evdev, état (bit 30 : relâchement).
    Forward {
        keyval: u32,
        keycode: u32,
        state: u32,
    },
}
#[cfg(test)]
mod tests {
    use super::ibus::{bus_address, bus_file};

    #[test]
    fn the_ibus_address_file_follows_libibus_naming() {
        assert_eq!(
            bus_file("/home/a/.config", "f348a6\n", ":0").as_deref(),
            Some("/home/a/.config/ibus/bus/f348a6-unix-0")
        );
        assert_eq!(
            bus_file("/c", "m", "remote:10.0").as_deref(),
            Some("/c/ibus/bus/m-remote-10")
        );
        assert_eq!(bus_file("/c", "m", "wayland-0"), None);
        let file = "# Ce fichier est créé par ibus-daemon\nIBUS_ADDRESS=unix:path=/tmp/x,guid=1\nIBUS_DAEMON_PID=42\n";
        assert_eq!(
            bus_address(file).as_deref(),
            Some("unix:path=/tmp/x,guid=1")
        );
        assert_eq!(bus_address("IBUS_DAEMON_PID=42"), None);
    }
}
