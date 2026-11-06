//! Relevé de l'oracle sur la feuille complète : chaque utilitaire accepté des `.svelte` de
//! RustMusic est comparé à la règle que Tailwind a générée (même normalisation que le test
//! de `tests::oracle`).
//!
//! ```text
//! cargo test -p karbon-ui-macros --lib tailwind::tools::oracle -- --ignored --nocapture
//! ```
//!
//! Variable : `RUSTMUSIC` (racine du projet, par défaut `J:/wamp/www/__rust/rustmusic`) ;
//! la feuille lue est la plus grande de `build/_app/immutable/assets`.
use super::coverage::{app_vars, walk_svelte};
use crate::tailwind::tests::oracle::{Oracle, difference, mine, rustmusic_theme};
use std::collections::BTreeMap;
#[test]
#[ignore = "outil de mesure : lit la feuille générée de RustMusic ; lancer avec --ignored --nocapture"]
fn every_accepted_rustmusic_utility_matches_the_generated_stylesheet() {
    let root = std::env::var("RUSTMUSIC").unwrap_or_else(|_| "J:/wamp/www/__rust/rustmusic".into());
    let dir = std::path::Path::new(&root).join("build/_app/immutable/assets");
    let Some(css) = std::fs::read_dir(&dir).ok().and_then(|d| {
        d.flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "css"))
            .max_by_key(|p| std::fs::metadata(p).map(|m| m.len()).unwrap_or(0))
            .and_then(|p| std::fs::read_to_string(p).ok())
    }) else {
        eprintln!("{} not found: skipped", dir.display());
        return;
    };
    let oracle = Oracle::new(&css);
    let app = std::fs::read_to_string(std::path::Path::new(&root).join("src/app.css"))
        .unwrap_or_default();
    let vars = app_vars(&app);
    let cfg = rustmusic_theme();
    let mut tokens = BTreeMap::new();
    walk_svelte(&std::path::Path::new(&root).join("src"), &mut tokens);
    let (mut same, mut compared, mut differences) = (0, 0, Vec::new());
    for token in tokens.keys() {
        let Ok(got) = mine(token, &cfg) else { continue };
        let Some(expected) = oracle.rule(token) else {
            continue;
        };
        compared += 1;
        match difference(&got, &oracle.normalized(&expected), &vars) {
            None => same += 1,
            Some(d) => differences.push(format!("{token}: {d}")),
        }
    }
    println!("{same} / {compared} accepted utilities match the generated stylesheet");
    for d in &differences {
        println!("  {d}");
    }
}
