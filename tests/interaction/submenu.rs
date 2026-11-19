//! Sous-menu ouvert depuis un popover (bouton « … » d'une ligne d'une liste déroulante) :
//! il se pose au-dessus de son parent, qui reste ouvert ; Échap et un clic dans le
//! parent ne ferment que lui, un clic hors des deux les ferme tous les deux.
use crate::css;
use karbon_ui::{
    platform::headless::{HeadlessApp, HeadlessOptions},
    ui::{prelude::*, runtime::Key},
};

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Toggle,
    Close,
    Manage(usize),
    CloseManage,
    Choose(usize),
    Rename,
    Page,
}

#[derive(Default)]
struct Libraries {
    open: bool,
    manage: Option<usize>,
    chosen: Vec<usize>,
    renamed: u32,
    page: u32,
}

const ROWS: [&str; 3] = ["row0", "row1", "row2"];
const MORE: [&str; 3] = ["more0", "more1", "more2"];

impl Libraries {
    /// Le bouton « … » de la ligne dont le sous-menu est ouvert porte l'ancre de celui-ci.
    fn more_id(&self, i: usize) -> &'static str {
        if self.manage == Some(i) {
            "more.open"
        } else {
            MORE[i]
        }
    }
}

impl Application for Libraries {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        let cx = TemplateContext::default();
        ui! { @context(cx); @styles(css);
            <div class="page">
                <div class="bar">
                    <button id="libraries" onclick={Msg::Toggle}>Bibliothèques</button>
                    <button id="elsewhere" onclick={Msg::Page}>Page</button>
                </div>
                {#if self.open}
                    <k-popover id="list" anchor="libraries" placement="bottom-start" aria-label="Bibliothèques"
                               onclose={Msg::Close} class="menu">
                        {#each ROWS as (i, row) (i)}
                            <div class="item">
                                <div id={*row} onclick={Msg::Choose(i)}>{row}</div>
                                <div id={self.more_id(i)} onclick={Msg::Manage(i)} aria-label="Gérer">{"…"}</div>
                            </div>
                        {/each}
                        {#if self.manage.is_some()}
                            <k-popover id="manage" anchor="more.open" menu placement="right-start" aria-label="Gérer"
                                       onclose={Msg::CloseManage} class="menu">
                                <div id="rename" class="item" role="menuitem" onclick={Msg::Rename}>Renommer</div>
                            </k-popover>
                        {/if}
                    </k-popover>
                {/if}
            </div>
        }
    }
    fn update(&mut self, message: Msg) -> bool {
        match message {
            Msg::Toggle => {
                self.open = !self.open;
                self.manage = None;
            }
            Msg::Close => {
                self.open = false;
                self.manage = None;
            }
            Msg::Manage(i) => self.manage = Some(i),
            Msg::CloseManage => self.manage = None,
            Msg::Choose(i) => self.chosen.push(i),
            Msg::Rename => {
                self.renamed += 1;
                self.manage = None;
            }
            Msg::Page => self.page += 1,
        }
        true
    }
}

#[test]
fn a_submenu_opens_over_its_popover_and_closes_alone() -> karbon_ui::Result<()> {
    let mut app = HeadlessApp::with_options(
        Libraries::default(),
        HeadlessOptions {
            width: 900.,
            height: 600.,
            ..Default::default()
        },
    )?;
    app.click("libraries")?;
    app.click("more1")?;
    // Le parent reste ouvert, le sous-menu se pose contre son bouton « … ».
    assert!(app.app().open && app.app().manage == Some(1));
    let (more, manage) = (
        app.bounds("more.open").unwrap(),
        app.bounds("manage").unwrap(),
    );
    assert!(manage.left >= more.right - 0.5, "{more:?} {manage:?}");
    // Un choix du sous-menu agit, puis il se ferme.
    app.click("rename")?;
    assert_eq!(app.app().renamed, 1);
    assert!(app.app().open && app.app().manage.is_none());
    // Échap ferme le sous-menu seulement.
    app.click("more0")?;
    app.press(Key::Escape)?;
    assert!(app.app().open && app.app().manage.is_none());
    // Un clic dans le parent ferme le sous-menu et agit quand même sur le parent.
    app.click("more2")?;
    app.click("row0")?;
    assert!(app.app().open && app.app().manage.is_none());
    assert_eq!(app.app().chosen, [0]);
    // Un clic hors des deux les ferme.
    app.click("more2")?;
    app.click("elsewhere")?;
    assert!(!app.app().open && app.app().manage.is_none());
    Ok(())
}
