//! Icône de la fenêtre et icône dans la zone de notification (barre système).
//! `cargo run --release --example tray_icon`.
//!
//! [`WindowOptions::icon`] donne l'icône de la fenêtre. [`WindowCommand::Tray`] pose
//! l'icône de la zone de notification avec son menu ; ses clics reviennent par
//! [`Application::tray_event`], même fenêtre masquée. « Masquer » cache la fenêtre sans
//! quitter : le clic principal sur l'icône (ou « Afficher » dans son menu) la ramène,
//! « Quitter » termine. Windows : zone de notification ; macOS : barre des menus ;
//! Linux : `StatusNotifierItem` (KDE, Xfce, Cinnamon… ; GNOME avec l'extension
//! AppIndicator).
use karbon_ui::{
    platform,
    ui::kit::prelude::*,
    window::{TrayEvent, TrayIcon, WindowChange, WindowCommand, WindowId, WindowOptions},
};

/// Une image carrée suffit, aux tailles que demande le système.
const ICON: &[u8] = include_bytes!("../tests/fixtures/images/cover.png");

// Identifiants des entrées du menu, rendus par `TrayEvent::MenuItem`.
const SHOW: u32 = 1;
const PAUSE: u32 = 2;
const QUIT: u32 = 3;

#[derive(Debug, Clone)]
enum Msg {
    Hide,
}

struct Player {
    playing: bool,
    clicks: u32,
    commands: Vec<WindowCommand>,
}

impl Player {
    /// L'icône et son menu ; renvoyer la commande la met à jour (texte de « Pause »).
    fn tray(&self) -> WindowCommand {
        let pause = if self.playing { "Pause" } else { "Lecture" };
        WindowCommand::Tray(
            WindowId::MAIN,
            Some(
                TrayIcon::new(ICON)
                    .tooltip("Karbon UI · icône")
                    .item(SHOW, "Afficher")
                    .item(PAUSE, pause)
                    .separator()
                    .item(QUIT, "Quitter"),
            ),
        )
    }
    fn show(&mut self) {
        self.commands
            .push(WindowCommand::Change(WindowId::MAIN, WindowChange::Focus));
    }
}

impl Application for Player {
    type Message = Msg;

    fn view(&self, _: &ViewContext) -> Element {
        let state = if self.playing {
            "En lecture"
        } else {
            "En pause"
        };
        let clicks = format!("Clics sur l'icône : {}", self.clicks);
        ui! {
            <Page padding="xl" gap="lg">
                <Title>Icône de la zone de notification</Title>
                <Text muted>{"Clic principal sur l'icône : la fenêtre revient. Clic secondaire : le menu."}</Text>
                <Card title={state}>
                    <Text mono>{clicks}</Text>
                    <Row gap="sm">
                        <Button primary onclick={Msg::Hide}>Masquer dans la barre système</Button>
                    </Row>
                </Card>
            </Page>
        }
    }

    fn update(&mut self, message: Msg) -> bool {
        match message {
            // Masquée, pas fermée : la boucle tourne, l'icône reste.
            Msg::Hide => self.commands.push(WindowCommand::Change(
                WindowId::MAIN,
                WindowChange::Visible(false),
            )),
        }
        false
    }

    fn tray_event(&mut self, event: TrayEvent) -> bool {
        match event {
            TrayEvent::Activated => {
                self.clicks += 1;
                self.show();
            }
            TrayEvent::MenuItem(SHOW) => self.show(),
            TrayEvent::MenuItem(PAUSE) => {
                self.playing = !self.playing;
                let tray = self.tray();
                self.commands.push(tray);
            }
            TrayEvent::MenuItem(QUIT) => self.commands.push(WindowCommand::Exit),
            TrayEvent::MenuItem(_) => {}
        }
        true
    }

    fn take_window_commands(&mut self) -> Vec<WindowCommand> {
        std::mem::take(&mut self.commands)
    }
}

fn main() -> karbon_ui::Result<()> {
    let options = WindowOptions::from_args(std::env::args().skip(1))?;
    let mut player = Player {
        playing: true,
        clicks: 0,
        commands: Vec::new(),
    };
    // Posée dès l'ouverture : la commande est lue après le chargement de la fenêtre.
    let tray = player.tray();
    player.commands.push(tray);
    platform::run(
        WindowOptions {
            title: "Karbon UI · icône",
            width: 720,
            height: 420,
            min_width: 520,
            min_height: 320,
            icon: Some(ICON),
            ..options
        },
        player,
    )
}
