//! Une page complète avec l'API de composition, sans appel à une API d'OS.
use karbon_ui::{platform, ui::prelude::*, window::WindowOptions};

#[derive(Debug, Clone)]
enum Msg {
    NameChanged(String),
    Notifications(bool),
    Save,
}

const PAGE: ViewStyle = ViewStyle::new().fill_height().padding(24.).gap(20.);
const CARD: ViewStyle = ViewStyle::new()
    .padding(20.)
    .gap(12.)
    .background(Color::hex(0x131715))
    .radius(16.);

#[derive(Default)]
struct App {
    name: String,
    notifications: bool,
    message: String,
}
impl Application for App {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        column(
            "page",
            [
                Text::new("Ma première interface")
                    .size(28.)
                    .bold()
                    .element("heading"),
                text(
                    "intro",
                    "Une structure Rust, des styles partagés et des actions typées.",
                ),
                column(
                    "form",
                    [
                        text("name-label", "Ton nom"),
                        TextInput::new(self.name.as_str(), Msg::NameChanged)
                            .label("Ton nom")
                            .placeholder("Ex. David")
                            .element("name"),
                        Checkbox::new(
                            "Activer les notifications",
                            self.notifications,
                            Msg::Notifications,
                        )
                        .element("notifications"),
                        row("actions", [button("save", "Valider", Msg::Save)]),
                    ],
                )
                .style(CARD),
            ],
        )
        .child_if(!self.message.is_empty(), || {
            text("message", self.message.as_str())
        })
        .style(PAGE)
    }
    fn update(&mut self, message: Msg) -> bool {
        match message {
            Msg::NameChanged(name) => self.name = name,
            Msg::Notifications(enabled) => self.notifications = enabled,
            Msg::Save => self.message = format!("Bonjour {} !", self.name.trim()),
        }
        true
    }
}
fn main() -> karbon_ui::Result<()> {
    let options = WindowOptions::from_args(std::env::args().skip(1))?;
    platform::run(
        WindowOptions {
            title: "Karbon UI · syntaxe simple",
            width: 720,
            height: 580,
            min_width: 520,
            min_height: 520,
            ..options
        },
        App::default(),
    )
}
