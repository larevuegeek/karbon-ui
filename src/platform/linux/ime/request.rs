/// Ce que la méthode d'entrée demande à l'application.
#[derive(Debug, PartialEq)]
pub(crate) enum Request {
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
    use super::super::ibus::{bus_address, bus_file};

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
