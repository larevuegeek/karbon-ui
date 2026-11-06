//! Composants : slots nommés et par défaut, fragments inclus, identités indépendantes des
//! instances, props explicites, et directives `style:` / `style="…"` appliquées à la
//! racine du composant.
use crate::support::*;
use karbon_ui::ui::{prelude::*, scene::RectF};

fn near(a: f32, b: f32) {
    assert!((a - b).abs() < 0.01, "{a} != {b}");
}

#[derive(Default)]
struct Shell;

impl Component for Shell {
    fn render(self, cx: &TemplateContext<'_>, mut slots: Slots) -> Element {
        ui! {@context(cx);@slots(slots);
            <div style="gap:8px">
                <div style:flex-direction={"row"}><slot name="header"><span>Fallback</span></slot></div>
                <div><slot /></div>
            </div>
        }
    }
}

struct RequiredProps {
    title: String,
}

impl Component for RequiredProps {
    fn render(self, cx: &TemplateContext<'_>, _: Slots) -> Element {
        ui! {@context(cx);<span>{self.title}</span>}
    }
}

#[test]
fn component_slots_move_children_and_instances_have_independent_ids() {
    let mut node = ui! {
        <div style:flex-direction={"row"}>
            <Shell><span slot="header">Premier</span><span>Corps</span></Shell>
            <Shell><span>Second</span></Shell>
        </div>
    };
    arrange(&mut node);
    assert_ne!(node.children[0].id, node.children[1].id);
    let first = &node.children[0].children[0].children[0];
    let second = &node.children[1].children[0].children[0];
    assert_eq!(first.widget.as_ref().unwrap().semantics().name, "Premier");
    assert_eq!(second.widget.as_ref().unwrap().semantics().name, "Fallback");
    assert_eq!(
        node.children[0].children[1].children[0]
            .widget
            .as_ref()
            .unwrap()
            .semantics()
            .name,
        "Corps"
    );
    let explicit =
        ui! {<RequiredProps id="component" props={RequiredProps{title:"Explicit".into()}} />};
    assert_eq!(explicit.id, ElementId::named("component"));
    assert_eq!(
        explicit.widget.as_ref().unwrap().semantics().name,
        "Explicit"
    );
}

#[test]
fn included_fragments_keep_their_named_and_default_slots() {
    let mut node = ui! {
        <Shell>{@include "ui/slots.ui"}</Shell>
    };
    arrange(&mut node);
    assert_eq!(
        node.children[0].children[0]
            .widget
            .as_ref()
            .unwrap()
            .semantics()
            .name,
        "En-tête inclus"
    );
    assert_eq!(
        node.children[1].children[0]
            .widget
            .as_ref()
            .unwrap()
            .semantics()
            .name,
        "Contenu inclus"
    );
}

/// Composant dont la racine reçoit les `style:` de l'appelant.
#[derive(Default)]
struct Badge {
    label: &'static str,
}

impl Component for Badge {
    fn render(self, cx: &TemplateContext<'_>, _: Slots) -> Element {
        ui! {@context(cx);<p style="margin:0">{self.label}</p>}
    }
}

/// `style:prop={…}` et `style="…"` sur un composant s'appliquent à sa racine
/// (ils étaient ignorés sans erreur) ; `style={…}` reste une prop.
#[test]
fn style_directives_on_a_component_apply_to_its_root() {
    let red = Color::hex(0xff2040);
    let mut node = ui! {
        <div>
            <Badge id="badge" label="Nouveau" style:background={red} style:width={120.}
                style="padding:4px" />
        </div>
    };
    let bounds = RectF::new(0., 0., 300., 200.);
    karbon_ui::ui::styling::resolve(&mut node, Default::default()).unwrap();
    karbon_ui::ui::layout::validate(&node).unwrap();
    karbon_ui::ui::layout::arrange(&mut node, bounds, bounds, &mut Measure).unwrap();
    let badge = node.find(ElementId::named("badge")).unwrap();
    assert_eq!(badge.style.background, Some(red));
    near(badge.bounds().right - badge.bounds().left, 120.);
    near(badge.style.padding.left, 4.);
}
