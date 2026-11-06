//! Chaîne d'effets natifs Direct2D (ID2D1Effect), identifiants CLSID du SDK.
use super::*;
use windows::core::GUID;
fn create(context: &ID2D1DeviceContext, id: &GUID) -> crate::Result<ID2D1Effect> {
    // SAFETY: contexte Direct2D valide ; CLSID d'effet intégré lu pendant l'appel.
    unsafe { context.CreateEffect(id) }.check("CreateEffect")
}
/// Écrit une propriété d'effet à partir de ses octets natifs (ordre de la machine).
fn property(
    effect: &ID2D1Effect,
    index: u32,
    kind: D2D1_PROPERTY_TYPE,
    data: &[u8],
) -> crate::Result<()> {
    // SAFETY: effet valide ; la taille des données correspond au type déclaré.
    unsafe { effect.SetValue(index, kind, data) }.check("Set filter property")
}
fn floats(values: &[f32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_ne_bytes()).collect()
}
fn affine_bytes(a: Affine) -> Vec<u8> {
    floats(&[a.m11, a.m12, a.m21, a.m22, a.dx, a.dy])
}
fn input(effect: &ID2D1Effect, index: u32, image: &ID2D1Image) {
    // SAFETY: effet et image valides ; l'effet conserve sa propre référence.
    unsafe { effect.SetInput(index, image, true) };
}
fn output(effect: &ID2D1Effect) -> crate::Result<ID2D1Image> {
    // SAFETY: effet valide ; la sortie est une référence possédée.
    unsafe { effect.GetOutput() }.check("GetOutput")
}
pub(super) fn chain(
    context: &ID2D1DeviceContext,
    mut image: ID2D1Image,
    filters: Filters,
    crop: Option<RectF>,
) -> crate::Result<ID2D1Image> {
    let index = |value: i32| value as u32;
    let mut last = None;
    for f in filters.values {
        let effect = match *f {
            Filter::Blur(n) => {
                let e = create(context, &CLSID_D2D1GaussianBlur)?;
                property(
                    &e,
                    index(D2D1_GAUSSIANBLUR_PROP_STANDARD_DEVIATION.0),
                    D2D1_PROPERTY_TYPE_FLOAT,
                    &(n * filters.scale).to_ne_bytes(),
                )?;
                input(&e, 0, &image);
                e
            }
            Filter::DropShadow { x, y, blur, color } => {
                let shadow = create(context, &CLSID_D2D1Shadow)?;
                property(
                    &shadow,
                    index(D2D1_SHADOW_PROP_BLUR_STANDARD_DEVIATION.0),
                    D2D1_PROPERTY_TYPE_FLOAT,
                    &(blur * filters.scale).to_ne_bytes(),
                )?;
                let color = color_f(color);
                property(
                    &shadow,
                    index(D2D1_SHADOW_PROP_COLOR.0),
                    D2D1_PROPERTY_TYPE_VECTOR4,
                    &floats(&[color.r, color.g, color.b, color.a]),
                )?;
                input(&shadow, 0, &image);
                let shift = create(context, &CLSID_D2D12DAffineTransform)?;
                property(
                    &shift,
                    index(D2D1_2DAFFINETRANSFORM_PROP_TRANSFORM_MATRIX.0),
                    D2D1_PROPERTY_TYPE_MATRIX_3X2,
                    &affine_bytes(Affine::translation(x * filters.scale, y * filters.scale)),
                )?;
                input(&shift, 0, &output(&shadow)?);
                let composite = create(context, &CLSID_D2D1Composite)?;
                // SAFETY: effet composite valide ; deux entrées renseignées ci-dessous.
                unsafe { composite.SetInputCount(2) }.check("Set composite inputs")?;
                input(&composite, 0, &output(&shift)?);
                input(&composite, 1, &image);
                composite
            }
            value => {
                let e = create(context, &CLSID_D2D1ColorMatrix)?;
                property(
                    &e,
                    index(D2D1_COLORMATRIX_PROP_COLOR_MATRIX.0),
                    D2D1_PROPERTY_TYPE_MATRIX_5X4,
                    &floats(&value.matrix().unwrap()),
                )?;
                // D2D1_COLORMATRIX_ALPHA_MODE_PREMULTIPLIED = 1 (constante absente des métadonnées).
                property(
                    &e,
                    index(D2D1_COLORMATRIX_PROP_ALPHA_MODE.0),
                    D2D1_PROPERTY_TYPE_ENUM,
                    &1u32.to_ne_bytes(),
                )?;
                property(
                    &e,
                    index(D2D1_COLORMATRIX_PROP_CLAMP_OUTPUT.0),
                    D2D1_PROPERTY_TYPE_BOOL,
                    &1u32.to_ne_bytes(),
                )?;
                input(&e, 0, &image);
                e
            }
        };
        image = output(&effect)?;
        last = Some(effect);
    }
    if let Some(rect) = crop {
        // Rectangle visible de la sortie, en DIP de l'espace du groupe (gauche, haut,
        // droite, bas).
        let effect = create(context, &CLSID_D2D1Crop)?;
        property(
            &effect,
            index(D2D1_CROP_PROP_RECT.0),
            D2D1_PROPERTY_TYPE_VECTOR4,
            &floats(&[rect.left, rect.top, rect.right, rect.bottom]),
        )?;
        input(&effect, 0, &image);
        image = output(&effect)?;
        last = Some(effect);
    }
    // Only the final stage keeps its GPU/WARP output while the recorded input is
    // unchanged: the intermediate stages are only read to compute it, and caching each
    // of them kept one more full-size texture per stage.
    if let Some(effect) = last {
        property(
            &effect,
            index(D2D1_PROPERTY_CACHED.0),
            D2D1_PROPERTY_TYPE_BOOL,
            &1u32.to_ne_bytes(),
        )?;
    }
    Ok(image)
}
