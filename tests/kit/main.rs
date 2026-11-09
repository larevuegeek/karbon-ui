//! Composants du kit vus par une application : événements, thème et personnalisation.
use karbon_ui::{
    platform::headless::{HeadlessApp, HeadlessOptions},
    ui::{
        kit::{ICON_NAMES, prelude::*},
        rendering::brush::Background,
        runtime::Key,
        scene::{DrawCommand, PointF},
    },
};

css_file!(mod css, "custom.css");

#[derive(Clone)]
enum Msg {
    Click,
    Name(String),
    Flag(bool),
    Tab(usize),
    Close,
    Hover(bool),
    Press(bool),
    Focus(bool),
    Double,
}

#[derive(Default)]
struct Demo {
    clicks: u32,
    name: String,
    flag: bool,
    tab: usize,
    dialog: bool,
    dark: bool,
    events: Vec<&'static str>,
}

impl Application for Demo {
    type Message = Msg;
    fn color_preferences(&self) -> ColorPreferences {
        ColorPreferences {
            scheme: if self.dark {
                ColorScheme::Dark
            } else {
                ColorScheme::Light
            },
            high_contrast: false,
        }
    }
    fn update(&mut self, message: Msg) -> bool {
        match message {
            Msg::Click => self.clicks += 1,
            Msg::Name(name) => self.name = name,
            Msg::Flag(flag) => self.flag = flag,
            Msg::Tab(tab) => self.tab = tab,
            Msg::Close => self.dialog = false,
            Msg::Hover(on) => self.events.push(if on { "hover" } else { "unhover" }),
            Msg::Press(on) => self.events.push(if on { "press" } else { "release" }),
            Msg::Focus(on) => self.events.push(if on { "focus" } else { "blur" }),
            Msg::Double => self.events.push("double"),
        }
        true
    }
    fn view(&self, _: &ViewContext) -> Element {
        ui! { @styles(css);
            <Page>
                <Stack gap="sm">
                    <Button id="go" primary onclick={Msg::Click} onhover={Msg::Hover} onpress={Msg::Press}
                        onfocus={Msg::Focus} ondblclick={Msg::Double}>Go</Button>
                    <Button id="idle">Idle</Button>
                    <Button id="custom" class="mine" onclick={Msg::Click}>Custom</Button>
                    <Input id="name" value={&self.name} oninput={Msg::Name} />
                    <Checkbox id="flag" label="Flag" checked={self.flag} onchange={Msg::Flag} />
                    <Tabs id="tabs" items={["One", "Two"]} value={self.tab} onchange={Msg::Tab} />
                    <div class="promo"><Tabs underlined items={["Red"]} value={0usize} /></div>
                </Stack>
                {#if self.dialog}
                    <Dialog title="Sure?" onclose={Msg::Close}><Text>Body</Text></Dialog>
                {/if}
            </Page>
        }
    }
}

fn demo() -> HeadlessApp<Demo> {
    HeadlessApp::with_options(
        Demo::default(),
        HeadlessOptions {
            width: 600.,
            height: 800.,
            ..Default::default()
        },
    )
    .expect("the page is valid")
}

fn colors(app: &HeadlessApp<Demo>) -> Vec<Color> {
    let mut out = Vec::new();
    for command in &app.scene().commands {
        match command {
            DrawCommand::Fill { color, .. }
            | DrawCommand::Stroke { color, .. }
            | DrawCommand::RichText { color, .. }
            | DrawCommand::FormattedText { color, .. }
            | DrawCommand::FontText { color, .. }
            | DrawCommand::Text { color, .. } => out.push(*color),
            DrawCommand::Shape {
                fill: Some(Background::Solid(color)),
                ..
            } => out.push(*color),
            _ => {}
        }
    }
    out
}

/// Centre de l'enfant `index` de l'élément `id`.
fn child_center(app: &HeadlessApp<Demo>, id: &str, index: usize) -> PointF {
    let bounds = app
        .root()
        .expect("a built view")
        .find(ElementId::named(id))
        .expect("element")
        .children[index]
        .bounds();
    PointF {
        x: (bounds.left + bounds.right) / 2.,
        y: (bounds.top + bounds.bottom) / 2.,
    }
}

#[test]
fn a_click_reaches_update_and_a_button_without_handler_does_nothing() -> karbon_ui::Result<()> {
    let mut app = demo();
    app.click("go")?;
    assert_eq!(app.app().clicks, 1);
    app.click("idle")?;
    assert_eq!(app.app().clicks, 1);
    Ok(())
}

#[test]
fn element_events_report_hover_press_focus_and_double_click() -> karbon_ui::Result<()> {
    let mut app = demo();
    app.hover("go")?;
    assert_eq!(app.app().events, ["hover"]);
    app.click("go")?;
    let seen = &app.app().events;
    assert!(
        seen.contains(&"press") && seen.contains(&"release"),
        "{seen:?}"
    );
    assert!(seen.contains(&"focus"), "{seen:?}");
    app.click("go")?;
    assert!(app.app().events.contains(&"double"));
    app.hover("idle")?;
    assert!(app.app().events.contains(&"unhover"));
    Ok(())
}

#[test]
fn fields_report_their_values() -> karbon_ui::Result<()> {
    let mut app = demo();
    app.click("name")?;
    app.type_text("Ada")?;
    assert_eq!(app.app().name, "Ada");
    let center = child_center(&app, "flag", 0);
    app.click_at(center)?;
    assert!(app.app().flag);
    Ok(())
}

#[test]
fn tabs_report_the_clicked_index() -> karbon_ui::Result<()> {
    let mut app = demo();
    let center = child_center(&app, "tabs", 1);
    app.click_at(center)?;
    assert_eq!(app.app().tab, 1);
    Ok(())
}

#[test]
fn escape_closes_a_dialog() -> karbon_ui::Result<()> {
    let mut app = demo();
    app.update_app(|demo| demo.dialog = true)?;
    assert!(app.contains_text("Sure?"));
    app.press(Key::Escape)?;
    assert!(!app.app().dialog);
    Ok(())
}

#[test]
fn the_theme_colours_the_components_in_both_schemes() -> karbon_ui::Result<()> {
    let red = Color::hex(0xe11d48);
    set_theme(KitTheme::default().primary(red));
    let mut app = demo();
    assert!(colors(&app).contains(&red));
    assert!(colors(&app).contains(&Palette::LIGHT.background));
    app.update_app(|demo| demo.dark = true)?;
    assert!(colors(&app).contains(&Palette::DARK.background));
    set_theme(KitTheme::default());
    Ok(())
}

#[test]
fn the_classes_and_variables_of_the_application_apply() {
    let app = demo();
    let colors = colors(&app);
    assert!(
        colors.contains(&Color::hex(0x123456)),
        "class on a component"
    );
    assert!(
        colors.contains(&Color::hex(0xff0000)),
        "variable of an ancestor"
    );
}

#[derive(Default)]
struct Slides {
    index: usize,
}
impl Application for Slides {
    type Message = usize;
    fn update(&mut self, index: usize) -> bool {
        self.index = index;
        true
    }
    fn view(&self, _: &ViewContext) -> Element {
        ui! {
            <Aurora>
                <Carousel id="slides" value={self.index} onchange={|i: usize| i}>
                    <Glass padding="lg"><Button>One</Button></Glass>
                    <Glass padding="lg"><Button>Two</Button></Glass>
                    <Glass padding="lg"><Button>Three</Button></Glass>
                </Carousel>
                <Animate up><Input label="Name" /></Animate>
            </Aurora>
        }
    }
}

#[test]
fn the_carousel_moves_to_the_next_slide_and_glass_blurs_the_background() -> karbon_ui::Result<()> {
    let mut app = HeadlessApp::new(Slides::default())?;
    assert!(
        app.scene()
            .commands
            .iter()
            .any(|c| matches!(c, DrawCommand::Backdrop { .. }))
    );
    // Enfants du carrousel : la fenêtre, puis les deux flèches et les points.
    let next = app
        .root()
        .expect("a built view")
        .find(ElementId::named("slides"))
        .expect("carousel")
        .children[2]
        .bounds();
    app.click_at(PointF {
        x: (next.left + next.right) / 2.,
        y: (next.top + next.bottom) / 2.,
    })?;
    assert_eq!(app.app().index, 1);
    Ok(())
}

/// Composants utilisables dans une application à ressources.
struct Assets;
#[test]
fn components_accept_any_resource_type() {
    let assets = Assets;
    let cx = TemplateContext::new(&assets, "page");
    let element: Element = ui! { @context(cx); <Stack><Button>Ok</Button></Stack> };
    assert_eq!(element.children.len(), 1);
}

#[test]
fn the_theme_variables_are_available_to_application_stylesheets() {
    assert!(karbon_ui::ui::kit::variables().get("--k-primary").is_some());
    assert!(karbon_ui::ui::kit::variables().get("--k-surface").is_some());
}

struct Icons;
impl Application for Icons {
    type Message = ();
    fn update(&mut self, _: ()) -> bool {
        false
    }
    fn view(&self, _: &ViewContext) -> Element {
        ui! {
            <Row wrap>
                {#each ICON_NAMES.iter() as name (*name)}
                    <Icon name={*name} />
                {/each}
            </Row>
        }
    }
}
#[test]
fn every_icon_is_drawn() -> karbon_ui::Result<()> {
    let app = HeadlessApp::new(Icons)?;
    let drawn = app
        .scene()
        .commands
        .iter()
        .filter(|c| matches!(c, DrawCommand::Svg { .. }))
        .count();
    assert_eq!(drawn, ICON_NAMES.len());
    Ok(())
}

struct Menu;
impl Application for Menu {
    type Message = ();
    fn update(&mut self, _: ()) -> bool {
        false
    }
    fn view(&self, _: &ViewContext) -> Element {
        ui! {
            <Page fixed>
                <Aurora full flat>
                    <Navbar brand="Marque" />
                    <Row grow clip stretch gap="none">
                        <Sidebar id="menu">
                            {#each (0..40u64) as n (n)}
                                <NavItem id={ElementId::named("item").item(n)}>Lien</NavItem>
                            {/each}
                        </Sidebar>
                        <Stack grow><Text>Contenu</Text></Stack>
                    </Row>
                </Aurora>
            </Page>
        }
    }
}

/// Une barre latérale plus haute que la fenêtre reste dans la fenêtre et défile.
#[test]
fn a_tall_sidebar_stays_in_the_window_and_scrolls() -> karbon_ui::Result<()> {
    let mut app = HeadlessApp::with_options(
        Menu,
        HeadlessOptions {
            width: 800.,
            height: 400.,
            ..Default::default()
        },
    )?;
    let menu = app.bounds("menu").expect("menu");
    assert!(menu.bottom <= 400.5, "{menu:?}");
    let first = ElementId::named("item").item(0);
    let before = app.bounds(first).expect("item").top;
    app.scroll("menu", 0., 300.)?;
    app.advance(std::time::Duration::from_millis(600))?;
    let after = app.bounds(first).expect("item").top;
    assert!(after < before, "{before} -> {after}");
    Ok(())
}
