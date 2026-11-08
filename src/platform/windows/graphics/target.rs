//! Cible de dessin (texture relue ou chaîne d'échange) : création, redimensionnement et
//! présentation.
use super::*;

pub(super) struct Target {
    pub(super) canvas: Canvas,
    pub(super) output: Output,
    pub(super) width: u32,
    pub(super) height: u32,
}
pub(super) enum Output {
    /// Texture hors écran, copiée dans `staging` puis relue par le CPU.
    Readback {
        texture: ID3D11Texture2D,
        staging: ID3D11Texture2D,
    },
    /// Back buffer d'une chaîne d'échange composée par DWM.
    SwapChain {
        chain: IDXGISwapChain1,
        premultiplied: bool,
        slot: swapchain::FrameSlot,
    },
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Readback,
    SwapChain { premultiplied: bool },
}
impl Target {
    fn kind(&self) -> Kind {
        match self.output {
            Output::Readback { .. } => Kind::Readback,
            Output::SwapChain { premultiplied, .. } => Kind::SwapChain { premultiplied },
        }
    }
}
impl std::ops::Deref for Target {
    type Target = Canvas;
    fn deref(&self) -> &Canvas {
        &self.canvas
    }
}
impl Renderer {
    pub(super) fn resize(
        &mut self,
        width: u32,
        height: u32,
        scale: f32,
        kind: Kind,
    ) -> crate::Result<SurfaceChange> {
        if width == 0
            || height == 0
            || width > 16384
            || height > 16384
            || !scale.is_finite()
            || scale <= 0.
        {
            return Err(crate::Error::invalid_input("invalid render size or scale"));
        }
        self.ready()?;
        let current = self.target.as_ref().map(|t| (t.kind(), t.width, t.height));
        let change = if current == Some((kind, width, height)) {
            SurfaceChange::Unchanged
        } else if current.is_some_and(|(current, ..)| current == kind)
            && let Some(Target {
                canvas,
                output: Output::SwapChain { chain, .. },
                ..
            }) = self.target.as_ref()
        {
            // Même chaîne, autre taille : les ressources du contexte restent valides.
            let resized = swapchain::resize(
                &self.gpu.gpu().context,
                &canvas.target,
                chain,
                width,
                height,
                scale,
            );
            if let Err(error) = resized {
                // Sans cible attachée, la prochaine préparation repartira de zéro.
                self.target = None;
                return Err(error);
            }
            let target = self.target.as_mut().unwrap();
            target.width = width;
            target.height = height;
            SurfaceChange::Resized
        } else {
            // Libérer l'ancienne cible avant d'allouer la nouvelle.
            self.target = None;
            self.target = Some(match kind {
                Kind::Readback => self.readback_target(width, height, scale)?,
                Kind::SwapChain { premultiplied } => {
                    let gpu = self.gpu.gpu();
                    let (chain, slot) =
                        swapchain::create(&gpu.device, width, height, premultiplied)?;
                    let context = device::context(&gpu.d2d)?;
                    swapchain::bind(&context, &chain, scale)?;
                    Target {
                        canvas: Canvas::new(context)?,
                        output: Output::SwapChain {
                            chain,
                            premultiplied,
                            slot,
                        },
                        width,
                        height,
                    }
                }
            });
            SurfaceChange::Created
        };
        let t = self.target.as_ref().unwrap();
        // SAFETY: contexte Direct2D valide, hors BeginDraw/EndDraw.
        unsafe { t.target.SetDpi(96. * scale, 96. * scale) };
        Ok(change)
    }

    /// Crée ou redimensionne la chaîne d'échange de présentation. Un changement de mode
    /// alpha recrée la chaîne : le visuel de composition doit alors la rattacher.
    pub(in crate::platform::windows) fn prepare_swap_chain(
        &mut self,
        width: u32,
        height: u32,
        scale: f32,
        premultiplied: bool,
    ) -> crate::Result<SurfaceChange> {
        self.resize(width, height, scale, Kind::SwapChain { premultiplied })
    }
    /// Dessine la scène dans le back buffer, sans relecture.
    pub(in crate::platform::windows) fn render_to_swap_chain(
        &mut self,
        scene: &Scene,
        width: u32,
        height: u32,
        scale: f32,
        premultiplied: bool,
    ) -> crate::Result<()> {
        self.draw_scene(
            scene,
            width,
            height,
            scale,
            Kind::SwapChain { premultiplied },
        )
    }
    pub(in crate::platform::windows) fn swap_chain(&self) -> Option<&IDXGISwapChain1> {
        match &self.target.as_ref()?.output {
            Output::SwapChain { chain, .. } => Some(chain),
            Output::Readback { .. } => None,
        }
    }
    /// Présente le back buffer (vsync). Une perte du périphérique renvoie
    /// [`crate::Error::DeviceLost`].
    pub(in crate::platform::windows) fn present(&mut self) -> crate::Result<()> {
        #[cfg(test)]
        if std::mem::take(&mut self.lose_device) {
            return Err(com_error(
                "IDXGISwapChain::Present",
                DXGI_ERROR_DEVICE_REMOVED,
            ));
        }
        match self.target.as_mut().map(|t| &mut t.output) {
            Some(Output::SwapChain { chain, slot, .. }) => swapchain::present(chain, slot),
            _ => Err(crate::Error::invalid_input("no swap chain to present")),
        }
    }
    /// Attend une place dans la file de présentation avant de préparer une image (voir
    /// `swapchain::FrameSlot`) ; sans effet sans chaîne d'échange ou place déjà prise.
    pub(in crate::platform::windows) fn wait_for_frame(&mut self) {
        if let Some(Output::SwapChain { slot, .. }) = self.target.as_mut().map(|t| &mut t.output) {
            slot.acquire();
        }
    }
    /// Temps passé à attendre la place de l'image en cours dans la file (pas du travail :
    /// la cadence de l'écran), remis à zéro par `present`.
    pub(in crate::platform::windows) fn frame_wait(&self) -> std::time::Duration {
        match self.target.as_ref().map(|t| &t.output) {
            Some(Output::SwapChain { slot, .. }) => slot.waited,
            _ => std::time::Duration::ZERO,
        }
    }
    #[cfg(test)]
    pub(in crate::platform::windows) fn simulate_device_loss(&mut self) {
        self.lose_device = true;
    }
}
