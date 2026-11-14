//! Interroge un serveur de mises à jour comme le ferait une application, sans rien installer.
//!
//! ```text
//! cargo run -p karbon-updater --example check -- <clé publique> <version actuelle> <adresse> [--download]
//! ```
//!
//! La clé publique est un fichier (`.pub` de minisign, ou le base64 de `tauri signer`) ou
//! le texte lui-même. Avec `--download`, le paquet est téléchargé et sa signature vérifiée.
use karbon_updater::{Installation, Updater};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [key, version, endpoint, rest @ ..] = args.as_slice() else {
        eprintln!("usage: check <public key> <current version> <endpoint> [--download]");
        std::process::exit(2);
    };
    let key = std::fs::read_to_string(key).unwrap_or_else(|_| key.clone());
    println!("installation : {:?}", Installation::current());
    let updater = Updater::new(key, version).endpoint(endpoint);
    let Some(update) = updater.check()? else {
        println!("à jour");
        return Ok(());
    };
    println!(
        "version {} ({})",
        update.version,
        update.date.as_deref().unwrap_or("?")
    );
    println!("paquet : {}", update.download_url());
    if rest.iter().any(|a| a == "--download") {
        let mut shown = 0;
        let package = update.download(|done, total| {
            if done >= shown + (1 << 20) || Some(done) == total {
                shown = done;
                println!("  {} / {} Kio", done >> 10, total.map_or(0, |t| t >> 10));
            }
        })?;
        println!(
            "signature vérifiée, paquet dans {}",
            package.path().display()
        );
    }
    Ok(())
}
