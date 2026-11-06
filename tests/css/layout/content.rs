//! Contenu HTML dans la mise en page : portions de texte d'une ligne flex, blocs à la
//! largeur de leur contenu, alignement sur la ligne de base, pistes `min-content`, ellipse
//! en tête (`direction: rtl`), débordement visible par défaut.
use crate::support::*;
use karbon_ui::{platform::headless::HeadlessApp, ui::prelude::*};
css_file!(mod css, "content.css");

struct Layouts;

impl Application for Layouts {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        let cx = TemplateContext::default();
        ui! { @context(cx); @styles(css);
            <div style="padding: 10px; gap: 20px">
                <div id="spans" class="row">
                    <span id="s1" class="tag">Un</span>
                    <span id="s2" class="tag">Deux</span>
                    <span id="s3">Trois</span>
                </div>
                <div id="inline-style" style="display: flex; gap: 4px"><span id="t1">A</span> <b id="t2">B</b></div>
                <div id="block"><span>Un</span> <span class="tag">Deux</span></div>
                <div id="blocks" class="row">
                    <div id="short">Court</div>
                    <div id="long">Un texte nettement plus long</div>
                </div>
                <div id="base" class="baseline">
                    <p id="big" class="big" style="margin:0">Titre</p>
                    <p id="small" class="small" style="margin:0">petit</p>
                </div>
                <div class="tracks">
                    <p id="cell" style="margin:0">Un mot interminablement long</p>
                    <p id="rest" style="margin:0">Reste</p>
                </div>
                <p id="path" class="path">{r"C:\Music\FLAC\a.flac"}</p>
                <div id="host" class="host"></div>
                <div id="host-clipped" class="host clipped"></div>
            </div>
        }
    }
    fn update(&mut self, _: Msg) -> bool {
        false
    }
}

#[test]
fn spans_in_a_flex_row_are_separate_flex_items() -> karbon_ui::Result<()> {
    let app = HeadlessApp::with_options(Layouts, options(800., 400.))?;
    let (s1, s2, s3) = (
        app.bounds("s1").unwrap(),
        app.bounds("s2").unwrap(),
        app.bounds("s3").unwrap(),
    );
    // Trois éléments flex, séparés par le gap de 10 px (les blancs sont ignorés).
    assert_eq!(s2.left - s1.right, 10.);
    assert_eq!(s3.left - s2.right, 10.);
    // Classe .tag (10 px) gardée par chaque portion : 2 caractères × 6 px.
    assert!((12. ..=13.).contains(&(s1.right - s1.left)), "{s1:?}");
    let (t1, t2) = (app.bounds("t1").unwrap(), app.bounds("t2").unwrap());
    assert_eq!(t2.left - t1.right, 4.);
    // Contexte bloc : un seul paragraphe, comme en HTML.
    assert!(app.contains_text("Un Deux"));
    Ok(())
}

#[test]
fn baseline_alignment_min_content_tracks_and_start_ellipsis() -> karbon_ui::Result<()> {
    let mut app = HeadlessApp::with_options(Layouts, options(800., 400.))?;
    // align-items: baseline : lignes de base (texte de 20 px : 18,5 ; 10 px : 9,25).
    let (big, small) = (app.bounds("big").unwrap(), app.bounds("small").unwrap());
    assert!(
        (small.top - big.top - 9.25).abs() < 0.01,
        "{big:?} {small:?}"
    );
    // min-content : la colonne prend le plus long mot (« interminablement », 16 × 6 px).
    let cell = app.bounds("cell").unwrap();
    assert!((cell.right - cell.left - 96.).abs() < 0.01, "{cell:?}");
    // direction: rtl + ellipsis : la fin du chemin reste visible (texte dessiné à droite).
    let path = app.bounds("path").unwrap();
    let image = app.render(1.)?;
    let row = (path.top + 6.) as u32;
    let glyph_at = |x: f32| pixel(&image, (path.left + x) as u32, row) != pixel(&image, 790, 395);
    assert!(glyph_at(3.) && glyph_at(57.));
    Ok(())
}

#[test]
fn block_children_of_a_flex_row_take_their_content_width() -> karbon_ui::Result<()> {
    let app = HeadlessApp::with_options(Layouts, options(800., 400.))?;
    let (short, long) = (app.bounds("short").unwrap(), app.bounds("long").unwrap());
    let width = |r: karbon_ui::ui::scene::RectF| r.right - r.left;
    assert!(width(short) < width(long), "{short:?} {long:?}");
    assert!(width(long) < 400., "{long:?}");
    Ok(())
}

#[test]
fn overflow_is_visible_by_default_and_rounded_clips_need_overflow() -> karbon_ui::Result<()> {
    let mut app = HeadlessApp::with_options(Layouts, options(800., 400.))?;
    let image = app.render(1.)?;
    // ::after placé 30 px sous un hôte de 20 px : visible (overflow: visible par défaut).
    let host = app.bounds("host").unwrap();
    let (x, y) = ((host.left + 20.) as u32, (host.top + 33.) as u32);
    assert_eq!(pixel(&image, x, y), [0, 255, 0, 255]);
    // overflow: hidden le rogne.
    let clipped = app.bounds("host-clipped").unwrap();
    let (x, y) = ((clipped.left + 20.) as u32, (clipped.top + 33.) as u32);
    assert_ne!(pixel(&image, x, y), [0, 255, 0, 255]);
    Ok(())
}
