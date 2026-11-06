//! Chaîne d'échange DXGI pour DirectComposition : Direct2D dessine directement dans
//! le back buffer, que DWM compose sans relecture CPU.
use super::*;
use windows::Win32::Graphics::Dxgi::{
    DXGI_PRESENT, DXGI_SCALING_STRETCH, DXGI_SWAP_CHAIN_DESC1, DXGI_SWAP_CHAIN_FLAG,
    DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL, DXGI_USAGE_RENDER_TARGET_OUTPUT, IDXGIFactory2,
    IDXGISwapChain1,
};

/// Modèle flip : un tampon affiché, un tampon de dessin.
const BUFFERS: u32 = 2;

pub(super) fn create(
    device: &ID3D11Device,
    width: u32,
    height: u32,
    premultiplied: bool,
) -> crate::Result<IDXGISwapChain1> {
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
        Flags: 0,
    };
    // SAFETY: description locale valide pendant l'appel ; aucune sortie restreinte.
    unsafe { factory.CreateSwapChainForComposition(device, &description, None) }
        .check("CreateSwapChainForComposition")
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
            DXGI_SWAP_CHAIN_FLAG(0),
        )
    }
    .check("IDXGISwapChain::ResizeBuffers")?;
    bind(context, chain, scale)
}

/// Présentation synchronisée sur le balayage vertical (GPU et WARP : l'intervalle 0
/// n'a pas réduit les cycles CPU mesurés sous WARP).
pub(super) fn present(chain: &IDXGISwapChain1) -> crate::Result<()> {
    // SAFETY: chaîne valide ; le back buffer a été refermé par EndDraw.
    unsafe { chain.Present(1, DXGI_PRESENT(0)) }
        .ok()
        .check("IDXGISwapChain::Present")
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
