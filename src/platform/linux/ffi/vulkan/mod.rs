//! Vulkan, chargé au lancement (`dlopen` de `libvulkan.so.1`, le chargeur de Khronos) :
//! aucun paquet de développement n'est nécessaire pour compiler, et l'absence de Vulkan
//! n'empêche pas l'application de démarrer (le renderer Cairo prend le relais).
//!
//! Les fonctions sont résolues par `vkGetInstanceProcAddr` et `vkGetDeviceProcAddr` en
//! tables (point d'entrée, instance, surface Wayland, appareil, chaîne d'échange) ; les
//! types sont transcrits des en-têtes (`device.rs`, `present.rs`, `resources.rs`,
//! `pipeline.rs`, `commands.rs`, `consts.rs`).
#![allow(non_snake_case)]
// Tables complètes pour toutes les étapes du renderer (chaîne d'échange, surfaces,
// envois de textures) : certaines fonctions ne servent pas encore.
#![allow(dead_code)]
#[macro_use]
mod library;
mod commands;
mod consts;
mod device;
mod pipeline;
mod present;
mod resources;

pub(in crate::platform::linux) use commands::*;
pub(in crate::platform::linux) use consts::*;
pub(in crate::platform::linux) use device::*;
pub(in crate::platform::linux) use library::*;
pub(in crate::platform::linux) use pipeline::*;
pub(in crate::platform::linux) use present::*;
pub(in crate::platform::linux) use resources::*;
pub(in crate::platform::linux) use std::ffi::{c_char, c_void};
