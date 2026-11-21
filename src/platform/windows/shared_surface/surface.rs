//! La surface partagée native : la texture où le producteur dessine, et la chaîne
//! d'échange de composition qui montre son image.
//!
//! Une seule texture suffit : DWM ne la lit jamais. `present` la recopie, sur la carte,
//! dans le tampon de la chaîne d'échange, sous le verrou à clé ; le producteur ne peut
//! reprendre la texture qu'ensuite. La copie d'une image de 1740 × 1176 coûte quelques
//! dizaines de microsecondes au processeur ; un anneau de textures présentées telles
//! quelles l'éviterait, au prix d'une interface de présentation réservée à Windows 11.
use super::{
    super::graphics::Checked,
    device::{self, Device},
    texture::Texture,
};
use crate::ui::shared_surface::AdapterLuid;
use std::sync::{
    Mutex, MutexGuard,
    atomic::{AtomicIsize, Ordering},
};
use windows::{
    Win32::Graphics::{
        Direct3D11::{ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D},
        Dxgi::{
            Common::{DXGI_ALPHA_MODE_IGNORE, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC},
            DXGI_PRESENT, DXGI_SCALING_STRETCH, DXGI_SWAP_CHAIN_DESC1,
            DXGI_SWAP_EFFECT_FLIP_DISCARD, DXGI_USAGE_RENDER_TARGET_OUTPUT, IDXGIDevice,
            IDXGIFactory2, IDXGISwapChain1,
        },
    },
    core::Interface,
};
use windows_sys::Win32::Graphics::Gdi::InvalidateRect;

/// Ce que `present` et `resize` se partagent, sous un verrou.
struct State {
    context: ID3D11DeviceContext,
    texture: Texture,
    chain: IDXGISwapChain1,
    /// La chaîne d'avant un changement de taille, que la fenêtre montre encore : gardée
    /// jusqu'à la première image présentée dans la nouvelle.
    previous: Option<IDXGISwapChain1>,
}

pub(crate) struct Native {
    device: ID3D11Device,
    luid: AdapterLuid,
    name: String,
    state: Mutex<State>,
    /// La fenêtre qui montre la surface (0 : aucune), réveillée quand la chaîne change.
    window: AtomicIsize,
}

// SAFETY : les objets Direct3D 11 et DXGI ne sont liés à aucun fil (pas d'appartement
// COM) ; le périphérique, créé sans `D3D11_CREATE_DEVICE_SINGLETHREADED`, se partage
// entre fils. Le contexte immédiat, la texture et la chaîne d'échange, qui ne se
// partagent pas, ne sont touchés que sous le verrou de `state` ; la couche de composition
// n'en garde que des références comptées (`AddRef` et `Release` sont sûrs entre fils).
unsafe impl Send for Native {}
// SAFETY : idem.
unsafe impl Sync for Native {}

impl Native {
    pub(crate) fn new(
        adapter: Option<AdapterLuid>,
        width: u32,
        height: u32,
    ) -> crate::Result<Self> {
        let Device {
            device,
            context,
            luid,
            name,
        } = device::create(adapter)?;
        let texture = Texture::new(&device, width, height)?;
        let chain = chain(&device, width, height)?;
        Ok(Self {
            device,
            luid,
            name,
            state: Mutex::new(State {
                context,
                texture,
                chain,
                previous: None,
            }),
            window: AtomicIsize::new(0),
        })
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub(crate) fn adapter(&self) -> AdapterLuid {
        self.luid
    }

    pub(crate) fn adapter_name(&self) -> String {
        self.name.clone()
    }

    pub(crate) fn size(&self) -> (u32, u32) {
        let state = self.state();
        (state.texture.width, state.texture.height)
    }

    pub(crate) fn handle(&self) -> isize {
        self.state().texture.handle()
    }

    /// La chaîne d'échange à montrer, pour la couche de composition : la courante, ou
    /// celle d'avant tant que la courante n'a reçu aucune image.
    pub(crate) fn chain(&self) -> IDXGISwapChain1 {
        let state = self.state();
        state.previous.as_ref().unwrap_or(&state.chain).clone()
    }

    /// Réveille la fenêtre qui montre la surface, pour qu'elle rattache la chaîne.
    fn wake_window(&self) {
        let window = self.window.load(Ordering::Acquire);
        if window != 0 {
            // SAFETY : un rectangle nul désigne toute la zone client ; Windows valide la
            // poignée (fenêtre peut-être déjà fermée) et accepte l'appel de tout fil.
            unsafe { InvalidateRect(window as _, std::ptr::null(), 0) };
        }
    }

    /// Note la fenêtre qui montre la surface (0 : plus aucune).
    pub(crate) fn shown_in(&self, window: isize) {
        self.window.store(window, Ordering::Release);
    }

    pub(crate) fn resize(&self, width: u32, height: u32) -> crate::Result<()> {
        if self.size() == (width, height) {
            return Ok(());
        }
        // Texture et chaîne neuves : la couche de composition garde l'ancienne chaîne,
        // et sa dernière image, jusqu'à la première image présentée dans la nouvelle.
        let texture = Texture::new(&self.device, width, height)?;
        let chain = chain(&self.device, width, height)?;
        let mut state = self.state();
        state.texture = texture;
        let replaced = std::mem::replace(&mut state.chain, chain);
        state.previous.get_or_insert(replaced);
        Ok(())
    }

    pub(crate) fn present(&self) -> crate::Result<()> {
        let mut state = self.state();
        state.texture.acquire_drawn()?;
        // SAFETY : chaîne valide ; l'IID provient du type demandé. Avec le modèle
        // « flip » de Direct3D 11, le tampon 0 est toujours celui à dessiner.
        let copied = unsafe { state.chain.GetBuffer::<ID3D11Texture2D>(0) }
            .check("IDXGISwapChain::GetBuffer")
            .map(|buffer| {
                // SAFETY : deux textures de ce périphérique, de même taille et de la même
                // famille de formats (BGRA 8 bits) ; les commandes partent vers la carte
                // avant que le verrou ne soit rendu au producteur.
                unsafe {
                    state.context.CopyResource(&buffer, &state.texture.texture);
                    state.context.Flush();
                }
            });
        state.texture.release_free()?;
        copied?;
        // Sans attendre l'écran : le fil du producteur garde sa propre cadence, et DWM
        // compose la dernière image présentée.
        // SAFETY : chaîne valide, tampon complet.
        unsafe { state.chain.Present(0, DXGI_PRESENT(0)) }
            .ok()
            .check("IDXGISwapChain::Present")?;
        if state.previous.take().is_some() {
            drop(state);
            self.wake_window();
        }
        Ok(())
    }

    pub(crate) fn read_pixels(&self) -> crate::Result<Vec<u8>> {
        let mut state = self.state();
        state.texture.acquire_drawn()?;
        let State {
            context, texture, ..
        } = &mut *state;
        let pixels = texture.read(&self.device, context);
        state.texture.release_free()?;
        pixels
    }
}

/// Chaîne d'échange de composition de cette taille, opaque, qui reçoit les octets de la
/// texture tels quels.
fn chain(device: &ID3D11Device, width: u32, height: u32) -> crate::Result<IDXGISwapChain1> {
    let dxgi: IDXGIDevice = device.cast().check("QueryInterface IDXGIDevice")?;
    // SAFETY : périphérique DXGI valide ; windows-rs gère la référence renvoyée.
    let adapter = unsafe { dxgi.GetAdapter() }.check("IDXGIDevice::GetAdapter")?;
    // SAFETY : carte valide ; l'IID provient du type demandé.
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
        // Quatre tampons. Tout se passe comme si Windows gardait le tampon affiché et
        // celui d'avant (lecture des mesures, non vérifiée autrement) : il en reste deux
        // libres par période d'écran. Une copie vers un tampon encore tenu attend l'écran
        // sur la carte, et le producteur, qui reprend la texture ensuite, attend avec
        // elle. Mesuré sur une RTX 4080 : avec deux tampons, jusqu'à 14 ms par image à 60
        // images par seconde ; avec trois, une image sur deux quand le producteur suit
        // une souris à 125 Hz (deux images par période) ; avec quatre, plus rien.
        BufferCount: 4,
        // Seul mode accepté par CreateSwapChainForComposition.
        Scaling: DXGI_SCALING_STRETCH,
        // La dernière image présentée remplace celles qui attendaient l'écran.
        SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
        AlphaMode: DXGI_ALPHA_MODE_IGNORE,
        Flags: 0,
    };
    // SAFETY : description locale valide pendant l'appel ; aucune sortie restreinte.
    let chain = unsafe { factory.CreateSwapChainForComposition(device, &description, None) }
        .check("CreateSwapChainForComposition")?;
    // Une première image, noire : avant celle du producteur, le trou de l'interface ne
    // laisse pas voir ce qui est derrière la fenêtre.
    // SAFETY : chaîne valide ; ses tampons neufs sont à zéro, l'alpha est ignoré.
    unsafe { chain.Present(0, DXGI_PRESENT(0)) }
        .ok()
        .check("IDXGISwapChain::Present")?;
    Ok(chain)
}
