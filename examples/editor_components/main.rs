//! Les composants d'éditeur de karbon-ui réunis dans une fenêtre : barre de menus,
//! arbre de 100 000 objets, grille de propriétés (champs qu'on fait glisser, vecteurs,
//! cases, listes, couleur), molette avec Ctrl, et fermeture qui demande d'enregistrer.
//!
//! ```powershell
//! cargo run --release --example editor_components
//! cargo run --release --example editor_components -- --bench 20   # défilement mesuré
//! ```
//!
//! `--export image.png [--menu N]` dessine une image de la fenêtre sans l'ouvrir (avec le
//! menu N ouvert et la ligne 2 de l'arbre choisie), pour la regarder.
//!
//! `--bench N` fait défiler l'arbre déplié de trois lignes par image pendant N secondes
//! (une image de 60 par seconde, le curseur descend et la liste le suit), avec le
//! profilage de la fenêtre, puis ferme : le rapport `Presentation : {…}` donne les temps.
mod view;
mod world;

use karbon_ui::ui::editor::{ColorMsg, ColorPickerState};
use karbon_ui::{
    platform,
    task::Inbox,
    ui::{
        editor::{
            FieldChange, FieldMsg, FieldReply, FieldState, MenuBarMsg, MenuBarState, TreeEvent,
            TreeMsg, TreeState,
        },
        prelude::*,
        runtime::{KeyCode, KeyPress},
    },
    window::{WindowCommand, WindowId, WindowOptions},
};
use std::{collections::HashSet, time::Duration};
use world::Level;

#[derive(Clone, Debug)]
pub enum Msg {
    Menus(MenuBarMsg<Msg>),
    Tree(TreeMsg),
    Field(FieldMsg),
    Fold(u64),
    Name(String),
    Visible(bool),
    Kind(u64),
    PickColor,
    /// Un geste sur le sélecteur de couleur de la lumière.
    Color(ColorMsg),
    Zoom(f32),
    /// Clic droit dans l'arbre : son menu, à cette place.
    TreeMenu(karbon_ui::ui::scene::PointF),
    CloseMenu,
    Rename,
    New,
    Save,
    Undo,
    Quit,
    Grid,
    Shadows,
    Projection(u8),
    About,
    /// Réponses à la question posée avant de fermer.
    SaveAndClose,
    Discard,
    KeepOpen,
}

/// Panneau qui reçoit les touches.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Tree,
    Other,
}

pub struct Editor {
    pub level: Level,
    pub tree: TreeState,
    pub fields: FieldState,
    /// Le sélecteur de couleur, et la couleur de la lumière (sRGB, de 0 à 1).
    pub colors: ColorPickerState,
    pub light: [f32; 3],
    pub menus: MenuBarState,
    pub folded: HashSet<u64>,
    pub panel: Panel,
    pub grid: bool,
    pub shadows: bool,
    pub projection: u8,
    pub zoom: f32,
    pub status: String,
    pub asking: bool,
    pub tree_menu: Option<karbon_ui::ui::scene::PointF>,
    commands: Vec<WindowCommand>,
    inbox: Inbox<Editor>,
    bench_ticks: Option<u32>,
}

impl Editor {
    fn new(bench: Option<u32>) -> Self {
        let level = Level::generate();
        let mut tree = TreeState::new(&level);
        if bench.is_some() {
            for key in level.children.keys().copied().collect::<Vec<_>>() {
                tree.set_expanded(key, true, &level);
            }
        }
        tree.select(level.roots[0]);
        let status = format!(
            "{} objets · clic : choisir · Ctrl+clic, Maj+clic : plusieurs · double-clic : ouvrir · glisser une ligne sur une autre : la rattacher",
            level.count()
        );
        Self {
            level,
            tree,
            fields: FieldState::default(),
            colors: ColorPickerState::default(),
            light: [1., 0.76, 0.48],
            menus: MenuBarState::default(),
            folded: HashSet::new(),
            panel: Panel::Tree,
            grid: true,
            shadows: true,
            projection: 0,
            zoom: 1.,
            status,
            asking: false,
            tree_menu: None,
            commands: Vec::new(),
            inbox: Inbox::new(),
            bench_ticks: bench,
        }
    }

    /// L'objet montré dans la grille : le curseur de l'arbre.
    pub fn current(&self) -> Option<u64> {
        self.tree.cursor()
    }

    fn tree_message(&mut self, message: TreeMsg) -> bool {
        self.panel = Panel::Tree;
        let reply = self.tree.update(message, &self.level);
        match reply.event {
            Some(TreeEvent::Activated(key)) => {
                self.status = format!("Ouvert : {}", self.level.props(key).name);
            }
            Some(TreeEvent::Dropped(drop)) => {
                self.level.reparent(&drop.keys, drop.target);
                self.tree.set_expanded(drop.target, true, &self.level);
                self.status = format!("{} objet(s) rattaché(s)", drop.keys.len());
            }
            _ => {}
        }
        true
    }

    /// Un geste sur le sélecteur de couleur : la couleur suit le glisser, et la ligne
    /// d'état dit la valeur retenue à la fin du geste.
    fn color_message(&mut self, message: ColorMsg) -> bool {
        self.panel = Panel::Other;
        let reply = self.colors.update(message);
        if let Some(event) = reply.event {
            self.light = event.rgb;
            if event.change == FieldChange::Commit {
                let hex = karbon_ui::ui::editor::hex_of(event.rgb);
                self.status = format!("Couleur de la lumière : {hex}");
            }
        }
        reply.rebuild
    }

    fn field_message(&mut self, message: FieldMsg) -> bool {
        self.panel = Panel::Other;
        let reply = self.fields.update(message);
        self.apply_field(reply)
    }

    /// Range la valeur d'un champ dans l'objet courant ; seule une validation entre dans
    /// l'historique (pas les aperçus d'un glisser, ni le retour par Échap).
    fn apply_field(&mut self, reply: FieldReply) -> bool {
        if let (Some(event), Some(key)) = (reply.event, self.current()) {
            let before = self.level.props(key);
            let record = (event.change == FieldChange::Commit).then_some(before);
            let (axis, value) = (event.field.axis, event.value);
            self.level.edit(key, record, |p| match event.field.key {
                1 => p.position[axis] = value,
                2 => p.rotation[axis] = value,
                3 => p.scale[axis] = value,
                _ => p.intensity = value,
            });
        }
        reply.rebuild || reply.event.is_some()
    }

    fn commit(&mut self, change: impl FnOnce(&mut world::Props)) {
        if let Some(key) = self.current() {
            let before = self.level.props(key);
            self.level.edit(key, Some(before), change);
        }
    }

    fn command(&mut self, message: Msg) {
        match message {
            Msg::New => self.status = "Nouveau niveau : pas dans cet exemple".into(),
            Msg::Save | Msg::SaveAndClose => {
                self.level.unsaved = false;
                self.status = "Enregistré".into();
                if matches!(message, Msg::SaveAndClose) {
                    self.commands.push(WindowCommand::Close(WindowId::MAIN));
                }
            }
            Msg::Undo => {
                if let Some(key) = self.level.undo() {
                    self.status = format!("Annulé sur {}", self.level.props(key).name);
                }
            }
            Msg::Quit => {
                if self.level.unsaved {
                    self.asking = true;
                } else {
                    self.commands.push(WindowCommand::Close(WindowId::MAIN));
                }
            }
            Msg::Discard => self.commands.push(WindowCommand::Close(WindowId::MAIN)),
            Msg::KeepOpen => self.asking = false,
            Msg::Grid => self.grid = !self.grid,
            Msg::Shadows => self.shadows = !self.shadows,
            Msg::Projection(p) => self.projection = p,
            Msg::About => self.status = "karbon-ui : composants d'éditeur".into(),
            _ => {}
        }
    }

    /// Une image du banc : le curseur descend de trois lignes, la liste le suit.
    fn bench_tick(&mut self) -> bool {
        let rows = self.tree.rows();
        let at = self
            .current()
            .and_then(|k| rows.iter().position(|r| r.key == k));
        let next = rows[(at.unwrap_or(0) + 3) % rows.len()].key;
        self.tree.select(next);
        true
    }
}

impl Application for Editor {
    type Message = Msg;
    fn view(&self, context: &ViewContext) -> Element {
        view::editor(self, context)
    }
    fn update(&mut self, message: Msg) -> bool {
        match message {
            Msg::Menus(message) => {
                let reply = self.menus.update(message);
                return match reply.command {
                    Some(command) => self.update(command),
                    None => reply.rebuild,
                };
            }
            Msg::Tree(message) => return self.tree_message(message),
            Msg::Field(message) => return self.field_message(message),
            Msg::Color(message) => return self.color_message(message),
            Msg::Fold(section) => {
                if !self.folded.remove(&section) {
                    self.folded.insert(section);
                }
            }
            Msg::Name(name) => self.commit(|p| p.name = name),
            Msg::Visible(on) => self.commit(|p| p.visible = on),
            Msg::Kind(kind) => self.commit(|p| p.kind = kind),
            Msg::TreeMenu(at) => self.tree_menu = Some(at),
            Msg::CloseMenu => self.tree_menu = None,
            Msg::Rename => self.status = "Renommer : dans la grille, champ « Nom »".into(),
            Msg::PickColor => self.status = "Le sélecteur de couleur viendra plus tard".into(),
            Msg::Zoom(notches) => {
                self.zoom = (self.zoom * 1.1f32.powf(-notches)).clamp(0.1, 10.);
                self.status = format!("Zoom de la vue : {:.0} %", self.zoom * 100.);
            }
            other => self.command(other),
        }
        true
    }
    fn key(&mut self, press: KeyPress) -> bool {
        let menus = view::menus(self);
        if self.menus.key(press, &menus) {
            return true;
        }
        let field = self.fields.key(press);
        if field.rebuild {
            return self.apply_field(field);
        }
        let color = self.colors.key(press);
        if let Some(event) = color.event {
            self.light = event.rgb;
            return true;
        }
        if press.pressed && press.control {
            let command = match press.key {
                KeyCode::S => Some(Msg::Save),
                KeyCode::Z => Some(Msg::Undo),
                KeyCode::N => Some(Msg::New),
                _ => None,
            };
            if let Some(command) = command {
                self.command(command);
                return true;
            }
        }
        if self.panel == Panel::Tree {
            return self.tree.key(press, &self.level).rebuild;
        }
        false
    }
    fn close_requested(&mut self) -> bool {
        self.asking = self.level.unsaved;
        !self.level.unsaved
    }
    fn inbox(&self) -> Option<Inbox<Self>> {
        Some(self.inbox.clone())
    }
    fn take_window_commands(&mut self) -> Vec<WindowCommand> {
        std::mem::take(&mut self.commands)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let bench = args.iter().position(|a| a == "--bench").map(|at| {
        let seconds = args
            .get(at + 1)
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(20);
        args.drain(at..(at + 2).min(args.len()));
        seconds
    });
    let mut take = |name: &str| -> Option<String> {
        let at = args.iter().position(|a| a == name)?;
        let value = args.get(at + 1).cloned();
        args.drain(at..(at + 2).min(args.len()));
        value
    };
    let export = take("--export");
    let menu = take("--menu").and_then(|m| m.parse::<usize>().ok());
    let options = WindowOptions::from_args(args)?;
    let mut editor = Editor::new(bench);
    if let Some(path) = export {
        if let Some(menu) = menu {
            editor.menus.open(menu);
        }
        let second = editor.level.roots[1];
        editor.tree.select(second);
        let image = karbon_ui::diagnostics::render_preview(
            karbon_ui::diagnostics::BenchmarkOptions {
                width: 1280,
                height: 800,
                scale: 1.5,
                reduced_motion: true,
                renderer: options.renderer,
                ..Default::default()
            },
            editor,
        )?;
        karbon_ui::diagnostics::write_png(&image, std::path::Path::new(&path))?;
        return Ok(());
    }
    if let Some(seconds) = editor.bench_ticks {
        let sender = editor.inbox.sender();
        std::thread::spawn(move || {
            for _ in 0..seconds * 60 {
                std::thread::sleep(Duration::from_nanos(16_666_667));
                if !sender.post(Editor::bench_tick) {
                    return;
                }
            }
            sender.post(|editor: &mut Editor| {
                editor.commands.push(WindowCommand::Exit);
                false
            });
        });
    }
    platform::run(
        WindowOptions {
            title: "Karbon UI · composants d'éditeur",
            width: 1280,
            height: 800,
            min_width: 800,
            min_height: 500,
            profile: options.profile || bench.is_some(),
            ..options
        },
        editor,
    )?;
    Ok(())
}
