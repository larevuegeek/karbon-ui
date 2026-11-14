//! Systèmes sans installation prise en charge.
use crate::{Error, Installation};
use std::path::Path;

pub(crate) fn install(_package: &Path, _installation: &Installation) -> Result<(), Error> {
    Err(Error::Unsupported("unsupported operating system".into()))
}
