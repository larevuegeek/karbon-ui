//! Le fil de la vue 3D factice : une image de 1740 × 1176 pixels, différente à chaque
//! fois, livrée 60 fois par seconde par un `ImageSlot`, comme le fil du rendu d'un moteur
//! en mode copie. Et les petites images des vignettes, faites une fois.
use crate::Bench;
use karbon_ui::{
    task::InboxSender,
    ui::scene::{Image, LiveImage},
    ui::template::ImageSlot,
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

/// Taille de l'image de la vue 3D, celle de la vue de l'éditeur de karbon-engine.
pub const VIEW_WIDTH: u32 = 1740;
pub const VIEW_HEIGHT: u32 = 1176;
/// Cadence de la vue 3D.
const PERIOD: Duration = Duration::from_nanos(16_666_667);

/// Ce que la vue 3D livre à chaque image.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Feed {
    /// Une image neuve, différente de la précédente : le cas réel.
    Live,
    /// La même image livrée de nouveau : la fenêtre repeint, sans nouvelle texture.
    Still,
}

/// Comment la vue livre ses images.
#[derive(Clone)]
pub enum Delivery {
    /// `Image::new` (chaque pixel vérifié) dans un `ImageSlot` : un bitmap neuf par image.
    Checked(ImageSlot),
    /// `Image::opaque_bgra` dans une `LiveImage` : le bitmap est mis à jour en place.
    Live(LiveImage),
}

impl Delivery {
    /// La source que la vue peint.
    pub fn slot(&self) -> &ImageSlot {
        match self {
            Self::Checked(slot) => slot,
            Self::Live(live) => live.slot(),
        }
    }
}

/// Livre des images jusqu'à la fin de l'application, et prévient l'application de
/// chacune avec le temps passé à la faire.
pub fn run(delivery: Delivery, window: InboxSender<Bench>, feed: Feed) {
    let mut next = Instant::now();
    let mut still: Option<Arc<Image>> = None;
    for frame in 0u32.. {
        let started = Instant::now();
        let image = match (&still, feed) {
            (Some(image), Feed::Still) => Arc::clone(image),
            _ => Arc::new(view_image(frame, matches!(delivery, Delivery::Live(_)))),
        };
        if feed == Feed::Still {
            still = Some(Arc::clone(&image));
        }
        match &delivery {
            Delivery::Checked(slot) => slot.set(image),
            Delivery::Live(live) => live.set(image),
        }
        let made = started.elapsed();
        if !window.post(move |bench| bench.delivered(made)) {
            return;
        }
        next += PERIOD;
        let now = Instant::now();
        if next > now {
            std::thread::sleep(next - now);
        } else {
            next = now; // En retard : on repart de maintenant plutôt que de rattraper.
        }
    }
}

/// Une image de la vue : bandes qui défilent et damier qui change, opaques, pour que
/// chaque image diffère de la précédente partout. `opaque` : par `Image::opaque_bgra`,
/// sans vérification de chaque pixel.
pub fn view_image(frame: u32, opaque: bool) -> Image {
    let (width, height) = (VIEW_WIDTH as usize, VIEW_HEIGHT as usize);
    let mut pixels = vec![0u8; width * height * 4];
    let shift = frame as usize;
    for (y, row) in pixels.chunks_exact_mut(width * 4).enumerate() {
        let green = ((y + shift * 2) & 0xFF) as u8;
        for (x, pixel) in row.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            let checker = (((x / 64) ^ (y / 64)) + shift / 8) & 1;
            pixel[0] = ((x + shift * 4) & 0xFF) as u8;
            pixel[1] = green;
            pixel[2] = if checker == 0 { 40 } else { 200 };
            pixel[3] = 255;
        }
    }
    if opaque {
        Image::opaque_bgra(VIEW_WIDTH, VIEW_HEIGHT, pixels).expect("buffer of the right size")
    } else {
        Image::new(VIEW_WIDTH, VIEW_HEIGHT, pixels).expect("opaque pixels are valid")
    }
}

/// La vignette `index` : un dégradé de sa couleur, 96 × 64 pixels.
pub fn thumbnail(index: usize) -> Arc<Image> {
    let (width, height) = (96usize, 64usize);
    let hue = (index * 37 % 256) as u8;
    let mut pixels = Vec::with_capacity(width * height * 4);
    for y in 0..height {
        for x in 0..width {
            pixels.extend_from_slice(&[
                hue,
                (x * 255 / width) as u8,
                (y * 255 / height) as u8,
                255,
            ]);
        }
    }
    Arc::new(Image::new(width as u32, height as u32, pixels).expect("opaque pixels are valid"))
}

/// Nom de la carte graphique que Direct3D prend par défaut (la première que DXGI
/// énumère) : celle de karbon-ui quand le rendu n'est pas logiciel.
#[cfg(windows)]
pub fn default_adapter() -> String {
    use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1};
    // SAFETY : appels DXGI sans pointeur fourni par l'appelant ; les interfaces sont
    // libérées à la sortie de portée.
    let description = unsafe {
        CreateDXGIFactory1::<IDXGIFactory1>()
            .and_then(|factory| factory.EnumAdapters1(0))
            .and_then(|adapter| adapter.GetDesc1())
    };
    match description {
        Ok(description) => {
            let name = String::from_utf16_lossy(&description.Description);
            name.trim_end_matches('\0').to_string()
        }
        Err(error) => format!("inconnue ({error})"),
    }
}

/// Hors de Windows, le banc ne sait pas nommer la carte.
#[cfg(not(windows))]
pub fn default_adapter() -> String {
    "inconnue (hors Windows)".into()
}
