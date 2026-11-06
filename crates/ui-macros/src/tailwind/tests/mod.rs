//! Tests du compilateur Tailwind : déclarations produites par des listes de classes
//! (thème par défaut, ordre, variantes, propriétés composées, couches) et diagnostics.
//! `oracle` compare un échantillon d'utilitaires à la feuille générée par Tailwind.
pub(super) mod oracle;
use super::{Config, Group, compile_set};

fn config() -> Config {
    Config::new(&[], &[], &[]).unwrap()
}
/// Groupes d'une liste, écrits comme une petite feuille CSS lisible.
fn render(set: &[Group]) -> String {
    let mut out = String::new();
    for g in set {
        let mut head = Vec::new();
        head.extend(g.media.iter().cloned());
        head.extend(
            g.states
                .iter()
                .map(|s| s.replace("__ui_root::ui::styling::StyleState::", "")),
        );
        if let Some(p) = g.pseudo {
            head.push(format!("::{p}"));
        }
        out += &format!("{} {{", head.join(" & "));
        for (n, v) in &g.declarations {
            out += &format!(" {n}: {v};");
        }
        out += " }\n";
    }
    out
}
fn css(classes: &str) -> String {
    let tokens = classes.split_whitespace().collect::<Vec<_>>();
    let set = compile_set(&tokens, &config()).unwrap_or_else(|(i, m)| panic!("{}: {m}", tokens[i]));
    render(&set.normal) + &render(&set.important)
}
fn error(classes: &str) -> String {
    let tokens = classes.split_whitespace().collect::<Vec<_>>();
    match compile_set(&tokens, &config()) {
        Ok(_) => panic!("{classes} should be rejected"),
        Err((_, m)) => m,
    }
}

#[test]
fn spacing_sizes_and_colors_follow_the_default_theme() {
    assert_eq!(
        css("flex items-center gap-1.5 px-3 py-0.5 w-1/2 h-full"),
        " { display: flex; height: 100%; width: 50%; align-items: center; gap: 6px; padding-top: 2px; padding-bottom: 2px; padding-left: 12px; padding-right: 12px; }\n"
    );
    assert_eq!(
        css("bg-white/5 text-neutral-300"),
        " { background-color: color-mix(in oklab, #fff 5%, transparent); color: oklch(87% 0 none); }\n"
    );
    assert_eq!(css("text-[13px]"), " { font-size: 13px; }\n");
    assert_eq!(
        css("text-sm"),
        " { font-size: 14px; line-height: 1.428571429; }\n"
    );
    assert_eq!(
        css("text-sm leading-tight"),
        " { font-size: 14px; line-height: 1.25; }\n"
    );
    assert_eq!(
        css("text-sm/6"),
        " { font-size: 14px; line-height: 24px; }\n"
    );
    assert_eq!(
        css("-mt-2 rounded-lg"),
        " { margin-top: -8px; border-radius: 8px; }\n"
    );
    assert_eq!(css("text-(--rg-mu)"), " { color: var(--rg-mu); }\n");
    assert_eq!(
        css("bg-[#123]/50"),
        " { background-color: color-mix(in oklab, #123 50%, transparent); }\n"
    );
}
#[test]
fn later_utilities_of_the_same_property_win_in_tailwind_order() {
    // `px-2` suit `p-4` dans la feuille de Tailwind, quel que soit l'ordre de l'attribut.
    assert_eq!(
        css("px-2 p-4"),
        " { padding: 16px; padding-left: 8px; padding-right: 8px; }\n"
    );
}
#[test]
fn variants_become_states_queries_and_pseudo_elements() {
    let out = css("bg-white/5 hover:bg-white/10 md:flex @3xl:grid dark:text-neutral-300");
    assert!(
        out.contains("Hover { background-color: color-mix(in oklab, #fff 10%, transparent); }"),
        "{out}"
    );
    assert!(out.contains("(width >= 48rem) { display: flex; }"), "{out}");
    assert!(
        out.contains("@container (width >= 48rem) { display: grid; }"),
        "{out}"
    );
    assert!(out.contains("Where(&Any"), "{out}");

    let out = css("group-hover:opacity-100 before:absolute");
    assert!(
        out.contains("Ancestor(&All(&[Where(&Class(\"group\")),Hover]))"),
        "{out}"
    );
    assert!(
        out.contains("::before { content: \"\"; position: absolute; }"),
        "{out}"
    );
    let out = css("group-hover/vol:opacity-100");
    assert!(out.contains("Class(\"group/vol\")"), "{out}");
    let out = css("max-[900px]:hidden @max-[760px]:hidden");
    assert!(out.contains("(width < 900px)"), "{out}");
    assert!(out.contains("@container (width < 760px)"), "{out}");
}
#[test]
fn composed_properties_resolve_tailwind_variables_per_condition() {
    assert_eq!(
        css("translate-x-2 hover:scale-95"),
        " { transform: translate(8px, 0); }\nHover { transform: translate(8px, 0) scale(0.95); }\n"
    );
    assert_eq!(
        css("shadow-lg ring-1 ring-white/10"),
        " { box-shadow: 0 0 0 1px color-mix(in oklab, #fff 10%, transparent), 0 10px 15px -3px rgb(0 0 0 / 0.1), 0 4px 6px -4px rgb(0 0 0 / 0.1); }\n"
    );
    assert_eq!(
        css("transition-colors duration-200"),
        " { transition: color 200ms cubic-bezier(0.4, 0, 0.2, 1), background-color 200ms cubic-bezier(0.4, 0, 0.2, 1), border-color 200ms cubic-bezier(0.4, 0, 0.2, 1); }\n"
    );
    // Largeur et hauteur s'animent aussi (barre latérale repliée, rail du volume).
    assert_eq!(
        css("transition-[width] duration-200"),
        " { transition: width 200ms cubic-bezier(0.4, 0, 0.2, 1); }\n"
    );
    assert_eq!(
        css("transition-[height,box-shadow] duration-150 ease-out"),
        " { transition: height 150ms cubic-bezier(0, 0, 0.2, 1); }\n"
    );
    assert_eq!(
        css("bg-linear-to-br from-black to-white"),
        " { background-image: linear-gradient(to bottom right, #000 0%, #fff 100%); }\n"
    );
    assert_eq!(
        css("bg-linear-to-r from-black/30 to-transparent"),
        " { background-image: linear-gradient(to right, color-mix(in oklab, #000 30%, transparent) 0%, transparent 100%); }\n"
    );
    assert_eq!(
        css("blur-sm brightness-50"),
        " { filter: blur(8px) brightness(50%); }\n"
    );
    assert_eq!(
        css("tabular-nums"),
        " { font-variant-numeric: tabular-nums; }\n"
    );
    assert_eq!(
        css("border border-white/8"),
        " { border-style: solid; border-width: 1px; border-color: color-mix(in oklab, #fff 8%, transparent); }\n"
    );
}
#[test]
fn one_sided_borders_keep_their_color_on_those_sides() {
    assert_eq!(
        css("border-b border-white/6"),
        " { border-bottom-style: solid; border-bottom-width: 1px; border-bottom-color: color-mix(in oklab, #fff 6%, transparent); }\n"
    );
    assert_eq!(
        css("border-t hover:border-white"),
        " { border-top-style: solid; border-top-width: 1px; }\nHover { border-top-color: #fff; }\n"
    );
}
#[test]
fn stacked_variants_sort_after_their_parts() {
    // `dark:hover:` suit `hover:` (même condition quand dark est la stratégie `&`) et
    // `md:` vient après `hover:`.
    let cfg = Config::new(&[], &[], &[("dark".into(), "&".into())]).unwrap();
    let set = compile_set(
        &["dark:hover:bg-white", "hover:bg-black", "md:p-2", "p-4"],
        &cfg,
    )
    .unwrap();
    assert_eq!(
        render(&set.normal),
        " { padding: 16px; }\nHover { background-color: #fff; }\n(width >= 48rem) { padding: 8px; }\n"
    );
}
#[test]
fn important_utilities_form_their_own_layer() {
    let tokens = ["font-bold", "!font-black"];
    let set = compile_set(&tokens, &config()).unwrap();
    assert_eq!(render(&set.important), " { font-weight: 900; }\n");
}
#[test]
fn near_misses_suggest_the_intended_utility() {
    assert!(error("itms-center").contains("did you mean 'items-center'"));
    assert!(error("bg-whit/5").contains("did you mean 'white'"));
    assert!(
        error("text-nuetral-300").contains("did you mean 'text-neutral-300'"),
        "{}",
        error("text-nuetral-300")
    );
    assert!(error("hovr:flex").contains("did you mean 'hover:'"));
    assert!(error("aria-sorted:flex").contains("aria-[sorted=…]"));
    assert!(error("[&_a]:flex").contains("[& a]"));
}
#[test]
fn theme_overrides_and_custom_variants_apply() {
    let cfg = Config::new(
        &[
            (
                "--color-accent-500".into(),
                "var(--acc-500, #22c55e)".into(),
            ),
            ("--spacing".into(), "0.3rem".into()),
        ],
        &[],
        &[("dark".into(), "&".into())],
    )
    .unwrap();
    let set = compile_set(&["bg-accent-500/50", "p-2", "dark:text-white"], &cfg).unwrap();
    assert_eq!(
        render(&set.normal),
        " { background-color: color-mix(in oklab, var(--acc-500, #22c55e) 50%, transparent); padding: 9.6px; color: #fff; }\n"
    );
}
