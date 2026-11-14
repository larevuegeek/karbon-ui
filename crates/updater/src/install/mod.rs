//! Installation d'un paquet vérifié, par plateforme, et relance de l'application.
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
mod other;
#[cfg(unix)]
mod relaunch;
#[cfg(windows)]
mod windows;

#[cfg(target_os = "linux")]
pub(crate) use linux::install;
#[cfg(target_os = "macos")]
pub(crate) use macos::install;
#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
pub(crate) use other::install;
#[cfg(windows)]
pub(crate) use windows::install;
