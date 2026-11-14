//! `icons_dir!` : dossier d'icônes compilé (`icons/`), noms `jeu:nom` et `nom`, fichiers
//! autres que `.svg` ignorés, icône peinte avec la couleur CSS de son élément.
use karbon_ui::{
    platform::headless::{HeadlessApp, HeadlessOptions},
    ui::{prelude::*, scene::DrawCommand},
};

icons_dir!(mod icons, "icons");

#[test]
fn names_follow_the_folder_layout() {
    assert_eq!(icons::NAMES, ["dot", "heroicons:heart"]);
    let cx = TemplateContext::default();
    assert!(icons::svg(&cx, "heroicons:heart").is_some());
    assert!(icons::svg(&cx, "dot").is_some());
    assert!(icons::svg(&cx, "heroicons:missing").is_none());
    assert!(icons::svg(&cx, "LICENSES").is_none());
}

struct Page;
impl Application for Page {
    type Message = ();
    fn view(&self, _: &ViewContext) -> Element {
        let cx = TemplateContext::default();
        let heart = icons::svg(&cx, "heroicons:heart").unwrap();
        let dot = icons::svg(&cx, "dot").unwrap();
        ui! { @context(cx);
            <div style="color: #336699">{@element heart}{@element dot}</div>
        }
    }
    fn update(&mut self, _: ()) -> bool {
        false
    }
}

#[test]
fn icons_are_painted_at_their_natural_size_with_the_css_colour() -> karbon_ui::Result<()> {
    let options = HeadlessOptions {
        width: 200.,
        height: 200.,
        ..Default::default()
    };
    let app = HeadlessApp::with_options(Page, options)?;
    let svgs = app
        .scene()
        .commands
        .iter()
        .filter_map(|c| match c {
            DrawCommand::Svg { rect, color, .. } => {
                Some((rect.right - rect.left, rect.bottom - rect.top, *color))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        svgs,
        [
            (24., 24., Color::hex(0x336699)),
            (16., 16., Color::hex(0x336699)),
        ]
    );
    Ok(())
}
