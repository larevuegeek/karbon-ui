//! Passwords and tokens of an application, encrypted by the operating system:
//! [`Secrets`].
// Organisation : contrôle des arguments ici, stockage natif dans `platform` :
// Gestionnaire d'identification (Windows), Trousseau (macOS), Secret Service par
// libsecret (Linux) ; ailleurs, `Unsupported`.
#[cfg(test)]
mod tests;

/// Longest service name or key, in bytes.
pub const MAX_NAME_BYTES: usize = 256;
/// Longest secret, in bytes (the Windows Credential Manager limit, with margin).
pub const MAX_SECRET_BYTES: usize = 2048;

/// Passwords and tokens of an application, kept encrypted by the operating system for
/// the current user: the Credential Manager on Windows, the Keychain on macOS, the
/// Secret Service (GNOME Keyring, KWallet…) on Linux. They are unreadable by other
/// users and from a copied disk, and survive reinstalls of the application.
///
/// Keep small secrets here (a server password, an API token); preferences go to
/// [`crate::storage`].
///
/// ```no_run
/// use karbon_ui::secrets::Secrets;
///
/// let secrets = Secrets::new("dev.rustmusic.RustMusic");
/// secrets.set("jellyfin/home", "hunter2")?;
/// assert_eq!(secrets.get("jellyfin/home")?.as_deref(), Some("hunter2"));
/// secrets.remove("jellyfin/home")?;
/// # Ok::<(), karbon_ui::Error>(())
/// ```
#[derive(Clone, Debug)]
pub struct Secrets {
    service: String,
}

impl Secrets {
    /// Secrets of `service`, a stable identifier of the application (its reverse-DNS
    /// name, like `dev.rustmusic.RustMusic`): two applications with different services
    /// never see each other's secrets.
    pub fn new(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }
    /// Stores `secret` under `key`, replacing any previous value.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidInput`](crate::Error::InvalidInput) for an empty or too long
    /// name, a secret over [`MAX_SECRET_BYTES`] or containing a NUL character;
    /// [`Error::Unsupported`](crate::Error::Unsupported) without a system store (Linux
    /// without libsecret, other platforms); [`Error::Platform`](crate::Error::Platform)
    /// when the store refuses (locked keyring, access denied).
    pub fn set(&self, key: &str, secret: &str) -> crate::Result<()> {
        self.check(key)?;
        if secret.len() > MAX_SECRET_BYTES || secret.contains('\0') {
            return Err(crate::Error::invalid_input(
                "secrets are at most 2048 bytes, without NUL characters",
            ));
        }
        native::set(&self.service, key, secret)
    }
    /// The secret stored under `key`, or `None`.
    ///
    /// # Errors
    ///
    /// Same as [`set`](Self::set).
    pub fn get(&self, key: &str) -> crate::Result<Option<String>> {
        self.check(key)?;
        native::get(&self.service, key)
    }
    /// Removes the secret stored under `key`; removing an absent secret does nothing.
    ///
    /// # Errors
    ///
    /// Same as [`set`](Self::set).
    pub fn remove(&self, key: &str) -> crate::Result<()> {
        self.check(key)?;
        native::remove(&self.service, key)
    }
    fn check(&self, key: &str) -> crate::Result<()> {
        let valid =
            |name: &str| !name.is_empty() && name.len() <= MAX_NAME_BYTES && !name.contains('\0');
        if valid(&self.service) && valid(key) {
            Ok(())
        } else {
            Err(crate::Error::invalid_input(
                "secret services and keys are 1 to 256 bytes, without NUL characters",
            ))
        }
    }
}

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
use crate::platform::secrets as native;

/// Plateformes sans magasin de secrets.
#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
mod native {
    const UNSUPPORTED: crate::Error = crate::Error::Unsupported {
        reason: "no system secret store on this platform",
    };
    pub(super) fn set(_: &str, _: &str, _: &str) -> crate::Result<()> {
        Err(UNSUPPORTED)
    }
    pub(super) fn get(_: &str, _: &str) -> crate::Result<Option<String>> {
        Err(UNSUPPORTED)
    }
    pub(super) fn remove(_: &str, _: &str) -> crate::Result<()> {
        Err(UNSUPPORTED)
    }
}
