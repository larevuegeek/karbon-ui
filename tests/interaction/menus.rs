//! Menus déroulants et listes de choix écrits en template : placement, clavier,
//! fermeture, focus et accessibilité (`k-popover menu|listbox`, `role`, `aria-expanded`).
use crate::css;
use karbon_ui::{
    platform::headless::{HeadlessApp, HeadlessOptions},
    ui::{element::Role, prelude::*, runtime::Key},
};

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Toggle,
    Close,
    Pick(u32),
    Suggest,
    CloseSuggest,
}

#[derive(Default)]
struct Profile {
    bottom: bool,
    open: bool,
    suggest: bool,
    picked: Vec<u32>,
    closes: u32,
}

impl Application for Profile {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        let cx = TemplateContext::default();
        ui! { @context(cx); @styles(css);
            <div class="page">
            <div class="bar" class:bottom={self.bottom}>
                <input id="search" type="text" value="que" readonly aria-label="Recherche" class="search">
                <button id="suggest" class="zoom" title="Afficher les suggestions" onclick={Msg::Suggest}>Suggestions</button>
                <button id="profile" class="trigger" onclick={Msg::Toggle}
                        aria-haspopup="menu" aria-expanded={self.open}>
                    David
                    <svg viewBox="0 0 24 24" width="14" height="14"><path d="m6 9 6 6 6-6" fill="none" stroke="currentColor" stroke-width="2"/></svg>
                </button>
                {#if self.open}
                    <k-popover id="menu" anchor="profile" menu placement="bottom-end" offset="8" cross-offset="-12"
                               aria-label="Menu du profil" onclose={Msg::Close} class="menu"
                               data-exit="fade-out 120ms">
                        <button id="m1" class="item" role="menuitem" onclick={Msg::Pick(1)}>
                            <svg viewBox="0 0 24 24" width="15" height="15"><circle cx="12" cy="12" r="8" fill="none" stroke="currentColor"/></svg>
                            Changer de profil
                        </button>
                        <div class="sep" role="separator"></div>
                        <button id="m2" class="item danger" role="menuitem" onclick={Msg::Pick(2)}>
                            <svg viewBox="0 0 24 24" width="15" height="15"><path d="M9 21H5V3h4m7 14 5-5-5-5m5 5H9" fill="none" stroke="currentColor"/></svg>
                            Quitter
                        </button>
                    </k-popover>
                {/if}
                {#if self.suggest}
                    <k-popover id="list" anchor="search" listbox match-width aria-label="Suggestions"
                               onclose={Msg::CloseSuggest} class="menu">
                        <div id="o1" class="item" role="option" onclick={Msg::Pick(10)}>Kings and Queens</div>
                        <div id="o2" class="item" role="option" onclick={Msg::Pick(11)}>Que du love</div>
                    </k-popover>
                {/if}
            </div>
            </div>
        }
    }
    fn update(&mut self, message: Msg) -> bool {
        match message {
            Msg::Toggle => self.open = !self.open,
            Msg::Close => {
                self.open = false;
                self.closes += 1;
            }
            Msg::Pick(n) => self.picked.push(n),
            Msg::Suggest => self.suggest = true,
            Msg::CloseSuggest => self.suggest = false,
        }
        true
    }
}

fn app(width: f32, height: f32) -> HeadlessApp<Profile> {
    HeadlessApp::with_options(
        Profile::default(),
        HeadlessOptions {
            width,
            height,
            ..Default::default()
        },
    )
    .unwrap()
}

#[test]
fn a_template_menu_opens_under_its_trigger_and_is_driven_by_the_keyboard() -> karbon_ui::Result<()>
{
    let mut app = app(800., 500.);
    assert_eq!(app.accessible("profile").unwrap().expanded, Some(false));
    app.click("profile")?;
    assert!(app.app().open);
    let trigger = app.bounds("profile").unwrap();
    let menu = app.bounds("menu").unwrap();
    // placement="bottom-end" offset="8" cross-offset="-12" : bord droit 12 px à gauche
    // de celui du bouton, 8 px sous le bouton.
    assert!(
        (menu.right - (trigger.right - 12.)).abs() < 0.01,
        "{menu:?} {trigger:?}"
    );
    assert!((menu.top - trigger.bottom - 8.).abs() < 0.01);
    // Seul le contenu du menu est exposé pendant son ouverture ; icônes décoratives absentes.
    assert!(app.accessible("profile").is_none());
    assert!(
        app.accessibility()
            .iter()
            .all(|a| a.role != Role::Custom && a.role != Role::Image)
    );
    let accessible = app.accessible("menu").unwrap();
    assert_eq!(accessible.role, Role::Menu);
    let item = app.accessible("m1").unwrap();
    assert_eq!(
        (item.role, item.name.as_str()),
        (Role::MenuItem, "Changer de profil")
    );
    // Le focus entre dans le menu ; flèches, fin et bouclage.
    assert_eq!(app.focused(), Some(ElementId::named("m1")));
    app.press(Key::Down)?;
    assert_eq!(app.focused(), Some(ElementId::named("m2")));
    app.press(Key::Down)?;
    assert_eq!(app.focused(), Some(ElementId::named("m1")));
    app.press(Key::End)?;
    assert_eq!(app.focused(), Some(ElementId::named("m2")));
    // Entrée : l'action, puis la fermeture ; le focus revient au déclencheur.
    app.press(Key::Enter)?;
    assert_eq!(app.app().picked, [2]);
    assert!(!app.app().open);
    assert_eq!(app.focused(), Some(ElementId::named("profile")));
    assert_eq!(app.accessible("profile").unwrap().expanded, Some(false));
    // Échap ferme aussi ; data-exit garde un fantôme du menu le temps de sa sortie.
    app.click("profile")?;
    app.press(Key::Escape)?;
    assert!(!app.app().open);
    assert_eq!(app.focused(), Some(ElementId::named("profile")));
    assert!(app.contains_text("Changer de profil"));
    app.advance(std::time::Duration::from_millis(400))?;
    assert!(!app.contains_text("Changer de profil"));
    Ok(())
}

#[test]
fn a_title_under_a_hover_transform_is_accepted() -> karbon_ui::Result<()> {
    let mut app = app(800., 500.);
    app.hover("suggest")?;
    assert!(app.find("suggest").is_some());
    Ok(())
}

#[test]
fn clicking_an_empty_spot_outside_closes_the_menu() -> karbon_ui::Result<()> {
    let mut app = app(800., 500.);
    app.click("profile")?;
    app.click_at(karbon_ui::ui::scene::PointF { x: 20., y: 400. })?;
    assert!(!app.app().open);
    assert!(app.app().picked.is_empty());
    Ok(())
}

#[test]
fn a_listbox_matches_its_anchor_width_and_flips_when_there_is_no_room() -> karbon_ui::Result<()> {
    let mut app = app(800., 500.);
    app.click("suggest")?;
    let anchor = app.bounds("search").unwrap();
    let list = app.bounds("list").unwrap();
    assert!((list.right - list.left - (anchor.right - anchor.left)).abs() < 0.01);
    assert!(list.top >= anchor.bottom, "{list:?} {anchor:?}");
    assert_eq!(app.accessible("list").unwrap().role, Role::List);
    assert_eq!(app.accessible("o1").unwrap().role, Role::ListItem);
    assert_eq!(app.focused(), Some(ElementId::named("o1")));
    app.press(Key::Down)?;
    app.press(Key::Enter)?;
    assert_eq!(app.app().picked, [11]);
    assert!(!app.app().suggest);
    // Fenêtre basse : la barre est en bas, la liste passe au-dessus du champ.
    let mut app = app_with_bar_at_bottom();
    app.click("suggest")?;
    let anchor = app.bounds("search").unwrap();
    let list = app.bounds("list").unwrap();
    assert!(list.bottom <= anchor.top, "{list:?} {anchor:?}");
    Ok(())
}

fn app_with_bar_at_bottom() -> HeadlessApp<Profile> {
    // Barre collée en bas de la fenêtre : pas de place sous le champ.
    HeadlessApp::with_options(
        Profile {
            bottom: true,
            ..Default::default()
        },
        HeadlessOptions {
            width: 800.,
            height: 400.,
            ..Default::default()
        },
    )
    .unwrap()
}
