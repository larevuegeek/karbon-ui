//! Hors de Windows : aucune surface partagée. Le type existe pour que tout compile ;
//! il ne peut pas être construit.
use super::surface::AdapterLuid;

/// La surface native, jamais construite ici.
pub(crate) enum Native {}

impl Native {
    pub(crate) fn new(_: Option<AdapterLuid>, _: u32, _: u32) -> crate::Result<Self> {
        Err(crate::Error::Unsupported {
            reason: "shared surfaces need Direct3D 11 and DirectComposition (Windows)",
        })
    }
    pub(crate) fn adapter(&self) -> AdapterLuid {
        match *self {}
    }
    pub(crate) fn adapter_name(&self) -> String {
        match *self {}
    }
    pub(crate) fn size(&self) -> (u32, u32) {
        match *self {}
    }
    pub(crate) fn handle(&self) -> isize {
        match *self {}
    }
    pub(crate) fn resize(&self, _: u32, _: u32) -> crate::Result<()> {
        match *self {}
    }
    pub(crate) fn present(&self) -> crate::Result<()> {
        match *self {}
    }
    pub(crate) fn read_pixels(&self) -> crate::Result<Vec<u8>> {
        match *self {}
    }
}
