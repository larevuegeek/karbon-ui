pub(super) use super::super::fonts;
pub(super) use super::canvas::Canvas;
pub(super) use super::convert::*;
pub(super) use super::geometry::*;
pub(super) use super::target::{Kind, Output, Target};
use super::*;
pub(super) use crate::ui::font::{Font, FontAsset};
pub(super) use crate::ui::rendering::transform::Affine;
pub(super) use crate::{
    ui::{scene::*, theme::Color},
    window::RenderMode,
};
pub(super) use std::{collections::HashMap, sync::Arc};
pub(super) use windows::Win32::{
    Foundation::HMODULE,
    Graphics::{
        Direct2D::{Common::*, *},
        Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP},
        Direct3D11::*,
        DirectWrite::*,
        Dxgi::{
            Common::*, DXGI_ERROR_DEVICE_HUNG, DXGI_ERROR_DEVICE_REMOVED, DXGI_ERROR_DEVICE_RESET,
            DXGI_ERROR_DRIVER_INTERNAL_ERROR, IDXGIAdapter, IDXGIDevice, IDXGISurface,
            IDXGISwapChain1,
        },
    },
};
pub(super) use windows::core::{HSTRING, Interface};
pub(super) use windows_numerics::{Matrix3x2, Vector2};

/// Convertit un échec COM en [`crate::Error::Platform`] (opération et HRESULT).
/// Une perte du périphérique devient [`crate::Error::DeviceLost`] pour sa récupération.
pub(crate) trait Checked<T> {
    fn check(self, operation: &'static str) -> crate::Result<T>;
}
impl<T> Checked<T> for windows::core::Result<T> {
    fn check(self, operation: &'static str) -> crate::Result<T> {
        self.map_err(|error| com_error(operation, error.code()))
    }
}
/// Erreur publique d'un HRESULT en échec ; les codes de perte du périphérique D3D
/// (retiré ou réinitialisé : toutes ses ressources sont à recréer) deviennent `DeviceLost`.
pub(super) fn com_error(operation: &'static str, code: windows::core::HRESULT) -> crate::Error {
    if [
        DXGI_ERROR_DEVICE_REMOVED,
        DXGI_ERROR_DEVICE_RESET,
        DXGI_ERROR_DEVICE_HUNG,
        DXGI_ERROR_DRIVER_INTERNAL_ERROR,
        windows::Win32::Foundation::D2DERR_RECREATE_TARGET,
    ]
    .contains(&code)
    {
        crate::Error::DeviceLost {
            operation,
            code: code.0,
        }
    } else {
        crate::Error::Platform {
            operation,
            code: code.0,
        }
    }
}
pub(crate) fn is_device_lost(error: &crate::Error) -> bool {
    matches!(error, crate::Error::DeviceLost { .. })
}
/// Résultat de la préparation d'une surface de présentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SurfaceChange {
    Unchanged,
    Resized,
    Created,
}
/// Interface de sortie obligatoire : une sortie nulle devient une erreur.
pub(super) fn required<T>(value: Option<T>) -> crate::Result<T> {
    // E_POINTER : l'appel a réussi sans fournir l'interface attendue.
    value.ok_or_else(|| com_error("COM output", windows::Win32::Foundation::E_POINTER))
}

pub(crate) struct Renderer {
    pub(super) filters: filters::Cache,
    /// D3D11 + Direct2D, éventuellement encore en création (voir [`Renderer::start`]).
    pub(super) gpu: device::Slot,
    pub(super) target: Option<Target>,
    pub(super) formats: HashMap<(u32, Font), IDWriteTextFormat>,
    pub(super) fonts: Option<fonts::FontCollection>,
    pub(super) factory: ID2D1Factory1,
    pub(super) text_factory: IDWriteFactory,
    pub(super) software: bool,
    pub(super) measurements: HashMap<(String, u32, Font, u32), crate::ui::element::SizeF>,
    pub(super) scopes: Vec<effects::Scope>,
    pub(super) typography: typography::Cache,
    /// Test : la prochaine présentation échoue comme après un retrait du périphérique.
    #[cfg(test)]
    pub(super) lose_device: bool,
}

impl Renderer {
    /// Renderer prêt à dessiner (attend son périphérique).
    pub(crate) fn new(mode: RenderMode) -> crate::Result<Self> {
        let mut renderer = Self::start(mode)?;
        renderer.ready()?;
        Ok(renderer)
    }
    /// Renderer dont le périphérique D3D11 se crée sur un thread de travail : la mesure
    /// du texte (DirectWrite) et le chargement des polices sont disponibles tout de
    /// suite ; le premier dessin, ou [`ready`](Self::ready), attend le périphérique.
    pub(crate) fn start(mode: RenderMode) -> crate::Result<Self> {
        let gpu = device::Slot::start(mode)?;
        // SAFETY: fabrique mono-thread sans options ; l'IID provient du type demandé.
        let factory: ID2D1Factory1 =
            unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None) }
                .check("D2D1CreateFactory")?;
        // SAFETY: fabrique DirectWrite partagée ; l'IID provient du type demandé.
        let text_factory: IDWriteFactory =
            unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED) }
                .check("DWriteCreateFactory")?;
        Ok(Self {
            filters: Default::default(),
            gpu,
            target: None,
            formats: HashMap::new(),
            fonts: None,
            factory,
            text_factory,
            software: mode == RenderMode::Software,
            measurements: HashMap::new(),
            scopes: Vec::new(),
            typography: Default::default(),
            #[cfg(test)]
            lose_device: false,
        })
    }
    pub(crate) fn label(&self) -> &'static str {
        if self.software {
            "CPU · WARP"
        } else {
            "GPU · Direct3D 11"
        }
    }
    pub(crate) fn is_software(&self) -> bool {
        self.software
    }
    /// Attend la fin de la création du périphérique (sans effet ensuite). En mode
    /// `Auto`, le libellé passe à WARP si le GPU était indisponible.
    pub(crate) fn ready(&mut self) -> crate::Result<()> {
        if matches!(self.gpu, device::Slot::Pending(_)) {
            self.software = self.gpu.ready(&self.factory)?.software;
        }
        Ok(())
    }
}
/// Largeur, hauteur et alpha prémultiplié d'une chaîne d'échange.
#[cfg(test)]
pub(crate) fn describe_swap_chain(chain: &IDXGISwapChain1) -> (u32, u32, bool) {
    swapchain::describe(chain)
}
