//! SVG en ligne dans les templates : formes compilées et peintes (headless), `<svg src>`,
//! `currentColor` suivi au survol, règle pair-impair, accessibilité ; coordonnées proches
//! de constantes sans avertissement Clippy dans le code généré.
use karbon_ui::{
    platform::headless::{HeadlessApp, HeadlessOptions},
    ui::{prelude::*, scene::DrawCommand},
};
css_file!(mod css, "svg.css");

fn pixel(image: &karbon_ui::ui::scene::Image, x: u32, y: u32) -> [u8; 4] {
    let offset = ((y * image.width + x) * 4) as usize;
    image.pixels[offset..offset + 4].try_into().unwrap()
}

fn options(width: f32, height: f32) -> HeadlessOptions {
    HeadlessOptions {
        width,
        height,
        ..Default::default()
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Play,
}

/// Icônes lucide et heroicons collées telles quelles (iconify), currentColor suivi.
struct Icons;

impl Application for Icons {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        let cx = TemplateContext::default();
        ui! { @context(cx); @styles(css);
            <div class="icons">
                <button id="play" class="icon-button" onclick={Msg::Play} aria-label="Lecture">
                    <svg id="play-icon" xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24">
                        <path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 3l14 9-14 9z"/>
                    </svg>
                </button>
                <svg id="dot" class="dot" viewBox="0 0 24 24" aria-label="Point">
                    <circle cx="12" cy="12" r="10" fill="currentColor"/>
                </svg>
                <svg id="ring" width="40" height="40" viewBox="0 0 24 24">
                    <g fill="none" stroke="#00ff00" stroke-width="4">
                        <circle cx="12" cy="12" r="8"/>
                    </g>
                </svg>
                <svg id="holes" width="40" height="40" viewBox="0 0 20 20">
                    <path fill="#0000ff" fill-rule="evenodd" d="M0 0H20V20H0ZM5 5V15H15V5Z"/>
                </svg>
                <svg id="file" src="../fixtures/icons/heart.svg" width="32" height="32" aria-label="Favori" />
                <svg id="arcs" width="40" height="40" viewBox="0 0 24 24">
                    <g transform="translate(12 12)">
                        <rect x="-10" y="-10" width="20" height="20" rx="4" fill="red" opacity="0.5"/>
                        <path d="M-6 0a6 6 0 1 0 12 0a6 6 0 1 0-12 0" fill="white"/>
                        <polyline points="-8,8 0,-8 8,8" fill="none" stroke="black"/>
                        <line x1="-8" y1="0" x2="8" y2="0" stroke="black"/>
                    </g>
                </svg>
            </div>
        }
    }
    fn update(&mut self, _: Msg) -> bool {
        false
    }
}

fn svg_commands(app: &HeadlessApp<Icons>) -> Vec<(karbon_ui::ui::scene::RectF, Color, usize)> {
    app.scene()
        .commands
        .iter()
        .filter_map(|c| match c {
            DrawCommand::Svg { rect, image, color } => Some((*rect, *color, image.shapes.len())),
            _ => None,
        })
        .collect()
}

#[test]
fn inline_svg_compiles_shapes_and_paints_them() -> karbon_ui::Result<()> {
    let mut app = HeadlessApp::with_options(Icons, options(400., 200.))?;
    let svgs = svg_commands(&app);
    assert_eq!(svgs.len(), 6);
    // <svg src> : dessin du fichier, attributs de la balise prioritaires.
    let file = app.bounds("file").unwrap();
    assert_eq!(file.right - file.left, 32.);
    assert_eq!(app.accessible("file").unwrap().name, "Favori");
    // width/height donnent la taille ; la classe la remplace (.dot : 16 px).
    let play = app.bounds("play-icon").unwrap();
    assert_eq!((play.right - play.left, play.bottom - play.top), (24., 24.));
    let dot = app.bounds("dot").unwrap();
    assert_eq!((dot.right - dot.left, dot.bottom - dot.top), (16., 16.));
    // currentColor : couleur CSS de l'élément (.dot { color: #ff0000 }).
    assert_eq!(svgs[1].1, Color::hex(0xff0000));
    // La couleur héritée du bouton (#336699) colore le tracé de l'icône.
    assert_eq!(svgs[0].1, Color::hex(0x336699));
    let image = app.render(1.)?;
    let center = |r: karbon_ui::ui::scene::RectF| {
        (
            ((r.left + r.right) / 2.) as u32,
            ((r.top + r.bottom) / 2.) as u32,
        )
    };
    let (x, y) = center(dot);
    assert_eq!(pixel(&image, x, y), [0, 0, 255, 255]);
    // Anneau tracé : bord vert, centre vide (fond transparent du thème ? non : fond).
    let ring = app.bounds("ring").unwrap();
    let (x, y) = center(ring);
    let background = pixel(&image, x, y);
    let edge = pixel(&image, x, (ring.top + 40. * 4. / 24.) as u32);
    assert_eq!(edge, [0, 255, 0, 255]);
    assert_ne!(background, edge);
    // Règle pair-impair : le carré intérieur est un trou.
    let holes = app.bounds("holes").unwrap();
    let (x, y) = center(holes);
    assert_ne!(pixel(&image, x, y), [255, 0, 0, 255]);
    assert_eq!(pixel(&image, holes.left as u32 + 2, y), [255, 0, 0, 255]);
    // Accessibilité : aria-label → image nommée, icône décorative sinon.
    let dot = app.accessible("dot").expect("named svg is exposed");
    assert_eq!(dot.name, "Point");
    Ok(())
}

#[test]
fn hovering_a_button_recolours_its_svg_icon() -> karbon_ui::Result<()> {
    let mut app = HeadlessApp::with_options(Icons, options(400., 200.))?;
    app.hover("play")?;
    let svgs = svg_commands(&app);
    assert_eq!(svgs[0].1, Color::hex(0xffcc00));
    app.click("play")?;
    Ok(())
}

/// Coordonnées proches de π, τ ou e dans un dessin : le code généré ne doit pas
/// déclencher `clippy::approx_constant` chez l'application (vérifié par Clippy sur les
/// tests).
struct NearConstants;

impl Application for NearConstants {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        let cx = TemplateContext::default();
        ui! { @context(cx);
            <svg id="pi" viewBox="0 0 24 24" width="24" height="24">
                <path d="M3.14 2.718L6.283 6.283" fill="none" stroke="currentColor"/>
            </svg>
        }
    }
    fn update(&mut self, _: Msg) -> bool {
        false
    }
}

#[test]
fn drawings_with_coordinates_near_constants_build() -> karbon_ui::Result<()> {
    let app = HeadlessApp::with_options(NearConstants, options(200., 200.))?;
    assert!(app.find("pi").is_some());
    Ok(())
}
