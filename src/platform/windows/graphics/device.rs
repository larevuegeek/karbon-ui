//! Direct3D 11 et Direct2D 1.1 : périphériques, contextes et cible bitmap sur surface
//! DXGI.
//!
//! `D3D11CreateDevice` coûte l'essentiel de la création d'un renderer (chargement du
//! pilote, ~140 ms sur la machine de développement) : [`start`] le lance sur un thread
//! de travail pendant que le thread UI charge l'application et calcule la première
//! mise en page, qui n'ont besoin que de DirectWrite.
use super::*;

/// Périphérique D3D11 et son contexte immédiat, avant leur remise au thread UI.
pub(super) struct D3d {
    pub device: ID3D11Device,
    pub context: ID3D11DeviceContext,
    pub software: bool,
}
// SAFETY : un périphérique D3D11 créé sans `D3D11_CREATE_DEVICE_SINGLETHREADED` est
// utilisable depuis n'importe quel thread ; le contexte immédiat n'est utilisé qu'après
// la remise, par le seul thread UI.
unsafe impl Send for D3d {}

/// Périphériques prêts à dessiner.
pub(super) struct Gpu {
    pub device: ID3D11Device,
    pub context: ID3D11DeviceContext,
    pub d2d: ID2D1Device,
    /// Pilote WARP (logiciel) plutôt que matériel.
    pub software: bool,
}
/// Création en cours sur un thread de travail, ou terminée.
pub(super) enum Slot {
    /// `None` après un échec déjà signalé.
    Pending(Option<std::thread::JoinHandle<crate::Result<D3d>>>),
    Ready(Gpu),
}
impl Slot {
    /// Lance la création du périphérique de `mode` sur un thread de travail.
    pub(super) fn start(mode: RenderMode) -> crate::Result<Self> {
        let handle = std::thread::Builder::new()
            .name("karbon-d3d11".into())
            .spawn(move || d3d(mode))
            .map_err(crate::Error::Io)?;
        Ok(Self::Pending(Some(handle)))
    }
    /// Attend le périphérique et crée son pendant Direct2D (sur le thread appelant).
    pub(super) fn ready(&mut self, factory: &ID2D1Factory1) -> crate::Result<&Gpu> {
        if let Self::Pending(handle) = self {
            let d3d = match handle.take().map(std::thread::JoinHandle::join) {
                Some(Ok(result)) => result?,
                Some(Err(_)) | None => {
                    return Err(com_error(
                        "D3D11CreateDevice",
                        windows::Win32::Foundation::E_FAIL,
                    ));
                }
            };
            let d2d = create(factory, &d3d.device)?;
            *self = Self::Ready(Gpu {
                device: d3d.device,
                context: d3d.context,
                d2d,
                software: d3d.software,
            });
        }
        Ok(self.gpu())
    }
    /// Périphériques déjà prêts (après [`ready`](Self::ready)).
    pub(super) fn gpu(&self) -> &Gpu {
        match self {
            Self::Ready(gpu) => gpu,
            Self::Pending(_) => unreachable!("the renderer waits for its device before drawing"),
        }
    }
}
fn d3d(mode: RenderMode) -> crate::Result<D3d> {
    match mode {
        RenderMode::Software => create_d3d(true),
        RenderMode::Gpu => create_d3d(false),
        RenderMode::Auto => create_d3d(false).or_else(|error| {
            eprintln!(
                "karbon-ui: GPU unavailable ({error}), falling back to WARP software rendering"
            );
            create_d3d(true)
        }),
    }
}
fn create_d3d(software: bool) -> crate::Result<D3d> {
    let (mut device, mut context) = (None, None);
    // SAFETY: sorties optionnelles valides ; ni adaptateur ni module logiciel explicites.
    unsafe {
        D3D11CreateDevice(
            None::<&IDXGIAdapter>,
            if software {
                D3D_DRIVER_TYPE_WARP
            } else {
                D3D_DRIVER_TYPE_HARDWARE
            },
            HMODULE::default(),
            D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            None,
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            Some(&mut context),
        )
    }
    .check("D3D11CreateDevice")?;
    Ok(D3d {
        device: required(device)?,
        context: required(context)?,
        software,
    })
}
pub(super) fn create(factory: &ID2D1Factory1, d3d: &ID3D11Device) -> crate::Result<ID2D1Device> {
    let dxgi: IDXGIDevice = d3d.cast().check("QueryInterface")?;
    // SAFETY: fabrique et périphérique DXGI valides, sur le thread de création.
    let device = unsafe { factory.CreateDevice(&dxgi) }.check("ID2D1Factory1::CreateDevice")?;
    // SAFETY: périphérique Direct2D valide ; simple réglage du budget de cache.
    unsafe { device.SetMaximumTextureMemory(64 * 1024 * 1024) };
    Ok(device)
}
pub(super) fn context(device: &ID2D1Device) -> crate::Result<ID2D1DeviceContext> {
    // SAFETY: périphérique Direct2D valide, sur le thread de création.
    unsafe { device.CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE) }
        .check("CreateDeviceContext")
}
pub(super) fn target(
    device: &ID2D1Device,
    surface: &IDXGISurface,
    scale: f32,
) -> crate::Result<ID2D1DeviceContext> {
    let context = context(device)?;
    bind(&context, surface, scale)?;
    Ok(context)
}
/// Attache une surface DXGI (texture ou back buffer) comme cible du contexte.
pub(super) fn bind(
    context: &ID2D1DeviceContext,
    surface: &IDXGISurface,
    scale: f32,
) -> crate::Result<()> {
    let properties = D2D1_BITMAP_PROPERTIES1 {
        pixelFormat: PIXEL_FORMAT,
        dpiX: 96. * scale,
        dpiY: 96. * scale,
        bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
        colorContext: std::mem::ManuallyDrop::new(None),
    };
    // SAFETY: surface DXGI issue de la texture cible ; propriétés lues pendant l'appel.
    let bitmap = unsafe { context.CreateBitmapFromDxgiSurface(surface, Some(&properties)) }
        .check("CreateBitmapFromDxgiSurface")?;
    // SAFETY: le contexte conserve sa propre référence sur la cible.
    unsafe { context.SetTarget(&bitmap) };
    Ok(())
}
