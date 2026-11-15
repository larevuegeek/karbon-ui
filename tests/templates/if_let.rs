//! `{#if let motif = valeur}` : la valeur liée sert dans le corps (texte, attributs,
//! messages, options d'une liste), `{:else if let}` et `{:else}` prennent le relais, et
//! une condition peut chaîner un test (`let Some(n) = x && n > 1`).
use crate::support::*;
use karbon_ui::ui::prelude::*;

fn names(node: &Element) -> Vec<String> {
    node.children
        .iter()
        .map(|c| c.widget.as_ref().unwrap().semantics().name.to_string())
        .collect()
}

fn artist(name: Option<&str>, fallback: Option<&str>) -> Element {
    ui! {
        <div>
            {#if let Some(n) = name}
                <p title={n}>{n}</p>
            {:else if let Some(f) = fallback}
                <p>{f} (album)</p>
            {:else}
                <p>Inconnu</p>
            {/if}
        </div>
    }
}

#[test]
fn a_bound_value_is_used_by_the_branch_that_matched() {
    assert_eq!(names(&artist(Some("Nirvana"), None)), ["Nirvana"]);
    assert_eq!(names(&artist(None, Some("Divers"))), ["Divers (album)"]);
    assert_eq!(names(&artist(None, None)), ["Inconnu"]);
    assert_eq!(names(&artist(Some("Nirvana"), Some("Divers"))), ["Nirvana"]);
}

#[test]
fn a_binding_reaches_inline_text_messages_and_chained_tests() {
    let line = |year: Option<u16>| {
        ui! {
            <p>Sorti {#if let Some(y) = year}en {y}{:else}à une date inconnue{/if}</p>
        }
    };
    assert_eq!(
        line(Some(1991)).widget.as_ref().unwrap().semantics().name,
        "Sorti en 1991"
    );
    assert_eq!(
        line(None).widget.as_ref().unwrap().semantics().name,
        "Sorti à une date inconnue"
    );

    let count = |n: Option<u8>| {
        ui! {
            <div>
                {#if let Some(n) = n && n > 1}
                    <button onclick={Msg::Click(n)}>{n} titres</button>
                {/if}
            </div>
        }
    };
    assert_eq!(names(&count(Some(3))), ["3 titres"]);
    assert!(count(Some(1)).children.is_empty());
    assert!(count(None).children.is_empty());
}

#[test]
fn a_binding_feeds_the_options_of_a_list() {
    let extra: Option<u64> = Some(7);
    let node = ui! {
        <select onchange={|_| Msg::Click(0)}>
            <option value={1u64}>Un</option>
            {#if let Some(v) = extra}<option value={v} selected>Sept</option>{/if}
        </select>
    };
    let select = node.widget.as_ref().unwrap().select().unwrap();
    assert_eq!(select.options.len(), 2);
    assert_eq!(select.selected, Some(7));
}
