//! Application de sonde et scénario.
use crate::layers::metal_shown;
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
    window::{ExitPolicy, RenderMode, WindowCommand, WindowDescriptor, WindowId, WindowOptions},
};
use objc2::{MainThreadMarker, Message, msg_send, rc::Retained, runtime::AnyObject, sel};
use objc2_app_kit::{NSApplication, NSEvent, NSEventModifierFlags, NSEventType, NSWindow};
use objc2_foundation::{NSObjectNSDelayedPerforming, NSPoint, NSString};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

const WIDTH: i32 = 420;
const HEIGHT: i32 = 300;

#[derive(Default, Debug)]
struct Log {
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
        self.log.lock().unwrap().steps = step as u32;
        let window = window(WindowId::MAIN);
        match step {
            1 => click(&window, self.ok),
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
            9 => {
                self.log.lock().unwrap().metal[2] = metal_shown(&window);
                let nested = platform::run_windows(
                    vec![WindowDescriptor::new(WindowId(9), options("x"), Second)],
                    ExitPolicy::LastWindowClosed,
                );
                self.log.lock().unwrap().nested = nested.err().map(|e| format!("{e:?}"));
                self.commands.push(WindowCommand::Exit);
                return true;
            }
            _ => return true,
        }
        self.next(Duration::from_millis(250));
        true
    }
}
/// Seconde fenêtre, sans contenu interactif.
struct Second;
impl Application for Second {
    type Message = ();
    fn view(&self, _: &ViewContext) -> Element {
        hide_windows();
        column("second", [text("t", "Seconde fenêtre")])
    }
    fn update(&mut self, _: ()) -> bool {
        false
    }
}
thread_local! {
    static LOG: std::cell::RefCell<Option<Arc<Mutex<Log>>>> = const { std::cell::RefCell::new(None) };
}
fn mtm() -> MainThreadMarker {
    MainThreadMarker::new().expect("la sonde tourne sur le thread principal")
}
fn windows() -> Vec<Retained<NSWindow>> {
    NSApplication::sharedApplication(mtm()).windows().to_vec()
}
/// Fenêtre visible de ce titre (`NSApp.windows` liste aussi des fenêtres d'AppKit et des
/// fenêtres fermées pas encore libérées).
fn visible(title: &str) -> bool {
    windows()
        .iter()
        .any(|w| w.isVisible() && w.title().to_string() == title)
}
fn hide_windows() {
    for window in windows() {
        window.setAlphaValue(0.);
    }
}
fn window(_: WindowId) -> Retained<NSWindow> {
    windows()
        .into_iter()
        .find(|w| w.title().to_string() == "sonde")
        .expect("fenêtre de la sonde")
}
type Job = Box<dyn FnOnce()>;
thread_local! {
    static QUEUE: std::cell::RefCell<Vec<Job>> = const { std::cell::RefCell::new(Vec::new()) };
    static TIMER: Retained<Timer> = Timer::new(mtm());
}
objc2::define_class!(
    // SAFETY : NSObject n'impose rien ; pas de `Drop`.
    #[unsafe(super(objc2_foundation::NSObject))]
    #[thread_kind = objc2::MainThreadOnly]
    #[name = "KarbonUiProbeTimer"]
    struct Timer;

    impl Timer {
        #[unsafe(method(fire:))]
        fn fire(&self, _argument: Option<&AnyObject>) {
            let jobs = QUEUE.with(|q| std::mem::take(&mut *q.borrow_mut()));
            for job in jobs {
                job();
            }
        }
    }
);
impl Timer {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = objc2::MainThreadOnly::alloc(mtm).set_ivars(());
        // SAFETY : initialiseur de NSObject.
        unsafe { msg_send![super(this), init] }
    }
}
/// Exécute `job` au prochain tour de la boucle AppKit, hors du rappel en cours (la
/// session de la fenêtre y est libre), comme un événement du système.
fn deferred(job: impl FnOnce() + 'static) {
    QUEUE.with(|q| q.borrow_mut().push(Box::new(job)));
    // SAFETY : `fire:` est défini ci-dessus ; aucun argument.
    TIMER.with(|t| unsafe { t.performSelector_withObject_afterDelay(sel!(fire:), None, 0.) });
}
/// Éléments accessibles sous `node`, en profondeur.
fn collect(node: &AnyObject, out: &mut Vec<Retained<AnyObject>>) {
    // SAFETY : `accessibilityChildren` renvoie un NSArray ou nil.
    let children: Option<Retained<objc2_foundation::NSArray<AnyObject>>> =
        unsafe { msg_send![node, accessibilityChildren] };
    for child in children.map(|c| c.to_vec()).unwrap_or_default() {
        collect(&child, out);
        out.push(child);
    }
}
/// Texte d'un accesseur NSAccessibility (`NSString` ou rien).
fn ax_text(node: &AnyObject, selector: objc2::runtime::Sel) -> String {
    // SAFETY : accesseur sans argument renvoyant un objet ou nil.
    let value: Option<Retained<AnyObject>> = unsafe { msg_send![node, performSelector: selector] };
    value
        .and_then(|v| v.downcast::<NSString>().ok())
        .map(|s| s.to_string())
        .unwrap_or_default()
}
fn send(window: &NSWindow, event: Retained<NSEvent>) {
    let window = window.retain();
    deferred(move || window.sendEvent(&event));
}
fn point(window: &NSWindow, (x, y): (f32, f32)) -> NSPoint {
    let height = window.contentView().map_or(0., |v| v.bounds().size.height);
    NSPoint::new(f64::from(x), height - f64::from(y))
}
fn click(window: &NSWindow, at: (f32, f32)) {
    for kind in [NSEventType::LeftMouseDown, NSEventType::LeftMouseUp] {
        let event = NSEvent::mouseEventWithType_location_modifierFlags_timestamp_windowNumber_context_eventNumber_clickCount_pressure(
            kind,
            point(window, at),
            NSEventModifierFlags(0),
            0.,
            window.windowNumber(),
            None,
            0,
            1,
            1.,
        )
        .expect("événement souris");
        send(window, event);
    }
}
fn key(window: &NSWindow, characters: &str, code: u16) {
    for kind in [NSEventType::KeyDown, NSEventType::KeyUp] {
        let c = NSString::from_str(characters);
        let event = NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(
            kind,
            NSPoint::new(0., 0.),
            NSEventModifierFlags(0),
            0.,
            window.windowNumber(),
            None,
            &c,
            &c,
            false,
            code,
        )
        .expect("événement clavier");
        send(window, event);
    }
}
fn options(title: &'static str) -> WindowOptions {
    WindowOptions {
        title,
        width: WIDTH,
        height: HEIGHT,
        min_width: 200,
        min_height: 200,
        title_bar: false,
        activate: false,
        ..WindowOptions::DEFAULT
    }
}
fn centre(app: &HeadlessApp<Probe>, id: &str) -> (f32, f32) {
    let r = app.bounds(id).expect("contrôle présent");
    ((r.left + r.right) / 2., (r.top + r.bottom) / 2.)
}
pub fn run() {
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
    assert!(log.painted);
    assert_eq!(log.clicks, 2, "clic sur le bouton, puis appui accessible");
    assert!(log.ax_button, "bouton accessible (AXButton « OK »)");
    assert_eq!(log.ax_value, "ab\u{e9}", "valeur accessible du champ");
    assert!(log.ax_pressed, "appui accessible accepté");
    assert_eq!(log.name, "ab\u{e9}", "frappes puis composition validée");
    assert!(log.composing_seen, "texte marqué pendant la composition");
    assert!(log.second_opened && log.second_closed, "seconde fenêtre");
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
        assert_eq!(
            log.backends,
            ["GPU · Metal", "CPU · Core Graphics", "GPU · Metal"],
            "libellés du renderer vus par la vue"
        );
    }
    println!("sonde macOS : ok");
}
