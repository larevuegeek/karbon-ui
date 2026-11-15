//! Chemins depuis la racine du paquet (`/…`) : gabarit, inclusion, feuille et dossier
//! d'icônes rangés hors de `src/`, comme un dossier de thème.
use karbon_ui::{platform::headless::HeadlessApp, ui::prelude::*};

css_file!(mod css, "/tests/templates/ui/root/root.css");
icons_dir!(mod icons, "/tests/templates/icons");

struct Page;
impl Application for Page {
    type Message = ();
    fn view(&self, _: &ViewContext) -> Element {
        let cx = TemplateContext::default();
        ui_file!(
            "/tests/templates/ui/root/page.ui",
            context = cx,
            styles = css
        )
    }
    fn update(&mut self, _: ()) -> bool {
        false
    }
}

#[test]
fn files_are_named_from_the_package_root() {
    assert_eq!(icons::NAMES, ["dot", "heroicons:heart"]);
    let app = HeadlessApp::new(Page).unwrap();
    assert!(app.contains_text("From the root"));
    assert!(app.contains_text("Included from the root"));
}
