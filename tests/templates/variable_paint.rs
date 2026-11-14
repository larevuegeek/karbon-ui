//! Dégradés et ombres dont les couleurs lisent des variables d'élément : chaque élément
//! teinté par `style:--h={…}` (posé sur lui ou hérité d'un ancêtre) est peint comme son
//! jumeau écrit en couleurs constantes, pixel pour pixel.
use karbon_ui::{
    platform::headless::{HeadlessApp, HeadlessOptions},
    ui::{prelude::*, scene::Image},
};
css_file!(mod css, "variable_paint.css");
css_file!(mod fixed, "variable_paint_fixed.css");

struct Page;
impl Application for Page {
    type Message = ();
    fn view(&self, _: &ViewContext) -> Element {
        let cx = TemplateContext::default();
        let (h, s) = (200f32, 0.8f32);
        let tiles = ui! { @context(cx); @styles(css);
            <div style="display: flex; gap: 40px; padding: 40px">
                <div id="tile" class="tile" style:--h={30f32}></div>
                <div style:--h={h} style:--s={s}><div id="glow" class="glow"></div></div>
                <div id="card" class="card" style:--c={Color::hex(0x3b82f6)}></div>
            </div>
        };
        let twins = ui! { @context(cx); @styles(fixed);
            <div style="display: flex; gap: 40px; padding: 40px">
                <div id="tile-fixed" class="tile"></div>
                <div><div id="glow-fixed" class="glow"></div></div>
                <div id="card-fixed" class="card"></div>
            </div>
        };
        column("page", [tiles, twins])
    }
    fn update(&mut self, _: ()) -> bool {
        false
    }
}

/// Pixels du rectangle de l'élément `id`, agrandi de `margin` (ombre portée comprise).
fn pixels(app: &HeadlessApp<Page>, image: &Image, id: &str, margin: u32) -> Vec<[u8; 4]> {
    let r = app.bounds(id).unwrap();
    let (x0, y0) = (r.left as u32 - margin, r.top as u32 - margin);
    let (w, h) = (
        (r.right - r.left) as u32 + 2 * margin,
        (r.bottom - r.top) as u32 + 2 * margin,
    );
    let mut out = Vec::new();
    for y in y0..y0 + h {
        for x in x0..x0 + w {
            let i = ((y * image.width + x) * 4) as usize;
            out.push(image.pixels[i..i + 4].try_into().unwrap());
        }
    }
    out
}

#[test]
fn gradients_and_shadows_follow_element_variables() -> karbon_ui::Result<()> {
    let options = HeadlessOptions {
        width: 400.,
        height: 400.,
        ..Default::default()
    };
    let mut app = HeadlessApp::with_options(Page, options)?;
    let image = app.render(1.)?;
    // Fond de la page, hors de tout élément.
    let page: [u8; 4] = image.pixels[..4].try_into().unwrap();
    for (id, margin) in [("tile", 36), ("glow", 0), ("card", 0)] {
        let tinted = pixels(&app, &image, id, margin);
        let twin = pixels(&app, &image, &format!("{id}-fixed"), margin);
        assert_eq!(tinted, twin, "{id} differs from its constant twin");
        // Le rasteriseur headless peint un dégradé de la moyenne de ses arrêts : elle
        // dépend des couleurs évaluées, et se distingue du fond de la page.
        assert!(tinted.iter().any(|p| *p != page), "{id} is not painted");
    }
    Ok(())
}
