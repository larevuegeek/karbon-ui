//! États ARIA des gabarits : bouton bascule (`aria-pressed`), onglets (`role="tab"`,
//! `aria-selected`), radios et interrupteurs dessinés (`aria-checked`), élément courant
//! (`aria-current`). L'arbre accessible les expose et les commandes d'assistance
//! activent l'élément comme un clic.
use karbon_ui::{
    platform::headless::HeadlessApp,
    ui::prelude::*,
    ui::{
        element::{CheckState, ChoiceState, CurrentItem, ElementId, Role},
        runtime::AccessibilityAction,
    },
};

#[derive(Debug, Clone, PartialEq)]
enum Msg {
    Pin,
    Tab(u8),
    Speed(u8),
    Shuffle,
    Page(u8),
}

#[derive(Default)]
struct Player {
    pinned: bool,
    tab: u8,
    speed: u8,
    page: u8,
    log: Vec<Msg>,
}

impl Application for Player {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        let pinned = self.pinned;
        let tab = self.tab;
        let speed = self.speed;
        let page = self.page;
        ui! {
            <div>
                <button id="pin" aria-pressed={pinned} onclick={Msg::Pin}>Épingler</button>
                <div id="tabs" role="tablist">
                    <button id="queue" role="tab" aria-selected={tab == 0} onclick={Msg::Tab(0)}>File</button>
                    <button id="lyrics" role="tab" aria-selected={tab == 1} onclick={Msg::Tab(1)}>Paroles</button>
                </div>
                <div id="panel" role="tabpanel" aria-label="File">
                    <p>Rien</p>
                </div>
                <div id="speeds" role="radiogroup" aria-label="Vitesse">
                    <button id="x1" role="radio" aria-checked={speed == 1} onclick={Msg::Speed(1)}>Normale</button>
                    <button id="x2" role="radio" aria-checked={speed == 2} onclick={Msg::Speed(2)}>Double</button>
                </div>
                <button id="shuffle" role="switch" aria-checked="mixed" onclick={Msg::Shuffle}>Aléatoire</button>
                <button id="home" aria-current={(page == 0).then_some("page")} onclick={Msg::Page(0)}>Accueil</button>
                <button id="albums" aria-current={(page == 1).then_some("page")} onclick={Msg::Page(1)}>Albums</button>
                <button id="step" aria-current="step" onclick={Msg::Page(2)}>Étape</button>
            </div>
        }
    }
    fn update(&mut self, message: Msg) -> bool {
        match &message {
            Msg::Pin => self.pinned = !self.pinned,
            Msg::Tab(t) => self.tab = *t,
            Msg::Speed(s) => self.speed = *s,
            Msg::Page(p) => self.page = *p,
            Msg::Shuffle => {}
        }
        self.log.push(message);
        true
    }
}

fn player() -> HeadlessApp<Player> {
    HeadlessApp::new(Player {
        speed: 1,
        ..Player::default()
    })
    .unwrap()
}

#[test]
fn aria_pressed_makes_a_toggle_button_toggled_by_assistive_technologies() {
    let mut app = player();
    let pin = app.accessible("pin").unwrap();
    assert_eq!(pin.role, Role::Button);
    assert_eq!(pin.name, "Épingler");
    assert_eq!(pin.choice, Some(ChoiceState::Toggle(CheckState::Off)));
    assert!(
        !pin.invokable,
        "un bouton bascule se bascule, il ne s'invoque pas"
    );
    app.accessibility_action(AccessibilityAction::Toggle(ElementId::named("pin")))
        .unwrap();
    assert_eq!(app.app().log, [Msg::Pin]);
    let pin = app.accessible("pin").unwrap();
    assert_eq!(pin.choice, Some(ChoiceState::Toggle(CheckState::On)));
    assert!(
        app.accessibility_action(AccessibilityAction::Invoke(ElementId::named("pin")))
            .is_err()
    );
}

#[test]
fn tabs_are_selected_in_their_tab_list() {
    let mut app = player();
    let tabs = app.accessible("tabs").unwrap();
    assert_eq!(tabs.role, Role::TabList);
    let queue = app.accessible("queue").unwrap();
    assert_eq!(queue.role, Role::Tab);
    assert_eq!(queue.choice, Some(ChoiceState::Radio(true)));
    assert_eq!(queue.selection_container, Some(tabs.id));
    let lyrics = app.accessible("lyrics").unwrap();
    assert_eq!(lyrics.choice, Some(ChoiceState::Radio(false)));
    assert_eq!(app.accessible("panel").unwrap().role, Role::TabPanel);
    app.accessibility_action(AccessibilityAction::SelectChoice(ElementId::named(
        "lyrics",
    )))
    .unwrap();
    assert_eq!(app.app().log, [Msg::Tab(1)]);
    assert_eq!(
        app.accessible("lyrics").unwrap().choice,
        Some(ChoiceState::Radio(true))
    );
    assert_eq!(
        app.accessible("queue").unwrap().choice,
        Some(ChoiceState::Radio(false))
    );
}

#[test]
fn drawn_radios_and_switches_expose_aria_checked() {
    let mut app = player();
    let group = app.accessible("speeds").unwrap();
    assert_eq!(group.role, Role::RadioGroup);
    let x1 = app.accessible("x1").unwrap();
    assert_eq!(x1.role, Role::RadioButton);
    assert_eq!(x1.choice, Some(ChoiceState::Radio(true)));
    assert_eq!(x1.selection_container, Some(group.id));
    app.accessibility_action(AccessibilityAction::SelectChoice(ElementId::named("x2")))
        .unwrap();
    assert_eq!(app.app().log, [Msg::Speed(2)]);
    assert_eq!(
        app.accessible("x2").unwrap().choice,
        Some(ChoiceState::Radio(true))
    );
    let shuffle = app.accessible("shuffle").unwrap();
    assert_eq!(shuffle.role, Role::Switch);
    assert_eq!(shuffle.choice, Some(ChoiceState::Toggle(CheckState::Mixed)));
    // Un clic reste un clic : le message part, l'application garde son état.
    app.click("shuffle").unwrap();
    assert_eq!(app.app().log.last(), Some(&Msg::Shuffle));
}

#[test]
fn aria_current_follows_the_application_state() {
    let mut app = player();
    assert_eq!(
        app.accessible("home").unwrap().current,
        Some(CurrentItem::Page)
    );
    assert_eq!(app.accessible("albums").unwrap().current, None);
    assert_eq!(
        app.accessible("step").unwrap().current,
        Some(CurrentItem::Step)
    );
    app.click("albums").unwrap();
    assert_eq!(app.accessible("home").unwrap().current, None);
    assert_eq!(
        app.accessible("albums").unwrap().current,
        Some(CurrentItem::Page)
    );
    // Un élément courant reste un bouton ordinaire.
    assert!(app.accessible("albums").unwrap().invokable);
}

css_file!(mod css, "aria.css");

/// États ARIA en expressions lus par les sélecteurs d'attributs : jeton HTML, et pas
/// d'attribut pour `aria-current={None}`.
struct Styled {
    on: bool,
    page: Option<&'static str>,
}
impl Application for Styled {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        let on = self.on;
        let page = self.page;
        let mixed = CheckState::Mixed;
        let name = String::from("page");
        ui! {
            @styles(css);
            <div>
                <div id="pressed" class="etat" aria-pressed={on}></div>
                <div id="mixed" class="etat" role="checkbox" aria-checked={mixed}></div>
                <div id="current" class="etat" aria-current={page}></div>
                <div id="named" class="etat" aria-current={name}></div>
            </div>
        }
    }
    fn update(&mut self, _: Msg) -> bool {
        false
    }
}

#[test]
fn aria_expressions_feed_attribute_selectors_with_html_tokens() {
    let width = |app: &HeadlessApp<Styled>, id: &str| {
        let r = app.bounds(id).unwrap();
        r.right - r.left
    };
    let mut app = HeadlessApp::new(Styled {
        on: true,
        page: Some("page"),
    })
    .unwrap();
    app.prepare().unwrap();
    assert_eq!(width(&app, "pressed"), 40.);
    assert_eq!(width(&app, "mixed"), 50.);
    assert_eq!(width(&app, "current"), 60.);
    assert_eq!(width(&app, "named"), 60.);
    let mut app = HeadlessApp::new(Styled {
        on: false,
        page: None,
    })
    .unwrap();
    app.prepare().unwrap();
    assert_eq!(width(&app, "pressed"), 20.);
    assert_eq!(width(&app, "current"), 20.);
}
