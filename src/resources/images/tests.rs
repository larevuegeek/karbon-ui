//! `ImageLoader` de bout en bout dans le harnais headless : la vue demande, le runtime
//! lance les tâches et livre les images sans passer par l'application.
use super::ImageLoader;
use crate::platform::headless::HeadlessApp;
use crate::resources::{ImageDecoder, ImageLoadOptions};
use crate::ui::{prelude::*, scene::Image, template::ImageSlot};
use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Décodeur de test : une image 3 × 3 dont le premier octet est la longueur des données.
struct Fake;
impl ImageDecoder for Fake {
    fn decode(&self, bytes: &[u8], _: ImageLoadOptions) -> crate::Result<Image> {
        Image::new(3, 3, [bytes.len() as u8, 0, 0, 255].repeat(9))
    }
}

/// `data:` de trois octets (`AAEC`), une autre de six, et un fichier absent.
const THREE: &str = "data:image/png;base64,AAEC";
const SIX: &str = "data:image/png;base64,AAECAwQF";
const MISSING: &str = "nowhere/cover.jpg";

struct Gallery {
    images: ImageLoader,
    slots: RefCell<Vec<ImageSlot>>,
    seen: Cell<Option<bool>>,
    builds: Cell<u32>,
    app_tasks: Cell<u32>,
}
impl Application for Gallery {
    type Message = ();
    fn view(&self, _: &ViewContext) -> Element {
        self.builds.set(self.builds.get() + 1);
        let mut slots = self.slots.borrow_mut();
        slots.clear();
        slots.push(self.images.load(THREE));
        slots.push(self.images.load_sized(MISSING, 64));
        self.seen.set(Some(self.images.get(SIX).is_some()));
        column("page", [])
    }
    fn update(&mut self, _: ()) -> bool {
        false
    }
    fn task_completed(&mut self, _: crate::task::TaskResult) -> bool {
        self.app_tasks.set(self.app_tasks.get() + 1);
        false
    }
    fn image_loader(&self) -> Option<ImageLoader> {
        Some(self.images.clone())
    }
}

#[test]
fn images_load_in_the_background_without_the_application() -> crate::Result<()> {
    let resolved = Arc::new(Mutex::new(Vec::new()));
    let log = resolved.clone();
    let images = ImageLoader::new(Some(Arc::new(Fake)))
        .thumbnail_edge(32)
        .resolve(move |source, edge| {
            log.lock().unwrap().push((source.to_string(), edge));
            PathBuf::from(source)
        });
    let mut app = HeadlessApp::new(Gallery {
        images: images.clone(),
        slots: RefCell::new(Vec::new()),
        seen: Cell::new(None),
        builds: Cell::new(0),
        app_tasks: Cell::new(0),
    })?;
    {
        let gallery = app.app();
        // Premier tour : l'image d'attente transparente, rien en cache.
        assert_eq!(gallery.slots.borrow()[0].image().unwrap().width, 1);
        assert_eq!(gallery.seen.get(), Some(false));
    }
    let builds = app.app().builds.get();
    app.run_tasks()?;
    let gallery = app.app();
    // La source a reçu son image ; `get` a fait reconstruire la vue, qui la trouve.
    let first = gallery.slots.borrow()[0].image().unwrap();
    assert_eq!((first.width, first.pixels[0]), (3, 3));
    assert_eq!(gallery.seen.get(), Some(true));
    assert!(gallery.builds.get() > builds);
    // Le fichier absent garde son image d'attente ; `resolve` a vu sa taille.
    assert_eq!(gallery.slots.borrow()[1].image().unwrap().width, 1);
    assert!(
        resolved
            .lock()
            .unwrap()
            .contains(&(MISSING.to_string(), 64))
    );
    // Les adresses `data:` ne passent pas par `resolve`, et l'application n'a rien reçu.
    assert!(
        resolved
            .lock()
            .unwrap()
            .iter()
            .all(|(s, _)| !s.starts_with("data:"))
    );
    assert_eq!(gallery.app_tasks.get(), 0);
    assert_eq!(images.peek(SIX).unwrap().pixels[0], 6);
    Ok(())
}

#[test]
fn cancelled_decodes_are_requested_again() {
    let images = ImageLoader::new(Some(Arc::new(Fake))).max_in_flight(1);
    images.preload([THREE, SIX]);
    // Une seule tâche à la fois : la seconde attend.
    assert_eq!(images.take_commands().len(), 1);
    assert!(images.take_commands().is_empty());
    // L'application annule tout : la tâche en vol est redemandée.
    images.cancelled_all();
    assert_eq!(images.take_commands().len(), 1);
}

#[test]
fn a_loader_without_decoder_shows_its_placeholder() {
    let placeholder = Arc::new(Image::new(2, 2, vec![0; 16]).unwrap());
    let images = ImageLoader::new(None).placeholder(placeholder);
    assert_eq!(images.load(THREE).image().unwrap().width, 2);
    assert!(images.take_commands().is_empty());
    assert!(images.load("").image().is_some());
}
