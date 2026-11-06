//! Lecture des calques de la fenêtre : chemin Metal ou image Core Graphics.
use objc2_app_kit::NSWindow;
use objc2_quartz_core::CAMetalLayer;

/// État du calque Metal de la fenêtre : `Some(true)` s'il est visible, à la taille en
/// pixels de la vue, sans image Core Graphics sur le calque de la vue ; `Some(false)`
/// s'il est masqué et que la vue montre une image Core Graphics ; `None` sans calque
/// Metal (ou dans un état mêlé).
pub(crate) fn metal_shown(window: &NSWindow) -> Option<bool> {
    let view = window.contentView()?;
    let layer = view.layer()?;
    let scale = window.backingScaleFactor();
    let bounds = view.bounds();
    // SAFETY : lecture des sous-calques, sans modification pendant l'appel.
    let sublayers = unsafe { layer.sublayers() }?;
    let metal = sublayers
        .to_vec()
        .into_iter()
        .find_map(|l| l.downcast::<CAMetalLayer>().ok())?;
    let size = metal.drawableSize();
    // SAFETY : lecture du contenu du calque.
    let contents = unsafe { layer.contents() }.is_some();
    let sized = (size.width - bounds.size.width * scale).abs() < 1.
        && (size.height - bounds.size.height * scale).abs() < 1.;
    match (metal.isHidden(), contents) {
        (false, false) if sized => Some(true),
        (true, true) => Some(false),
        _ => None,
    }
}
