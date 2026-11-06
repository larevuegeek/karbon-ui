//! Outils de mesure (tests ignorés, lancés à la main) : scores de compatibilité cités
//! dans `docs/css-frameworks.md` (« Scores de compatibilité »).
//!
//! ```text
//! cargo test -p karbon-ui-macros compatibility -- --nocapture --include-ignored
//! ```
//!
//! La sortie Tailwind de RustMusic n'est pas versionnée ; son chemin se règle par
//! `KARBON_TAILWIND_CSS`.
use super::Sheet;

fn fixture(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// Score de compatibilité de Bootstrap 5.3.3 (`tests/fixtures/bootstrap`).
#[test]
#[ignore = "outil de mesure : score de compatibilité de Bootstrap (docs/css-frameworks.md)"]
fn bootstrap_compatibility() {
    let sheet = Sheet::compile(&fixture("bootstrap/bootstrap.css"));
    let r = &sheet.report;
    println!("{}", r.summary("bootstrap.css 5.3.3", 30));
    assert!(r.rules > 1500, "{}", r.rules);
    assert!(r.selectors_accepted * 100 >= r.selectors * 90);
    assert!(r.declarations_accepted * 100 >= r.declarations * 75);
    assert!(sheet.classes.contains("btn-primary") && sheet.classes.contains("show"));
}
/// Score de la sortie Tailwind 4 de RustMusic (fichier local, non versionné).
#[test]
#[ignore = "outil de mesure : lit la sortie Tailwind locale de RustMusic"]
fn rustmusic_tailwind_compatibility() {
    let path = std::env::var("KARBON_TAILWIND_CSS").unwrap_or_else(|_| {
        "J:/wamp/www/__rust/rustmusic/build/_app/immutable/assets/0.C_GSG_gi.css".into()
    });
    let Ok(text) = std::fs::read_to_string(&path) else {
        println!("skipped: {path} not found");
        return;
    };
    let sheet = Sheet::compile(&text);
    println!(
        "{}",
        sheet.report.summary("RustMusic Tailwind 4 output", 30)
    );
}
