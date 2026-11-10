//! XCB (protocole X11), chargé au lancement (`dlopen` de `libxcb.so.1`) : aucun paquet de
//! développement n'est nécessaire pour compiler, et un système sans X11 démarre en
//! Wayland. Les fonctions sont résolues une fois pour tout le processus (`xcb()`), de même
//! que celles des curseurs (`xcb_cursor()`) et de XInput 2 (`xinput()`). Les types sont
//! transcrits de `xcb/xcb.h` et `xcb/xproto.h` : réponses et structures du serveur dans
//! `types.rs`, événements dans `events.rs`, constantes dans `constants.rs`, avec une
//! assertion de taille par structure, d'après les en-têtes.
#![allow(non_camel_case_types)]
#[macro_use]
mod library;
mod constants;
mod events;
mod types;
pub(in crate::platform::linux) use library::*;
pub(in crate::platform::linux) use {constants::*, events::*, types::*};
