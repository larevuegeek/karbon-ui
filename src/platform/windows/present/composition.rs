//! Choix du chemin de présentation et visuel DirectComposition d'une fenêtre.
//!
//! Composition : fenêtre WS_EX_NOREDIRECTIONBITMAP, chaîne d'échange du renderer
//! posée comme contenu d'un visuel racine, sans relecture CPU. Layered : relecture
//! GPU → DIB puis UpdateLayeredWindow, conservé en repli et pour le clic traversant.
use super::{
    super::{
        ffi::{Hwnd, client_area, last_error, scale},
        graphics::{Checked, Renderer},
    },
    surface::LayeredSurface,
};
use crate::window::{PresentMode, WindowOptions};
use windows::Win32::{
    Foundation::HWND,
    Graphics::{
        DirectComposition::{
            DCompositionCreateDevice, IDCompositionDevice, IDCompositionTarget, IDCompositionVisual,
        },
        Dxgi::{IDXGIDevice, IDXGISwapChain1},
    },
};

/// Chemin effectivement utilisé par une fenêtre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Path {
    Composition,
    Layered,
}
impl Path {
    pub(super) fn label(self) -> &'static str {
        match self {
            Path::Composition => "composition",
            Path::Layered => "layered",
        }
    }
}

/// Chemin tenté en premier. WS_EX_TRANSPARENT ne rend une fenêtre traversante que si
/// elle est aussi WS_EX_LAYERED : le clic traversant impose donc le chemin layered.
pub(crate) fn requested(options: &WindowOptions) -> Path {
    if options.click_through || options.presentation == PresentMode::Layered {
        Path::Layered
    } else {
        Path::Composition
    }
}
/// Après un échec de la composition, seul le mode automatique se replie sur layered.
pub(crate) fn falls_back(options: &WindowOptions) -> bool {
    options.presentation == PresentMode::Auto
}
/// Alpha prémultiplié dès que des pixels peuvent être transparents : fenêtre transparente,
/// ou coins arrondis du fond hors agrandissement. Une image opaque laisse DWM ignorer l'alpha.
pub(super) fn premultiplied(transparent: bool, maximized: bool) -> bool {
    transparent || !maximized
}

/// Visuel racine d'une fenêtre ; il référence la chaîne d'échange courante du renderer.
pub(crate) struct Visual {
    device: IDCompositionDevice,
    // Conservé : sa libération détacherait l'arbre de la fenêtre.
    _target: IDCompositionTarget,
    visual: IDCompositionVisual,
    bound: Option<IDXGISwapChain1>,
}
impl Visual {
    fn new(window: Hwnd) -> crate::Result<Self> {
        #[cfg(test)]
        if tests::FAIL.with(std::cell::Cell::get) {
            return Err(crate::Error::Unsupported {
                reason: "composition disabled by the test",
            });
        }
        // Sans périphérique DXGI : le visuel n'affiche qu'une chaîne d'échange, ce qui
        // le rend indépendant des recréations du renderer (F2, perte du périphérique).
        // SAFETY: appel sans pointeur ; l'IID provient du type demandé.
        let device: IDCompositionDevice = unsafe { DCompositionCreateDevice(None::<&IDXGIDevice>) }
            .check("DCompositionCreateDevice")?;
        // SAFETY: fenêtre valide créée sur ce thread ; la cible est conservée par `Visual`.
        let target = unsafe { device.CreateTargetForHwnd(HWND(window), true) }
            .check("CreateTargetForHwnd")?;
        // SAFETY: périphérique valide ; le visuel est possédé par `Visual`.
        let visual = unsafe { device.CreateVisual() }.check("CreateVisual")?;
        // SAFETY: cible et visuel issus du même périphérique.
        unsafe { target.SetRoot(&visual) }.check("IDCompositionTarget::SetRoot")?;
        Ok(Self {
            device,
            _target: target,
            visual,
            bound: None,
        })
    }
    /// Rattache la chaîne courante si elle a changé (création, mode alpha, nouveau renderer).
    pub(super) fn bind(&mut self, chain: &IDXGISwapChain1) -> crate::Result<()> {
        if self.bound.as_ref() == Some(chain) {
            return Ok(());
        }
        // SAFETY: visuel valide ; DirectComposition prend sa propre référence sur la chaîne.
        unsafe { self.visual.SetContent(chain) }.check("IDCompositionVisual::SetContent")?;
        // SAFETY: périphérique valide ; valide la transaction sur ce thread.
        unsafe { self.device.Commit() }.check("IDCompositionDevice::Commit")?;
        self.bound = Some(chain.clone());
        Ok(())
    }
    #[cfg(test)]
    pub(super) fn bound(&self) -> Option<&IDXGISwapChain1> {
        self.bound.as_ref()
    }
}

/// Surface de présentation possédée par l'état de dessin d'une fenêtre.
pub(crate) enum Presenter {
    /// DIB persistant, créé à la première peinture.
    Layered(Option<LayeredSurface>),
    Composition(Visual),
}
impl Presenter {
    pub(super) fn path(&self) -> Path {
        match self {
            Presenter::Layered(_) => Path::Layered,
            Presenter::Composition(_) => Path::Composition,
        }
    }
}

/// Crée le visuel et une première chaîne d'échange à la taille du client : toute la
/// chaîne de composition est validée avant que l'application ne soit confiée à la fenêtre.
pub(crate) fn attach(
    window: Hwnd,
    renderer: &mut Renderer,
    options: &WindowOptions,
) -> crate::Result<Visual> {
    let mut visual = Visual::new(window)?;
    let area = client_area(window).ok_or_else(|| last_error("GetClientRect"))?;
    let scale = scale(window);
    renderer.prepare_swap_chain(
        area.right.max(1) as u32,
        area.bottom.max(1) as u32,
        scale,
        premultiplied(options.transparent, false),
    )?;
    visual.bind(renderer.swap_chain().expect("swap chain prepared above"))?;
    Ok(visual)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    thread_local! {
        /// Simule l'indisponibilité de DirectComposition sur le thread de test.
        pub(crate) static FAIL: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }
    #[test]
    fn click_through_and_explicit_layered_select_the_layered_path() {
        let options = |presentation, click_through| WindowOptions {
            presentation,
            transparent: true,
            click_through,
            ..WindowOptions::DEFAULT
        };
        assert_eq!(
            requested(&options(PresentMode::Auto, false)),
            Path::Composition
        );
        assert_eq!(
            requested(&options(PresentMode::Composition, false)),
            Path::Composition
        );
        assert_eq!(
            requested(&options(PresentMode::Layered, false)),
            Path::Layered
        );
        for mode in [
            PresentMode::Auto,
            PresentMode::Composition,
            PresentMode::Layered,
        ] {
            assert_eq!(requested(&options(mode, true)), Path::Layered);
        }
        assert!(falls_back(&options(PresentMode::Auto, false)));
        assert!(!falls_back(&options(PresentMode::Composition, false)));
    }
    #[test]
    fn opaque_alpha_mode_is_reserved_for_maximized_opaque_windows() {
        assert!(premultiplied(true, true));
        assert!(premultiplied(true, false));
        assert!(premultiplied(false, false));
        assert!(!premultiplied(false, true));
    }
}
