//! Sélecteurs (combinateurs, pseudo-éléments) et requêtes `@media` / `@container`.
use crate::css::{media, parser, selector, values};
#[test]
fn combinators_compile_to_ancestor_parent_and_sibling_predicates() {
    let parse = |s: &str| selector::parse(s);
    let s = parse(".group:hover .reveal").unwrap();
    assert_eq!(s.class, "reveal");
    assert!(s.classes.contains(&"group".to_string()));
    let p = s.predicate.unwrap();
    assert!(
        p.contains("Ancestor(&") && p.contains("Class(\"group\")") && p.contains("Hover"),
        "{p}"
    );
    let p = parse(".a > .b.c:first-child").unwrap().predicate.unwrap();
    assert!(
        p.contains("Parent(&") && p.contains("FirstChild") && p.contains("Class(\"c\")"),
        "{p}"
    );
    assert!(
        parse(".a+.b")
            .unwrap()
            .predicate
            .unwrap()
            .contains("PreviousSibling")
    );
    assert!(
        parse(".a ~ .b")
            .unwrap()
            .predicate
            .unwrap()
            .contains("AnyPreviousSibling")
    );
    // Chaîne : .a .b > .c  =>  Parent(All[Class(b), Ancestor(Class(a))]).
    let p = parse(".a .b > .c").unwrap().predicate.unwrap();
    assert!(
        p.find("Parent").unwrap() < p.find("Ancestor").unwrap(),
        "{p}"
    );
    assert!(parse(".a:nth-child(2n + 1) .b").is_ok());
    // Balises et attributs écrits dans le template ; le sujet garde une classe.
    assert!(parse("div .a").is_ok());
    assert!(parse(".liste > p.note:first-child").is_ok());
    assert!(parse(".a > .b[data-x=\"1\" i]").is_ok());
    for bad in [
        "> .a",
        ".a >",
        ".a > > .b",
        ".a div",
        ".a #b",
        ".a [x]",
        ".a .b .c .d .e .f .g .h .i",
    ] {
        assert!(parse(bad).is_err(), "{bad}");
    }
}
#[test]
fn pseudo_elements_attach_to_the_host_class() {
    let s = selector::parse(".group:hover .badge::after").unwrap();
    assert_eq!((s.class.as_str(), s.pseudo), ("badge", Some("after")));
    assert!(s.predicate.unwrap().contains("Ancestor"));
    assert_eq!(selector::parse(".a:before").unwrap().pseudo, Some("before"));
    assert_eq!(
        selector::parse(".champ::placeholder").unwrap().pseudo,
        Some("placeholder")
    );
    for bad in [".a::marker", ".a::before .b", ".a::selection", "::before"] {
        assert!(selector::parse(bad).is_err(), "{bad}");
    }
    assert_eq!(values::pseudo_content("\"\"").unwrap(), Some(String::new()));
    assert_eq!(values::pseudo_content("'→'").unwrap(), Some("→".into()));
    assert_eq!(values::pseudo_content("none").unwrap(), None);
    assert!(values::pseudo_content("attr(title)").is_err());
    assert!(values::pseudo_content("counter(x)").is_err());
}
#[test]
fn media_queries_reject_missing_conditions() {
    for invalid in ["", "not", "screen and", "(min-width: 1px) and", ", screen"] {
        assert!(media::condition(invalid).is_err(), "{invalid}");
    }
    for valid in [
        "screen",
        "not all",
        "screen and (min-width: 1px)",
        "(orientation: portrait)",
    ] {
        assert!(media::condition(valid).is_ok(), "{valid}");
    }
}
#[test]
fn container_queries_and_range_syntax_compile_to_bounds() {
    let c = media::condition("@container sidebar (width >= 48rem) and (height < 300px)").unwrap();
    assert!(
        c.contains("Container(") && c.contains("Some(\"sidebar\")"),
        "{c}"
    );
    assert!(
        c.contains("min_width: 768.0") && c.contains("max_height: 299.99"),
        "{c}"
    );
    let c = media::condition("@container (400px < width <= 800px)").unwrap();
    assert!(
        c.contains("name: None")
            && c.contains("min_width: 400.01")
            && c.contains("max_width: 800.0"),
        "{c}"
    );
    let c = media::condition("(width >= 700px)").unwrap();
    assert!(
        c.contains("Media(") && c.contains("min_width: 700.0"),
        "{c}"
    );
    for bad in [
        "@container style(--x: 1)",
        "@container (orientation: landscape)",
        "@container panel",
        "@container (width >= 10px) or (width < 2px)",
        "(width ~ 10px)",
    ] {
        assert!(media::condition(bad).is_err(), "{bad}");
    }
    // Requêtes imbriquées dans une règle : même sélecteur, après la règle.
    let doc =
        parser::parse(".a { color: red; @container card (min-width: 10px) { color: blue; } }")
            .unwrap();
    assert_eq!(doc.len(), 2);
    let parser::Item::Media { query, items, .. } = &doc[1] else {
        panic!()
    };
    assert_eq!(query, "@container card (min-width: 10px)");
    let parser::Item::Rule {
        selectors,
        declarations,
        ..
    } = &items[0]
    else {
        panic!()
    };
    assert_eq!(selectors, &[".a"]);
    assert_eq!(declarations[0].value, "blue");
    assert!(parser::parse(".a { @supports (x) { color: red; } }").is_err());
    assert_eq!(
        values::property("container", "card / inline-size").unwrap()[0].1,
        "&[\"card\"]"
    );
    assert!(values::property("container-type", "block-size").is_err());
}
