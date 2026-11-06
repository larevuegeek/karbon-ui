//! Mode framework (`css_file!(…, framework)`) sur des feuilles génériques : couches et
//! `!important`, sélecteurs d'attributs, images SVG en URL `data:` (statiques et par
//! `var()`), transition composée lue par `var()` et `data-reveal`.
use crate::support::*;
use karbon_ui::ui::{
    element::ElementId,
    layout,
    prelude::*,
    scene::{DrawCommand, PointF},
    styling,
};
css_file!(mod fw, "framework_mode.css", framework);
css_file!(mod img, "framework_images.css", framework);

#[test]
fn cascade_layers_important_and_attribute_selectors() {
    let mut root = ui! {@styles(fw);
        <div>
            <div id="a" class="box special">A</div>
            <div id="b" class="box" data-state="on">B</div>
            <input id="c" type="text" class="field" readonly value="x">
        </div>
    };
    arrange(&mut root, 800., 600.);
    // `.box` (non stratifiée) l'emporte sur `@layer base .box.special` ; `!important`
    // de la couche l'emporte sur tout.
    assert_eq!(
        computed(find(&root, "a")).local.background,
        Some(Color::hex(0x00ff00))
    );
    assert_eq!(
        computed(find(&root, "a")).text.color,
        Some(Color::hex(0x0000ff))
    );
    assert_eq!(
        computed(find(&root, "b")).local.background,
        Some(Color::hex(0xff00ff))
    );
    assert_eq!(
        computed(find(&root, "c")).local.border,
        Some(Color::hex(0x123456))
    );
}

#[test]
fn svg_data_url_backgrounds_static_and_through_var() {
    fn view() -> Element {
        ui! {@styles(img);
            <div>
                <div id="static" class="static-icon"></div>
                <div id="var" class="var-icon"></div>
                <div id="off" class="var-icon off"></div>
                <div id="tiled" class="tiled"></div>
            </div>
        }
    }
    let ui = runtime(view);
    let root = ui.root().unwrap();
    let bounds = |id: &str| root.find(ElementId::named(id)).unwrap().bounds();
    let rects = svgs(ui.scene());
    // `background` statique (URL encodée en pourcentages) : 10 × 10 centré.
    let s = bounds("static");
    assert!(rects.iter().any(|r| {
        (r.left - (s.left + 45.)).abs() < 0.01
            && (r.top - (s.top + 15.)).abs() < 0.01
            && layout::width(*r) == 10.
    }));
    // `var(--icon)` (URL base64) : 8 px de large, hauteur du rapport 8:6, à 4 px du bord
    // droit de la boîte de remplissage (bordure de 2 px), centré verticalement.
    let v = bounds("var");
    assert!(rects.iter().any(|r| {
        (r.right - (v.right - 2. - 4.)).abs() < 0.01
            && (layout::height(*r) - 6.).abs() < 0.01
            && ((r.top + r.bottom) / 2. - (v.top + v.bottom) / 2.).abs() < 0.01
    }));
    // `background-image: none` retire la couche.
    let off = bounds("off");
    assert!(
        !rects
            .iter()
            .any(|r| r.top >= off.top && r.bottom <= off.bottom)
    );
    // Répétition par défaut : 3 × 2 tuiles de 8 × 6 dans 20 × 10.
    let t = bounds("tiled");
    let tiles = rects
        .iter()
        .filter(|r| {
            r.left >= t.left && r.right <= t.left + 24. && r.top >= t.top && r.top < t.bottom
        })
        .count();
    assert_eq!(tiles, 6);
    let computed = root
        .find(ElementId::named("var"))
        .unwrap()
        .computed_style()
        .unwrap();
    assert_eq!(
        computed.local.background_images.position.unwrap()[0],
        styling::BackgroundPosition {
            x: (1., -4.),
            y: (0.5, 0.)
        }
    );
}

#[test]
fn composed_transition_through_var_interpolates_colours_and_transforms() {
    // `transition: var(--t)` avec `--t` déclarée par un ancêtre : liste composée évaluée
    // à l'exécution ; `box-shadow` et `border-radius` ignorés, minutage du premier
    // élément animable (`background-color 100ms linear`).
    use karbon_ui::ui::runtime::UiEvent;
    use std::time::{Duration, Instant};
    let mut ui = runtime(|| {
        ui! {@styles(fw);
            <div class="scope"><div id="t" class="trans">Text</div></div>
        }
    });
    let start = Instant::now();
    prepare_at(&mut ui, start);
    let background = |ui: &karbon_ui::ui::runtime::UiRuntime| {
        computed(find(ui.root().unwrap(), "t"))
            .local
            .background
            .unwrap()
    };
    let shift = |ui: &karbon_ui::ui::runtime::UiRuntime| {
        ui.scene()
            .commands
            .iter()
            .find_map(|c| match c {
                DrawCommand::PushEffect { transform, .. } => Some(transform.dx),
                _ => None,
            })
            .unwrap_or(0.)
    };
    assert_eq!(background(&ui), Color::hex(0x000000));
    let b = find(ui.root().unwrap(), "t").bounds();
    ui.event(UiEvent::PointerMove(PointF {
        x: b.left + 5.,
        y: b.top + 5.,
    }));
    prepare_at(&mut ui, start);
    assert!(ui.next_wake().is_some());
    prepare_at(&mut ui, start + Duration::from_millis(50));
    let mid = background(&ui);
    assert!((100..=155).contains(&mid.red), "{mid:?}");
    let dx = shift(&ui);
    assert!(2. < dx && dx < 8., "translateX mid-transition: {dx}");
    prepare_at(&mut ui, start + Duration::from_millis(150));
    assert_eq!(background(&ui), Color::hex(0xffffff));
    near(shift(&ui), 10.);
}

static CLOSED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

#[test]
fn data_reveal_reads_the_height_transition_of_a_class() {
    // `data-reveal=".collapsing"` : minutage de `.collapsing { transition: height 200ms
    // linear }`, hauteur 0 ↔ naturelle quand `.closed { display: none }` bascule.
    use std::{
        sync::atomic::Ordering,
        time::{Duration, Instant},
    };
    fn view() -> Element {
        let closed = CLOSED.load(Ordering::Relaxed);
        ui! {@styles(fw);
            <div>
                <div id="p" class:closed={closed} data-reveal=".collapsing">
                    <div class="trans">Text</div>
                </div>
                <div id="after" class="box">After</div>
            </div>
        }
    }
    CLOSED.store(true, Ordering::Relaxed);
    let mut ui = runtime(view);
    let start = Instant::now();
    prepare_at(&mut ui, start);
    let height = |ui: &karbon_ui::ui::runtime::UiRuntime| {
        layout::height(find(ui.root().unwrap(), "p").bounds())
    };
    assert_eq!(height(&ui), 0.);
    CLOSED.store(false, Ordering::Relaxed);
    ui.invalidate();
    prepare_at(&mut ui, start);
    prepare_at(&mut ui, start + Duration::from_millis(400));
    let natural = height(&ui);
    assert!(natural > 20., "{natural}");
    CLOSED.store(true, Ordering::Relaxed);
    ui.invalidate();
    let t = start + Duration::from_millis(500);
    prepare_at(&mut ui, t);
    prepare_at(&mut ui, t + Duration::from_millis(50));
    near(height(&ui), natural * 0.75);
    let root = ui.root().unwrap();
    near(
        find(root, "after").bounds().top,
        find(root, "p").bounds().bottom,
    );
    prepare_at(&mut ui, t + Duration::from_millis(200));
    assert_eq!(height(&ui), 0.);
    CLOSED.store(true, Ordering::Relaxed);
}
