//! Conversions partagées entre le moteur et Core Foundation / Core Graphics.
use crate::ui::{
    rendering::transform::Affine,
    scene::{PointF, RectF},
    theme::Color,
};
use objc2_core_foundation::{
    CFIndex, CFRange, CFRetained, CFString, CGAffineTransform, CGFloat, CGPoint, CGRect, CGSize,
};
use objc2_core_graphics::{CGColor, CGColorSpace, kCGColorSpaceSRGB};

/// Échec d'un appel Core Foundation / Core Graphics qui ne fournit pas de code d'erreur :
/// une sortie nulle devient `Error::Platform` avec le code générique `E_FAIL`.
pub(super) fn failure(operation: &'static str) -> crate::Error {
    crate::Error::Platform {
        operation,
        code: 0x8000_4005_u32 as i32,
    }
}
/// Sortie obligatoire d'un appel natif.
pub(super) fn required<T>(value: Option<T>, operation: &'static str) -> crate::Result<T> {
    value.ok_or_else(|| failure(operation))
}
pub(super) fn rect(r: RectF) -> CGRect {
    CGRect::new(
        CGPoint::new(CGFloat::from(r.left), CGFloat::from(r.top)),
        CGSize::new(
            CGFloat::from(r.right - r.left),
            CGFloat::from(r.bottom - r.top),
        ),
    )
}
pub(super) fn point(p: PointF) -> CGPoint {
    CGPoint::new(CGFloat::from(p.x), CGFloat::from(p.y))
}
/// Même convention que `Affine` (vecteur ligne) : `a = m11`, `b = m12`, `c = m21`…
pub(super) fn affine(a: Affine) -> CGAffineTransform {
    CGAffineTransform {
        a: a.m11.into(),
        b: a.m12.into(),
        c: a.m21.into(),
        d: a.m22.into(),
        tx: a.dx.into(),
        ty: a.dy.into(),
    }
}
/// Espace sRGB : pixels, couleurs, dégradés et images y sont exprimés sans conversion.
pub(super) fn srgb() -> crate::Result<CFRetained<CGColorSpace>> {
    // SAFETY : constante Core Graphics, chaîne statique valide.
    required(
        CGColorSpace::with_name(Some(unsafe { kCGColorSpaceSRGB })),
        "CGColorSpaceCreateWithName",
    )
}
pub(super) fn components(color: Color) -> [CGFloat; 4] {
    [
        CGFloat::from(color.red) / 255.,
        CGFloat::from(color.green) / 255.,
        CGFloat::from(color.blue) / 255.,
        CGFloat::from(color.alpha) / 255.,
    ]
}
pub(super) fn cg_color(color: Color) -> CFRetained<CGColor> {
    let [r, g, b, a] = components(color);
    CGColor::new_srgb(r, g, b, a)
}
pub(super) fn cf_string(value: &str) -> CFRetained<CFString> {
    CFString::from_str(value)
}
pub(super) fn range(start: usize, length: usize) -> CFRange {
    CFRange {
        location: start as CFIndex,
        length: length as CFIndex,
    }
}
/// Index UTF-16 de chaque octet UTF-8 situé sur une frontière de caractère (et de la fin).
pub(super) fn utf16_map(text: &str) -> Vec<usize> {
    let mut map = vec![0usize; text.len() + 1];
    let mut unit = 0;
    for (byte, c) in text.char_indices() {
        map[byte] = unit;
        unit += c.len_utf16();
    }
    map[text.len()] = unit;
    map
}
/// Octet UTF-8 de chaque index UTF-16 (inverse de [`utf16_map`] ; les unités du milieu
/// d'une paire de substitution renvoient l'octet du caractère).
pub(super) fn byte_map(text: &str) -> Vec<usize> {
    let mut map = Vec::with_capacity(text.len() + 1);
    for (byte, c) in text.char_indices() {
        for _ in 0..c.len_utf16() {
            map.push(byte);
        }
    }
    map.push(text.len());
    map
}
