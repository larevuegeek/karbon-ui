//! Erreurs de la vérification, du téléchargement et de l'installation.
use std::fmt;

/// Why a check, a download or an installation failed.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// The endpoint could not be reached, or answered with an error status.
    Network(String),
    /// The endpoint's answer is not a release manifest.
    Manifest(String),
    /// The manifest has no package for this platform (the keys that were tried).
    NoPackage(Vec<String>),
    /// The package's signature is missing, malformed, or does not match the public key,
    /// or it was signed for another version than the one announced.
    Signature(String),
    /// This installation cannot update itself (Flatpak, Snap, development build, a
    /// program not installed by a package…); the text says why.
    Unsupported(String),
    /// Writing the package or replacing the application failed.
    Io(std::io::Error),
    /// The installer or the system tool that installs the package failed.
    Install(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network(reason) => write!(f, "update server unreachable: {reason}"),
            Self::Manifest(reason) => write!(f, "invalid release manifest: {reason}"),
            Self::NoPackage(keys) => {
                write!(
                    f,
                    "no package for this platform (looked for {})",
                    keys.join(", ")
                )
            }
            Self::Signature(reason) => write!(f, "invalid package signature: {reason}"),
            Self::Unsupported(reason) => write!(f, "self-update unavailable: {reason}"),
            Self::Io(error) => write!(f, "update failed: {error}"),
            Self::Install(reason) => write!(f, "installation failed: {reason}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}
