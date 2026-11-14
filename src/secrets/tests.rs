//! Aller-retour dans le magasin du système, sous un service propre au test (supprimé à la
//! fin), et contrôle des arguments.
use super::{MAX_NAME_BYTES, MAX_SECRET_BYTES, Secrets};

#[test]
fn arguments_are_checked_before_the_system_store() {
    let secrets = Secrets::new("dev.karbon-ui.tests");
    assert!(secrets.set("", "x").is_err());
    assert!(secrets.set(&"k".repeat(MAX_NAME_BYTES + 1), "x").is_err());
    assert!(secrets.set("k", &"x".repeat(MAX_SECRET_BYTES + 1)).is_err());
    assert!(secrets.set("k", "a\0b").is_err());
    assert!(Secrets::new("").get("k").is_err());
}

/// Le Gestionnaire d'identification est disponible sur tout Windows ; ailleurs le
/// trousseau peut manquer (intégration continue sans session) : le test est alors sauté.
#[test]
fn a_secret_round_trips_through_the_system_store() {
    let secrets = Secrets::new(format!("dev.karbon-ui.tests.{}", std::process::id()));
    let key = "server/home";
    match secrets.set(key, "mot de passe é 🔑") {
        Ok(()) => {}
        Err(error) if !cfg!(windows) => {
            eprintln!("magasin de secrets indisponible, test sauté : {error}");
            return;
        }
        Err(error) => panic!("{error}"),
    }
    assert_eq!(
        secrets.get(key).unwrap().as_deref(),
        Some("mot de passe é 🔑")
    );
    secrets.set(key, "nouveau").unwrap();
    assert_eq!(secrets.get(key).unwrap().as_deref(), Some("nouveau"));
    secrets.remove(key).unwrap();
    assert_eq!(secrets.get(key).unwrap(), None);
    // Supprimer un secret absent ne fait rien.
    secrets.remove(key).unwrap();
    assert_eq!(secrets.get("absent").unwrap(), None);
}
