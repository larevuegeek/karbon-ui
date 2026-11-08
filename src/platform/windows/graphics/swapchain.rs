//! Chaîne d'échange DXGI pour DirectComposition : Direct2D dessine directement dans
//! le back buffer, que DWM compose sans relecture CPU.
use super::*;
use windows::Win32::Graphics::Dxgi::{
    DXGI_PRESENT, DXGI_SCALING_STRETCH, DXGI_SWAP_CHAIN_DESC1,
    DXGI_SWAP_CHAIN_FLAG_FRAME_LATENCY_WAITABLE_OBJECT, DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
    DXGI_USAGE_RENDER_TARGET_OUTPUT, IDXGIFactory2, IDXGISwapChain1, IDXGISwapChain2,
};
use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE},
    System::Threading::WaitForSingleObjectEx,
};

/// Modèle flip : un tampon affiché, un tampon de dessin.
const BUFFERS: u32 = 2;
/// Attente maximale d'une place dans la file de présentation (deux images à 60 Hz) :
/// au-delà (fenêtre que DWM ne compose plus), l'image part quand même plutôt que de
/// figer la fenêtre.
const FRAME_WAIT_MS: u32 = 34;

/// Place dans la file de présentation (objet attendable de DXGI, latence d'une image).
/// Comme le `BeginFrame` de Chromium, une image d'animation attend sa place avant de lire
/// l'horloge : elle est échantillonnée juste après le balayage qui a pris la précédente
/// et s'affiche au suivant, à intervalle constant. Sans cette attente, `Present` rendait
/// la main tout de suite au départ d'une animation : deux images lues à quelques
/// millisecondes d'écart s'affichaient à une image d'écart (à-coup à chaque glissement),
/// et la file ajoutait jusqu'à trois images de latence.
pub(super) struct FrameSlot {
    handle: HANDLE,
    /// Place prise et pas encore rendue par `Present`.
    acquired: bool,
    /// Temps passé à attendre la place de l'image en cours (mesures de `--profile`).
    pub(super) waited: std::time::Duration,
}
impl FrameSlot {
    fn new(chain: &IDXGISwapChain1) -> crate::Result<Self> {
        let chain: IDXGISwapChain2 = chain.cast().check("QueryInterface IDXGISwapChain2")?;
        // SAFETY: chaîne créée avec DXGI_SWAP_CHAIN_FLAG_FRAME_LATENCY_WAITABLE_OBJECT.
        unsafe { chain.SetMaximumFrameLatency(1) }.check("SetMaximumFrameLatency")?;
        // SAFETY: même chaîne ; le handle renvoyé appartient à l'appelant (CloseHandle).
        let handle = unsafe { chain.GetFrameLatencyWaitableObject() };
        Ok(Self {
            handle: handle.0,
            acquired: false,
            waited: std::time::Duration::ZERO,
        })
    }
    /// Attend une place dans la file (sans effet si elle est déjà prise).
    pub(super) fn acquire(&mut self) {
        if !self.acquired && !self.handle.is_null() {
            let started = std::time::Instant::now();
            // SAFETY: handle valide tant que `self` vit ; attente bornée, non alertable.
            unsafe { WaitForSingleObjectEx(self.handle, FRAME_WAIT_MS, 0) };
            self.waited += started.elapsed();
        }
        self.acquired = true;
    }
}
impl Drop for FrameSlot {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            // SAFETY: handle obtenu par GetFrameLatencyWaitableObject, fermé une fois.
            unsafe { CloseHandle(self.handle) };
        }
    }
}

pub(super) fn create(
    device: &ID3D11Device,
    width: u32,
    height: u32,
    premultiplied: bool,
) -> crate::Result<(IDXGISwapChain1, FrameSlot)> {
    let dxgi: IDXGIDevice = device.cast().check("QueryInterface IDXGIDevice")?;
    // SAFETY: périphérique DXGI valide ; windows-rs gère la référence renvoyée.
    let adapter = unsafe { dxgi.GetAdapter() }.check("IDXGIDevice::GetAdapter")?;
    // SAFETY: adaptateur valide ; l'IID provient du type demandé.
    let factory: IDXGIFactory2 = unsafe { adapter.GetParent() }.check("IDXGIAdapter::GetParent")?;
    let description = DXGI_SWAP_CHAIN_DESC1 {
        Width: width,
        Height: height,
        Format: DXGI_FORMAT_B8G8R8A8_UNORM,
        Stereo: false.into(),
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
        BufferCount: BUFFERS,
        // Seul mode accepté par CreateSwapChainForComposition ; le visuel n'a aucune
        // transformation, la chaîne garde donc la taille exacte de la zone client.
        Scaling: DXGI_SCALING_STRETCH,
        SwapEffect: DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
        AlphaMode: if premultiplied {
            DXGI_ALPHA_MODE_PREMULTIPLIED
        } else {
            DXGI_ALPHA_MODE_IGNORE
        },
        Flags: DXGI_SWAP_CHAIN_FLAG_FRAME_LATENCY_WAITABLE_OBJECT.0 as u32,
    };
    // SAFETY: description locale valide pendant l'appel ; aucune sortie restreinte.
    let chain = unsafe { factory.CreateSwapChainForComposition(device, &description, None) }
        .check("CreateSwapChainForComposition")?;
    let slot = FrameSlot::new(&chain)?;
    Ok((chain, slot))
}

/// Attache le back buffer courant comme cible Direct2D. Avec le modèle flip de D3D11,
/// le tampon 0 désigne toujours le prochain tampon de dessin : une seule cible suffit.
pub(super) fn bind(
    context: &ID2D1DeviceContext,
    chain: &IDXGISwapChain1,
    scale: f32,
) -> crate::Result<()> {
    // SAFETY: chaîne valide ; l'IID provient du type demandé.
    let surface: IDXGISurface = unsafe { chain.GetBuffer(0) }.check("IDXGISwapChain::GetBuffer")?;
    device::bind(context, &surface, scale)
}

/// Libère la cible, redimensionne les tampons puis recrée la cible sur le nouveau back buffer.
pub(super) fn resize(
    d3d: &ID3D11DeviceContext,
    context: &ID2D1DeviceContext,
    chain: &IDXGISwapChain1,
    width: u32,
    height: u32,
    scale: f32,
) -> crate::Result<()> {
    // SAFETY: contexte Direct2D valide, hors BeginDraw/EndDraw ; retirer la cible libère
    // la dernière référence du bitmap sur le back buffer.
    unsafe { context.SetTarget(None::<&ID2D1Image>) };
    // SAFETY: contexte immédiat D3D11 valide ; vide les commandes qui citent encore les tampons.
    unsafe { d3d.Flush() };
    // SAFETY: plus aucune référence aux tampons n'est conservée par ce renderer.
    unsafe {
        chain.ResizeBuffers(
            BUFFERS,
            width,
            height,
            DXGI_FORMAT_B8G8R8A8_UNORM,
            // Les drapeaux de création doivent être redonnés à chaque redimensionnement.
            DXGI_SWAP_CHAIN_FLAG_FRAME_LATENCY_WAITABLE_OBJECT,
        )
    }
    .check("IDXGISwapChain::ResizeBuffers")?;
    bind(context, chain, scale)
}

/// Présentation synchronisée sur le balayage vertical (GPU et WARP : l'intervalle 0
/// n'a pas réduit les cycles CPU mesurés sous WARP).
pub(super) fn present(chain: &IDXGISwapChain1, slot: &mut FrameSlot) -> crate::Result<()> {
    // SAFETY: chaîne valide ; le back buffer a été refermé par EndDraw.
    unsafe { chain.Present(1, DXGI_PRESENT(0)) }
        .ok()
        .check("IDXGISwapChain::Present")?;
    slot.acquired = false;
    slot.waited = std::time::Duration::ZERO;
    Ok(())
}

/// Taille et mode alpha réels de la chaîne, pour les tests.
#[cfg(test)]
pub(super) fn describe(chain: &IDXGISwapChain1) -> (u32, u32, bool) {
    // SAFETY: chaîne valide ; description copiée par valeur.
    let description = unsafe { chain.GetDesc1() }.expect("GetDesc1");
    (
        description.Width,
        description.Height,
        description.AlphaMode == DXGI_ALPHA_MODE_PREMULTIPLIED,
    )
}
