//! Rendu : coins hérités de la carte, anneau de focus au-dessus de l'élément suivant,
//! bordure nette à 150 % après l'offcanvas, bouton de l'interrupteur coché et focalisé.
use super::{Demo, center, demo, demo_at_150, scroll_to, with_class};
use karbon_ui::{platform::headless::HeadlessApp, ui::scene::PointF};

#[test]
fn last_list_item_inherits_the_card_corners() {
    // `.list-group-item:last-child { border-bottom-*-radius: inherit }` : le fond blanc du
    // dernier élément suit l'arrondi intérieur de la carte au lieu de couvrir sa bordure.
    let app = demo();
    let root = app.root().unwrap();
    let card = with_class(root, "card")
        .into_iter()
        .find(|e| e.classes().contains(&"text-bg-dark"))
        .unwrap();
    let items = with_class(card, "list-group-item");
    let last = items.last().unwrap();
    let local = last.computed_style().expect("styles").local;
    let mut corners = local
        .corner_radii
        .unwrap_or([local.radius.unwrap_or(0.); 4]);
    for (corner, edge) in corners.iter_mut().zip(local.corner_edges) {
        if let Some(edge) = edge {
            *corner = edge;
        }
    }
    assert_eq!(corners[0], 0.);
    assert!(corners[2] > 0. && corners[3] > 0., "{corners:?}");
}

#[test]
fn accordion_focus_ring_rises_above_the_next_item() {
    // `.accordion-button:focus { z-index: 3; box-shadow: 0 0 0 .25rem rgba(13,110,253,.25) }`
    // (`:focus`, pas `:focus-visible` : un clic suffit). Le premier bouton refermé : son
    // anneau déborde sur l'élément suivant, peint par-dessus comme dans un navigateur.
    let mut app = demo();
    let first = scroll_to(&mut app, "accordion-button", 0);
    app.click_at(first).unwrap();
    app.advance(std::time::Duration::from_millis(600)).unwrap();
    let button = with_class(app.root().unwrap(), "accordion-button")[0];
    assert_eq!(app.focused(), Some(button.id));
    assert!(button.computed_style().unwrap().local.shadows.is_some());
    let b = button.bounds();
    let img = app.render(1.).unwrap();
    // Pixels BGRA : (rouge, bleu).
    let pixel = |x: f32, y: f32| {
        let i = ((y as u32 * img.width + x as u32) * 4) as usize;
        (img.pixels[i + 2], img.pixels[i])
    };
    // Sous le bord inférieur du bouton, sur le fond blanc de l'élément suivant : la
    // teinte bleue de l'anneau (sans `z-index`, l'élément suivant la recouvrait).
    let below = (1..8)
        .map(|dy| pixel((b.left + b.right) / 2., b.bottom + dy as f32))
        .collect::<Vec<_>>();
    assert!(
        below.iter().any(|&(r, blue)| blue > 240 && r < 215),
        "ring below the button: {below:?}"
    );
}

/// Pixels BGRA de `img` le long d'une ligne (`horizontal`) ou d'une colonne.
fn run(
    img: &karbon_ui::ui::scene::Image,
    at: u32,
    from: u32,
    to: u32,
    horizontal: bool,
) -> Vec<u8> {
    (from..to)
        .map(|v| {
            let (x, y) = if horizontal { (v, at) } else { (at, v) };
            let i = ((y * img.width + x) * 4) as usize;
            // Canal vert : la bordure #212529 sur fond blanc.
            img.pixels[i + 1]
        })
        .collect()
}

/// Le trait de la bordure de `r` dans la scène : ses bords extérieurs et son épaisseur
/// tombent sur des pixels physiques (sans quoi Direct2D l'étale sur deux rangées).
fn assert_aligned_border(app: &HeadlessApp<Demo>, r: karbon_ui::ui::geometry::RectF, s: f32) {
    use karbon_ui::ui::scene::DrawCommand;
    let (rect, width) = app
        .scene()
        .commands
        .iter()
        .find_map(|c| match c {
            DrawCommand::Stroke {
                rect, width, color, ..
            } if (color.red, color.green, color.blue) == (33, 37, 41)
                && rect.left > r.left - 1.
                && rect.right < r.right + 1.
                && rect.top > r.top - 1.
                && rect.bottom < r.bottom + 1. =>
            {
                Some((*rect, *width))
            }
            _ => None,
        })
        .expect("border stroke of the button");
    let aligned = |v: f32| (v * s - (v * s).round()).abs() < 0.01;
    let half = width / 2.;
    let edges = [
        rect.left - half,
        rect.top - half,
        rect.right + half,
        rect.bottom + half,
    ];
    assert!(
        aligned(width) && edges.into_iter().all(aligned),
        "border stroke between device pixels: {rect:?} width {width} in {r:?}"
    );
}

/// Les quatre côtés de la bordure de 1 px de `r` (logique) sont un trait net d'un pixel
/// physique, aussi foncé que la couleur de la bordure.
fn assert_crisp_border(
    img: &karbon_ui::ui::scene::Image,
    r: karbon_ui::ui::geometry::RectF,
    s: f32,
) {
    let (cx, cy) = (
        ((r.left + r.right) / 2. * s) as u32,
        ((r.top + r.bottom) / 2. * s) as u32,
    );
    let span = |edge: f32| ((edge * s).round() as u32 - 3, (edge * s).round() as u32 + 3);
    let sides = [
        ("top", run(img, cx, span(r.top).0, span(r.top).1, false)),
        (
            "bottom",
            run(img, cx, span(r.bottom).0, span(r.bottom).1, false),
        ),
        ("left", run(img, cy, span(r.left).0, span(r.left).1, true)),
        (
            "right",
            run(img, cy, span(r.right).0, span(r.right).1, true),
        ),
    ];
    for (side, pixels) in sides {
        let inked = pixels.iter().filter(|&&g| g < 240).count();
        let darkest = *pixels.iter().min().unwrap();
        assert!(
            inked == 1 && darkest < 0x30,
            "{side} border is not a crisp device pixel: {pixels:02x?}"
        );
    }
}

#[test]
fn outline_button_border_stays_crisp_after_the_offcanvas() {
    // `.btn-outline-dark` à 150 % : la bordure de 1 px (un pixel physique) d'une boîte
    // placée à un demi-pixel physique s'étalait sur deux rangées grises en haut et en bas
    // et restait nette à gauche et à droite. Les boîtes des feuilles de frameworks sont
    // peintes calées sur les pixels physiques, comme dans un navigateur ; ouvrir puis
    // fermer l'offcanvas ne laisse ni survol, ni focus visible, ni anneau.
    let s = 1.5;
    let mut app = demo_at_150();
    let index = with_class(app.root().unwrap(), "btn")
        .iter()
        .position(|e| e.classes().contains(&"btn-outline-dark"))
        .unwrap();
    let button = |app: &HeadlessApp<Demo>| with_class(app.root().unwrap(), "btn")[index].bounds();
    scroll_to(&mut app, "btn", index);
    // Bord supérieur à un demi-pixel physique.
    let fraction = (button(&app).top * s).rem_euclid(1.);
    let delta = (fraction - 0.5).rem_euclid(1.) / s;
    app.scroll("page", 0., delta + 1. / s).unwrap();
    let r = button(&app);
    assert!(((r.top * s).rem_euclid(1.) - 0.5).abs() < 0.01, "{r:?}");
    let crop = |img: &karbon_ui::ui::scene::Image| {
        let mut v = Vec::new();
        for y in (r.top * s) as u32 - 6..(r.bottom * s) as u32 + 6 {
            v.extend(run(
                img,
                y,
                (r.left * s) as u32 - 6,
                (r.right * s) as u32 + 6,
                true,
            ));
        }
        v
    };
    let img = app.render(s).unwrap();
    assert_crisp_border(&img, r, s);
    assert_aligned_border(&app, r, s);
    let fresh = crop(&img);
    // Ouvrir, fermer par le voile, puis éloigner le pointeur.
    app.click_at(center(with_class(app.root().unwrap(), "btn")[index]))
        .unwrap();
    app.advance(std::time::Duration::from_millis(400)).unwrap();
    assert!(!with_class(app.root().unwrap(), "offcanvas-backdrop").is_empty());
    app.click_at(PointF { x: 1100., y: 500. }).unwrap();
    app.advance(std::time::Duration::from_millis(400)).unwrap();
    assert!(with_class(app.root().unwrap(), "offcanvas-backdrop").is_empty());
    app.event(karbon_ui::ui::runtime::UiEvent::PointerMove(PointF {
        x: 1200.,
        y: 100.,
    }))
    .unwrap();
    app.advance(std::time::Duration::from_millis(400)).unwrap();
    let now = button(&app);
    assert!(
        (now.top - r.top).abs() < 1e-3 && (now.left - r.left).abs() < 1e-3,
        "{now:?}"
    );
    let style = with_class(app.root().unwrap(), "btn")[index]
        .computed_style()
        .unwrap()
        .local;
    assert!(style.shadows.is_none(), "no focus ring after a mouse click");
    let img = app.render(s).unwrap();
    assert_crisp_border(&img, r, s);
    assert_aligned_border(&app, r, s);
    assert!(crop(&img) == fresh, "the button looks as before its use");
}

#[test]
fn checked_focused_switch_draws_one_white_knob() {
    // `.form-switch .form-check-input:checked` (`--bs-form-switch-bg` : cercle blanc
    // `r=3` dans `viewBox="-4 -4 8 8"`, `background-size: contain`, `right center`)
    // l'emporte sur `:focus` (cercle #86b7fe) ; `--bs-form-check-bg-image` (la coche)
    // n'est jamais peinte par l'interrupteur.
    use karbon_ui::ui::scene::DrawCommand;
    let s = 1.5;
    let mut app = demo_at_150();
    let switch = scroll_to(&mut app, "form-check-input", 1);
    let knobs = |app: &HeadlessApp<Demo>| {
        let input = with_class(app.root().unwrap(), "form-check-input")[1];
        let r = input.bounds();
        let svgs = app
            .scene()
            .commands
            .iter()
            .filter_map(|c| match c {
                DrawCommand::Svg { rect, image, .. }
                    if rect.left >= r.left - 1.
                        && rect.right <= r.right + 1.
                        && rect.top >= r.top - 1.
                        && rect.bottom <= r.bottom + 1. =>
                {
                    Some((*rect, image.view_box, format!("{:?}", image.shapes[0].fill)))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        (r, svgs)
    };
    app.click_at(switch).unwrap();
    for wait in [0, 50, 400] {
        app.advance(std::time::Duration::from_millis(wait)).unwrap();
        let input = with_class(app.root().unwrap(), "form-check-input")[1];
        assert!(app.app().switched && app.focused() == Some(input.id));
        let (r, svgs) = knobs(&app);
        assert_eq!(svgs.len(), 1, "one knob after {wait} ms: {svgs:?}");
        let (knob, view_box, fill) = &svgs[0];
        assert_eq!(*view_box, [-4., -4., 8., 8.]);
        assert!(
            fill.contains("red: 255, green: 255, blue: 255, alpha: 255"),
            "{fill}"
        );
        // `contain` dans la boîte de remplissage (bordure de 1 px physique) de la boîte
        // peinte, calée sur les pixels physiques : un carré de la hauteur intérieure,
        // collé au bord droit.
        let border = 1. / s;
        let snap = |v: f32| (v * s + 0.5 + 1. / 64.).floor() / s;
        let r = karbon_ui::ui::geometry::RectF {
            left: snap(r.left),
            top: snap(r.top),
            right: snap(r.right),
            bottom: snap(r.bottom),
        };
        assert!(
            (knob.right - (r.right - border)).abs() < 0.01,
            "{knob:?} in {r:?}"
        );
        assert!(((knob.bottom - knob.top) - (r.bottom - r.top - 2. * border)).abs() < 0.01);
        assert!(((knob.right - knob.left) - (knob.bottom - knob.top)).abs() < 0.01);
    }
    // Pixels : centre du bouton blanc, pilule bleue, anneau de focus bleu clair autour.
    let (r, svgs) = knobs(&app);
    let knob = svgs[0].0;
    let img = app.render(s).unwrap();
    let rgb = |x: f32, y: f32| {
        let i = (((y * s) as u32 * img.width + (x * s) as u32) * 4) as usize;
        (img.pixels[i + 2], img.pixels[i + 1], img.pixels[i])
    };
    let my = (r.top + r.bottom) / 2.;
    assert_eq!(rgb((knob.left + knob.right) / 2., my), (255, 255, 255));
    assert_eq!(rgb(r.left + 6., my), (13, 110, 253));
}
