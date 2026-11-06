//! Données envoyées au GPU par les renderers Metal (macOS) et Vulkan (Linux) : une
//! instance par quad, arrêts des dégradés et maillons des découpes arrondies. Les
//! structures sont le miroir exact de celles de `macos/metal/shaders.metal` et de
//! `linux/vulkan/shaders/common.glsl` (champs de 16 octets, sans remplissage).
use crate::ui::{rendering::transform::Affine, scene::RectF, theme::Color};

pub(in crate::platform) const SHAPE_NONE: u32 = 0;
pub(in crate::platform) const SHAPE_BOX: u32 = 1;
pub(in crate::platform) const SHAPE_RING: u32 = 2;
pub(in crate::platform) const SHAPE_SEGMENT: u32 = 3;
pub(in crate::platform) const PAINT_SOLID: u32 = 0;
pub(in crate::platform) const PAINT_LINEAR: u32 = 1;
pub(in crate::platform) const PAINT_RADIAL: u32 = 2;
pub(in crate::platform) const PAINT_TEXTURE: u32 = 3;
pub(in crate::platform) const PAINT_MASK: u32 = 4;

/// Découpe rectangulaire neutre (pixels de la cible).
pub(in crate::platform) const NO_CLIP: [f32; 4] = [-1e9, -1e9, 1e9, 1e9];

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub(in crate::platform) struct Instance {
    pub quad: [f32; 4],
    pub m: [f32; 4],
    pub t: [f32; 4],
    pub shape: [f32; 4],
    pub radii: [f32; 4],
    pub inner: [f32; 4],
    pub iradii: [f32; 4],
    pub color: [f32; 4],
    pub paint: [f32; 4],
    pub uv: [f32; 4],
    pub clip: [f32; 4],
    pub kind: [u32; 4],
    pub extra: [i32; 4],
}
const _: () = assert!(std::mem::size_of::<Instance>() == 13 * 16);

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub(in crate::platform) struct Stop {
    pub color: [f32; 4],
    pub position: [f32; 4],
}
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub(in crate::platform) struct ClipNode {
    pub m: [f32; 4],
    pub t: [f32; 4],
    pub rect: [f32; 4],
    pub radii: [f32; 4],
    pub link: [i32; 4],
}
#[repr(C)]
#[derive(Clone, Copy, Debug)]
// Vulkan passe le repère par un tableau de quatre flottants (`commands.rs`).
#[cfg_attr(target_os = "linux", allow(dead_code))]
pub(in crate::platform) struct Frame {
    pub origin: [f32; 2],
    pub size: [f32; 2],
}

pub(in crate::platform) fn rect(r: RectF) -> [f32; 4] {
    [r.left, r.top, r.right, r.bottom]
}
pub(in crate::platform) fn color(c: Color) -> [f32; 4] {
    [c.red, c.green, c.blue, c.alpha].map(|v| f32::from(v) / 255.)
}
/// Taille d'un pixel de la cible dans le repère local de `m` (similitudes : exacte).
pub(in crate::platform) fn pixel(m: Affine) -> f32 {
    let det = (m.m11 * m.m22 - m.m12 * m.m21).abs();
    if det > 1e-12 { 1. / det.sqrt() } else { 1. }
}
/// `true` si `m` garde les axes, sans miroir (échelle et translation seulement).
pub(in crate::platform) fn axis_aligned(m: Affine) -> bool {
    m.m12 == 0. && m.m21 == 0. && m.m11 > 0. && m.m22 > 0.
}
impl Instance {
    /// Instance vide sur `quad` (repère de `m`), peinte en `paint`.
    pub fn new(quad: RectF, m: Affine, shape: u32, paint: u32) -> Self {
        Self {
            quad: rect(quad),
            m: [m.m11, m.m12, m.m21, m.m22],
            t: [m.dx, m.dy, pixel(m), 1.],
            shape: rect(quad),
            radii: [0.; 4],
            inner: [0.; 4],
            iradii: [0.; 4],
            color: [0.; 4],
            paint: [0.; 4],
            uv: [0., 0., 1., 1.],
            clip: NO_CLIP,
            kind: [shape, paint, 0, 0],
            extra: [-1, 0, 0, 0],
        }
    }
    /// Boîte arrondie `shape` (son quad déborde d'un pixel pour l'anticrénelage).
    pub fn rounded(shape: RectF, radii: [f32; 4], m: Affine) -> Self {
        let p = pixel(m);
        let quad = RectF {
            left: shape.left - p,
            top: shape.top - p,
            right: shape.right + p,
            bottom: shape.bottom + p,
        };
        let mut i = Self::new(quad, m, SHAPE_BOX, PAINT_SOLID);
        i.shape = rect(shape);
        i.radii = radii;
        i
    }
    /// Bornes du quad en pixels de la cible.
    pub fn device_bounds(&self) -> RectF {
        let m = Affine {
            m11: self.m[0],
            m12: self.m[1],
            m21: self.m[2],
            m22: self.m[3],
            dx: self.t[0],
            dy: self.t[1],
        };
        m.bounds(RectF {
            left: self.quad[0],
            top: self.quad[1],
            right: self.quad[2],
            bottom: self.quad[3],
        })
    }
}
