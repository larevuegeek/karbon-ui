use super::*;
pub(super) use crate::window::RenderMode;

#[test]
#[ignore = "nécessite un adaptateur GPU matériel ; lancer explicitement avec --include-ignored"]
fn gpu_renders_alpha_clip_image_path_text_and_dpi() {
    paint::verify(RenderMode::Gpu);
    effects::verify_effects(RenderMode::Gpu);
    gradients::verify_gradients(RenderMode::Gpu);
    text::verify_typography(RenderMode::Gpu);
    shapes::verify_shapes(RenderMode::Gpu);
    svg::verify_svg(RenderMode::Gpu);
    super::super::filters::tests::verify_filters(RenderMode::Gpu);
    swapchain::verify_swap_chain(RenderMode::Gpu);
}
