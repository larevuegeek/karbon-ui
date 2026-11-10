pub(super) use super::super::{
    events::caption,
    ffi::{Hwnd, client_area, last_error, scale},
    graphics::Renderer,
    imaging::DrawingEngine,
    window::{startup, state},
};
pub(super) use super::composition::Presenter;
use super::*;
pub(super) use crate::{
    application::{DynApplication, ViewContext},
    ui::{
        runtime::{ChromeState, UiRuntime},
        theme::Theme,
        title_bar::TITLE_BAR_HEIGHT,
    },
    window::RenderMode,
};
pub(super) use std::{
    cell::{Cell, RefCell},
    time::Instant,
};
pub(super) use windows_sys::Win32::UI::{HiDpi::GetDpiForSystem, WindowsAndMessaging::IsZoomed};

pub(crate) struct DrawingState {
    pub(super) renderer: Renderer,
    pub(super) runtime: UiRuntime,
    pub(super) presenter: Presenter,
    pub(super) mode: RenderMode,
    pub(super) profile: Option<recorder::Recorder>,
    /// Dernière image soumise : taille, échelle, alpha prémultiplié et numéro de peinture
    /// de la scène. Un `WM_PAINT` sans changement (`ShowWindow`/`UpdateWindow` juste
    /// après la première image, fenêtre découverte…) ne redessine pas : la chaîne
    /// d'échange ou la surface layered gardent leur contenu.
    pub(super) presented: Option<(u32, u32, u32, bool, u64)>,
    /// Moment de la préparation faite avant l'ouverture (voir [`load`]), jusqu'à la
    /// première peinture.
    pub(super) prepared: Option<Instant>,
}
pub(super) static DRAWING: state::WindowLocal<RefCell<Option<DrawingState>>> =
    state::WindowLocal::new(|state| &state.drawing);

/// `renderer` et `presenter` sont créés à l'ouverture de la fenêtre, qui choisit le
/// chemin de présentation avant de confier l'application.
/// Avant la fenêtre : polices, chargement de l'application puis première mise en page
/// à la taille prévue de la fenêtre. Tout cela n'a besoin que de DirectWrite et se fait
/// pendant que `renderer` crée son périphérique D3D11 sur un autre thread ; la première
/// peinture ne refait la mise en page que si la fenêtre n'a pas la taille prévue. Les
/// animations de cette image sont décalées à sa première présentation.
pub(crate) fn load(
    renderer: &mut Renderer,
    decoder: &DrawingEngine,
    application: Box<dyn DynApplication>,
) -> crate::Result<Prepared> {
    renderer.load_fonts(application.fonts())?;
    startup::mark("fonts registered");
    let mut runtime = UiRuntime::from_dyn(application, decoder)?;
    startup::mark("application loaded");
    let options = state::options();
    // Même géométrie que `window::create::create_window` (zone client = fenêtre, sans cadre).
    // SAFETY : simple lecture du DPI système.
    let scale = unsafe { GetDpiForSystem() } as f32 / 96.;
    let width = (options.width as f32 * scale).round().max(0.);
    let height = (options.height as f32 * scale).round().max(0.);
    let context = view_context(width, height, scale, renderer.label());
    runtime.set_reduced_motion(options.reduced_motion || system_reduced_motion());
    runtime.set_system_dark(system_dark());
    let at = Instant::now();
    runtime.prepare_at(context, chrome(false), renderer, at)?;
    startup::mark("first view built, styled and laid out (predicted size)");
    Ok(Prepared { runtime, at })
}
/// Application chargée et première image préparée avant l'ouverture de la fenêtre.
pub(crate) struct Prepared {
    pub(super) runtime: UiRuntime,
    pub(super) at: Instant,
}
/// Contexte de vue d'une zone client de `width` × `height` pixels physiques.
pub(super) fn view_context(
    width: f32,
    height: f32,
    scale: f32,
    backend: &'static str,
) -> ViewContext {
    let options = state::options();
    ViewContext {
        width: width / scale,
        height: (height / scale
            - if options.title_bar {
                TITLE_BAR_HEIGHT as f32
            } else {
                0.
            })
        .max(0.),
        theme: if options.light {
            Theme::LIGHT
        } else {
            Theme::DARK
        },
        transparent: options.transparent,
        backend,
        scale,
    }
}
pub(super) fn chrome(maximized: bool) -> ChromeState {
    ChromeState {
        visible: state::options().title_bar,
        hovered: caption::HOVERED_BUTTON.with(Cell::get),
        pressed: caption::PRESSED_BUTTON.with(Cell::get),
        maximized,
    }
}
pub(crate) fn initialize(
    mode: RenderMode,
    (renderer, presenter): (Renderer, Presenter),
    Prepared { runtime, at }: Prepared,
    id: crate::window::WindowId,
) -> crate::Result<()> {
    println!("karbon-ui: renderer: {}", renderer.label());
    println!("karbon-ui: presentation: {}", presenter.path().label());
    let profile = state::options().profile.then(|| {
        let mut profile = recorder::Recorder::new(id, presenter.path());
        // La chaîne d'échange a été créée à l'ouverture pour valider la composition.
        profile.surface_creations = u64::from(presenter.path() == composition::Path::Composition);
        profile
    });
    DRAWING.with(|slot| {
        *slot.borrow_mut() = Some(DrawingState {
            renderer,
            runtime,
            presenter,
            mode,
            profile,
            presented: None,
            prepared: Some(at),
        })
    });
    Ok(())
}
pub(crate) fn switch_renderer() -> crate::Result<()> {
    DRAWING.with(|slot| {
        let mut value = slot.borrow_mut();
        if let Some(state) = value.as_mut() {
            let mode = if state.renderer.is_software() {
                RenderMode::Gpu
            } else {
                RenderMode::Software
            };
            let mut renderer = Renderer::new(mode)?;
            renderer.load_fonts(state.runtime.font_assets())?;
            if let Some(profile) = state.profile.as_mut() {
                profile.emit(state.renderer.label(), "backend-switch");
                // Ne pas mélanger GPU et WARP dans les mêmes percentiles.
                *profile = profile.restart();
            }
            println!("karbon-ui: renderer: {}", renderer.label());
            state.renderer = renderer;
            state.mode = mode;
            state.runtime.invalidate();
        }
        Ok(())
    })
}
impl DrawingState {
    pub(super) fn prepare(&mut self, window: Hwnd) -> crate::Result<(u32, u32, f32)> {
        let Some(area) = client_area(window) else {
            return Err(last_error("GetClientRect"));
        };
        let scale = scale(window);
        let context = view_context(
            area.right as f32,
            area.bottom as f32,
            scale,
            self.renderer.label(),
        );
        self.runtime
            .set_reduced_motion(state::options().reduced_motion || system_reduced_motion());
        self.runtime.set_system_dark(system_dark());
        // SAFETY : simple lecture de l'état de la fenêtre.
        let chrome = chrome(unsafe { IsZoomed(window) } != 0);
        self.runtime.prepare(context, chrome, &mut self.renderer)?;
        Ok((area.right.max(0) as u32, area.bottom.max(0) as u32, scale))
    }
}
thread_local! {
    /// Dernière lecture du réglage système de mouvement réduit sur ce thread ; `None` :
    /// à relire. `SystemParametersInfoW` passe par win32k et peut prendre plus d'une
    /// milliseconde quand d'autres threads créent des fenêtres : il n'est pas appelé à
    /// chaque image.
    pub(crate) static SYSTEM_REDUCED_MOTION: Cell<Option<bool>> = const { Cell::new(None) };
    /// Dernière lecture du thème du système ; `None` : à relire.
    pub(super) static SYSTEM_DARK: Cell<Option<bool>> = const { Cell::new(None) };
}
/// Thème sombre des applications dans les réglages de Windows (lu une fois, puis après
/// chaque `WM_SETTINGCHANGE`).
pub(crate) fn system_dark() -> bool {
    SYSTEM_DARK.with(|cached| {
        cached.get().unwrap_or_else(|| {
            let value = read_system_dark();
            cached.set(Some(value));
            value
        })
    })
}
pub(super) fn read_system_dark() -> bool {
    use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
    let wide = |text: &str| text.encode_utf16().chain([0]).collect::<Vec<u16>>();
    let key = wide(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
    let value = wide("AppsUseLightTheme");
    let (mut light, mut size) = (1u32, 4u32);
    // SAFETY : chaînes terminées par un nul, `light` et `size` valides pendant l'appel.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            (&raw mut light).cast(),
            &raw mut size,
        )
    };
    status == 0 && light == 0
}
/// Réglage système « Afficher les animations dans Windows » désactivé (lu une fois, puis
/// après chaque `WM_SETTINGCHANGE`).
pub(crate) fn system_reduced_motion() -> bool {
    SYSTEM_REDUCED_MOTION.with(|cached| {
        cached.get().unwrap_or_else(|| {
            let value = read_system_reduced_motion();
            cached.set(Some(value));
            value
        })
    })
}
/// `WM_SETTINGCHANGE` : le réglage est relu à la prochaine image, demandée ici (une
/// animation en cours se termine si le mouvement réduit vient d'être activé).
pub(crate) fn settings_changed(window: Hwnd) {
    SYSTEM_REDUCED_MOTION.with(|cached| cached.set(None));
    SYSTEM_DARK.with(|cached| cached.set(None));
    super::super::ffi::invalidate(window);
}
pub(super) fn read_system_reduced_motion() -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SPI_GETCLIENTAREAANIMATION, SystemParametersInfoW,
    };
    let mut enabled = 1i32;
    // SAFETY : Windows écrit un BOOL dans `enabled`, valide pendant l'appel.
    let ok = unsafe {
        SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION, 0, (&raw mut enabled).cast(), 0)
    };
    ok != 0 && enabled == 0
}
