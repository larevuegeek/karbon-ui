//! Application de sonde et scénario.
use crate::appkit::{appearance, ax_text, click, collect, cpu_ms, key, nested_loop};
use crate::harness::*;
use crate::layers::{drawable_width, metal_shown};
use karbon_ui::{
    Error,
    application::{Application, ViewContext},
    platform::{
        self,
        headless::{HeadlessApp, HeadlessOptions},
    },
    task::{Task, TaskCommand, TaskId, TaskResult},
    ui::prelude::*,
    ui::{
        rendering::filter::{Filter, Filters},
        template::Node,
    },
    window::{
        ExitPolicy, RenderMode, WindowChange, WindowCommand, WindowDescriptor, WindowId,
        WindowOptions,
    },
};
use objc2::{msg_send, sel};
use objc2_app_kit::NSApplication;
use objc2_foundation::NSString;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

pub(crate) const WIDTH: i32 = 420;
pub(crate) const HEIGHT: i32 = 300;

#[derive(Default, Debug)]
pub(crate) struct Log {
    loads: u32,
    painted: bool,
    clicks: u32,
    name: String,
    composing_seen: bool,
    second_opened: bool,
    second_closed: bool,
    nested: Option<String>,
    /// Libellés successifs du renderer vus par la vue (`ViewContext::backend`).
    backends: Vec<&'static str>,
    /// Calque Metal visible (et plus d'image Core Graphics sur la vue) : vue filtrée,
    /// pendant le repli forcé (`KARBON_METAL_FALLBACK`), après son retrait. `None` : pas
    /// de calque Metal.
    metal: [Option<bool>; 3],
    steps: u32,
    /// Accessibilité : bouton trouvé (rôle et libellé), valeur du champ, appui accepté.
    ax_button: bool,
    ax_value: String,
    ax_pressed: bool,
    /// Thème vu par la vue (`color_scheme`, `ColorScheme::System`), à chaque changement.
    dark: Vec<bool>,
    /// Largeur des drawables Metal pendant que la fenêtre est dans le Dock.
    docked: Option<f64>,
    /// Temps processeur du processus pendant le séjour dans le Dock (ms) : la boucle doit
    /// dormir, l'écran ne bat plus pour la fenêtre.
    docked_cpu: Option<u64>,
    /// Calque Metal visible et à la taille de la vue après le retour du Dock.
    restored: Option<bool>,
    /// Taille de la vue après `WindowChange::Size`, et image refaite à cette taille.
    resized: (Option<(f64, f64)>, Option<bool>),
    /// Application au premier plan juste après l'ouverture (journal seulement, voir `run`).
    active_at_start: Option<bool>,
    /// Fenêtre principale couverte : encore visible pour AppKit, et largeur de ses
    /// drawables après une invalidation.
    covered: Option<(bool, Option<f64>)>,
}
#[derive(Clone)]
enum Msg {
    Ok,
    Name(String),
}
struct Probe {
    log: Arc<Mutex<Log>>,
    name: String,
    step: u64,
    /// Centres des contrôles, mesurés par le harnais headless (tailles fixes).
    ok: (f32, f32),
    field: (f32, f32),
    tasks: Vec<TaskCommand>,
    commands: Vec<WindowCommand>,
    /// Repli vers Core Graphics forcé (`KARBON_METAL_FALLBACK`) ; un émoji sous
    /// `background-clip: text` s'ajoute alors à la vue et y reste (dessiné par Metal ensuite).
    fallback: bool,
}
impl Probe {
    fn new(log: Arc<Mutex<Log>>, ok: (f32, f32), field: (f32, f32)) -> Self {
        Self {
            log,
            name: String::new(),
            step: 0,
            ok,
            field,
            tasks: Vec::new(),
            commands: Vec::new(),
            fallback: false,
        }
    }
    fn next(&mut self, delay: Duration) {
        self.step += 1;
        self.tasks.push(TaskCommand::Spawn(Task::new(
            TaskId(self.step),
            move |_| std::thread::sleep(delay),
        )));
    }
}
fn view(name: &str) -> Element {
    column(
        "root",
        [
            button("ok", "OK", Msg::Ok)
                .width(Length::Px(200.))
                .height(Length::Px(40.)),
            input("name", name, Msg::Name)
                .width(Length::Px(200.))
                .height(Length::Px(40.)),
        ],
    )
}
impl Application for Probe {
    type Message = Msg;
    fn load(&mut self, _: &dyn karbon_ui::application::Resources) -> karbon_ui::Result<()> {
        self.log.lock().unwrap().loads += 1;
        self.next(Duration::from_millis(300));
        Ok(())
    }
    fn view(&self, context: &ViewContext) -> Element {
        // Fenêtre de test invisible : elle reçoit les événements sans s'afficher.
        hide_windows();
        let mut log = self.log.lock().unwrap();
        log.painted = true;
        if log.backends.last() != Some(&context.backend) {
            log.backends.push(context.backend);
        }
        let dark = color_scheme().0;
        if log.dark.last() != Some(&dark) {
            log.dark.push(dark);
        }
        // Un filtre CSS couvre la vue, et l'émoji sous `background-clip: text` : ils
        // restent au chemin Metal.
        static BLUR: [Filter; 1] = [Filter::Blur(0.5)];
        let root = Node::element(view(&self.name))
            .filters(Filters::new(&BLUR))
            .build();
        if self.fallback {
            let label = Node::element(text("emoji", "Note 😀"))
                .text_clip(true)
                .background_color(Color::rgb(200, 0, 0))
                .build();
            column("fallback", [root, label])
        } else {
            root
        }
    }
    fn update(&mut self, message: Msg) -> bool {
        match message {
            Msg::Ok => self.log.lock().unwrap().clicks += 1,
            Msg::Name(name) => {
                self.name = name;
                self.log.lock().unwrap().name = self.name.clone();
            }
        }
        true
    }
    fn take_task_commands(&mut self) -> Vec<TaskCommand> {
        std::mem::take(&mut self.tasks)
    }
    fn take_window_commands(&mut self) -> Vec<WindowCommand> {
        std::mem::take(&mut self.commands)
    }
    fn task_completed(&mut self, result: TaskResult) -> bool {
        assert!(result.into_value::<()>().is_ok());
        let step = self.step;
        PROGRESS.store(step as u32, std::sync::atomic::Ordering::SeqCst);
        self.log.lock().unwrap().steps = step as u32;
        let window = window(WindowId::MAIN);
        match step {
            1 => {
                // Fenêtres toutes ouvertes avec `activate: false` : l'application ne prend
                // pas le premier plan (M7 de la revue).
                self.log.lock().unwrap().active_at_start =
                    Some(NSApplication::sharedApplication(mtm()).isActive());
                click(&window, self.ok);
            }
            2 => {
                click(&window, self.field);
                for (c, code) in [("a", 0u16), ("b", 11)] {
                    key(&window, c, code);
                }
            }
            3 => {
                // Composition : « ´ » marqué (touche morte, IME), puis « é » validé.
                deferred(move || {
                    let responder = window.firstResponder().expect("vue focalisée");
                    let marked = NSString::from_str("\u{b4}");
                    let range = objc2_foundation::NSRange::new(1, 0);
                    let none =
                        objc2_foundation::NSRange::new(objc2_foundation::NSNotFound as usize, 0);
                    // SAFETY : la vue implémente NSTextInputClient.
                    unsafe {
                        let _: () = msg_send![&*responder, setMarkedText: &*marked, selectedRange: range, replacementRange: none];
                        let has: bool = msg_send![&*responder, hasMarkedText];
                        LOG.with(|l| {
                            l.borrow().as_ref().unwrap().lock().unwrap().composing_seen = has
                        });
                        let committed = NSString::from_str("\u{e9}");
                        let _: () =
                            msg_send![&*responder, insertText: &*committed, replacementRange: none];
                    }
                });
            }
            4 => {
                // Arbre accessible, interrogé comme le ferait VoiceOver.
                deferred(move || {
                    let view = window.contentView().expect("vue de contenu");
                    let mut nodes = Vec::new();
                    collect(&view, &mut nodes);
                    let button = nodes.iter().find(|n| {
                        ax_text(n, sel!(accessibilityRole)) == "AXButton"
                            && ax_text(n, sel!(accessibilityLabel)) == "OK"
                    });
                    let field = nodes
                        .iter()
                        .find(|n| ax_text(n, sel!(accessibilityRole)) == "AXTextField");
                    let log = LOG.with(|l| l.borrow().clone().unwrap());
                    log.lock().unwrap().ax_button = button.is_some();
                    log.lock().unwrap().ax_value = field
                        .map(|f| ax_text(f, sel!(accessibilityValue)))
                        .unwrap_or_default();
                    if let Some(button) = button {
                        // SAFETY : action NSAccessibility d'un bouton.
                        let pressed: bool =
                            unsafe { msg_send![&**button, accessibilityPerformPress] };
                        log.lock().unwrap().ax_pressed = pressed;
                    }
                });
            }
            5 => {
                self.commands
                    .push(WindowCommand::Open(WindowDescriptor::new(
                        WindowId(2),
                        options("sonde 2"),
                        Second,
                    )));
            }
            6 => {
                self.log.lock().unwrap().second_opened = visible("sonde 2");
                self.commands.push(WindowCommand::Close(WindowId(2)));
            }
            7 => {
                self.log.lock().unwrap().second_closed = !visible("sonde 2");
                // Chemin GPU (vue filtrée), puis repli vers Core Graphics, puis retour.
                self.log.lock().unwrap().metal[0] = metal_shown(&window);
                // SAFETY : modifié sur le thread principal ; les tâches de fond de la sonde
                // ne lisent pas l'environnement.
                unsafe { std::env::set_var("KARBON_METAL_FALLBACK", "1") };
                self.fallback = true;
            }
            8 => {
                self.log.lock().unwrap().metal[1] = metal_shown(&window);
                // SAFETY : idem.
                unsafe { std::env::remove_var("KARBON_METAL_FALLBACK") };
            }
            9 | 10 => {
                if step == 9 {
                    self.log.lock().unwrap().metal[2] = metal_shown(&window);
                }
                // Apparence de la fenêtre changée, comme par le réglage du système : sombre,
                // puis claire (`viewDidChangeEffectiveAppearance`).
                deferred(move || appearance(&window, step == 9));
            }
            // Dans le Dock, puis de retour : drawables abandonnés, puis image refaite.
            11 => {
                self.log.lock().unwrap().docked_cpu = Some(cpu_ms());
                deferred(move || window.miniaturize(None));
            }
            12 => {
                let mut log = self.log.lock().unwrap();
                log.docked_cpu = log.docked_cpu.map(|start| cpu_ms() - start);
                drop(log);
                self.log.lock().unwrap().docked = Some(window.isMiniaturized())
                    .filter(|m| *m)
                    .and(drawable_width(&window));
                deferred(move || window.deminiaturize(None));
            }
            13 => {
                self.log.lock().unwrap().restored = metal_shown(&window);
                // Taille demandée par l'application : AppKit signale le redimensionnement
                // pendant l'appel, la boucle doit le traiter (M2 de la revue).
                self.commands.push(WindowCommand::Change(
                    WindowId::MAIN,
                    WindowChange::Size {
                        width: 500,
                        height: 360,
                    },
                ));
            }
            14 => {
                let size = window.contentView().map(|v| v.bounds().size);
                self.log.lock().unwrap().resized =
                    (size.map(|s| (s.width, s.height)), metal_shown(&window));
                // Fenêtre opaque plus grande par-dessus : la fenêtre principale est
                // entièrement couverte (M5 de la revue).
                self.commands
                    .push(WindowCommand::Open(WindowDescriptor::new(
                        WindowId(3),
                        WindowOptions {
                            width: 900,
                            height: 700,
                            ..options(COVER)
                        },
                        Second,
                    )));
            }
            // L'étape 15 invalide la vue (comme toutes) ; la 16 regarde ce qui a été peint.
            // Pendant ce temps, une boucle imbriquée consomme le réveil de la tâche de
            // l'étape : les suivantes doivent encore réveiller la boucle (M9 de la revue).
            15 => deferred(|| nested_loop(0.4)),
            16 => {
                // Une fenêtre couverte ne peint pas la vue invalidée : ses drawables
                // restent ceux d'un pixel.
                self.log.lock().unwrap().covered = Some((
                    window
                        .occlusionState()
                        .contains(objc2_app_kit::NSWindowOcclusionState::Visible),
                    drawable_width(&window),
                ));
                self.commands.push(WindowCommand::Close(WindowId(3)));
            }
            17 => {
                let nested = platform::run_windows(
                    vec![WindowDescriptor::new(WindowId(9), options("x"), Second)],
                    ExitPolicy::LastWindowClosed,
                );
                self.log.lock().unwrap().nested = nested.err().map(|e| format!("{e:?}"));
                // « Quitter » du Dock ou du menu : `terminate:`, que le délégué de
                // l'application change en fin de boucle (M4 de la revue).
                deferred(|| NSApplication::sharedApplication(mtm()).terminate(None));
                return true;
            }
            _ => return true,
        }
        // L'animation du Dock dure ~0,5 s.
        self.next(Duration::from_millis(if step >= 11 { 900 } else { 250 }));
        true
    }
}
fn centre(app: &HeadlessApp<Probe>, id: &str) -> (f32, f32) {
    let r = app.bounds(id).expect("contrôle présent");
    ((r.left + r.right) / 2., (r.top + r.bottom) / 2.)
}
pub fn run() {
    // SAFETY : fonction `extern "C"` sans argument, valide pendant toute la vie du processus.
    unsafe { libc::atexit(unfinished) };
    watchdog();
    // Erreurs de validation, avant toute fenêtre.
    assert!(matches!(
        platform::run_windows(Vec::new(), ExitPolicy::LastWindowClosed),
        Err(Error::InvalidInput { .. })
    ));
    let log = Arc::new(Mutex::new(Log::default()));
    let twice = vec![
        WindowDescriptor::new(WindowId(1), options("a"), Second),
        WindowDescriptor::new(WindowId(1), options("b"), Second),
    ];
    assert!(matches!(
        platform::run_windows(twice, ExitPolicy::LastWindowClosed),
        Err(Error::DuplicateWindowId(_))
    ));
    let invalid = WindowOptions {
        width: 0,
        ..options("c")
    };
    assert!(matches!(
        platform::run(invalid, Second),
        Err(Error::InvalidWindowOptions { .. })
    ));
    // Positions des contrôles : tailles fixes, mesurées sans fenêtre.
    let headless = HeadlessApp::with_options(
        Probe::new(Arc::new(Mutex::new(Log::default())), (0., 0.), (0., 0.)),
        HeadlessOptions {
            width: WIDTH as f32,
            height: HEIGHT as f32,
            ..Default::default()
        },
    )
    .expect("vue de la sonde");
    let (ok, field) = (centre(&headless, "ok"), centre(&headless, "name"));
    LOG.with(|l| *l.borrow_mut() = Some(Arc::clone(&log)));
    // Chemin par défaut (`RenderMode::Auto`) : Metal dès qu'un périphérique existe.
    let window = options("sonde");
    assert_eq!(window.renderer, RenderMode::Auto);
    let result = platform::run(window, Probe::new(Arc::clone(&log), ok, field));
    let log = log.lock().unwrap();
    println!("sonde macOS : {log:?}");
    assert!(result.is_ok(), "{result:?}");
    assert_eq!(log.loads, 1);
    // `active_at_start` reste au journal sans être exigé : macOS peut passer le premier
    // plan à une application qui démarre quand le processus qui la lance l'a (M7 vérifié à
    // la main : faux sans fenêtre à activer, vrai si l'activation est forcée).
    assert!(log.painted);
    assert_eq!(log.clicks, 2, "clic sur le bouton, puis appui accessible");
    assert!(log.ax_button, "bouton accessible (AXButton « OK »)");
    assert_eq!(log.ax_value, "ab\u{e9}", "valeur accessible du champ");
    assert!(log.ax_pressed, "appui accessible accepté");
    assert_eq!(log.name, "ab\u{e9}", "frappes puis composition validée");
    assert!(log.composing_seen, "texte marqué pendant la composition");
    assert!(log.second_opened && log.second_closed, "seconde fenêtre");
    assert!(
        log.dark.ends_with(&[true, false]),
        "thème système suivi : sombre puis clair ({:?})",
        log.dark
    );
    assert!(
        log.nested
            .as_deref()
            .is_some_and(|e| e.contains("AlreadyRunning")),
        "{:?}",
        log.nested
    );
    // Sans GPU Metal (machine virtuelle), la fenêtre reste au chemin Core Graphics.
    if objc2_metal::MTLCopyAllDevices().count() == 0 {
        println!("sonde macOS : Metal indisponible, chemin GPU non vérifié");
    } else {
        assert_eq!(
            log.metal,
            [Some(true), Some(false), Some(true)],
            "Metal (vue filtrée), puis Core Graphics (repli forcé), puis Metal (émoji rempli compris)"
        );
        assert_eq!(log.docked, Some(1.), "drawables abandonnés dans le Dock");
        // ~0,9 s dans le Dock : quelques millisecondes si la boucle dort, toute la durée
        // si elle attend un battement qui ne vient pas (M1 de la revue).
        assert!(
            log.docked_cpu.is_some_and(|ms| ms < 300),
            "boucle endormie dans le Dock : {:?} ms de processeur",
            log.docked_cpu
        );
        assert_eq!(log.restored, Some(true), "image refaite au retour du Dock");
        assert_eq!(
            log.covered,
            Some((false, Some(1.))),
            "fenêtre couverte : ni visible pour AppKit, ni repeinte"
        );
        assert_eq!(
            log.resized,
            (Some((500., 360.)), Some(true)),
            "taille demandée appliquée et image refaite à cette taille"
        );
        assert_eq!(
            log.backends,
            ["GPU · Metal", "CPU · Core Graphics", "GPU · Metal"],
            "libellés du renderer vus par la vue"
        );
    }
    FINISHED.store(true, std::sync::atomic::Ordering::SeqCst);
    println!("sonde macOS : ok");
}
