//! Pango (mise en page et mise en forme par HarfBuzz) et son pont Cairo : carte des
//! polices fontconfig, contexte, mises en page, descriptions de polices et attributs.
//! Les structures publiques de Pango lues par le backend sont transcrites dans
//! `types.rs`.
mod types;

use super::{
    cairo,
    fontconfig::RawConfig,
    glib::{Gboolean, Object},
    required,
};
use std::{
    ffi::{c_char, c_int, c_void},
    ptr::NonNull,
};
pub(in crate::platform::linux) use types::*;

#[link(name = "pango-1.0")]
unsafe extern "C" {
    fn pango_font_map_create_context(map: *mut c_void) -> *mut c_void;
    fn pango_context_set_round_glyph_positions(context: *mut c_void, round: Gboolean);
    fn pango_context_get_metrics(
        context: *mut c_void,
        desc: *const c_void,
        language: *mut c_void,
    ) -> *mut c_void;
    fn pango_context_load_font(context: *mut c_void, desc: *const c_void) -> *mut c_void;
    fn pango_font_describe(font: *mut c_void) -> *mut c_void;
    fn pango_font_metrics_get_ascent(metrics: *mut c_void) -> c_int;
    fn pango_font_metrics_get_descent(metrics: *mut c_void) -> c_int;
    fn pango_font_metrics_get_height(metrics: *mut c_void) -> c_int;
    fn pango_font_metrics_get_underline_position(metrics: *mut c_void) -> c_int;
    fn pango_font_metrics_get_underline_thickness(metrics: *mut c_void) -> c_int;
    fn pango_font_metrics_get_strikethrough_position(metrics: *mut c_void) -> c_int;
    fn pango_font_metrics_get_strikethrough_thickness(metrics: *mut c_void) -> c_int;
    fn pango_font_metrics_unref(metrics: *mut c_void);
    fn pango_layout_new(context: *mut c_void) -> *mut c_void;
    fn pango_layout_set_text(layout: *mut c_void, text: *const c_char, length: c_int);
    fn pango_layout_set_attributes(layout: *mut c_void, attrs: *mut c_void);
    fn pango_layout_set_font_description(layout: *mut c_void, desc: *const c_void);
    fn pango_layout_set_width(layout: *mut c_void, width: c_int);
    fn pango_layout_set_wrap(layout: *mut c_void, wrap: c_int);
    fn pango_layout_set_ellipsize(layout: *mut c_void, mode: c_int);
    fn pango_layout_set_justify(layout: *mut c_void, justify: Gboolean);
    fn pango_layout_get_line_count(layout: *mut c_void) -> c_int;
    fn pango_layout_get_line_readonly(layout: *mut c_void, line: c_int) -> *mut LayoutLine;
    fn pango_layout_get_log_attrs_readonly(
        layout: *mut c_void,
        n_attrs: *mut c_int,
    ) -> *const LogAttr;
    fn pango_layout_line_get_extents(
        line: *mut LayoutLine,
        ink: *mut Rectangle,
        logical: *mut Rectangle,
    );
    fn pango_layout_line_index_to_x(
        line: *mut LayoutLine,
        index: c_int,
        trailing: Gboolean,
        x_pos: *mut c_int,
    );
    pub(in crate::platform::linux) fn pango_glyph_string_get_width(
        glyphs: *mut GlyphString,
    ) -> c_int;
    pub(in crate::platform::linux) fn pango_glyph_string_new() -> *mut GlyphString;
    pub(in crate::platform::linux) fn pango_glyph_string_set_size(
        glyphs: *mut GlyphString,
        size: c_int,
    );
    pub(in crate::platform::linux) fn pango_glyph_string_free(glyphs: *mut GlyphString);
    pub(in crate::platform::linux) fn pango_font_get_glyph_extents(
        font: *mut c_void,
        glyph: u32,
        ink: *mut Rectangle,
        logical: *mut Rectangle,
    );
    fn pango_font_description_new() -> *mut c_void;
    fn pango_font_description_copy(desc: *const c_void) -> *mut c_void;
    fn pango_font_description_free(desc: *mut c_void);
    fn pango_font_description_from_string(text: *const c_char) -> *mut c_void;
    fn pango_font_description_set_family(desc: *mut c_void, family: *const c_char);
    fn pango_font_description_get_family(desc: *const c_void) -> *const c_char;
    fn pango_font_description_set_weight(desc: *mut c_void, weight: c_int);
    fn pango_font_description_set_style(desc: *mut c_void, style: c_int);
    fn pango_font_description_set_absolute_size(desc: *mut c_void, size: f64);
    fn pango_font_description_set_variations(desc: *mut c_void, variations: *const c_char);
    fn pango_attr_list_new() -> *mut c_void;
    fn pango_attr_list_unref(list: *mut c_void);
    fn pango_attr_list_insert(list: *mut c_void, attr: *mut Attribute);
    fn pango_attr_font_desc_new(desc: *const c_void) -> *mut Attribute;
    fn pango_attr_letter_spacing_new(spacing: c_int) -> *mut Attribute;
    fn pango_attr_font_features_new(features: *const c_char) -> *mut Attribute;
    fn pango_attr_foreground_new(red: u16, green: u16, blue: u16) -> *mut Attribute;
}
#[link(name = "pangoft2-1.0")]
unsafe extern "C" {
    fn pango_fc_font_map_set_config(map: *mut c_void, config: *mut RawConfig);
}
#[link(name = "pangocairo-1.0")]
unsafe extern "C" {
    fn pango_cairo_font_map_new_for_font_type(font_type: c_int) -> *mut c_void;
    fn pango_cairo_context_set_font_options(
        context: *mut c_void,
        options: *const cairo::RawFontOptions,
    );
    pub(in crate::platform::linux) fn pango_cairo_show_glyph_string(
        cr: *mut cairo::RawContext,
        font: *mut c_void,
        glyphs: *mut GlyphString,
    );
}
/// Carte des polices Cairo/FreeType d'une configuration fontconfig, et contexte de mise
/// en page sans hinting (positions des glyphes non arrondies).
pub(in crate::platform::linux) struct FontMap {
    context: Object,
    _map: Object,
}
impl FontMap {
    /// `config` : configuration privée (polices embarquées) ou `None` pour celle du
    /// système. La carte référence la configuration.
    pub fn new(config: Option<*mut RawConfig>) -> crate::Result<Self> {
        // SAFETY : constructeur ; référence possédée.
        let map = unsafe {
            Object::from_full(pango_cairo_font_map_new_for_font_type(cairo::FONT_TYPE_FT))
        }
        .ok_or_else(|| super::failure("pango_cairo_font_map_new_for_font_type"))?;
        if let Some(config) = config {
            // SAFETY : carte fontconfig (type FT) et configuration valides.
            unsafe { pango_fc_font_map_set_config(map.as_ptr(), config) };
        }
        // SAFETY : carte valide ; référence possédée.
        let context = unsafe { Object::from_full(pango_font_map_create_context(map.as_ptr())) }
            .ok_or_else(|| super::failure("pango_font_map_create_context"))?;
        let options = cairo::FontOptions::unhinted()?;
        // SAFETY : contexte et options valides ; Pango copie les options.
        unsafe {
            pango_cairo_context_set_font_options(context.as_ptr(), options.raw());
            pango_context_set_round_glyph_positions(context.as_ptr(), 0);
        }
        Ok(Self { context, _map: map })
    }
    pub fn layout(&self) -> crate::Result<Layout> {
        // SAFETY : contexte valide ; référence possédée.
        unsafe { Object::from_full(pango_layout_new(self.context.as_ptr())) }
            .map(Layout)
            .ok_or_else(|| super::failure("pango_layout_new"))
    }
    /// Métriques (en pixels logiques) de la police choisie pour `desc`.
    pub fn metrics(&self, desc: &FontDescription) -> crate::Result<Metrics> {
        // SAFETY : contexte et description valides ; langue par défaut.
        let raw = required(
            unsafe {
                pango_context_get_metrics(self.context.as_ptr(), desc.raw(), std::ptr::null_mut())
            },
            "pango_context_get_metrics",
        )?
        .as_ptr();
        let px = |v: c_int| v as f32 / SCALE as f32;
        // SAFETY : métriques valides jusqu'au `unref`.
        let metrics = unsafe {
            Metrics {
                ascent: px(pango_font_metrics_get_ascent(raw)),
                descent: px(pango_font_metrics_get_descent(raw)),
                height: px(pango_font_metrics_get_height(raw)),
                underline_position: px(pango_font_metrics_get_underline_position(raw)),
                underline_thickness: px(pango_font_metrics_get_underline_thickness(raw)),
                strikethrough_position: px(pango_font_metrics_get_strikethrough_position(raw)),
                strikethrough_thickness: px(pango_font_metrics_get_strikethrough_thickness(raw)),
            }
        };
        // SAFETY : référence possédée, rendue une fois.
        unsafe { pango_font_metrics_unref(raw) };
        Ok(metrics)
    }
    /// Famille de la police réellement chargée pour `desc` (tests et diagnostics).
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn loaded_family(&self, desc: &FontDescription) -> Option<String> {
        // SAFETY : contexte et description valides ; police possédée.
        let font = unsafe {
            Object::from_full(pango_context_load_font(self.context.as_ptr(), desc.raw()))
        }?;
        // SAFETY : police valide ; description possédée, libérée par `FontDescription`.
        let described =
            FontDescription(NonNull::new(unsafe { pango_font_describe(font.as_ptr()) })?);
        // SAFETY : chaîne possédée par la description, lue avant sa libération.
        let family = unsafe { pango_font_description_get_family(described.raw()) };
        (!family.is_null()).then(|| {
            unsafe { std::ffi::CStr::from_ptr(family) }
                .to_string_lossy()
                .into_owned()
        })
    }
}
/// Métriques d'une police, en pixels logiques (positions au-dessus de la ligne de base).
#[derive(Clone, Copy, Debug)]
pub(in crate::platform::linux) struct Metrics {
    pub ascent: f32,
    pub descent: f32,
    /// Distance recommandée entre deux lignes de base (interligne compris).
    pub height: f32,
    pub underline_position: f32,
    pub underline_thickness: f32,
    pub strikethrough_position: f32,
    pub strikethrough_thickness: f32,
}
/// Mise en page Pango possédée. Ses lignes (`line`) et ses attributs (`log_attrs`)
/// restent valides tant qu'elle n'est pas modifiée : les modificateurs demandent
/// `&mut self`, ce qui interdit de garder `log_attrs` pendant un changement ; `line`
/// rend un pointeur brut, dont l'appelant ne se sert que sur une mise en page figée.
pub(in crate::platform::linux) struct Layout(Object);
impl Layout {
    pub fn raw(&self) -> *mut c_void {
        self.0.as_ptr()
    }
    /// Texte de la mise en page ; Pango le copie.
    pub fn set_text(&mut self, text: &str) -> crate::Result<()> {
        let length = c_int::try_from(text.len())
            .map_err(|_| crate::Error::invalid_input("text too long"))?;
        // SAFETY : octets UTF-8 lus pendant l'appel (longueur explicite).
        unsafe { pango_layout_set_text(self.raw(), text.as_ptr().cast(), length) };
        Ok(())
    }
    pub fn set_attributes(&mut self, attrs: &AttrList) {
        // SAFETY : mise en page et liste valides ; Pango référence la liste.
        unsafe { pango_layout_set_attributes(self.raw(), attrs.0.as_ptr()) };
    }
    pub fn set_font_description(&mut self, desc: &FontDescription) {
        // SAFETY : description copiée par Pango.
        unsafe { pango_layout_set_font_description(self.raw(), desc.raw()) };
    }
    /// Largeur de coupure en pixels logiques ; `None` : pas de coupure.
    pub fn set_width(&mut self, width: Option<f32>) {
        let width = width.map_or(-1, |w| {
            (f64::from(w) * SCALE).round().clamp(1., 1e9) as c_int
        });
        // SAFETY : mise en page valide.
        unsafe { pango_layout_set_width(self.raw(), width) };
    }
    pub fn set_wrap(&mut self, wrap: c_int) {
        // SAFETY : idem.
        unsafe { pango_layout_set_wrap(self.raw(), wrap) };
    }
    pub fn set_ellipsize(&mut self, mode: c_int) {
        // SAFETY : idem.
        unsafe { pango_layout_set_ellipsize(self.raw(), mode) };
    }
    pub fn set_justify(&mut self, justify: bool) {
        // SAFETY : idem.
        unsafe { pango_layout_set_justify(self.raw(), Gboolean::from(justify)) };
    }
    pub fn line_count(&self) -> usize {
        // SAFETY : idem.
        unsafe { pango_layout_get_line_count(self.raw()) }.max(0) as usize
    }
    /// Ligne `index` (0 ≤ index < `line_count`), possédée par la mise en page.
    pub fn line(&self, index: usize) -> *mut LayoutLine {
        // SAFETY : index dans les lignes ; pointeur emprunté.
        unsafe { pango_layout_get_line_readonly(self.raw(), index as c_int) }
    }
    /// Attributs de chaque caractère, et de la fin du texte.
    pub fn log_attrs(&self) -> &[LogAttr] {
        let mut count = 0;
        // SAFETY : tableau possédé par la mise en page, non modifiée tant que l'emprunt
        // dure (modificateurs en `&mut self`).
        let attrs = unsafe { pango_layout_get_log_attrs_readonly(self.raw(), &mut count) };
        if attrs.is_null() || count <= 0 {
            return &[];
        }
        // SAFETY : `count` éléments valides tant que la mise en page n'est pas modifiée.
        unsafe { std::slice::from_raw_parts(attrs, count as usize) }
    }
}
/// Rectangle logique de `line` (repère de sa ligne de base), en unités Pango.
pub(in crate::platform::linux) fn line_extents(line: *mut LayoutLine) -> Rectangle {
    let mut logical = Rectangle::default();
    // SAFETY : ligne valide d'après l'appelant ; encre non demandée.
    unsafe { pango_layout_line_get_extents(line, std::ptr::null_mut(), &mut logical) };
    logical
}
/// Abscisse (unités Pango, depuis le bord gauche de la ligne) de l'octet `index`.
pub(in crate::platform::linux) fn index_to_x(line: *mut LayoutLine, index: usize) -> c_int {
    let mut x = 0;
    // SAFETY : ligne valide d'après l'appelant ; sortie locale.
    unsafe { pango_layout_line_index_to_x(line, index as c_int, 0, &mut x) };
    x
}
/// Description de police possédée.
pub(in crate::platform::linux) struct FontDescription(NonNull<c_void>);
impl FontDescription {
    pub fn new(
        family: &std::ffi::CStr,
        weight: u16,
        italic: bool,
        size: f32,
        variations: Option<&std::ffi::CStr>,
    ) -> crate::Result<Self> {
        // SAFETY : constructeur ; description possédée.
        let desc = Self(required(
            unsafe { pango_font_description_new() },
            "pango_font_description_new",
        )?);
        // SAFETY : description valide ; chaînes copiées par Pango.
        unsafe {
            pango_font_description_set_family(desc.raw(), family.as_ptr());
            pango_font_description_set_weight(desc.raw(), c_int::from(weight));
            pango_font_description_set_style(
                desc.raw(),
                if italic { STYLE_ITALIC } else { STYLE_NORMAL },
            );
            pango_font_description_set_absolute_size(desc.raw(), f64::from(size) * SCALE);
            if let Some(variations) = variations {
                pango_font_description_set_variations(desc.raw(), variations.as_ptr());
            }
        }
        Ok(desc)
    }
    pub fn raw(&self) -> *mut c_void {
        self.0.as_ptr()
    }
}
impl Clone for FontDescription {
    fn clone(&self) -> Self {
        // SAFETY : description valide ; copie possédée (non nulle sauf épuisement mémoire).
        Self(
            NonNull::new(unsafe { pango_font_description_copy(self.raw()) })
                .expect("pango_font_description_copy"),
        )
    }
}
impl Drop for FontDescription {
    fn drop(&mut self) {
        // SAFETY : description possédée, libérée une fois.
        unsafe { pango_font_description_free(self.raw()) };
    }
}
/// Liste d'attributs possédée ; chaque attribut couvre un intervalle d'octets.
pub(in crate::platform::linux) struct AttrList(NonNull<c_void>);
impl AttrList {
    pub fn new() -> crate::Result<Self> {
        // SAFETY : constructeur ; liste possédée.
        Ok(Self(required(
            unsafe { pango_attr_list_new() },
            "pango_attr_list_new",
        )?))
    }
    /// Insère `attr` (cédé à la liste) sur les octets `range`.
    fn insert(&self, attr: *mut Attribute, range: std::ops::Range<usize>) -> crate::Result<()> {
        let attr = required(attr, "pango_attr_new")?.as_ptr();
        // SAFETY : attribut fraîchement créé, cédé à la liste valide.
        unsafe {
            (*attr).start_index = range.start as u32;
            (*attr).end_index = range.end as u32;
            pango_attr_list_insert(self.0.as_ptr(), attr);
        }
        Ok(())
    }
    pub fn font(&self, desc: &FontDescription, range: std::ops::Range<usize>) -> crate::Result<()> {
        // SAFETY : description copiée par l'attribut.
        self.insert(unsafe { pango_attr_font_desc_new(desc.raw()) }, range)
    }
    pub fn letter_spacing(&self, spacing: f32, range: std::ops::Range<usize>) -> crate::Result<()> {
        let units = (f64::from(spacing) * SCALE).round() as c_int;
        // SAFETY : constructeur sans pointeur.
        self.insert(unsafe { pango_attr_letter_spacing_new(units) }, range)
    }
    /// Fonctionnalités OpenType (`"tnum"`).
    pub fn features(
        &self,
        features: &std::ffi::CStr,
        range: std::ops::Range<usize>,
    ) -> crate::Result<()> {
        // SAFETY : chaîne copiée par l'attribut.
        self.insert(
            unsafe { pango_attr_font_features_new(features.as_ptr()) },
            range,
        )
    }
    /// Couleur sans effet sur le dessin (le backend colore chaque run) : sépare les runs
    /// aux bornes des portions colorées.
    pub fn split(&self, range: std::ops::Range<usize>) -> crate::Result<()> {
        // SAFETY : constructeur sans pointeur.
        self.insert(unsafe { pango_attr_foreground_new(0, 0, 0) }, range)
    }
}
impl Drop for AttrList {
    fn drop(&mut self) {
        // SAFETY : référence possédée, rendue une fois.
        unsafe { pango_attr_list_unref(self.0.as_ptr()) };
    }
}
/// Famille d'une description de police écrite (« Inter 10 », « Cantarell Bold 11 »,
/// « Noto Sans CJK JP, 10 ») : réglages des polices du bureau.
pub(in crate::platform::linux) fn family_of(description: &str) -> Option<String> {
    let text = std::ffi::CString::new(description).ok()?;
    // SAFETY : texte terminé par un octet nul ; description possédée, libérée ici, sa
    // famille lue avant.
    unsafe {
        let desc = pango_font_description_from_string(text.as_ptr());
        if desc.is_null() {
            return None;
        }
        let family = pango_font_description_get_family(desc);
        let name = (!family.is_null())
            .then(|| {
                std::ffi::CStr::from_ptr(family)
                    .to_string_lossy()
                    .trim()
                    .to_owned()
            })
            .filter(|n| !n.is_empty());
        pango_font_description_free(desc);
        name
    }
}
