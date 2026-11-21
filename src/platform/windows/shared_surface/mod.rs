//! Surface partagée sur la carte graphique (`ui::shared_surface`), côté Windows.
//!
//! | Fichier | Rôle |
//! | --- | --- |
//! | `device.rs` | périphérique Direct3D 11 sur la carte demandée (LUID), à part de celui du renderer |
//! | `texture.rs` | texture partagée (poignée NT, verrou à clé), sa relecture |
//! | `surface.rs` | `Native` : texture et chaîne d'échange de composition, présentation |
//!
//! La couche DirectComposition qui montre la surface sous l'interface est dans
//! `present/surface_visual.rs`, le trou dans l'interface dans `graphics/surface_hole.rs`.
mod device;
mod surface;
mod texture;

pub(crate) use surface::Native;
