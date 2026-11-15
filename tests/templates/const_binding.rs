//! `{@const motif = valeur}` : une valeur nommée une fois, visible jusqu'à la fin de la
//! liste où la directive apparaît (élément, branche de bloc, corps de boucle).
use crate::support::*;
use karbon_ui::ui::prelude::*;

fn names(node: &Element) -> Vec<String> {
    node.children
        .iter()
        .map(|c| c.widget.as_ref().unwrap().semantics().name.to_string())
        .collect()
}

#[test]
fn a_constant_reaches_the_following_siblings() {
    let tracks = ["Polly", "Lithium", "Breed"];
    let node = ui! {
        <div>
            <p>Album</p>
            {@const count = tracks.len()}
            <p title={count.to_string()}>{count} titres</p>
            <button onclick={Msg::Click(count as u8)}>Lire {count}</button>
        </div>
    };
    assert_eq!(names(&node), ["Album", "3 titres", "Lire 3"]);
}

#[test]
fn the_value_lives_for_the_whole_scope() {
    // Le temporaire `String` vit pendant tout le corps, comme l'itérable d'un `for`.
    let node = ui! {
        <div>
            {@const name = String::from("Nevermind").as_str()}
            <p>{name}</p>
            <p>{name.len()}</p>
        </div>
    };
    assert_eq!(names(&node), ["Nevermind", "9"]);

    // Un littéral de structure est une valeur comme une autre.
    struct Span {
        from: u8,
        to: u8,
    }
    let node = ui! {
        <div>
            {@const Span { from, to } = Span { from: 1, to: 4 }}
            <p>{from} à {to}</p>
        </div>
    };
    assert_eq!(names(&node), ["1 à 4"]);
}

#[test]
fn a_constant_ends_with_its_branch() {
    let label = |n: usize| {
        ui! {
            <div>
                {#if n > 1}
                    {@const shown = n - 1}
                    <p>{shown} de plus</p>
                {:else}
                    {@const shown = "seul"}
                    <p>{shown}</p>
                {/if}
                <p>fin</p>
            </div>
        }
    };
    assert_eq!(names(&label(4)), ["3 de plus", "fin"]);
    assert_eq!(names(&label(1)), ["seul", "fin"]);
}

#[test]
fn patterns_comparisons_and_loops() {
    let pairs = [("a", 1u8), ("b", 2)];
    let node = ui! {
        <div>
            {#each pairs as pair (pair.0)}
                {@const (name, n) = *pair}
                {@const even = n % 2 == 0}
                <p>{name}{if even { " pair" } else { "" }}</p>
            {/each}
        </div>
    };
    assert_eq!(names(&node), ["a", "b pair"]);
}

#[test]
fn a_constant_inside_text_and_options() {
    let year = 1991;
    let line = ui! {
        <p>Sorti {@const decade = year / 10 * 10}en {year} (années {decade})</p>
    };
    assert_eq!(
        line.widget.as_ref().unwrap().semantics().name,
        "Sorti en 1991 (années 1990)"
    );

    let node = ui! {
        <select onchange={|_| Msg::Click(0)}>
            {@const picked = 2u64}
            <option value={1u64}>Un</option>
            <option value={2u64} selected={picked == 2}>Deux</option>
        </select>
    };
    let select = node.widget.as_ref().unwrap().select().unwrap();
    assert_eq!(select.options.len(), 2);
    assert_eq!(select.selected, Some(2));
}
