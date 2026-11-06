//! Pile native : restauration des clips, calques d'opacité et matrices, même après erreur.
use super::*;
use std::mem::ManuallyDrop;
pub(super) enum Scope {
    Clip,
    Mask,
    Effect { previous: Affine, layer: bool },
}
/// Paramètres de calque D2D1_LAYER_PARAMETERS (ID2D1RenderTarget::PushLayer).
fn layer(bounds: RectF, mask: Option<ID2D1Geometry>, opacity: f32) -> D2D1_LAYER_PARAMETERS {
    D2D1_LAYER_PARAMETERS {
        contentBounds: rect_f(bounds),
        geometricMask: ManuallyDrop::new(mask),
        maskAntialiasMode: D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
        maskTransform: matrix(Affine::IDENTITY),
        opacity,
        opacityBrush: ManuallyDrop::new(None),
        layerOptions: D2D1_LAYER_OPTIONS_NONE,
    }
}
fn push_layer(target: &ID2D1RenderTarget, params: D2D1_LAYER_PARAMETERS) {
    // SAFETY: passe de dessin ouverte ; Direct2D prend sa propre référence sur le masque.
    unsafe { target.PushLayer(&params, None) };
    // Libère la référence prêtée au calque pendant l'appel.
    drop(ManuallyDrop::into_inner(params.geometricMask));
}
pub(super) fn push(
    target: &ID2D1RenderTarget,
    stack: &mut Vec<Scope>,
    transform: Affine,
    opacity: f32,
    bounds: RectF,
) {
    let mut native = matrix(Affine::IDENTITY);
    // SAFETY: passe de dessin ouverte ; matrices lues ou écrites pendant l'appel seulement.
    let previous = unsafe {
        target.GetTransform(&mut native);
        let previous = affine(native);
        target.SetTransform(&matrix(transform.then(previous)));
        previous
    };
    let layer_pushed = opacity < 1.;
    if layer_pushed {
        push_layer(target, layer(bounds, None, opacity));
    }
    stack.push(Scope::Effect {
        previous,
        layer: layer_pushed,
    });
}
pub(super) fn mask(
    target: &ID2D1RenderTarget,
    stack: &mut Vec<Scope>,
    bounds: RectF,
    geometry: &ID2D1PathGeometry,
) {
    push_layer(target, layer(bounds, Some(geometry.clone().into()), 1.));
    stack.push(Scope::Mask);
}
/// Calque d'opacité (`mask-image`) : l'alpha du pinceau multiplie le contenu.
pub(super) fn opacity_mask(
    target: &ID2D1RenderTarget,
    stack: &mut Vec<Scope>,
    bounds: RectF,
    brush: &ID2D1Brush,
) {
    let mut params = layer(bounds, None, 1.);
    params.opacityBrush = ManuallyDrop::new(Some(brush.clone()));
    // SAFETY: passe de dessin ouverte ; Direct2D prend sa propre référence sur le pinceau.
    unsafe { target.PushLayer(&params, None) };
    drop(ManuallyDrop::into_inner(params.opacityBrush));
    drop(ManuallyDrop::into_inner(params.geometricMask));
    stack.push(Scope::Mask);
}
pub(super) fn pop(target: &ID2D1RenderTarget, scope: Scope) {
    // SAFETY: chaque portée correspond à un Push* effectué pendant la même passe de dessin.
    unsafe {
        match scope {
            Scope::Mask => target.PopLayer(),
            Scope::Clip => target.PopAxisAlignedClip(),
            Scope::Effect { previous, layer } => {
                if layer {
                    target.PopLayer();
                }
                target.SetTransform(&matrix(previous));
            }
        }
    }
}
