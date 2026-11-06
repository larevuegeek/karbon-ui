//! Backend Direct2D / DirectWrite sur texture D3D11 BGRA.
//!
//! GPU = D3D_DRIVER_TYPE_HARDWARE, logiciel = D3D_DRIVER_TYPE_WARP.
//! Les interfaces COM sont typées par la crate officielle `windows` (métadonnées du SDK).
//! Deux cibles : une texture relue par le CPU (rendu hors écran, chemin layered) ou le
//! back buffer d'une chaîne d'échange DirectComposition (présentation sans relecture).
mod backdrop;
mod canvas;
mod convert;
mod device;
mod effects;
mod filters;
mod formats;
mod frame;
mod geometry;
mod gradients;
mod images;
mod measure;
mod radials;
mod readback;
mod shapes;
mod svg;
mod swapchain;
mod target;
#[cfg(test)]
mod tests;
mod typography;
use super::fonts;
use crate::ui::font::{Font, FontAsset};
use crate::ui::rendering::transform::Affine;
use crate::{
    ui::{scene::*, theme::Color},
    window::RenderMode,
};
use canvas::Canvas;
use convert::*;
use geometry::*;
use std::{collections::HashMap, sync::Arc};
use target::{Kind, Output, Target};
use windows::Win32::{
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
use windows::core::{HSTRING, Interface, w};
use windows_numerics::{Matrix3x2, Vector2};

/// Convertit un échec COM en [`crate::Error::Platform`] (opération et HRESULT).
/// Une perte du périphérique devient [`crate::Error::DeviceLost`] pour sa récupération.
pub(super) trait Checked<T> {
    fn check(self, operation: &'static str) -> crate::Result<T>;
}
impl<T> Checked<T> for windows::core::Result<T> {
    fn check(self, operation: &'static str) -> crate::Result<T> {
        self.map_err(|error| com_error(operation, error.code()))
    }
}
/// Erreur publique d'un HRESULT en échec ; les codes de perte du périphérique D3D
/// (retiré ou réinitialisé : toutes ses ressources sont à recréer) deviennent `DeviceLost`.
fn com_error(operation: &'static str, code: windows::core::HRESULT) -> crate::Error {
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
pub(super) fn is_device_lost(error: &crate::Error) -> bool {
    matches!(error, crate::Error::DeviceLost { .. })
}
/// Résultat de la préparation d'une surface de présentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SurfaceChange {
    Unchanged,
    Resized,
    Created,
}
/// Interface de sortie obligatoire : une sortie nulle devient une erreur.
fn required<T>(value: Option<T>) -> crate::Result<T> {
    // E_POINTER : l'appel a réussi sans fournir l'interface attendue.
    value.ok_or_else(|| com_error("COM output", windows::Win32::Foundation::E_POINTER))
}

pub(super) struct Renderer {
    filters: filters::Cache,
    /// D3D11 + Direct2D, éventuellement encore en création (voir [`Renderer::start`]).
    gpu: device::Slot,
    target: Option<Target>,
    formats: HashMap<(u32, Font), IDWriteTextFormat>,
    fonts: Option<fonts::FontCollection>,
    factory: ID2D1Factory1,
    text_factory: IDWriteFactory,
    software: bool,
    measurements: HashMap<(String, u32, Font, u32), crate::ui::element::SizeF>,
    scopes: Vec<effects::Scope>,
    typography: typography::Cache,
    /// Test : la prochaine présentation échoue comme après un retrait du périphérique.
    #[cfg(test)]
    lose_device: bool,
}

impl Renderer {
    /// Renderer prêt à dessiner (attend son périphérique).
    pub(super) fn new(mode: RenderMode) -> crate::Result<Self> {
        let mut renderer = Self::start(mode)?;
        renderer.ready()?;
        Ok(renderer)
    }
    /// Renderer dont le périphérique D3D11 se crée sur un thread de travail : la mesure
    /// du texte (DirectWrite) et le chargement des polices sont disponibles tout de
    /// suite ; le premier dessin, ou [`ready`](Self::ready), attend le périphérique.
    pub(super) fn start(mode: RenderMode) -> crate::Result<Self> {
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
    pub(super) fn label(&self) -> &'static str {
        if self.software {
            "CPU · WARP"
        } else {
            "GPU · Direct3D 11"
        }
    }
    pub(super) fn is_software(&self) -> bool {
        self.software
    }
    /// Attend la fin de la création du périphérique (sans effet ensuite). En mode
    /// `Auto`, le libellé passe à WARP si le GPU était indisponible.
    pub(super) fn ready(&mut self) -> crate::Result<()> {
        if matches!(self.gpu, device::Slot::Pending(_)) {
            self.software = self.gpu.ready(&self.factory)?.software;
        }
        Ok(())
    }
}
/// Largeur, hauteur et alpha prémultiplié d'une chaîne d'échange.
#[cfg(test)]
pub(super) fn describe_swap_chain(chain: &IDXGISwapChain1) -> (u32, u32, bool) {
    swapchain::describe(chain)
}
