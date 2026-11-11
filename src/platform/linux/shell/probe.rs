//! Sonde d'une boucle de fenêtres Linux (Wayland, X11), sur une vraie connexion au système
//! de fenêtres : erreurs de validation, fenêtre affichée et peinte, tâches de fond qui
//! réveillent la boucle, seconde fenêtre ouverte puis fermée par commande, boucle déjà en
//! cours, sortie. Les entrées (pointeur, clavier) ne peuvent pas être simulées par un
//! client ordinaire : elles se vérifient à la main.
use crate::{
    Error,
    application::{Application, ViewContext},
    task::{Task, TaskCommand, TaskId, TaskResult},
    ui::element::Element,
    window::{ExitPolicy, WindowCommand, WindowDescriptor, WindowId, WindowOptions},
};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

/// Boucle sondée (`run_windows` d'un backend).
pub(in crate::platform::linux) type Run =
    fn(Vec<WindowDescriptor>, ExitPolicy) -> crate::Result<()>;

#[derive(Default, Debug)]
struct Log {
    /// Vues construites par fenêtre (1 : principale, 2 : seconde).
    views: [u32; 2],
    /// Plus grande échelle vue par la fenêtre principale.
    scale: f32,
    tasks: u32,
    nested: Option<String>,
}
struct Probe {
    log: Arc<Mutex<Log>>,
    run: Run,
    tasks: Vec<TaskCommand>,
    commands: Vec<WindowCommand>,
    step: u64,
    /// Tâches d'attente supplémentaires (seconde fenêtre pas encore affichée).
    waits: u64,
}
/// Attentes au plus que la seconde fenêtre soit affichée (40 × 150 ms).
const MAX_WAITS: u64 = 40;
impl Probe {
    fn next(&mut self) {
        self.step += 1;
        self.wait();
    }
    /// Tâche de 150 ms sous un identifiant neuf, sans changer d'étape.
    fn wait(&mut self) {
        let id = TaskId(self.step + 1000 * self.waits);
        self.tasks.push(TaskCommand::Spawn(Task::new(id, |_| {
            std::thread::sleep(Duration::from_millis(150));
        })));
    }
}
impl Application for Probe {
    type Message = ();
    fn update(&mut self, _: ()) -> bool {
        false
    }
    fn load(&mut self, _: &dyn crate::application::Resources) -> crate::Result<()> {
        self.next();
        Ok(())
    }
    fn task_completed(&mut self, _: TaskResult) -> bool {
        // Seconde fenêtre pas encore configurée par le compositeur (machine chargée) :
        // la fermer maintenant ne laisserait voir aucune de ses vues.
        if self.step == 2 && self.log.lock().unwrap().views[1] == 0 && self.waits < MAX_WAITS {
            self.waits += 1;
            self.wait();
            return true;
        }
        self.log.lock().unwrap().tasks += 1;
        match self.step {
            1 => self
                .commands
                .push(WindowCommand::Open(WindowDescriptor::new(
                    WindowId(2),
                    options("sonde 2"),
                    Second(Arc::clone(&self.log)),
                ))),
            2 => self.commands.push(WindowCommand::Close(WindowId(2))),
            _ => {
                let nested = (self.run)(
                    vec![WindowDescriptor::new(
                        WindowId(9),
                        options("x"),
                        Second(Arc::clone(&self.log)),
                    )],
                    ExitPolicy::LastWindowClosed,
                );
                self.log.lock().unwrap().nested = nested.err().map(|e| format!("{e:?}"));
                self.commands.push(WindowCommand::Exit);
                return true;
            }
        }
        self.next();
        true
    }
    fn take_task_commands(&mut self) -> Vec<TaskCommand> {
        std::mem::take(&mut self.tasks)
    }
    fn take_window_commands(&mut self) -> Vec<WindowCommand> {
        std::mem::take(&mut self.commands)
    }
    fn view(&self, context: &ViewContext) -> Element {
        let mut log = self.log.lock().unwrap();
        log.views[0] += 1;
        log.scale = log.scale.max(context.scale);
        crate::ui::widgets::button(1, "Sonde", ())
    }
}
struct Second(Arc<Mutex<Log>>);
impl Application for Second {
    type Message = ();
    fn update(&mut self, _: ()) -> bool {
        false
    }
    fn view(&self, _: &ViewContext) -> Element {
        self.0.lock().unwrap().views[1] += 1;
        crate::ui::widgets::button(1, "Seconde fenêtre", ())
    }
}
fn options(title: &'static str) -> WindowOptions {
    WindowOptions {
        title,
        width: 320,
        height: 240,
        min_width: 200,
        min_height: 150,
        ..WindowOptions::DEFAULT
    }
}
/// Sonde de la boucle `run` du système `name` : rien si `available` est faux (sautée en
/// l'écrivant, échec si la variable `require` est posée).
pub(in crate::platform::linux) fn probe(run: Run, name: &str, available: bool, require: &str) {
    // Erreurs de validation, avant toute connexion.
    assert!(matches!(
        run(Vec::new(), ExitPolicy::LastWindowClosed),
        Err(Error::InvalidInput { .. })
    ));
    let log = Arc::new(Mutex::new(Log::default()));
    let twice = vec![
        WindowDescriptor::new(WindowId(1), options("a"), Second(Arc::clone(&log))),
        WindowDescriptor::new(WindowId(1), options("b"), Second(Arc::clone(&log))),
    ];
    assert!(matches!(
        run(twice, ExitPolicy::LastWindowClosed),
        Err(Error::DuplicateWindowId(_))
    ));
    if !available {
        assert!(
            std::env::var_os(require).is_none(),
            "{require} : {name} absent"
        );
        eprintln!("sonde {name} sautée : système de fenêtres absent");
        return;
    }
    let probe = Probe {
        log: Arc::clone(&log),
        run,
        tasks: Vec::new(),
        commands: Vec::new(),
        step: 0,
        waits: 0,
    };
    let start = Instant::now();
    run(
        vec![WindowDescriptor::new(WindowId(1), options("sonde"), probe)],
        ExitPolicy::LastWindowClosed,
    )
    .unwrap();
    let log = log.lock().unwrap();
    assert!(start.elapsed() < Duration::from_secs(20), "{log:?}");
    assert_eq!(log.tasks, 3, "{log:?}");
    assert!(log.views[0] >= 1 && log.views[1] >= 1, "{log:?}");
    assert!(log.scale >= 1., "{log:?}");
    assert!(
        log.nested
            .as_deref()
            .is_some_and(|e| e.contains("AlreadyRunning")),
        "{log:?}"
    );
}
