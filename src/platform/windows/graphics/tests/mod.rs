//! Suite minimale du renderer Direct2D / DirectWrite : scène → pixels sur WARP (DPI 1x et
//! mis à l'échelle), clips et alpha, chaîne d'échange, mesure du texte ; le test GPU
//! rejoue les mêmes vérifications sur le matériel.
use crate::window::RenderMode;
mod effects;
mod gradients;
mod paint;
mod shapes;
mod svg;
mod swapchain;
mod text;

#[test]
#[ignore = "nécessite un adaptateur GPU matériel ; lancer explicitement avec --include-ignored"]
fn gpu_renders_alpha_clip_image_path_text_and_dpi() {
    paint::verify(RenderMode::Gpu);
    effects::verify_effects(RenderMode::Gpu);
    gradients::verify_gradients(RenderMode::Gpu);
    text::verify_typography(RenderMode::Gpu);
    shapes::verify_shapes(RenderMode::Gpu);
    svg::verify_svg(RenderMode::Gpu);
    super::filters::tests::verify_filters(RenderMode::Gpu);
    swapchain::verify_swap_chain(RenderMode::Gpu);
}
