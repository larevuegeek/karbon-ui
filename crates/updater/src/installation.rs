//! Comment l'application en cours a été installée, donc comment la mettre à jour.
use std::path::{Path, PathBuf};

/// How the running application was installed, which decides how an update is applied.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Installation {
    /// Windows: installed by an NSIS or MSI installer; the update runs the new installer.
    WindowsInstaller,
    /// macOS: this `.app` bundle, replaced by the one in the update.
    MacBundle(PathBuf),
    /// Linux: this AppImage file (`$APPIMAGE`), replaced by the new one.
    AppImage(PathBuf),
    /// Linux: a Debian package, installed with `pkexec dpkg -i` (password prompt).
    Deb,
    /// Linux: an RPM package, installed with `pkexec rpm -U` (password prompt).
    Rpm,
    /// This installation cannot update itself; the text says why (Flatpak or Snap, whose
    /// store updates the application; a development build; a program not installed by a
    /// package).
    Unsupported(String),
}

impl Installation {
    /// The current process's installation.
    pub fn current() -> Self {
        let Ok(exe) = std::env::current_exe() else {
            return Self::Unsupported("the executable's path is unknown".into());
        };
        if development_build(&exe) {
            return Self::Unsupported("development build (run from a Cargo target folder)".into());
        }
        detect(&exe)
    }

    /// The application can update itself.
    pub fn can_update(&self) -> bool {
        !matches!(self, Self::Unsupported(_))
    }

    /// Kind of package, as in the manifest keys `<os>-<arch>-<kind>` (`{{bundle_type}}`).
    pub(crate) fn kind(&self) -> Option<&'static str> {
        match self {
            Self::WindowsInstaller => Some("nsis"),
            Self::MacBundle(_) => Some("app"),
            Self::AppImage(_) => Some("appimage"),
            Self::Deb => Some("deb"),
            Self::Rpm => Some("rpm"),
            Self::Unsupported(_) => None,
        }
    }
}

/// Exécutable lancé depuis `target/debug`, `target/release` ou `target/<cible>/…` : une
/// mise à jour y remplacerait la compilation du développeur.
pub(crate) fn development_build(exe: &Path) -> bool {
    let parts: Vec<String> = exe
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_ascii_lowercase())
        .collect();
    parts.windows(3).any(|w| {
        w[0] == "target"
            && (matches!(w[1].as_str(), "debug" | "release")
                || matches!(w[2].as_str(), "debug" | "release"))
    })
}

#[cfg(windows)]
fn detect(_exe: &Path) -> Installation {
    Installation::WindowsInstaller
}

#[cfg(target_os = "macos")]
fn detect(exe: &Path) -> Installation {
    // …/Nom.app/Contents/MacOS/exécutable
    exe.parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .filter(|bundle| bundle.extension().is_some_and(|e| e == "app"))
        .map(|bundle| Installation::MacBundle(bundle.to_path_buf()))
        .unwrap_or_else(|| Installation::Unsupported("not running from an .app bundle".into()))
}

#[cfg(target_os = "linux")]
fn detect(exe: &Path) -> Installation {
    if std::env::var_os("FLATPAK_ID").is_some() || Path::new("/.flatpak-info").exists() {
        return Installation::Unsupported("Flatpak: updates come from the store".into());
    }
    if std::env::var_os("SNAP").is_some() {
        return Installation::Unsupported("Snap: updates come from the store".into());
    }
    if let Some(image) = std::env::var_os("APPIMAGE") {
        return Installation::AppImage(PathBuf::from(image));
    }
    let owned_by = |tool: &str, arg: &str| {
        std::process::Command::new(tool)
            .arg(arg)
            .arg(exe)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    };
    if owned_by("dpkg", "-S") {
        Installation::Deb
    } else if owned_by("rpm", "-qf") {
        Installation::Rpm
    } else {
        Installation::Unsupported("not installed by a package (deb, rpm or AppImage)".into())
    }
}

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
fn detect(_exe: &Path) -> Installation {
    Installation::Unsupported("unsupported operating system".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cargo_builds_are_development_builds() {
        for path in [
            "/home/u/app/target/debug/app",
            "/home/u/app/target/release/app",
            "C:/work/app/target/x86_64-pc-windows-msvc/release/app.exe",
            "C:/work/app/Target/Debug/app.exe",
        ] {
            assert!(development_build(Path::new(path)), "{path}");
        }
        for path in [
            "/usr/bin/app",
            "C:/Users/u/AppData/Local/App/app.exe",
            "/Applications/App.app/Contents/MacOS/app",
            "/opt/target/app",
        ] {
            assert!(!development_build(Path::new(path)), "{path}");
        }
    }
}
