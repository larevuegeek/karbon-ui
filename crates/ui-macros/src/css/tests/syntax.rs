//! Lecture des feuilles strictes : règles, chaînes, commentaires, erreurs et noms de classes.
use crate::css::parser;
#[test]
fn parser_keeps_strings_comments_imports_and_last_declarations() {
    let doc = parser::parse(
        r#"@import "palette.css";
        /* comment with { } */
        .card, .second:hover { color: rgb(1, 2, 3); font-family: "A /* not a comment */ B" }
        :root {--Accent: #abc;}
    "#,
    )
    .unwrap();
    assert_eq!(doc.len(), 3);
    let parser::Item::Rule {
        selectors,
        declarations,
        ..
    } = &doc[1]
    else {
        panic!()
    };
    assert_eq!(selectors, &[".card", ".second:hover"]);
    assert_eq!(declarations[1].value, "\"A /* not a comment */ B\"");
    let parser::Item::Rule { declarations, .. } = &doc[2] else {
        panic!()
    };
    assert_eq!(declarations[0].name, "--Accent");
}
#[test]
fn malformed_css_returns_errors_instead_of_panicking() {
    for bad in [
        ".a { color: red;",
        "/* unclosed",
        ".a ) {color:red}",
        ".a { color: rgb(1;}",
        ".a { padding: ; }",
        ".a { padding 2px; }",
        ".a {opacity:1!important;}",
    ] {
        assert!(parser::parse(bad).is_err(), "{bad}");
    }
    assert!(parser::class_name("bad name").is_err());
    assert_ne!(
        parser::class_symbol("Card").unwrap(),
        parser::class_symbol("card").unwrap()
    );
}
/// Les blocs imbriqués sont lus récursivement : leur profondeur est bornée (des milliers
/// de `@media` faisaient déborder la pile du compilateur).
#[test]
fn deeply_nested_blocks_are_refused_at_their_position() {
    let nest = |n: usize, inner: &str| {
        let mut css = "@media (min-width: 1px) {".repeat(n);
        css += inner;
        css += &"}".repeat(n);
        css
    };
    assert!(parser::parse(&nest(32, ".a { color: red; }")).is_ok());
    let error = parser::parse(&nest(33, ".a { color: red; }")).unwrap_err();
    assert_eq!(error.message, "CSS blocks nest deeper than 32 levels");
    assert_eq!(error.at, 32 * "@media (min-width: 1px) {".len());
    let error =
        parser::parse(&nest(32, ".a { @media (min-width: 2px) { color: red; } }")).unwrap_err();
    assert_eq!(error.at, 32 * "@media (min-width: 1px) {".len() + 5);
    assert!(parser::parse(&nest(10_000, "")).is_err());
}
