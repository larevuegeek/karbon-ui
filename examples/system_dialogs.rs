//! Dialogues de fichiers du système et thème du bureau.
//! `cargo run --release --example system_dialogs`.
//!
//! Les quatre sortes de [`FileDialog`] (un fichier, plusieurs, un dossier, enregistrer
//! sous) ; le choix revient par [`Application::task_completed`]. La page suit le thème
//! clair ou sombre du système ([`ColorScheme::System`](karbon_ui::application::ColorScheme),
//! le défaut) : changer le réglage du bureau fenêtre ouverte doit la repeindre. Le champ
//! sert à essayer la saisie (méthode d'entrée, touches mortes, copier-coller).
use karbon_ui::{
    platform,
    task::{TaskId, TaskResult},
    ui::kit::prelude::*,
    window::{FileDialog, FileDialogResult, WindowCommand, WindowId, WindowOptions},
};

#[derive(Debug, Clone)]
enum Msg {
    OpenFile,
    OpenFiles,
    Folder,
    Save,
    Note(String),
}

#[derive(Default)]
struct Dialogs {
    /// Dernier choix, ou l'erreur du système.
    status: String,
    note: String,
    commands: Vec<WindowCommand>,
}

impl Dialogs {
    fn show(&mut self, dialog: FileDialog) {
        self.status = "Dialogue ouvert…".into();
        self.commands
            .push(WindowCommand::FileDialog(WindowId::MAIN, TaskId(1), dialog));
    }
}

impl Application for Dialogs {
    type Message = Msg;

    fn view(&self, _: &ViewContext) -> Element {
        let status = self.status.as_str();
        let note = self.note.as_str();
        ui! {
            <Page padding="xl" gap="lg">
                <Title>Dialogues du système</Title>
                <Text muted>"Le thème suit le réglage clair ou sombre du bureau."</Text>
                <Card title="Fichiers" subtitle="Portail XDG sous Linux">
                    <Row gap="sm">
                        <Button primary icon="file" onclick={Msg::OpenFile}>"Un fichier"</Button>
                        <Button soft onclick={Msg::OpenFiles}>"Plusieurs fichiers"</Button>
                        <Button soft icon="folder" onclick={Msg::Folder}>"Un dossier"</Button>
                        <Button outline onclick={Msg::Save}>"Enregistrer sous"</Button>
                    </Row>
                    <Text mono>{status}</Text>
                </Card>
                <Card title="Saisie">
                    <Input label="Texte libre" value={note} oninput={Msg::Note} />
                </Card>
            </Page>
        }
    }

    fn update(&mut self, message: Msg) -> bool {
        match message {
            Msg::OpenFile => self.show(
                FileDialog::open_file()
                    .title("Ouvrir une image")
                    .filter("Images", &["png", "jpg", "jpeg", "webp"])
                    .filter("Texte", &["txt", "md"]),
            ),
            Msg::OpenFiles => self.show(FileDialog::open_files().title("Ajouter des fichiers")),
            Msg::Folder => self.show(FileDialog::pick_folder()),
            Msg::Save => self.show(
                FileDialog::save_file()
                    .file_name("notes.txt")
                    .filter("Texte", &["txt"]),
            ),
            Msg::Note(value) => self.note = value,
        }
        true
    }

    fn take_window_commands(&mut self) -> Vec<WindowCommand> {
        std::mem::take(&mut self.commands)
    }

    fn task_completed(&mut self, result: TaskResult) -> bool {
        self.status = match result.into_value::<FileDialogResult>() {
            Ok(choice) if choice.paths.is_empty() => "Annulé.".into(),
            Ok(choice) => choice
                .paths
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join("\n"),
            Err(error) => format!("Dialogue impossible : {error}"),
        };
        true
    }
}

fn main() -> karbon_ui::Result<()> {
    let options = WindowOptions::from_args(std::env::args().skip(1))?;
    platform::run(
        WindowOptions {
            title: "Karbon UI · dialogues",
            width: 820,
            height: 560,
            min_width: 560,
            min_height: 420,
            ..options
        },
        Dialogs::default(),
    )
}
