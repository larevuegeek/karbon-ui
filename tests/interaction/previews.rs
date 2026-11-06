//! Aperçus hors écran (`diagnostics`) : arbre mis en page inspectable (bornes, classes,
//! style calculé) et rendu avec un point survolé.
use karbon_ui::ui::prelude::*;
css_file!(mod css, "previews.css");

fn near(a: f32, b: f32) {
    assert!((a - b).abs() < 0.01, "{a} != {b}");
}

/// Page minimale pour l'inspection de la mise en page hors écran.
struct Inspected;

impl Application for Inspected {
    type Message = ();
    fn view(&self, _: &ViewContext) -> Element {
        ui! {@styles(css);
            <div class="stack">
                <p id="inspected" class="line label">Inspecté</p>
            </div>
        }
    }
    fn update(&mut self, _: ()) -> bool {
        false
    }
}

/// `diagnostics::inspect_preview` donne l'arbre mis en page (bornes, classes et style
/// calculé), pour comparer la page à une mise en page de référence.
#[test]
fn inspect_preview_exposes_bounds_classes_and_computed_style() {
    let options = karbon_ui::diagnostics::BenchmarkOptions {
        width: 400,
        height: 300,
        scale: 1.,
        title_bar: false,
        reduced_motion: true,
        ..Default::default()
    };
    let (classes, size, top) =
        karbon_ui::diagnostics::inspect_preview(options, Inspected, |root| {
            let p = root.find(ElementId::named("inspected")).unwrap();
            let style = p.computed_style().expect("a resolved style");
            (p.classes().to_vec(), style.text.size, p.bounds().top)
        })
        .unwrap();
    assert_eq!(classes, ["line", "label"]);
    assert_eq!(size, Some(14.));
    near(top, 0.);
}

/// Page dont le bouton change de couleur au survol.
struct Hovered;

impl Application for Hovered {
    type Message = ();
    fn view(&self, _: &ViewContext) -> Element {
        ui! {@styles(css);
            <div class="stack">
                <button id="survol" class="survol" onclick={()}><p class="line">Survol</p></button>
            </div>
        }
    }
    fn update(&mut self, _: ()) -> bool {
        false
    }
}

/// `BenchmarkOptions::pointer` rend l'aperçu avec ce point survolé (styles `:hover`).
#[test]
fn previews_can_hover_a_point() {
    let colour = |pointer| {
        let options = karbon_ui::diagnostics::BenchmarkOptions {
            width: 400,
            height: 300,
            scale: 1.,
            title_bar: false,
            reduced_motion: true,
            pointer,
            ..Default::default()
        };
        karbon_ui::diagnostics::inspect_preview(options, Hovered, |root| {
            let b = root.find(ElementId::named("survol")).unwrap();
            b.computed_style().and_then(|s| s.text.color)
        })
        .unwrap()
    };
    assert_eq!(colour(None), Some(Color::hex(0x000000)));
    assert_eq!(colour(Some((20., 20.))), Some(Color::hex(0xff0000)));
}
