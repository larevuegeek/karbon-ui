//! Une mise à jour annoncée : téléchargement de son paquet, vérification de sa signature,
//! puis installation.
use crate::{Error, Installation, signature};
use std::{
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};

/// A newer version announced by the server, returned by
/// [`Updater::check`](crate::Updater::check).
#[derive(Debug, Clone)]
pub struct Update {
    /// Version announced (without a leading `v`).
    pub version: String,
    /// Release notes, as the server wrote them.
    pub notes: Option<String>,
    /// Publication date, as the server wrote it (RFC 3339 for the Tauri format).
    pub date: Option<String>,
    pub(crate) url: String,
    pub(crate) signature: String,
    pub(crate) public_key: String,
    pub(crate) user_agent: String,
}

impl Update {
    /// URL of the package.
    pub fn download_url(&self) -> &str {
        &self.url
    }

    /// Download the package, reporting `(bytes received, total size if known)` to
    /// `progress`, verify its signature, and keep it in the temporary folder. Blocks until
    /// the download ends: call it from a background task.
    ///
    /// # Errors
    ///
    /// [`Error::Network`] when the download fails, [`Error::Signature`] when the package
    /// is not signed by the key given to the updater or was signed for another version,
    /// [`Error::Io`] when it cannot be written.
    pub fn download(&self, mut progress: impl FnMut(u64, Option<u64>)) -> Result<Package, Error> {
        // Pas de limite globale : un gros paquet sur une connexion lente doit aboutir.
        let agent = ureq::Agent::config_builder()
            .timeout_connect(Some(Duration::from_secs(30)))
            .timeout_recv_response(Some(Duration::from_secs(60)))
            .user_agent(self.user_agent.as_str())
            .build()
            .new_agent();
        let response = agent
            .get(&self.url)
            .call()
            .map_err(|e| Error::Network(e.to_string()))?;
        let body = response.into_body();
        let total = body.content_length();
        let mut reader = body.into_reader();
        let mut bytes = Vec::with_capacity(total.unwrap_or(0).min(512 << 20) as usize);
        let mut chunk = vec![0; 64 * 1024];
        loop {
            let read = reader
                .read(&mut chunk)
                .map_err(|e| Error::Network(e.to_string()))?;
            if read == 0 {
                break;
            }
            bytes.extend_from_slice(&chunk[..read]);
            progress(bytes.len() as u64, total);
        }
        signature::verify(&bytes, &self.signature, &self.public_key, &self.version)?;
        let folder = std::env::temp_dir().join(format!("{}-{}-update", app_name(), self.version));
        std::fs::create_dir_all(&folder)?;
        let path = folder.join(file_name(&self.url));
        std::fs::write(&path, &bytes)?;
        Ok(Package {
            path,
            version: self.version.clone(),
        })
    }
}

/// A downloaded and verified package, ready to install.
#[derive(Debug)]
pub struct Package {
    path: PathBuf,
    version: String,
}

impl Package {
    /// Where the package was saved.
    pub fn path(&self) -> &Path {
        &self.path
    }
    /// Version of the package.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Install the package and schedule the restart of the application; **the
    /// application must then exit** (with karbon-ui: `WindowCommand::Exit`). The new
    /// version starts once this process has ended.
    ///
    /// - Windows: runs the installer in passive mode (`/P /UPDATE /R`, or `msiexec /i …
    ///   /passive` for an MSI); the installer restarts the application.
    /// - macOS: replaces the `.app` bundle with the one of the `.app.tar.gz` package (asks
    ///   for an administrator password when the bundle is not writable).
    /// - Linux: replaces the AppImage, or installs the `.deb` / `.rpm` with `pkexec`
    ///   (password prompt).
    ///
    /// # Errors
    ///
    /// [`Error::Unsupported`] when this installation cannot update itself (see
    /// [`Installation`]), [`Error::Install`] or [`Error::Io`] when installing fails; the
    /// installed application is then left as it was.
    pub fn install(self) -> Result<(), Error> {
        crate::install::install(&self.path, &Installation::current())
    }
}

/// Nom de l'exécutable, pour nommer le dossier temporaire.
fn app_name() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "app".into())
}

/// Dernier segment de l'adresse, sans requête ni caractère hors du nom d'un fichier.
pub(crate) fn file_name(url: &str) -> String {
    let path = url.split(['?', '#']).next().unwrap_or(url);
    let name: String = path
        .rsplit('/')
        .next()
        .unwrap_or("")
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect();
    if name.trim_matches('.').is_empty() {
        "package".into()
    } else {
        name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_names_come_from_the_url() {
        assert_eq!(
            file_name("https://x/downloads/windows/nsis/App_0.5.0_x64-setup.exe"),
            "App_0.5.0_x64-setup.exe"
        );
        assert_eq!(
            file_name("https://x/macos/0.5.0/App.app.tar.gz?token=1"),
            "App.app.tar.gz"
        );
        assert_eq!(file_name("https://x/a b%20c.deb"), "a_b_20c.deb");
        assert_eq!(file_name("https://x/"), "package");
        assert_eq!(file_name("https://x/.."), "package");
    }
}
