//! `@keyframes` (étapes compilées en `Keyframe` du moteur) et propriétés `animation-*`
//! d'une classe (liste d'animations : nom, durée, courbe, répétitions, direction…).
mod instance;
mod keyframes;
#[cfg(test)]
mod tests;
pub(super) use instance::{declaration, instance, names, state_override};
pub(super) use keyframes::{frames, frames_at};
mod timing;
use timing::*;
