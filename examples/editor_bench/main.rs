//! Banc « éditeur » : une interface d'éditeur chargée (arbre de 100 000 objets, 200
//! propriétés, 100 vignettes, frise, ligne d'état) autour d'une vue 3D factice qu'un fil
//! remplit 60 fois par seconde d'une image de 1740 × 1176 pixels, en mode copie
//! (`ImageSlot`). Il mesure le temps du fil de l'interface par image avec le profilage
//! de la fenêtre (`--profile`, toujours actif ici), puis se ferme de lui-même.
//!
//! ```powershell
//! cargo run --release --example editor_bench -- --seconds 20 --feed live
//! ```
//!
//! Options du banc : `--seconds N` (durée, 20 par défaut), `--feed live|still` (`live` :
//! une image neuve à chaque fois ; `still` : la même image relivrée, la fenêtre repeint
//! sans nouvelle texture), `--size LxH` (taille de la fenêtre en unités, 1600×1000 par
//! défaut), `--image checked|live` (`checked` : `Image::new` dans un `ImageSlot`, un
//! bitmap neuf par image ; `live` : `Image::opaque_bgra` dans une `LiveImage`, bitmap mis
//! à jour en place, KUI-002). Les autres options sont celles de `WindowOptions::from_args`
//! (`--renderer=gpu`…). Le rapport `Presentation : {…}` de la fermeture donne les temps ;
//! le banc écrit ensuite son propre résumé. `--headless N` mesure N images dans la
//! fenêtre simulée, sans écran (préparation seule). Mesures : docs/performance.md.
mod headless;
mod panels;
mod producer;

use karbon_ui::{
    platform,
    task::Inbox,
    ui::{
        prelude::*,
        scene::{Image, LiveImage},
        template::ImageSlot,
    },
    window::{WindowCommand, WindowOptions},
};
use producer::{Delivery, Feed};
use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

/// Intervalle entre deux mises à jour de la ligne d'état (une reconstruction de la vue),
/// comme dans l'éditeur de karbon-engine.
const STATUS_EVERY: Duration = Duration::from_millis(500);

/// Images livrées et temps passé à les faire (microsecondes), lus après la fermeture.
static DELIVERED: AtomicU64 = AtomicU64::new(0);
static MAKING_US: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone)]
pub enum Msg {
    Tool(usize),
    Edit(usize, String),
    Toggle(usize, bool),
    Slide(usize, f64),
}

pub struct Bench {
    inbox: Inbox<Bench>,
    slot: ImageSlot,
    thumbnails: Vec<Arc<Image>>,
    selected: usize,
    values: Vec<String>,
    checks: Vec<bool>,
    slides: Vec<f64>,
    status: String,
    /// Images reçues depuis la dernière ligne d'état.
    frames: u32,
    since: Instant,
    started: Instant,
    duration: Duration,
    commands: Vec<WindowCommand>,
}

impl Bench {
    fn new(slot: ImageSlot, inbox: Inbox<Bench>, duration: Duration) -> Self {
        let count = panels::PROPERTIES;
        Self {
            inbox,
            slot,
            thumbnails: (0..panels::THUMBNAILS).map(producer::thumbnail).collect(),
            selected: 42,
            values: (0..count)
                .map(|i| format!("{:.3}", i as f32 * 0.125))
                .collect(),
            checks: (0..count).map(|i| i % 3 == 0).collect(),
            slides: (0..count).map(|i| (i % 100) as f64).collect(),
            status: "Banc éditeur : en route".into(),
            frames: 0,
            since: Instant::now(),
            started: Instant::now(),
            duration,
            commands: Vec::new(),
        }
    }

    /// Une image de la vue 3D est arrivée (`made` : le temps du fil de la vue pour la
    /// faire). Rend `true` quand la ligne d'état change, ce qui reconstruit la vue.
    pub fn delivered(&mut self, made: Duration) -> bool {
        DELIVERED.fetch_add(1, Ordering::Relaxed);
        MAKING_US.fetch_add(made.as_micros() as u64, Ordering::Relaxed);
        self.frames += 1;
        if self.started.elapsed() >= self.duration && self.commands.is_empty() {
            self.commands.push(WindowCommand::Exit);
        }
        let elapsed = self.since.elapsed();
        if elapsed < STATUS_EVERY {
            return false;
        }
        self.status = format!(
            "{} objets · {} propriétés · vue {}×{} · {:.1} images par seconde",
            panels::OBJECTS,
            panels::PROPERTIES,
            producer::VIEW_WIDTH,
            producer::VIEW_HEIGHT,
            f64::from(self.frames) / elapsed.as_secs_f64()
        );
        self.frames = 0;
        self.since = Instant::now();
        true
    }
}

impl Application for Bench {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        panels::editor(self)
    }
    fn update(&mut self, message: Msg) -> bool {
        match message {
            Msg::Tool(index) => self.selected = index,
            Msg::Edit(index, value) => self.values[index] = value,
            Msg::Toggle(index, on) => self.checks[index] = on,
            Msg::Slide(index, value) => self.slides[index] = value,
        }
        true
    }
    fn inbox(&self) -> Option<Inbox<Self>> {
        Some(self.inbox.clone())
    }
    fn take_window_commands(&mut self) -> Vec<WindowCommand> {
        std::mem::take(&mut self.commands)
    }
}

/// Options propres au banc, et le reste pour la fenêtre.
struct Settings {
    duration: Duration,
    feed: Feed,
    live: bool,
    size: (i32, i32),
    window: Vec<String>,
    headless: Option<usize>,
}

fn settings() -> Result<Settings, String> {
    let invalid = |text: String| format!("invalid option: {text}");
    let mut settings = Settings {
        duration: Duration::from_secs(20),
        feed: Feed::Live,
        live: false,
        size: (1600, 1000),
        window: Vec::new(),
        headless: None,
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = || {
            args.next()
                .ok_or_else(|| invalid(format!("{arg} needs a value")))
        };
        match arg.as_str() {
            "--seconds" => {
                let seconds: f64 = value()?.parse().map_err(|_| invalid("--seconds".into()))?;
                settings.duration = Duration::from_secs_f64(seconds.max(1.));
            }
            "--feed" => {
                settings.feed = match value()?.as_str() {
                    "live" => Feed::Live,
                    "still" => Feed::Still,
                    other => return Err(invalid(format!("--feed {other}"))),
                }
            }
            "--image" => {
                settings.live = match value()?.as_str() {
                    "checked" => false,
                    "live" => true,
                    other => return Err(invalid(format!("--image {other}"))),
                }
            }
            "--headless" => {
                let frames = value()?.parse().map_err(|_| invalid("--headless".into()))?;
                settings.headless = Some(frames);
            }
            "--size" => {
                let text = value()?;
                let (width, height) = text
                    .split_once('x')
                    .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))
                    .ok_or_else(|| invalid(format!("--size {text}")))?;
                settings.size = (width, height);
            }
            _ => settings.window.push(arg),
        }
    }
    Ok(settings)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let settings = settings()?;
    if let Some(frames) = settings.headless {
        return headless::run(frames, settings.size);
    }
    let options = WindowOptions::from_args(settings.window.iter().cloned())?;
    let delivery = if settings.live {
        Delivery::Live(LiveImage::new())
    } else {
        Delivery::Checked(ImageSlot::new())
    };
    let slot = delivery.slot().clone();
    let inbox = Inbox::new();
    let sender = inbox.sender();
    let feed = settings.feed;
    let live = settings.live;
    let producer = std::thread::spawn(move || producer::run(delivery, sender, feed));
    let bench = Bench::new(slot, inbox, settings.duration);
    let started = Instant::now();
    platform::run(
        WindowOptions {
            title: "Karbon UI · banc éditeur",
            width: settings.size.0,
            height: settings.size.1,
            min_width: 800,
            min_height: 600,
            profile: true,
            ..options
        },
        bench,
    )?;
    let wall = started.elapsed().as_secs_f64();
    let _ = producer.join();
    let delivered = DELIVERED.load(Ordering::Relaxed);
    let making = MAKING_US.load(Ordering::Relaxed) as f64 / 1000. / delivered.max(1) as f64;
    println!(
        "Banc éditeur : vue {}×{} ({}, {}), {delivered} images livrées en {wall:.1} s ({:.1} par seconde), {making:.2} ms par image sur le fil de la vue, carte Direct3D par défaut (celle de karbon-ui hors rendu logiciel) : {}",
        producer::VIEW_WIDTH,
        producer::VIEW_HEIGHT,
        match feed {
            Feed::Live => "image neuve à chaque fois",
            Feed::Still => "même image relivrée",
        },
        if live {
            "Image::opaque_bgra dans une LiveImage"
        } else {
            "Image::new dans un ImageSlot"
        },
        delivered as f64 / wall,
        producer::default_adapter()
    );
    Ok(())
}
