//! Syntaxe des templates : classes typées et styles locaux à l'échelle, valeurs Rust,
//! boucles à clé, conditions paresseuses, `{#each}` sur un chemin de module, contrôles
//! intégrés, échappement vers Rust, entités, validation des dégradés, et grande page
//! faite d'`{@include}` construite sur une pile de 1 Mio.
use crate::support::*;
use karbon_ui::ui::{
    element::{Layout, SizeF},
    layout,
    prelude::*,
};

styles! {
    pub CARD { padding: 12; gap: 8; radius: 10; background: "#18202a"; }
    pub LABEL { font_size: 18; weight: 700; color: "#ffffff"; }
    pub WIDE { width: 100; }
}

#[test]
fn typed_classes_local_overrides_scaling_and_rust_values() {
    let cx = TemplateContext::default().with_scale(2.);
    let title = "Bonjour";
    let mut node = ui! {
        @context(cx);
        <div class={CARD} style="gap:10px">
            <span id="title" class={LABEL} style="width:120px">{title}</span>
            <div class={[WIDE,CARD]} style:flex-direction={"row"} style="width:140px; align-items:center">
                <button onclick={Msg::Click(1)}>Valider</button>
            </div>
        </div>
    };
    arrange(&mut node);
    assert_eq!(node.style.gap, 20.);
    assert_eq!(node.style.padding.left, 24.);
    assert_eq!(
        node.children[1].bounds().right - node.children[1].bounds().left,
        280.
    );
    let text = node.find(ElementId::named("title")).unwrap();
    assert_eq!(text.widget.as_ref().unwrap().semantics().name, "Bonjour");
    let mut scene = karbon_ui::ui::scene::Scene::new(Color::hex(0));
    text.widget.as_ref().unwrap().paint(
        &karbon_ui::ui::element::PaintContext {
            rect: text.bounds(),
            intrinsic: SizeF::default(),
            theme: Theme::DARK,
            state: Default::default(),
        },
        &mut scene,
    );
    assert!(
        matches!(&scene.commands[0],karbon_ui::ui::scene::DrawCommand::FontText{size,font,..} if *size==36.&&font.weight==700)
    );
}

fn list(items: &[u64], show: bool) -> Element {
    ui! {
        <div>
            {#each items as item (*item)}
                <div style:flex-direction={"row"}><span>Item {item}</span></div>
            {/each}
            {#if show}<span>Visible</span>{:else}<span>Masqué</span>{/if}
        </div>
    }
}

#[test]
fn keyed_loops_preserve_identity_when_reordered_and_conditionals_are_lazy() {
    let mut first = list(&[3, 7], true);
    let mut second = list(&[7, 3], false);
    arrange(&mut first);
    arrange(&mut second);
    assert_eq!(first.children[0].id, second.children[1].id);
    assert_eq!(
        first.children[1].children[0].id,
        second.children[0].children[0].id
    );
    assert_eq!(
        first.children[0].children[0]
            .widget
            .as_ref()
            .unwrap()
            .semantics()
            .name,
        "Item 3"
    );
    assert_eq!(
        second.children[2].widget.as_ref().unwrap().semantics().name,
        "Masqué"
    );
}

#[test]
fn built_in_controls_native_escape_and_effects_construct_real_elements() {
    let mut node = ui! {
        <div style="opacity:0.9">
            <div style="display:grid; grid-template-columns:1fr 1fr"><div>Un</div><div>Deux</div></div>
            <input type="text" value="Bonjour" oninput={Msg::Text}>
            <input type="range" value="30" oninput={Msg::Value} min="0" max="100">
            <input type="checkbox" checked onchange={Msg::Checked}> Actif
            {@element text("escape","Rust libre")}
            <div style="overflow:auto; height:60px"><div><span>Défilement</span></div></div>
        </div>
    };
    arrange(&mut node);
    assert!(matches!(
        node.children[0].layout,
        Layout::Grid { columns: 2 }
    ));
    assert!(
        node.children[1]
            .widget
            .as_ref()
            .unwrap()
            .text_input()
            .is_some()
    );
    assert!(node.children[2].widget.as_ref().unwrap().slider().is_some());
    assert_eq!(
        node.children[3].widget.as_ref().unwrap().semantics().name,
        "Actif"
    );
    assert!(node.find(ElementId::named("escape")).is_some());
    assert!(matches!(node.children[5].layout, Layout::Scroll(_)));
}

#[test]
fn gradients_validate_stops_and_rust_elements_keep_their_identity() {
    let bad = LinearGradient::new(
        0.,
        [
            GradientStop {
                position: 0.8,
                color: Color::hex(0),
            },
            GradientStop {
                position: 0.2,
                color: Color::hex(0xffffff),
            },
        ],
    );
    let invalid = ui! {<div style:background={bad}></div>};
    assert!(matches!(
        layout::validate(&invalid),
        Err(karbon_ui::Error::InvalidInput { .. })
    ));
    let invalid =
        ui! {<div style:background={linear_gradient(f32::NAN,[Color::hex(0),Color::hex(1)])} />};
    assert!(matches!(
        layout::validate(&invalid),
        Err(karbon_ui::Error::InvalidInput { .. })
    ));
    let valid = ui! {<div>{@element text("old","Texte")}</div>};
    assert_eq!(valid.children[0].id, ElementId::named("old"));
    let widget = ui! {<div>{@element Button::new("Rust",Msg::Click(2))}</div>};
    assert!(
        widget.children[0]
            .widget
            .as_ref()
            .unwrap()
            .action()
            .is_some()
    );
    let disabled = ui! {<button disabled>Disabled</button>};
    assert!(disabled.widget.as_ref().unwrap().action().is_none());
    let escaped = ui! {<span>&#x42;&#111;njour &amp; adieu</span>};
    assert_eq!(
        escaped.widget.as_ref().unwrap().semantics().name,
        "Bonjour & adieu"
    );
}

/// Données statiques d'un module, comme `data.rs` de l'exemple.
mod data {
    /// Non `Copy` : une itération par valeur ne compilerait pas (`static`).
    pub struct Track {
        pub title: &'static str,
    }
    pub static TRACKS: [Track; 2] = [Track { title: "Un" }, Track { title: "Deux" }];
    pub static TITLES: [&str; 2] = ["Trois", "Quatre"];
    pub const PAIRS: [(u64, &str); 1] = [(5, "Cinq")];
}

/// `{#each data::LISTE as x}` parcourt la liste par référence (comme `self.x`) au lieu
/// de la consommer.
#[test]
fn each_over_a_module_path_iterates_by_reference() {
    fn title(track: &data::Track) -> &str {
        track.title
    }
    let node = ui! {
        <div>
            {#each data::TRACKS as track (track.title)}<p>{title(track)}</p>{/each}
            {#each data::TITLES as t (*t)}<p>{t}</p>{/each}
            {#each self::data::PAIRS as pair (pair.0)}<p>{pair.1}</p>{/each}
        </div>
    };
    let names = node
        .children
        .iter()
        .map(|c| c.widget.as_ref().unwrap().semantics().name)
        .collect::<Vec<_>>();
    assert_eq!(names, ["Un", "Deux", "Trois", "Quatre", "Cinq"]);
}

#[derive(Clone, Debug)]
enum Large {
    Play,
}

/// Page de 24 sections de 12 cartes (~1 500 éléments), entièrement en `{@include}`.
struct LargePage {
    n: u32,
}

impl LargePage {
    fn view(&self) -> Element {
        ui_file!("ui/large/page.ui")
    }
}

fn count(node: &Element) -> usize {
    1 + node.children.iter().map(count).sum::<usize>()
}

/// Le code généré ne met pas toute la page dans un seul cadre de pile : en debug, une
/// page faite d'`{@include}` se construit sur un thread de 1 Mio (la pile du thread
/// principal sous Windows).
#[test]
fn a_large_included_template_builds_on_a_one_megabyte_stack() {
    let built = std::thread::Builder::new()
        .stack_size(1 << 20)
        .spawn(|| count(&LargePage { n: 7 }.view()))
        .unwrap()
        .join()
        .unwrap();
    assert!(built > 1500, "{built}");
}

/// `title=""` (littéral ou calculé) n'affiche aucune infobulle et ne bloque pas la vue,
/// comme en HTML.
#[test]
fn an_empty_title_shows_no_tooltip() {
    let empty = String::new();
    let node = ui! {
        <div>
            <div title={empty.as_str()}>Un</div>
            <button title="" onclick={Msg::Click(1)}>Deux</button>
            <button title={" "} onclick={Msg::Click(2)}><div>Trois</div></button>
        </div>
    };
    assert!(layout::validate(&node).is_ok());
    // Le constructeur Rust garde son contrôle : une infobulle vide y est une erreur.
    assert!(layout::validate(&Element::column(9, Vec::new()).tooltip("")).is_err());
}
