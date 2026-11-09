//! Thème du kit : palette claire et sombre, rayons, variables CSS fournies aux composants.
use crate::ui::styling::{CssVariables, VariableValue, color_scheme};
use crate::ui::theme::Color;
use std::{
    cell::{Cell, RefCell},
    sync::Arc,
};

/// Colours of one colour scheme.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    /// Page background.
    pub background: Color,
    /// Cards, menus and dialogs.
    pub surface: Color,
    /// Quiet fills: table headers, neutral buttons.
    pub subtle: Color,
    /// Borders and separators.
    pub border: Color,
    /// Main text.
    pub text: Color,
    /// Secondary text.
    pub muted: Color,
    /// Brand colour.
    pub primary: Color,
    /// Second colour of gradients and light effects.
    pub accent: Color,
    /// Positive state.
    pub success: Color,
    /// Attention state.
    pub warning: Color,
    /// Destructive state.
    pub danger: Color,
    /// Informative state.
    pub info: Color,
}
impl Palette {
    /// Default light palette.
    pub const LIGHT: Self = Self {
        background: Color::hex(0xffffff),
        surface: Color::hex(0xffffff),
        subtle: Color::hex(0xf4f4f5),
        border: Color::hex(0xe4e4e7),
        text: Color::hex(0x11181c),
        muted: Color::hex(0x71717a),
        primary: Color::hex(0x006fee),
        accent: Color::hex(0x7828c8),
        success: Color::hex(0x17c964),
        warning: Color::hex(0xf5a524),
        danger: Color::hex(0xf31260),
        info: Color::hex(0x0ea5e9),
    };
    /// Default dark palette.
    pub const DARK: Self = Self {
        background: Color::hex(0x000000),
        surface: Color::hex(0x18181b),
        subtle: Color::hex(0x27272a),
        border: Color::hex(0x3f3f46),
        text: Color::hex(0xecedee),
        muted: Color::hex(0xa1a1aa),
        primary: Color::hex(0x006fee),
        accent: Color::hex(0x9353d3),
        success: Color::hex(0x17c964),
        warning: Color::hex(0xf5a524),
        danger: Color::hex(0xf31260),
        info: Color::hex(0x0ea5e9),
    };
}

/// Appearance of every kit component, set with [`set_theme`].
///
/// ```
/// use karbon_ui::ui::{kit::{KitTheme, set_theme}, theme::Color};
/// set_theme(KitTheme::default().primary(Color::hex(0xe11d48)).radius(12.));
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KitTheme {
    /// Colours of the light scheme.
    pub light: Palette,
    /// Colours of the dark scheme.
    pub dark: Palette,
    /// Corner radius of controls, in logical pixels.
    pub radius: f32,
}
impl Default for KitTheme {
    fn default() -> Self {
        DEFAULT
    }
}
const DEFAULT: KitTheme = KitTheme {
    light: Palette::LIGHT,
    dark: Palette::DARK,
    radius: 12.,
};
impl KitTheme {
    /// Sets the brand colour of both schemes.
    pub fn primary(mut self, color: Color) -> Self {
        self.light.primary = color;
        self.dark.primary = color;
        self
    }
    /// Sets the second colour of gradients and light effects, in both schemes.
    pub fn accent(mut self, color: Color) -> Self {
        self.light.accent = color;
        self.dark.accent = color;
        self
    }
    /// Sets the corner radius of controls; small and large radii follow it.
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius.max(0.);
        self
    }
}

thread_local! {
    static THEME: Cell<KitTheme> = const { Cell::new(DEFAULT) };
    static REVISION: Cell<u64> = const { Cell::new(0) };
    /// Variables du thème déjà calculées : (révision, sombre, variables).
    static BUILT: RefCell<Option<(u64, bool, CssVariables)>> = const { RefCell::new(None) };
    /// Dernière fusion avec les variables de l'application : (révision, sombre, table de
    /// l'application, fusion). La table est retenue : son adresse reste une clé sûre.
    static MERGED: RefCell<Option<(u64, bool, CssVariables, CssVariables)>> =
        const { RefCell::new(None) };
}

/// Replaces the theme of the kit. Open windows are redrawn with it on their next frame.
pub fn set_theme(theme: KitTheme) {
    THEME.with(|t| t.set(theme));
    REVISION.with(|r| r.set(r.get() + 1));
}
/// The current theme of the kit.
pub fn theme() -> KitTheme {
    THEME.with(Cell::get)
}
/// Compteur des changements de thème : le runtime refait la vue quand il bouge.
pub(crate) fn revision() -> u64 {
    REVISION.with(Cell::get)
}
thread_local! {
    /// Un composant du kit a été construit depuis la dernière lecture.
    static USED: Cell<bool> = const { Cell::new(false) };
}
/// `true` si un composant du kit a été construit depuis le dernier appel.
pub(crate) fn take_used() -> bool {
    USED.with(|used| used.replace(false))
}
/// Thème du moteur tiré du kit : les parties dessinées par les contrôles natifs (cases,
/// pistes, listes ouvertes) suivent ainsi les mêmes couleurs.
pub(crate) fn engine_theme(dark: bool) -> crate::ui::theme::Theme {
    let theme = theme();
    let p = if dark { theme.dark } else { theme.light };
    crate::ui::theme::Theme {
        background: p.background,
        surface: p.surface,
        title_bar: p.subtle,
        text: p.text,
        muted: p.muted,
        accent: p.primary,
        on_accent: super::tone::on(p.primary),
        border: p.border,
        hover: p.subtle,
        pressed: p.border,
        danger: p.danger,
    }
}
/// Palette du thème actif (clair ou sombre).
pub(super) fn palette() -> Palette {
    let theme = theme();
    if color_scheme().0 {
        theme.dark
    } else {
        theme.light
    }
}
fn built() -> CssVariables {
    let dark = color_scheme().0;
    let revision = revision();
    BUILT.with(|slot| {
        let mut slot = slot.borrow_mut();
        if let Some((r, d, variables)) = slot.as_ref()
            && *r == revision
            && *d == dark
        {
            return variables.clone();
        }
        let theme = theme();
        let p = if dark { theme.dark } else { theme.light };
        let mut v = CssVariables::default();
        for (name, color) in [
            ("--k-bg", p.background),
            ("--k-surface", p.surface),
            ("--k-subtle", p.subtle),
            ("--k-border", p.border),
            ("--k-text", p.text),
            ("--k-muted", p.muted),
            ("--k-primary", p.primary),
            ("--k-accent", p.accent),
            ("--k-success", p.success),
            ("--k-warning", p.warning),
            ("--k-danger", p.danger),
            ("--k-info", p.info),
        ] {
            v.set_static(name, VariableValue::Color(color));
        }
        let length = |px: f32| VariableValue::Length(px.into());
        v.set_static("--k-radius", length(theme.radius));
        v.set_static("--k-radius-sm", length((theme.radius * 0.67).round()));
        v.set_static("--k-radius-lg", length((theme.radius * 1.17).round()));
        *slot = Some((revision, dark, v.clone()));
        v
    })
}
/// CSS variables of the current theme (`--k-primary`, `--k-surface`…), for an application's
/// own stylesheets: `TemplateContext::default().with_variables(&kit::variables())`. Call it
/// while building the view: the values follow the light or dark colour scheme.
pub fn variables() -> CssVariables {
    built()
}
/// Variables à donner à un composant : celles du thème, sous celles de l'application.
pub(super) fn scope(application: Option<&CssVariables>) -> CssVariables {
    USED.with(|used| used.set(true));
    let kit = built();
    let Some(application) = application else {
        return kit;
    };
    if Arc::ptr_eq(&application.values, &kit.values) {
        return kit;
    }
    let (revision, dark) = (revision(), color_scheme().0);
    MERGED.with(|slot| {
        let mut slot = slot.borrow_mut();
        if let Some((r, d, source, merged)) = slot.as_ref()
            && (*r, *d) == (revision, dark)
            && (Arc::ptr_eq(&source.values, &application.values)
                || Arc::ptr_eq(&merged.values, &application.values))
        {
            return merged.clone();
        }
        let mut merged = kit.clone();
        for (name, value) in application.values.iter() {
            merged.set_named(name.clone(), value.clone());
        }
        *slot = Some((revision, dark, application.clone(), merged.clone()));
        merged
    })
}
