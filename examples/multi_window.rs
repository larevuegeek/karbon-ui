//! Deux fenêtres dès le démarrage, avec état indépendant et fermeture par commande.
use karbon_ui::{
    application::{Application, ViewContext},
    platform,
    ui::{element::Element, style::SurfaceStyle, widgets},
    window::{ExitPolicy, WindowCommand, WindowDescriptor, WindowId, WindowOptions},
};

#[derive(Debug, Clone)]
enum Msg {
    Increment,
    Close,
}

struct Counter {
    id: WindowId,
    value: u64,
    commands: Vec<WindowCommand>,
}
impl Application for Counter {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        Element::column(
            1,
            vec![
                widgets::text(2, format!("Fenêtre {} : {}", self.id.0, self.value)),
                widgets::button(3, "Ajouter +1", Msg::Increment),
                widgets::button(4, "Fermer cette fenêtre", Msg::Close),
            ],
        )
        .styled(SurfaceStyle::default())
    }
    fn update(&mut self, message: Msg) -> bool {
        match message {
            Msg::Increment => self.value += 1,
            Msg::Close => self.commands.push(WindowCommand::Close(self.id)),
        }
        true
    }
    fn take_window_commands(&mut self) -> Vec<WindowCommand> {
        std::mem::take(&mut self.commands)
    }
}
fn window(id: u64, x: i32, light: bool) -> WindowDescriptor {
    let id = WindowId(id);
    WindowDescriptor::new(
        id,
        WindowOptions {
            title: "Karbon UI - deux fenêtres",
            x,
            y: 120,
            width: 400,
            height: 300,
            min_width: 300,
            min_height: 240,
            light,
            ..WindowOptions::DEFAULT
        },
        Counter {
            id,
            value: 0,
            commands: Vec::new(),
        },
    )
}
fn main() -> karbon_ui::Result<()> {
    platform::run_windows(
        vec![window(1, 120, false), window(2, 560, true)],
        ExitPolicy::LastWindowClosed,
    )
}
