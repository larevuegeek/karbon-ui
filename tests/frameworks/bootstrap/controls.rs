//! Contrôles dessinés par la feuille (`appearance: none`) : case, interrupteur, curseur,
//! sans partie native ; caret de menu et arc du spinner en bordures en onglet.
use super::bs;
use crate::support::*;
use karbon_ui::ui::{
    element::ElementId,
    layout,
    prelude::*,
    scene::{DrawCommand, PointF},
    styling,
};

#[test]
fn bootstrap_checkbox_switch_and_range_are_drawn_from_css() {
    let mut root = ui! {@styles(bs);
        <body data-bs-theme="light">
            <div class="form-check">
                <input id="check" class="form-check-input" type="checkbox" checked={true} onchange={Msg::Check} aria-label="Check">
            </div>
            <div class="form-check form-switch">
                <input id="off" class="form-check-input" type="checkbox" switch checked={false} onchange={Msg::Check} aria-label="Off">
            </div>
            <div class="form-check form-switch">
                <input id="on" class="form-check-input" type="checkbox" switch checked={true} onchange={Msg::Check} aria-label="On">
            </div>
            <input id="range" type="range" class="form-range" value={50.0} oninput={Msg::Range}>
        </body>
    };
    arrange(&mut root, 800., 600.);
    // Case cochée : `appearance: none`, fond bleu, coche SVG blanche ajustée à la boîte.
    let check = computed(find(&root, "check"));
    assert_eq!(check.local.appearance_none, Some(true));
    assert_eq!(check.local.background, Some(Color::hex(0x0d6efd)));
    let images = check.local.background_images;
    assert!(!images.is_empty());
    let tick = images.layers.unwrap()[0].unwrap();
    assert_eq!(tick.view_box, [0., 0., 20., 20.]);
    assert_eq!(images.size.unwrap()[0], styling::BackgroundSize::Contain);
    // La case garde sa taille CSS (1em), pas celle du contrôle natif.
    let r = find(&root, "check").bounds();
    near(layout::width(r), 16.);
    near(layout::height(r), 16.);
    // Interrupteur : rond à gauche, puis à droite une fois coché.
    let position = |id| {
        computed(find(&root, id))
            .local
            .background_images
            .position
            .unwrap()[0]
    };
    assert_eq!(position("off").x, (0., 0.));
    assert_eq!(position("on").x, (1., 0.));
    assert_eq!(position("on").y, (0.5, 0.));
    near(layout::width(find(&root, "off").bounds()), 32.);
    // Curseur : pouce et piste de `::-webkit-slider-thumb` / `-runnable-track`.
    let range = computed(find(&root, "range"));
    assert_eq!(range.local.appearance_none, Some(true));
    let (thumb, track) = (range.local.slider_thumb, range.local.slider_track);
    assert_eq!(thumb.background, Some(Color::hex(0x0d6efd)));
    assert_eq!((thumb.width, thumb.height), (Some(16.), Some(16.)));
    assert_eq!(track.height, Some(8.));
    assert_eq!(track.background, Some(Color::hex(0xe9ecef)));
    assert_eq!(track.radius, Some(16.));
}

#[test]
fn css_drawn_controls_paint_no_native_parts() {
    fn view() -> Element {
        ui! {@styles(bs);
            <body data-bs-theme="light">
                <div class="form-check">
                    <input class="form-check-input" type="checkbox" checked={true} onchange={Msg::Check} aria-label="Check">
                </div>
                <input type="range" class="form-range" value={50.0} oninput={Msg::Range}>
            </body>
        }
    }
    let ui = runtime(view);
    let scene = ui.scene();
    // Coche : image SVG ; aucun trait de coche natif.
    assert_eq!(svgs(scene).len(), 1);
    assert!(
        !scene
            .commands
            .iter()
            .any(|c| matches!(c, DrawCommand::Line { .. }))
    );
    // Pouce bleu de 16 px, centré sur la valeur, sur une piste grise de 8 px.
    let fills = scene
        .commands
        .iter()
        .filter_map(|c| match c {
            DrawCommand::Fill { rect, color, .. } => Some((*rect, *color)),
            _ => None,
        })
        .collect::<Vec<_>>();
    let track = fills
        .iter()
        .find(|(r, c)| *c == Color::hex(0xe9ecef) && layout::height(*r) == 8.)
        .expect("track");
    let middle = (track.0.top + track.0.bottom) / 2.;
    let thumb = fills
        .iter()
        .find(|(r, c)| {
            *c == Color::hex(0x0d6efd)
                && layout::width(*r) == 16.
                && ((r.top + r.bottom) / 2. - middle).abs() < 0.01
        })
        .expect("thumb");
    near(
        (thumb.0.left + thumb.0.right) / 2.,
        (track.0.left + track.0.right) / 2.,
    );
}

#[test]
fn dropdown_caret_and_spinner_arc_are_mitred_borders() {
    fn view() -> Element {
        ui! {@styles(bs);
            <body data-bs-theme="light">
                <div class="d-flex">
                    <button id="toggle" class="btn btn-secondary dropdown-toggle" onclick={Msg::Noop}>Menu</button>
                    <div id="spinner" class="spinner-border text-primary"></div>
                </div>
            </body>
        }
    }
    let ui = runtime(view);
    let root = ui.root().unwrap();
    let toggle = root.find(ElementId::named("toggle")).unwrap();
    // Le bouton porte le texte puis la boîte `::after` (bordures seules : .3em × 2 de
    // large, .3em de haut).
    let caret = toggle.children.last().unwrap().bounds();
    near(layout::width(caret), 9.6);
    near(layout::height(caret), 4.8);
    assert!(caret.left > toggle.children[0].bounds().right - 0.01);
    let paths = ui
        .scene()
        .commands
        .iter()
        .filter_map(|c| match c {
            DrawCommand::Path {
                points,
                closed: true,
                color,
            } => Some((points.clone(), *color)),
            _ => None,
        })
        .collect::<Vec<_>>();
    // Caret : triangle plein (bordure du haut), côtés transparents non peints.
    let triangle = paths
        .iter()
        .find(|(p, _)| p.len() == 3)
        .expect("caret triangle");
    let apex = triangle.0[2];
    near(apex.x, (caret.left + caret.right) / 2.);
    near(apex.y, caret.bottom);
    // Spinner : un seul arc des trois côtés visibles (le droit est transparent),
    // anneau de 4 px au bord du cercle.
    let spinner = root.find(ElementId::named("spinner")).unwrap().bounds();
    let (arc, color) = paths
        .iter()
        .find(|(p, _)| p.len() > 3 && p.iter().all(|q| q.x >= spinner.left - 0.01))
        .expect("spinner arc");
    assert_eq!(*color, Color::hex(0x0d6efd));
    // Boîte peinte calée sur les pixels physiques (échelle 1 : pixels entiers).
    let snap = |v: f32| (v + 0.5 + 1. / 64.).floor();
    let center = PointF {
        x: (snap(spinner.left) + snap(spinner.right)) / 2.,
        y: (snap(spinner.top) + snap(spinner.bottom)) / 2.,
    };
    let radius = |q: &PointF| (q.x - center.x).hypot(q.y - center.y);
    assert!(arc.iter().all(|q| radius(q) > 11.9 && radius(q) < 16.1));
    // Pas de point dans le quart droit (côté transparent).
    assert!(
        !arc.iter()
            .any(|q| q.x - center.x > (q.y - center.y).abs() + 0.5)
    );
}
