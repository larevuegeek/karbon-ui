//! Linux, comme le module de mise à jour de Tauri : AppImage remplacée sur place (le paquet
//! est l'AppImage, ou une archive `.AppImage.tar.gz` qui la contient) ; paquet `.deb` ou
//! `.rpm` installé par `pkexec dpkg -i` ou `pkexec rpm -U` (fenêtre de mot de passe du
//! bureau). L'application est ensuite relancée une fois ce processus terminé.
use super::relaunch;
use crate::{Error, Installation};
use std::{
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};

pub(crate) fn install(package: &Path, installation: &Installation) -> Result<(), Error> {
    let program = match installation {
        Installation::AppImage(image) => {
            replace_appimage(package, image)?;
            image.clone().into_os_string()
        }
        Installation::Deb => {
            privileged("dpkg", "-i", package)?;
            std::env::current_exe()?.into_os_string()
        }
        Installation::Rpm => {
            privileged("rpm", "-U", package)?;
            std::env::current_exe()?.into_os_string()
        }
        Installation::Unsupported(reason) => return Err(Error::Unsupported(reason.clone())),
        _ => return Err(Error::Unsupported("unknown Linux installation".into())),
    };
    relaunch::after_exit(program, relaunch::current_arguments())
}

/// `pkexec <outil> <option> <paquet>` : le bureau demande le mot de passe.
fn privileged(tool: &str, option: &str, package: &Path) -> Result<(), Error> {
    let status = Command::new("pkexec")
        .arg(tool)
        .arg(option)
        .arg(package)
        .status()
        .map_err(|e| Error::Install(format!("pkexec is unavailable ({e})")))?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::Install(format!(
            "{tool} {option} failed or the password was refused ({status})"
        )))
    }
}

/// Nouvelle AppImage écrite à côté de l'ancienne, rendue exécutable, puis renommée
/// par-dessus (l'AppImage en cours reste lisible jusqu'à la fin du processus).
fn replace_appimage(package: &Path, image: &Path) -> Result<(), Error> {
    let folder = image
        .parent()
        .ok_or_else(|| Error::Install("the AppImage has no folder".into()))?;
    let fresh = folder.join(format!(".{}.update", std::process::id()));
    let source = appimage_in(package, folder)?;
    std::fs::copy(&source, &fresh)?;
    std::fs::set_permissions(&fresh, std::fs::Permissions::from_mode(0o755))?;
    if let Err(error) = std::fs::rename(&fresh, image) {
        let _ = std::fs::remove_file(&fresh);
        return Err(error.into());
    }
    if source != package {
        let _ = std::fs::remove_file(&source);
    }
    Ok(())
}

/// L'AppImage du paquet : le paquet lui-même, ou celle de l'archive gzip (extraite par
/// `tar` dans `folder`).
fn appimage_in(package: &Path, folder: &Path) -> Result<PathBuf, Error> {
    let mut magic = [0; 2];
    let gzip = std::fs::File::open(package)
        .and_then(|mut file| std::io::Read::read_exact(&mut file, &mut magic))
        .is_ok()
        && magic == [0x1F, 0x8B];
    if !gzip {
        return Ok(package.to_path_buf());
    }
    let extracted = folder.join(format!(".{}.extract", std::process::id()));
    std::fs::create_dir_all(&extracted)?;
    let status = Command::new("tar")
        .arg("-xzf")
        .arg(package)
        .arg("-C")
        .arg(&extracted)
        .status()?;
    let found = std::fs::read_dir(&extracted)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("appimage"))
        });
    match (status.success(), found) {
        (true, Some(image)) => {
            let target = folder.join(format!(".{}.appimage", std::process::id()));
            std::fs::rename(&image, &target)?;
            let _ = std::fs::remove_dir_all(&extracted);
            Ok(target)
        }
        _ => {
            let _ = std::fs::remove_dir_all(&extracted);
            Err(Error::Install("the package contains no AppImage".into()))
        }
    }
}
