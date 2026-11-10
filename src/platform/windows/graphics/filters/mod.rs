//! Cached command lists and native effects. No intermediate GPU-to-CPU readback.
mod cache;
mod effect;
#[cfg(test)]
pub(super) mod tests;
pub(crate) use cache::*;
