//! Vérification : interroge les adresses du serveur dans l'ordre, lit la dernière version et
//! la compare à celle de l'application.
use crate::{Error, Installation, Update, manifest::Release, version::Version};
use std::time::Duration;

/// Checks a release endpoint for a newer version of the application.
///
/// The endpoint URL may contain these variables, replaced before the request (the same as
/// the Tauri updater's, so its servers work unchanged):
///
/// | Variable | Value |
/// | --- | --- |
/// | `{{current_version}}` | the version given to [`Updater::new`] |
/// | `{{target}}` | `windows`, `darwin` or `linux`, or the value of [`Updater::target`] |
/// | `{{arch}}` | `x86_64`, `aarch64`, `i686`, `armv7` or `riscv64` |
/// | `{{bundle_type}}` | `nsis`, `app`, `appimage`, `deb` or `rpm` (see [`Installation`]) |
///
/// The server answers `204 No Content` when there is nothing newer, or a release manifest
/// (see `README.md`). In a manifest listing several platforms, the package is looked up
/// under `<os>-<arch>-<bundle_type>`, then `<os>-<arch>` (or under [`Updater::target`]).
#[derive(Debug, Clone)]
pub struct Updater {
    public_key: String,
    current_version: String,
    endpoints: Vec<String>,
    target: Option<String>,
    timeout: Duration,
    user_agent: String,
}

impl Updater {
    /// An updater for the application at `current_version` (usually
    /// `env!("CARGO_PKG_VERSION")`), whose packages are signed with the secret key of
    /// `public_key`: the minisign public key, as the base64 text `tauri signer generate`
    /// prints, the content of the `.pub` file, or its `RW…` line.
    pub fn new(public_key: impl Into<String>, current_version: impl Into<String>) -> Self {
        Self {
            public_key: public_key.into(),
            current_version: current_version.into(),
            endpoints: Vec::new(),
            target: None,
            timeout: Duration::from_secs(20),
            user_agent: concat!("karbon-updater/", env!("CARGO_PKG_VERSION")).to_owned(),
        }
    }
    /// Add an endpoint; several are tried in order until one answers.
    pub fn endpoint(mut self, url: impl Into<String>) -> Self {
        self.endpoints.push(url.into());
        self
    }
    /// Replace `{{target}}` with this value and look the package up under this key only.
    pub fn target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }
    /// Time limit of a check request (20 seconds by default).
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
    /// `User-Agent` of the requests (`karbon-updater/<version>` by default).
    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = user_agent.into();
        self
    }

    /// Ask the endpoints whether a newer version exists: `Ok(None)` when the application
    /// is up to date. Blocks for the duration of the request: call it from a background
    /// task.
    ///
    /// # Errors
    ///
    /// [`Error::Network`] when no endpoint could be reached, [`Error::Manifest`] when the
    /// answer is not a release manifest, [`Error::NoPackage`] when a newer version has no
    /// package for this platform.
    pub fn check(&self) -> Result<Option<Update>, Error> {
        let current = Version::parse(&self.current_version).ok_or_else(|| {
            Error::Manifest(format!("`{}` is not a version", self.current_version))
        })?;
        let installation = Installation::current();
        let mut last_error = Error::Network("no endpoint configured".into());
        for endpoint in &self.endpoints {
            let url = self.url(endpoint, &installation);
            match self.fetch(&url) {
                Ok(None) => return Ok(None),
                Ok(Some(release)) => {
                    if release.parsed <= current {
                        return Ok(None);
                    }
                    let asset = release.asset(&self.keys(&installation))?;
                    return Ok(Some(Update {
                        version: release.version.clone(),
                        notes: release.notes.clone(),
                        date: release.date.clone(),
                        url: asset.url.clone(),
                        signature: asset.signature.clone(),
                        public_key: self.public_key.clone(),
                        user_agent: self.user_agent.clone(),
                    }));
                }
                Err(error) => last_error = error,
            }
        }
        Err(last_error)
    }

    /// Réponse d'une adresse : `None` pour 204.
    fn fetch(&self, url: &str) -> Result<Option<Release>, Error> {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(self.timeout))
            .user_agent(self.user_agent.as_str())
            .build()
            .new_agent();
        let mut response = agent
            .get(url)
            .header("Accept", "application/json")
            .call()
            .map_err(|e| Error::Network(e.to_string()))?;
        if response.status() == 204 {
            return Ok(None);
        }
        let text = response
            .body_mut()
            .read_to_string()
            .map_err(|e| Error::Network(e.to_string()))?;
        Release::parse(&text).map(Some)
    }

    fn url(&self, endpoint: &str, installation: &Installation) -> String {
        endpoint
            .replace("{{current_version}}", &self.current_version)
            .replace("{{target}}", self.target.as_deref().unwrap_or(os()))
            .replace("{{arch}}", arch())
            .replace("{{bundle_type}}", installation.kind().unwrap_or("unknown"))
    }

    /// Clés du paquet dans un manifeste statique, de la plus précise à la plus générale.
    fn keys(&self, installation: &Installation) -> Vec<String> {
        if let Some(target) = &self.target {
            return vec![target.clone()];
        }
        let base = format!("{}-{}", os(), arch());
        let mut keys = Vec::new();
        if let Some(kind) = installation.kind() {
            keys.push(format!("{base}-{kind}"));
        }
        keys.push(base);
        keys
    }
}

/// Nom du système dans les adresses et les clés (ceux du module de Tauri).
pub(crate) fn os() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "darwin"
    } else {
        "linux"
    }
}

pub(crate) fn arch() -> &'static str {
    if cfg!(target_arch = "x86") {
        "i686"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else if cfg!(target_arch = "arm") {
        "armv7"
    } else if cfg!(target_arch = "riscv64") {
        "riscv64"
    } else {
        "x86_64"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_variables_are_replaced() {
        let updater = Updater::new("key", "0.4.3");
        let url = updater.url(
            "https://x/{{target}}-{{arch}}/{{current_version}}?kind={{bundle_type}}",
            &Installation::Deb,
        );
        assert_eq!(url, format!("https://x/{}-{}/0.4.3?kind=deb", os(), arch()));
        let targeted = Updater::new("key", "1.0.0").target("windows-x86_64");
        assert_eq!(
            targeted.url("https://x/{{target}}", &Installation::WindowsInstaller),
            "https://x/windows-x86_64"
        );
        assert_eq!(
            targeted.keys(&Installation::WindowsInstaller),
            ["windows-x86_64"]
        );
        let keys = updater.keys(&Installation::MacBundle("/A.app".into()));
        assert_eq!(
            keys,
            [
                format!("{}-{}-app", os(), arch()),
                format!("{}-{}", os(), arch())
            ]
        );
    }

    #[test]
    fn a_check_without_endpoint_or_with_a_bad_version_fails() {
        assert!(matches!(
            Updater::new("k", "1.0.0").check(),
            Err(Error::Network(_))
        ));
        assert!(matches!(
            Updater::new("k", "dev").endpoint("https://x").check(),
            Err(Error::Manifest(_))
        ));
    }
}
