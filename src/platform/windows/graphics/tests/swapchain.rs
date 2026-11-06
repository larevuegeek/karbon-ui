//! Chaîne d'échange de composition : rendu direct, redimensionnement, mode alpha,
//! perte du périphérique et parité exacte avec le rendu hors écran relu.
use super::super::readback::copy_to_cpu;
use super::super::*;

fn scene() -> Scene {
    let mut scene = Scene::new(Color::rgba(0, 0, 0, 0));
    scene.fill(
        RectF::new(4., 4., 40., 30.),
        8.,
        Color::rgba(200, 100, 50, 128),
    );
    scene.fill(RectF::new(30., 20., 30., 20.), 0., Color::rgb(0, 120, 255));
    scene.text(
        RectF::new(2., 40., 60., 20.),
        "Été",
        12.,
        true,
        Color::rgb(255, 255, 255),
    );
    scene
}

/// Relit le back buffer courant (avant Present) via une texture de relecture dédiée.
fn back_buffer(renderer: &Renderer) -> Vec<u8> {
    let target = renderer.target.as_ref().unwrap();
    let Output::SwapChain { chain, .. } = &target.output else {
        panic!("la cible doit être une chaîne d'échange");
    };
    // SAFETY: chaîne valide ; l'IID provient du type demandé.
    let texture: ID3D11Texture2D = unsafe { chain.GetBuffer(0) }.unwrap();
    let mut desc = D3D11_TEXTURE2D_DESC::default();
    // SAFETY: texture valide ; description copiée dans une structure locale.
    unsafe { texture.GetDesc(&mut desc) };
    desc.Usage = D3D11_USAGE_STAGING;
    desc.BindFlags = 0;
    desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
    desc.MiscFlags = 0;
    let mut staging = None;
    // SAFETY: description valide, sans données initiales ; sortie optionnelle valide.
    unsafe {
        renderer
            .gpu
            .gpu()
            .device
            .CreateTexture2D(&desc, None, Some(&mut staging))
    }
    .unwrap();
    let mut pixels = vec![0; target.width as usize * target.height as usize * 4];
    copy_to_cpu(
        &renderer.gpu.gpu().context,
        &texture,
        &staging.unwrap(),
        target.width,
        target.height,
        &mut pixels,
    )
    .unwrap();
    pixels
}

pub(super) fn verify_swap_chain(mode: RenderMode) {
    let mut renderer = Renderer::new(mode).unwrap();
    let scene = scene();
    let expected = renderer.render(&scene, 96, 64, 1.5).unwrap();
    assert!(renderer.swap_chain().is_none());

    assert_eq!(
        renderer.prepare_swap_chain(96, 64, 1.5, true).unwrap(),
        SurfaceChange::Created
    );
    assert_eq!(
        renderer.prepare_swap_chain(96, 64, 1.5, true).unwrap(),
        SurfaceChange::Unchanged
    );
    renderer
        .render_to_swap_chain(&scene, 96, 64, 1.5, true)
        .unwrap();
    assert_eq!(
        back_buffer(&renderer),
        expected,
        "Le back buffer doit contenir exactement les pixels du rendu relu"
    );
    renderer.present().unwrap();
    let first = renderer.swap_chain().unwrap().clone();
    assert_eq!(swapchain::describe(&first), (96, 64, true));

    // Redimensionnement : même chaîne, tampons et cible Direct2D recréés.
    assert_eq!(
        renderer.prepare_swap_chain(120, 80, 2., true).unwrap(),
        SurfaceChange::Resized
    );
    assert_eq!(renderer.swap_chain(), Some(&first));
    assert_eq!(swapchain::describe(&first), (120, 80, true));
    renderer
        .render_to_swap_chain(&scene, 120, 80, 2., true)
        .unwrap();
    assert_eq!(
        back_buffer(&renderer),
        renderer.render(&scene, 120, 80, 2.).unwrap()
    );
    // Le rendu hors écran a remplacé la cible : la chaîne est recréée ensuite.
    assert!(renderer.swap_chain().is_none());
    assert_eq!(
        renderer.prepare_swap_chain(120, 80, 2., true).unwrap(),
        SurfaceChange::Created
    );
    renderer
        .render_to_swap_chain(&scene, 120, 80, 2., true)
        .unwrap();
    renderer.present().unwrap();

    // Changement de mode alpha : nouvelle chaîne, opaque.
    let premultiplied = renderer.swap_chain().unwrap().clone();
    assert_eq!(
        renderer.prepare_swap_chain(120, 80, 2., false).unwrap(),
        SurfaceChange::Created
    );
    assert_ne!(renderer.swap_chain(), Some(&premultiplied));
    assert_eq!(
        swapchain::describe(renderer.swap_chain().unwrap()),
        (120, 80, false)
    );
    renderer
        .render_to_swap_chain(&scene, 120, 80, 2., false)
        .unwrap();
    renderer.present().unwrap();

    // L'export hors écran reste identique après la présentation.
    assert_eq!(renderer.render(&scene, 96, 64, 1.5).unwrap(), expected);

    // Perte du périphérique : erreur typée, distincte des autres échecs.
    renderer.prepare_swap_chain(96, 64, 1.5, true).unwrap();
    renderer
        .render_to_swap_chain(&scene, 96, 64, 1.5, true)
        .unwrap();
    renderer.simulate_device_loss();
    assert!(is_device_lost(&renderer.present().unwrap_err()));
    renderer.present().unwrap();
    let removed: windows::core::Result<()> = Err(DXGI_ERROR_DEVICE_REMOVED.into());
    assert!(is_device_lost(&removed.check("Present").unwrap_err()));
    let invalid: windows::core::Result<()> = Err(windows::Win32::Foundation::E_INVALIDARG.into());
    assert!(!is_device_lost(&invalid.check("Present").unwrap_err()));
    assert!(renderer.prepare_swap_chain(0, 64, 1., true).is_err());
}

#[test]
fn software_swap_chain_matches_readback_resizes_and_switches_alpha_mode() {
    verify_swap_chain(RenderMode::Software);
}
