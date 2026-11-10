//! Commandes d'une fenêtre ouverte (`WindowChange`) et état remis à l'application, sur
//! une fenêtre cachée : taille et bornes, style fixe, premier plan, position, maintien
//! dans l'écran, plein écran et refus d'un changement invalide ; poignée de
//! redimensionnement de l'application sur une fenêtre fixe.
use super::super::window::take_error;
use super::*;
use crate::window::{WindowChange, WindowState};
use windows_sys::Win32::{
    Foundation::{POINT, RECT},
    Graphics::Gdi::ClientToScreen,
    UI::WindowsAndMessaging::{
        DestroyWindow, GWL_EXSTYLE, GWL_STYLE, GetClientRect, GetWindowLongPtrW, GetWindowRect,
        HTCAPTION, HTCLIENT, SendMessageW, WM_NCHITTEST, WS_EX_TOPMOST, WS_MAXIMIZEBOX,
        WS_THICKFRAME,
    },
};

struct Watcher {
    commands: Rc<RefCell<Vec<WindowCommand>>>,
    states: Rc<RefCell<Vec<WindowState>>>,
}
impl Application for Watcher {
    type Message = ();
    fn view(&self, _: &ViewContext) -> Element {
        widgets::text(1, "Fenêtre")
    }
    fn update(&mut self, _: ()) -> bool {
        false
    }
    fn take_window_commands(&mut self) -> Vec<WindowCommand> {
        std::mem::take(&mut self.commands.borrow_mut())
    }
    fn window_changed(&mut self, state: WindowState) -> bool {
        self.states.borrow_mut().push(state);
        false
    }
}

fn client_size(window: Hwnd) -> (i32, i32) {
    let mut rect = RECT::default();
    // SAFETY : structure locale valide en écriture ; fenêtre de ce thread.
    unsafe { GetClientRect(window, &mut rect) };
    (rect.right, rect.bottom)
}
fn style(window: Hwnd, index: i32) -> u32 {
    // SAFETY : simple lecture du style d'une fenêtre de ce thread.
    unsafe { GetWindowLongPtrW(window, index) as u32 }
}

#[test]
fn window_changes_apply_natively_and_report_the_new_state() {
    let (_apartment, session) = session();
    let commands = Rc::new(RefCell::new(Vec::new()));
    let states = Rc::new(RefCell::new(Vec::new()));
    let app = Watcher {
        commands: Rc::clone(&commands),
        states: Rc::clone(&states),
    };
    let window = session
        .open(
            WindowDescriptor::new(
                WindowId(1),
                WindowOptions {
                    title: "Karbon UI - changements",
                    width: 300,
                    height: 220,
                    min_width: 200,
                    min_height: 160,
                    activate: false,
                    transparent: true,
                    renderer: RenderMode::Software,
                    ..WindowOptions::DEFAULT
                },
                app,
            ),
            false,
        )
        .unwrap();
    // L'état initial est remis à l'ouverture.
    let first = *states.borrow().first().expect("state reported at opening");
    assert_eq!((first.width, first.height), (300, 220));
    assert!(first.resizable && !first.topmost && !first.fullscreen);
    let scale = first.scale;
    let physical = |v: i32| (v as f32 * scale).round() as i32;

    commands.borrow_mut().extend([
        WindowCommand::Change(WindowId(1), WindowChange::MinSize(None)),
        WindowCommand::Change(
            WindowId(1),
            WindowChange::Size {
                width: 380,
                height: 150,
            },
        ),
        WindowCommand::Change(WindowId(1), WindowChange::Resizable(false)),
        WindowCommand::Change(WindowId(1), WindowChange::Topmost(true)),
        WindowCommand::Change(WindowId(1), WindowChange::Position { x: 40, y: 50 }),
    ]);
    assert!(session.commands(false).unwrap());
    assert!(take_error().is_none());
    assert_eq!(client_size(window), (physical(380), physical(150)));
    assert_eq!(
        style(window, GWL_STYLE) & (WS_THICKFRAME | WS_MAXIMIZEBOX),
        0
    );
    // Le style natif n'est pas vérifié ici : pour remonter une fenêtre, `SetWindowPos`
    // exige le droit de premier plan, que n'a pas toujours un test lancé en arrière-plan
    // (Windows répond alors succès sans rien faire). Voir le test suivant.
    let last = *states.borrow().last().unwrap();
    assert_eq!((last.width, last.height), (380, 150));
    assert_eq!((last.x, last.y), (40, 50));
    assert!(!last.resizable && last.topmost);

    // Bornes : la taille demandée est ramenée au maximum, puis le maximum levé.
    commands.borrow_mut().extend([
        WindowCommand::Change(WindowId(1), WindowChange::MaxSize(Some((220, 220)))),
        WindowCommand::Change(
            WindowId(1),
            WindowChange::Size {
                width: 660,
                height: 660,
            },
        ),
    ]);
    session.commands(false).unwrap();
    assert_eq!(client_size(window), (physical(220), physical(220)));

    // Hors de l'écran : ramenée dans la zone de travail.
    commands.borrow_mut().extend([
        WindowCommand::Change(
            WindowId(1),
            WindowChange::Position {
                x: -30_000,
                y: -30_000,
            },
        ),
        WindowCommand::Change(WindowId(1), WindowChange::KeepOnScreen),
    ]);
    session.commands(false).unwrap();
    let last = *states.borrow().last().unwrap();
    assert!(last.x > -30_000 && last.y > -30_000, "{last:?}");

    // Plein écran puis retour au rectangle d'avant.
    let mut before = RECT::default();
    // SAFETY : structure locale valide en écriture ; fenêtre de ce thread.
    unsafe { GetWindowRect(window, &mut before) };
    commands.borrow_mut().push(WindowCommand::Change(
        WindowId(1),
        WindowChange::MaxSize(None),
    ));
    commands.borrow_mut().push(WindowCommand::Change(
        WindowId(1),
        WindowChange::Fullscreen(true),
    ));
    session.commands(false).unwrap();
    assert!(states.borrow().last().unwrap().fullscreen);
    commands.borrow_mut().push(WindowCommand::Change(
        WindowId(1),
        WindowChange::Fullscreen(false),
    ));
    session.commands(false).unwrap();
    let mut after = RECT::default();
    // SAFETY : voir ci-dessus.
    unsafe { GetWindowRect(window, &mut after) };
    assert_eq!(
        (after.left, after.top, after.right, after.bottom),
        (before.left, before.top, before.right, before.bottom)
    );
    let last = *states.borrow().last().unwrap();
    assert!(!last.fullscreen && !last.resizable);

    // Un changement invalide est une erreur de programmation, comme des options invalides.
    commands.borrow_mut().push(WindowCommand::Change(
        WindowId(1),
        WindowChange::Size {
            width: 0,
            height: 10,
        },
    ));
    assert!(matches!(
        session.commands(false),
        Err(crate::Error::InvalidWindowOptions { .. })
    ));
    // SAFETY : fenêtre de test créée sur ce thread.
    unsafe { DestroyWindow(window) };
}

/// Effet natif de `Topmost`, dans le sens qui ne demande pas le droit de premier plan :
/// le style posé à la création, puis retiré par la commande.
#[test]
fn a_topmost_window_is_lowered_natively() {
    let (_apartment, session) = session();
    let commands = Rc::new(RefCell::new(Vec::new()));
    let states = Rc::new(RefCell::new(Vec::new()));
    let app = Watcher {
        commands: Rc::clone(&commands),
        states: Rc::clone(&states),
    };
    let window = session
        .open(
            WindowDescriptor::new(
                WindowId(1),
                WindowOptions {
                    title: "Karbon UI - premier plan",
                    width: 300,
                    height: 220,
                    activate: false,
                    topmost: true,
                    renderer: RenderMode::Software,
                    ..WindowOptions::DEFAULT
                },
                app,
            ),
            false,
        )
        .unwrap();
    assert_ne!(style(window, GWL_EXSTYLE) & WS_EX_TOPMOST, 0);
    assert!(states.borrow().first().unwrap().topmost);

    commands.borrow_mut().push(WindowCommand::Change(
        WindowId(1),
        WindowChange::Topmost(false),
    ));
    session.commands(false).unwrap();
    assert_eq!(style(window, GWL_EXSTYLE) & WS_EX_TOPMOST, 0);
    assert!(!states.borrow().last().unwrap().topmost);
    // SAFETY : fenêtre de test créée sur ce thread.
    unsafe { DestroyWindow(window) };
}

/// Pochette du micro-lecteur de RustMusic : toute la fenêtre la déplace, sauf la
/// poignée du coin (`data-no-drag`), qui la redimensionne.
struct Micro {
    commands: Rc<RefCell<Vec<WindowCommand>>>,
    states: Rc<RefCell<Vec<WindowState>>>,
}
impl Application for Micro {
    type Message = ();
    fn view(&self, _: &ViewContext) -> Element {
        use crate::{
            ui::view::{column, row, space},
            window::ResizeEdge,
        };
        column(
            1,
            [
                space(2).height(204.),
                row(
                    3,
                    [
                        space(4).width(204.).height(16.),
                        space(5)
                            .width(16.)
                            .height(16.)
                            .no_drag(true)
                            .resize_region(ResizeEdge::BottomRight),
                    ],
                )
                .height(16.),
            ],
        )
        .drag_region(true)
    }
    fn update(&mut self, _: ()) -> bool {
        false
    }
    fn take_window_commands(&mut self) -> Vec<WindowCommand> {
        std::mem::take(&mut self.commands.borrow_mut())
    }
    fn window_changed(&mut self, state: WindowState) -> bool {
        self.states.borrow_mut().push(state);
        false
    }
}
/// Résultat de WM_NCHITTEST pour un point client en pixels physiques.
fn hit(window: Hwnd, (x, y): (i32, i32)) -> u32 {
    let mut point = POINT { x, y };
    // SAFETY : fenêtre de test de ce thread ; `point` est valide en lecture et écriture.
    assert_ne!(unsafe { ClientToScreen(window, &mut point) }, 0);
    let packed = (point.x as u16 as u32 | ((point.y as u16 as u32) << 16)) as isize;
    // SAFETY : fenêtre de test de ce thread ; lparam ne contient que des coordonnées.
    unsafe { SendMessageW(window, WM_NCHITTEST, 0, packed) as u32 }
}
fn top_left(window: Hwnd) -> (i32, i32) {
    let mut rect = RECT::default();
    // SAFETY : structure locale valide en écriture ; fenêtre de ce thread.
    unsafe { GetWindowRect(window, &mut rect) };
    (rect.left, rect.top)
}

#[test]
fn a_resize_handle_grows_a_fixed_square_window_by_the_larger_pointer_move() {
    use super::super::{events::grip, window::state};
    use crate::window::ResizeEdge;
    let (_apartment, session) = session();
    let commands = Rc::new(RefCell::new(Vec::new()));
    let states = Rc::new(RefCell::new(Vec::new()));
    let app = Micro {
        commands: Rc::clone(&commands),
        states: Rc::clone(&states),
    };
    let window = session
        .open(
            WindowDescriptor::new(
                WindowId(1),
                WindowOptions {
                    title: "Karbon UI - poignée",
                    width: 220,
                    height: 220,
                    min_width: 100,
                    min_height: 100,
                    // Fenêtre sans barre du moteur : la pochette occupe tout.
                    title_bar: false,
                    activate: false,
                    renderer: RenderMode::Software,
                    ..WindowOptions::DEFAULT
                },
                app,
            ),
            false,
        )
        .unwrap();
    // Micro-lecteur (`enterMicroPlayer`) : fenêtre fixe, carrée, bornée 220-660.
    commands.borrow_mut().extend(
        [
            WindowChange::Resizable(false),
            WindowChange::MinSize(Some((220, 220))),
            WindowChange::MaxSize(Some((660, 660))),
            WindowChange::AspectRatio(Some((1, 1))),
        ]
        .map(|change| WindowCommand::Change(WindowId(1), change)),
    );
    session.commands(false).unwrap();
    let scale = states.borrow().last().unwrap().scale;
    let px = |v: f32| (v * scale).round() as i32;
    let thick = || style(window, GWL_STYLE) & WS_THICKFRAME;
    assert_eq!(thick(), 0);

    // Poignée : zone cliente (geste suivi par le moteur), prioritaire sur la zone de
    // déplacement qui la contient ; le reste de la pochette déplace la fenêtre, et les
    // bords d'une fenêtre fixe ne redimensionnent pas.
    assert_eq!(hit(window, (px(211.), px(211.))), HTCLIENT);
    assert_eq!(hit(window, (px(110.), px(110.))), HTCAPTION);
    assert_eq!(hit(window, (px(1.), px(110.))), HTCAPTION);
    assert_eq!(hit(window, (px(110.), px(219.))), HTCAPTION);
    assert_eq!(hit(window, (px(219.), px(110.))), HTCAPTION);

    // Geste (`resizeMicroPlayer`) : côté = départ + max(dx, dy) en coordonnées de
    // l'écran, borné 220-660, coin haut-gauche immobile, sans cadre épais.
    let scope = state::enter(window);
    let corner = top_left(window);
    let origin = POINT { x: 900, y: 700 };
    let at = |dx: f32, dy: f32| POINT {
        x: origin.x + px(dx),
        y: origin.y + px(dy),
    };
    assert!(grip::start(window, ResizeEdge::BottomRight, origin));
    assert!(state::sizing());
    assert!(states.borrow().last().unwrap().dragging);
    assert!(grip::follow(window, at(50., -80.)));
    assert_eq!(client_size(window), (px(270.), px(270.)));
    assert!(grip::follow(window, at(10., 40.)));
    assert_eq!(client_size(window), (px(260.), px(260.)));
    assert!(grip::follow(window, at(900., 0.)));
    assert_eq!(client_size(window), (px(660.), px(660.)));
    assert!(grip::follow(window, at(-60., -30.)));
    assert_eq!(client_size(window), (px(220.), px(220.)));
    assert!(grip::follow(window, at(30., 10.)));
    assert_eq!(client_size(window), (px(250.), px(250.)));
    assert_eq!(top_left(window), corner);
    assert_eq!(thick(), 0);

    // Relâchement (`endMicroResize`) : l'état final remis à l'application porte la
    // taille retenue, geste terminé.
    assert!(grip::finish(window));
    assert!(!grip::finish(window));
    assert!(!grip::follow(window, at(100., 100.)));
    assert!(!state::sizing());
    let last = *states.borrow().last().unwrap();
    assert!(!last.dragging && !last.resizable);
    assert_eq!((last.width, last.height), (250, 250));
    drop(scope);
    // SAFETY : fenêtre de test créée sur ce thread.
    unsafe { DestroyWindow(window) };
}
