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
    /// Réponse du moteur à la touche envoyée sous ce numéro : prise ou non.
    Answered { serial: u32, taken: bool },
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

    /// Touches envoyées sans attendre au vrai démon IBus (sauté sans lui) : chacune reçoit
    /// sa réponse, sous son numéro, dans l'ordre d'envoi.
    #[test]
    fn keys_sent_to_ibus_are_answered_asynchronously_in_order() {
        use super::super::{Ibus, Request};
        let Some(mut ibus) = Ibus::connect() else {
            eprintln!("karbon-ui: no IBus, test skipped");
            return;
        };
        ibus.focus(true);
        // `a` puis `b`, appui et relâchement (symbole XKB, code evdev, sans modificateur).
        let serials: Vec<u32> = [(0x61, 30, true), (0x61, 30, false), (0x62, 48, true)]
            .iter()
            .map(|&(sym, code, pressed)| ibus.key(sym, code, 0, pressed).expect("sent"))
            .collect();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        let mut answered = Vec::new();
        while answered.len() < serials.len() && std::time::Instant::now() < deadline {
            for request in ibus.requests() {
                // Les autres appels (`SetCapabilities`, `FocusIn`) ont aussi leur réponse.
                if let Request::Answered { serial, .. } = request
                    && serials.contains(&serial)
                {
                    answered.push(serial);
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        ibus.focus(false);
        assert_eq!(answered, serials);
    }
}
