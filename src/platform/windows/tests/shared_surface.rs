//! Surface partagée (`ui::shared_surface`) sur des fenêtres cachées : le gabarit peint
//! dans une fenêtre composée lui donne une couche et sa place ; dans une fenêtre
//! « layered », il se peint tel quel. Sans carte graphique matérielle, rien n'est
//! vérifié.
use super::super::{
    ffi::scale,
    present::{composition::Path, runtime, surface_visual},
    window::{state, take_error},
};
use super::*;
use crate::{
    ui::{
        element::{Length, PaintContext, SizeF, TextMeasurer, Widget},
        scene::{DrawCommand, Scene},
        shared_surface::SharedSurface,
    },
    window::PresentMode,
};
use windows_sys::Win32::UI::WindowsAndMessaging::DestroyWindow;

/// L'élément où la surface doit paraître : il peint son gabarit.
struct Hole(SharedSurface);
impl Widget for Hole {
    fn measure(&self, _: SizeF, _: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
        Ok(SizeF::default())
    }
    fn paint(&self, context: &PaintContext, scene: &mut Scene) {
        scene.commands.push(DrawCommand::Image {
            rect: context.rect,
            image: self.0.placeholder(),
        });
    }
}

struct Viewer(SharedSurface);
impl Application for Viewer {
    type Message = ();
    fn update(&mut self, _: ()) -> bool {
        false
    }
    fn view(&self, _: &ViewContext) -> Element {
        Element::column(
            1,
            vec![
                widgets::text(2, "Vue").height(Length::Px(24.)),
                Element::custom(3, Hole(self.0.clone()))
                    .width(Length::Px(120.))
                    .height(Length::Px(80.)),
            ],
        )
    }
}

fn open(session: &Session, id: u64, surface: &SharedSurface, mode: PresentMode) -> Hwnd {
    let options = WindowOptions {
        title: "Karbon UI - surface partagée cachée",
        width: 300,
        height: 220,
        min_width: 200,
        min_height: 160,
        activate: false,
        renderer: RenderMode::Software,
        presentation: mode,
        ..WindowOptions::DEFAULT
    };
    let viewer = Viewer(surface.clone());
    session
        .open(WindowDescriptor::new(WindowId(id), options, viewer), false)
        .unwrap()
}

#[test]
fn a_composed_window_shows_the_surface_under_its_element() {
    let Ok(surface) = SharedSurface::new(None, 64, 48) else {
        return;
    };
    let (_apartment, session) = session();
    let window = open(&session, 1, &surface, PresentMode::Composition);
    let _scope = state::enter(window);
    assert!(matches!(
        runtime::presentation_state(),
        Some((Path::Composition, Some((_, _, true))))
    ));
    // La première peinture a reconnu le gabarit : la fenêtre a sa couche, et la surface
    // connaît la place de l'élément, en pixels.
    assert!(surface_visual::shown_in(window));
    let placed = surface.placement().expect("surface montrée");
    let pixels = |length: f32| (length * scale(window)).round() as u32;
    assert_eq!((placed.width, placed.height), (pixels(120.), pixels(80.)));
    assert!(placed.x >= 0 && placed.y >= pixels(24.) as i32);
    // Une autre taille de texture ne change pas la place ; et sans image du producteur,
    // rien n'est montré : la texture ne s'obtient pas.
    surface.resize(96, 64).unwrap();
    assert_eq!(surface.size(), (96, 64));
    assert!(matches!(
        surface.present(),
        Err(crate::Error::TimedOut { .. })
    ));
    // SAFETY : fenêtre de test de ce thread ; aucun pointeur dans wparam/lparam.
    unsafe { SendMessageW(window, WM_PAINT, 0, 0) };
    assert_eq!(surface.placement(), Some(placed));
    assert!(take_error().is_none());
    // SAFETY : fenêtre de test créée sur ce thread ; Windows valide le handle.
    assert_ne!(unsafe { DestroyWindow(window) }, 0);
    // La fenêtre fermée a défait sa couche : la surface n'est plus montrée nulle part.
    assert!(!surface_visual::shown_in(window));
    assert_eq!(surface.placement(), None);
}

#[test]
fn a_layered_window_paints_the_placeholder_instead() {
    let Ok(surface) = SharedSurface::new(None, 64, 48) else {
        return;
    };
    let (_apartment, session) = session();
    let window = open(&session, 1, &surface, PresentMode::Layered);
    let _scope = state::enter(window);
    assert_eq!(runtime::presentation_state(), Some((Path::Layered, None)));
    assert!(!surface_visual::shown_in(window));
    assert_eq!(surface.placement(), None);
    assert!(take_error().is_none());
    // SAFETY : fenêtre de test créée sur ce thread ; Windows valide le handle.
    assert_ne!(unsafe { DestroyWindow(window) }, 0);
}
