//! Le type public d'une surface partagée, commun à tous les systèmes ; ce qui touche à la
//! carte graphique est dans `Native` (Windows : `platform/windows/shared_surface/`).
#[cfg(not(windows))]
use super::unavailable::Native;
#[cfg(windows)]
use crate::platform::shared_surface::Native;
use crate::ui::scene::Image;
use std::sync::{Arc, Mutex};

/// The number Windows gives a graphics card (its LUID), as eight bytes: the value of
/// `VkPhysicalDeviceIDProperties::deviceLUID` for the same card in Vulkan.
pub type AdapterLuid = [u8; 8];

/// Where a [`SharedSurface`] is shown: the rectangle of its element in the window, in
/// physical pixels from the top-left corner of the client area.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfacePlacement {
    /// Left edge.
    pub x: i32,
    /// Top edge.
    pub y: i32,
    /// Width of the element.
    pub width: u32,
    /// Height of the element.
    pub height: u32,
}

/// Ce que les copies d'une [`SharedSurface`] partagent, et ce que le backend en voit.
pub(crate) struct Shared {
    native: Native,
    placeholder: Arc<Image>,
    /// La place de l'élément quand la surface est montrée sous l'interface.
    placement: Mutex<Option<SurfacePlacement>>,
}

#[cfg_attr(not(windows), allow(dead_code))]
impl Shared {
    pub(crate) fn native(&self) -> &Native {
        &self.native
    }
    pub(crate) fn placeholder(&self) -> &Arc<Image> {
        &self.placeholder
    }
    /// Note où la fenêtre montre la surface ; `None` : elle ne la montre pas (ou plus).
    pub(crate) fn set_placement(&self, placement: Option<SurfacePlacement>) {
        *self.placement.lock().unwrap_or_else(|e| e.into_inner()) = placement;
    }
}

impl Drop for Shared {
    fn drop(&mut self) {
        super::registry::unregister(Arc::as_ptr(&self.placeholder) as usize);
    }
}

/// A texture on the graphics card that another renderer draws into and the window shows
/// in place of an element, under the interface.
///
/// The texture is a Direct3D 11 texture in BGRA sRGB (`DXGI_FORMAT_B8G8R8A8_UNORM_SRGB`),
/// shared by an NT handle and protected by a keyed mutex: the producer takes the mutex
/// with key 0, draws, and releases it with key 1; [`present`](Self::present) takes it
/// with key 1, shows the image and releases it with key 0. Neither side ever reads a
/// half-drawn image.
///
/// Clones share the same surface; every method may be called from any thread. The
/// usual split: the interface thread paints [`placeholder`](Self::placeholder), the
/// thread of the producer calls [`present`](Self::present) and
/// [`resize`](Self::resize).
///
/// ```
/// use karbon_ui::ui::shared_surface::SharedSurface;
///
/// match SharedSurface::new(None, 640, 360) {
///     Ok(surface) => {
///         assert_eq!(surface.size(), (640, 360));
///         // Give `surface.texture_handle()` and `surface.adapter()` to the producer;
///         // after each image it draws, call `surface.present()`.
///     }
///     // No graphics card, or not Windows: keep copying images (`LiveImage`).
///     Err(error) => eprintln!("no shared surface: {error}"),
/// }
/// ```
#[derive(Clone)]
pub struct SharedSurface(Arc<Shared>);

impl SharedSurface {
    /// Make a surface of `width` × `height` pixels on the graphics card `adapter`
    /// (`None`: the default card, the one windows are drawn with). Give the card of the
    /// producer: a texture can only be shared between renderers on the same card.
    ///
    /// # Errors
    ///
    /// [`Error::Unsupported`](crate::Error::Unsupported) outside Windows;
    /// [`Error::InvalidInput`](crate::Error::InvalidInput) for a size of zero or above
    /// 16384; [`Error::Platform`](crate::Error::Platform) when no card has this number or
    /// Direct3D refuses the device or the texture.
    pub fn new(adapter: Option<AdapterLuid>, width: u32, height: u32) -> crate::Result<Self> {
        let native = Native::new(adapter, width, height)?;
        // Le gabarit : un pixel noir opaque, étiré là où la surface ne peut pas être
        // montrée (fenêtre « layered », export hors écran).
        let placeholder = Arc::new(Image::new(1, 1, vec![0, 0, 0, 255])?);
        let shared = Arc::new(Shared {
            native,
            placeholder,
            placement: Mutex::new(None),
        });
        super::registry::register(&shared);
        Ok(Self(shared))
    }

    /// The graphics card the surface lives on.
    pub fn adapter(&self) -> AdapterLuid {
        self.0.native.adapter()
    }

    /// The name of that card, for a status line or a log.
    pub fn adapter_name(&self) -> String {
        self.0.native.adapter_name()
    }

    /// The size of the texture, in pixels.
    pub fn size(&self) -> (u32, u32) {
        self.0.native.size()
    }

    /// The NT handle of the texture, for the producer to import (Vulkan:
    /// `VK_EXTERNAL_MEMORY_HANDLE_TYPE_D3D11_TEXTURE_BIT`). It belongs to the surface
    /// and is valid until the next [`resize`](Self::resize) or until the surface is
    /// dropped; the producer must have let go of the texture by then.
    pub fn texture_handle(&self) -> isize {
        self.0.native.handle()
    }

    /// Replace the texture by one of another size. The producer lets go of the old
    /// texture first, then imports the new [`texture_handle`](Self::texture_handle).
    /// The window keeps showing the last image, stretched, until the first
    /// [`present`](Self::present) at the new size.
    ///
    /// # Errors
    ///
    /// As [`new`](Self::new); the surface keeps its texture when the new one fails.
    pub fn resize(&self, width: u32, height: u32) -> crate::Result<()> {
        self.0.native.resize(width, height)
    }

    /// Show the image the producer has just drawn. Waits until the producer has
    /// released the texture (key 1), copies it on the graphics card to what the window
    /// composes, and releases it for the next image (key 0). The interface is not
    /// repainted.
    ///
    /// # Errors
    ///
    /// [`Error::TimedOut`](crate::Error::TimedOut) when the producer has not released
    /// the texture within a second (it drew nothing, or without the keyed mutex);
    /// [`Error::Platform`](crate::Error::Platform) or
    /// [`Error::DeviceLost`](crate::Error::DeviceLost) when Direct3D fails.
    pub fn present(&self) -> crate::Result<()> {
        self.0.native.present()
    }

    /// Read back the image the producer has just drawn instead of showing it: BGRA
    /// bytes as stored in the texture (sRGB-encoded), rows from top to bottom. For
    /// tests and checks; like [`present`](Self::present), it takes the image (key 1)
    /// and frees the texture for the next one (key 0).
    ///
    /// # Errors
    ///
    /// As [`present`](Self::present).
    pub fn read_pixels(&self) -> crate::Result<Vec<u8>> {
        self.0.native.read_pixels()
    }

    /// The image an element paints where the surface must appear (a
    /// [`DrawCommand::Image`](crate::ui::scene::DrawCommand::Image) over the rectangle of
    /// the element). A window composed by DirectComposition recognises it: it clears
    /// that rectangle to transparent and keeps the surface under it. Anywhere else (a
    /// layered window, an export) it is painted as it is, a black rectangle.
    ///
    /// The element must not be inside an opacity group, a rounded clip or a mask: the
    /// hole would be cut in the group instead of the window.
    pub fn placeholder(&self) -> Arc<Image> {
        Arc::clone(&self.0.placeholder)
    }

    /// Where the window shows the surface, once an element has painted
    /// [`placeholder`](Self::placeholder) in a window that can show it; `None` before
    /// that, and whenever the surface cannot be shown: the application then falls back
    /// on copying images. The size is the one the producer should draw at.
    pub fn placement(&self) -> Option<SurfacePlacement> {
        *self.0.placement.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::shared_surface::surface_of;

    #[test]
    fn only_the_placeholder_of_a_living_surface_is_recognised() {
        let plain = Arc::new(Image::new(1, 1, vec![0, 0, 0, 255]).unwrap());
        assert!(surface_of(&plain).is_none());
        // Sans carte graphique (ou hors de Windows), rien à vérifier de plus.
        let Ok(surface) = SharedSurface::new(None, 32, 16) else {
            return;
        };
        let placeholder = surface.placeholder();
        assert!(surface_of(&placeholder).is_some());
        assert!(surface_of(&plain).is_none());
        assert_eq!(surface.placement(), None);
        drop(surface);
        assert!(surface_of(&placeholder).is_none());
    }

    #[test]
    fn a_size_of_zero_is_refused() {
        assert!(SharedSurface::new(None, 0, 16).is_err());
    }
}
