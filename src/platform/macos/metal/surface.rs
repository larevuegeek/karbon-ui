//! Calque `CAMetalLayer` d'une fenêtre : sous-calque du calque de la vue, BGRA8 non
//! linéaire en espace sRGB (comme la fenêtre et les images Core Graphics), réservé au
//! rendu (`framebufferOnly`), présenté dans la transaction Core Animation de la session
//! (`presentsWithTransaction`) pour que le passage entre Metal et Core Graphics change de
//! calque visible et de contenu dans la même image.
use super::device::{Device, FORMAT};
use super::presentation::Presentation;
use objc2::{rc::Retained, runtime::ProtocolObject};
use objc2_core_foundation::{CGRect, CGSize};
use objc2_quartz_core::{CALayer, CAMetalDrawable, CAMetalLayer};

pub(in crate::platform::macos) struct Surface {
    pub layer: Retained<CAMetalLayer>,
    size: (usize, usize),
    /// Instants d'affichage relevés pendant la sonde de défilement.
    presentation: Option<Presentation>,
}
pub(super) type Drawable = Retained<ProtocolObject<dyn CAMetalDrawable>>;
impl Surface {
    /// Calque Metal ajouté (masqué) au-dessus du contenu de `parent`.
    pub(super) fn new(device: &Device, parent: &CALayer, opaque: bool) -> crate::Result<Self> {
        let layer = CAMetalLayer::new();
        layer.setDevice(Some(device));
        layer.setPixelFormat(FORMAT);
        layer.setFramebufferOnly(true);
        layer.setPresentsWithTransaction(true);
        layer.setOpaque(opaque);
        let space = super::super::ffi::srgb()?;
        layer.setColorspace(Some(&space));
        layer.setHidden(true);
        // Trois drawables (valeur par défaut, ~17 Mo chacun pour une fenêtre Retina
        // 1440 × 791) : avec deux, l'attente du drawable suivant double le temps d'image
        // pendant un redimensionnement (mesuré : 17 → 30 ms par image).
        parent.addSublayer(&layer);
        // Même orientation des pixels que les images posées sur le calque de la vue.
        if layer.contentsAreFlipped() != parent.contentsAreFlipped() {
            layer.setGeometryFlipped(!layer.isGeometryFlipped());
        }
        Ok(Self {
            layer,
            size: (0, 0),
            presentation: Presentation::from_env(),
        })
    }
    /// Suit la vue : cadre `bounds` (points), `scale` pixels par point, drawable de
    /// `width` × `height` pixels. À appeler dans une transaction sans animation.
    pub(in crate::platform::macos) fn fit(
        &mut self,
        bounds: CGRect,
        scale: f64,
        (width, height): (usize, usize),
    ) {
        self.layer.setFrame(bounds);
        self.layer.setContentsScale(scale);
        if self.size != (width, height) {
            self.layer
                .setDrawableSize(CGSize::new(width as f64, height as f64));
            self.size = (width, height);
        }
    }
    pub(super) fn next(&self) -> Option<Drawable> {
        self.layer.nextDrawable()
    }
    /// À appeler avant de présenter `drawable` : relève son affichage pendant la sonde.
    pub(super) fn watch(&self, drawable: &Drawable) {
        if let Some(presentation) = &self.presentation {
            presentation.watch(ProtocolObject::from_ref(&**drawable));
        }
    }
    /// Fenêtre réduite ou cachée : les drawables à la taille de la fenêtre sont abandonnés
    /// (drawables d'un pixel) ; la prochaine image (`fit`) leur rend leur taille.
    pub(in crate::platform::macos) fn release(&mut self) {
        self.layer.setDrawableSize(CGSize::new(1., 1.));
        self.size = (0, 0);
    }
    pub(in crate::platform::macos) fn show(&self, visible: bool) {
        self.layer.setHidden(!visible);
    }
}
