//! macOS : remplace le paquet `.app` par celui de l'archive `.app.tar.gz`, comme le module
//! de mise à jour de Tauri. L'archive est extraite par `tar` (fourni par le système) à
//! côté de l'application, sur le même volume, pour que les renommages soient immédiats ;
//! l'ancienne application est gardée jusqu'à ce que la nouvelle soit en place. Sans droit
//! d'écriture (application d'un autre utilisateur dans `/Applications`), le remplacement
//! passe par `osascript` et demande le mot de passe d'un administrateur.
use super::relaunch;
use crate::{Error, Installation};
use std::{
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    process::Command,
};

pub(crate) fn install(package: &Path, installation: &Installation) -> Result<(), Error> {
    let bundle = match installation {
        Installation::MacBundle(bundle) => bundle.clone(),
        Installation::Unsupported(reason) => return Err(Error::Unsupported(reason.clone())),
        _ => return Err(Error::Unsupported("not running from an .app bundle".into())),
    };
    let work = work_folder(&bundle)?;
    let result = replace(package, &bundle, &work);
    let _ = std::fs::remove_dir_all(&work);
    result?;
    relaunch::after_exit(
        "/usr/bin/open".into(),
        ["-n".into(), bundle.into_os_string(), "--args".into()]
            .into_iter()
            .chain(relaunch::current_arguments())
            .collect(),
    )
}

/// Dossier de travail sur le volume de l'application : le dossier temporaire s'il y est,
/// sinon à côté de l'application.
fn work_folder(bundle: &Path) -> Result<PathBuf, Error> {
    let device = std::fs::symlink_metadata(bundle)?.dev();
    let name = format!("karbon-update-{}", std::process::id());
    let candidates = [
        Some(std::env::temp_dir()),
        bundle.parent().map(Path::to_path_buf),
    ];
    for root in candidates.into_iter().flatten() {
        if root.metadata().is_ok_and(|m| m.dev() == device) {
            let folder = root.join(&name);
            if std::fs::create_dir_all(&folder).is_ok() {
                return Ok(folder);
            }
        }
    }
    Err(Error::Install(
        "no writable folder on the application's volume".into(),
    ))
}

fn replace(package: &Path, bundle: &Path, work: &Path) -> Result<(), Error> {
    let extracted = work.join("new");
    std::fs::create_dir_all(&extracted)?;
    let status = Command::new("/usr/bin/tar")
        .arg("-xzf")
        .arg(package)
        .arg("-C")
        .arg(&extracted)
        .status()?;
    if !status.success() {
        return Err(Error::Install(
            "the .app.tar.gz package could not be extracted".into(),
        ));
    }
    let new_app = std::fs::read_dir(&extracted)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.extension().is_some_and(|e| e == "app"))
        .ok_or_else(|| Error::Install("the package contains no .app bundle".into()))?;
    let backup = work.join("previous.app");
    match std::fs::rename(bundle, &backup) {
        Ok(()) => {
            if let Err(error) = std::fs::rename(&new_app, bundle) {
                // L'ancienne application reprend sa place.
                let _ = std::fs::rename(&backup, bundle);
                return Err(error.into());
            }
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            privileged_replace(&new_app, bundle, &backup)
        }
        Err(error) => Err(error.into()),
    }
}

/// Remplacement avec les droits d'un administrateur (`osascript`, mot de passe demandé) ;
/// l'ancienne application n'est supprimée qu'une fois la nouvelle en place.
fn privileged_replace(new_app: &Path, bundle: &Path, backup: &Path) -> Result<(), Error> {
    let quote = |path: &Path| format!("'{}'", path.display().to_string().replace('\'', r"'\''"));
    let command = format!(
        "mv {bundle} {backup} && (mv {new} {bundle} || (mv {backup} {bundle}; exit 1)) && rm -rf {backup}",
        bundle = quote(bundle),
        backup = quote(backup),
        new = quote(new_app),
    );
    let script = format!(
        "do shell script \"{}\" with administrator privileges",
        command.replace('\\', r"\\").replace('"', "\\\"")
    );
    let status = Command::new("/usr/bin/osascript")
        .arg("-e")
        .arg(script)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::Install(
            "the application could not be replaced (administrator password refused?)".into(),
        ))
    }
}
