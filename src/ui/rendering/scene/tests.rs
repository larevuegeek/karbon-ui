use super::*;
#[test]
fn effect_scopes_are_ordered_validated_and_conjugated_when_a_scene_is_moved() {
    use crate::ui::rendering::transform::Affine;
    let mut scene = Scene::new(Color::rgba(0, 0, 0, 0));
    let transform = Affine {
        m11: 2.,
        m22: 2.,
        ..Affine::IDENTITY
    };
    scene.commands.push(DrawCommand::PushEffect {
        transform,
        opacity: 0.5,
        bounds: RectF::new(0., 0., 20., 20.),
    });
    scene
        .commands
        .push(DrawCommand::PushClip(RectF::new(0., 0., 20., 20.)));
    scene.commands.push(DrawCommand::PopEffect);
    assert!(scene.validate().is_err());
    scene.commands.pop();
    scene.commands.push(DrawCommand::PopClip);
    scene.commands.push(DrawCommand::PopEffect);
    assert!(scene.validate().is_ok());
    let mut translated = Scene::new(Color::rgba(0, 0, 0, 0));
    translated.append_translated(scene, 10., 20.);
    if let DrawCommand::PushEffect {
        transform, bounds, ..
    } = translated.commands[0]
    {
        assert_eq!(transform.dx, -10.);
        assert_eq!(transform.dy, -20.);
        assert_eq!(bounds.left, 10.);
    } else {
        panic!("effet absent");
    }
}
#[test]
fn clips_must_be_balanced_and_nested() {
    let mut scene = Scene::new(Color::rgba(0, 0, 0, 0));
    scene.commands.push(DrawCommand::PopClip);
    assert!(scene.validate().is_err());
    scene
        .commands
        .insert(0, DrawCommand::PushClip(RectF::new(0., 0., 10., 10.)));
    assert!(scene.validate().is_ok());
    scene.commands.pop();
    assert!(scene.validate().is_err());
}
#[test]
fn rejects_invalid_image_size_and_nonpremultiplied_pixels() {
    assert!(Image::new(1, 1, vec![255, 0, 0, 128]).is_err());
    assert!(Image::new(2, 1, vec![0; 4]).is_err());
    assert!(Image::new(u32::MAX, u32::MAX, vec![]).is_err());
    assert!(Image::new(1, 1, vec![64, 32, 0, 128]).is_ok());
}
