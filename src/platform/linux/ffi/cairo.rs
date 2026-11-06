//! Cairo : surfaces image, contexte de dessin, motifs (dégradés, images) et options des
//! polices. `Context` et `Pattern` possèdent leur objet ; leurs méthodes sont sûres car
//! le pointeur reste valide pendant toute leur vie. Un contexte ou un motif créé sur une
//! surface emprunte cette surface (`'a`) : les pixels d'un `Bitmap` ne peuvent pas être
//! rendus (`into_pixels`) ni libérés tant qu'un contexte y dessine.
use super::{failure, required};
use crate::platform::linux::bitmap::Surface;
use std::{
    ffi::{c_int, c_uint, c_void},
    marker::PhantomData,
    ptr::NonNull,
};

#[repr(C)]
pub(in crate::platform::linux) struct RawContext {
    _private: [u8; 0],
}
#[repr(C)]
pub(in crate::platform::linux) struct RawSurface {
    _private: [u8; 0],
}
#[repr(C)]
pub(in crate::platform::linux) struct RawPattern {
    _private: [u8; 0],
}
#[repr(C)]
pub(in crate::platform::linux) struct RawFontOptions {
    _private: [u8; 0],
}
/// Matrice affine de Cairo : `x' = xx·x + xy·y + x0`, `y' = yx·x + yy·y + y0`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::platform::linux) struct Matrix {
    pub xx: f64,
    pub yx: f64,
    pub xy: f64,
    pub yy: f64,
    pub x0: f64,
    pub y0: f64,
}
pub(in crate::platform::linux) const STATUS_SUCCESS: c_int = 0;
pub(in crate::platform::linux) const FORMAT_ARGB32: c_int = 0;
/// Alpha seul ; lignes alignées sur 4 octets (`cairo_format_stride_for_width`).
pub(in crate::platform::linux) const FORMAT_A8: c_int = 2;
pub(in crate::platform::linux) const CONTENT_ALPHA: c_int = 0x2000;
pub(in crate::platform::linux) const OPERATOR_SOURCE: c_int = 1;
pub(in crate::platform::linux) const EXTEND_PAD: c_int = 3;
pub(in crate::platform::linux) const FILTER_GOOD: c_int = 1;
pub(in crate::platform::linux) const FILTER_BILINEAR: c_int = 4;
pub(in crate::platform::linux) const FILL_WINDING: c_int = 0;
pub(in crate::platform::linux) const FILL_EVEN_ODD: c_int = 1;
pub(in crate::platform::linux) const CAP_BUTT: c_int = 0;
pub(in crate::platform::linux) const CAP_ROUND: c_int = 1;
pub(in crate::platform::linux) const CAP_SQUARE: c_int = 2;
pub(in crate::platform::linux) const JOIN_MITER: c_int = 0;
pub(in crate::platform::linux) const JOIN_ROUND: c_int = 1;
pub(in crate::platform::linux) const JOIN_BEVEL: c_int = 2;
pub(in crate::platform::linux) const ANTIALIAS_GRAY: c_int = 2;
pub(in crate::platform::linux) const HINT_STYLE_NONE: c_int = 1;
pub(in crate::platform::linux) const HINT_METRICS_OFF: c_int = 1;
pub(in crate::platform::linux) const FONT_TYPE_FT: c_int = 1;
/// Rappel d'écriture de `cairo_surface_write_to_png_stream`.
pub(in crate::platform::linux) type WriteFunc =
    unsafe extern "C" fn(closure: *mut c_void, data: *const u8, length: c_uint) -> c_int;

#[link(name = "cairo")]
unsafe extern "C" {
    #[cfg(test)]
    fn cairo_version() -> c_int;
    pub(in crate::platform::linux) fn cairo_image_surface_create_for_data(
        data: *mut u8,
        format: c_int,
        width: c_int,
        height: c_int,
        stride: c_int,
    ) -> *mut RawSurface;
    pub(in crate::platform::linux) fn cairo_surface_status(surface: *mut RawSurface) -> c_int;
    pub(in crate::platform::linux) fn cairo_surface_destroy(surface: *mut RawSurface);
    pub(in crate::platform::linux) fn cairo_surface_flush(surface: *mut RawSurface);
    pub(in crate::platform::linux) fn cairo_surface_mark_dirty(surface: *mut RawSurface);
    pub(in crate::platform::linux) fn cairo_surface_write_to_png_stream(
        surface: *mut RawSurface,
        write: WriteFunc,
        closure: *mut c_void,
    ) -> c_int;
    fn cairo_create(target: *mut RawSurface) -> *mut RawContext;
    fn cairo_destroy(cr: *mut RawContext);
    fn cairo_status(cr: *mut RawContext) -> c_int;
    fn cairo_save(cr: *mut RawContext);
    fn cairo_restore(cr: *mut RawContext);
    fn cairo_set_operator(cr: *mut RawContext, op: c_int);
    fn cairo_set_source_rgba(cr: *mut RawContext, r: f64, g: f64, b: f64, a: f64);
    fn cairo_set_source(cr: *mut RawContext, source: *mut RawPattern);
    fn cairo_paint(cr: *mut RawContext);
    fn cairo_paint_with_alpha(cr: *mut RawContext, alpha: f64);
    fn cairo_mask(cr: *mut RawContext, pattern: *mut RawPattern);
    fn cairo_fill(cr: *mut RawContext);
    fn cairo_stroke(cr: *mut RawContext);
    fn cairo_clip(cr: *mut RawContext);
    fn cairo_new_path(cr: *mut RawContext);
    fn cairo_set_fill_rule(cr: *mut RawContext, rule: c_int);
    fn cairo_set_line_width(cr: *mut RawContext, width: f64);
    fn cairo_set_line_cap(cr: *mut RawContext, cap: c_int);
    fn cairo_set_line_join(cr: *mut RawContext, join: c_int);
    fn cairo_set_miter_limit(cr: *mut RawContext, limit: f64);
    fn cairo_move_to(cr: *mut RawContext, x: f64, y: f64);
    fn cairo_line_to(cr: *mut RawContext, x: f64, y: f64);
    fn cairo_curve_to(cr: *mut RawContext, x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64);
    fn cairo_close_path(cr: *mut RawContext);
    fn cairo_rectangle(cr: *mut RawContext, x: f64, y: f64, width: f64, height: f64);
    fn cairo_transform(cr: *mut RawContext, matrix: *const Matrix);
    fn cairo_identity_matrix(cr: *mut RawContext);
    fn cairo_clip_extents(
        cr: *mut RawContext,
        x1: *mut f64,
        y1: *mut f64,
        x2: *mut f64,
        y2: *mut f64,
    );
    fn cairo_push_group(cr: *mut RawContext);
    fn cairo_push_group_with_content(cr: *mut RawContext, content: c_int);
    fn cairo_pop_group(cr: *mut RawContext) -> *mut RawPattern;
    fn cairo_pop_group_to_source(cr: *mut RawContext);
    fn cairo_pattern_create_linear(x0: f64, y0: f64, x1: f64, y1: f64) -> *mut RawPattern;
    fn cairo_pattern_create_radial(
        cx0: f64,
        cy0: f64,
        radius0: f64,
        cx1: f64,
        cy1: f64,
        radius1: f64,
    ) -> *mut RawPattern;
    fn cairo_pattern_create_for_surface(surface: *mut RawSurface) -> *mut RawPattern;
    fn cairo_pattern_add_color_stop_rgba(
        pattern: *mut RawPattern,
        offset: f64,
        r: f64,
        g: f64,
        b: f64,
        a: f64,
    );
    fn cairo_pattern_set_extend(pattern: *mut RawPattern, extend: c_int);
    fn cairo_pattern_set_filter(pattern: *mut RawPattern, filter: c_int);
    fn cairo_pattern_set_matrix(pattern: *mut RawPattern, matrix: *const Matrix);
    fn cairo_pattern_status(pattern: *mut RawPattern) -> c_int;
    fn cairo_pattern_destroy(pattern: *mut RawPattern);
    fn cairo_font_options_create() -> *mut RawFontOptions;
    fn cairo_font_options_destroy(options: *mut RawFontOptions);
    fn cairo_font_options_set_antialias(options: *mut RawFontOptions, antialias: c_int);
    fn cairo_font_options_set_hint_style(options: *mut RawFontOptions, style: c_int);
    fn cairo_font_options_set_hint_metrics(options: *mut RawFontOptions, metrics: c_int);
}
/// La bibliothèque Cairo chargée place les glyphes au quart de pixel (positionnement
/// sous-pixel, depuis Cairo 1.17.4) ; avant, au pixel entier. Le renderer Vulkan
/// reproduit le modèle du quart de pixel : sur un Cairo plus ancien (Ubuntu 22.04 :
/// 1.16), son texte ne coïncide plus pixel pour pixel avec le dessin Cairo, et les
/// tests qui l'exigent se sautent.
#[cfg(test)]
pub(in crate::platform::linux) fn subpixel_glyphs() -> bool {
    // SAFETY : fonction pure, sans argument.
    let version = unsafe { cairo_version() };
    version >= 1_17_04
}
impl Matrix {
    pub const IDENTITY: Self = Self {
        xx: 1.,
        yx: 0.,
        xy: 0.,
        yy: 1.,
        x0: 0.,
        y0: 0.,
    };
    /// Même convention que `Affine` (vecteur ligne) : `xx = m11`, `yx = m12`, `xy = m21`…
    pub fn from_affine(a: crate::ui::rendering::transform::Affine) -> Self {
        Self {
            xx: a.m11.into(),
            yx: a.m12.into(),
            xy: a.m21.into(),
            yy: a.m22.into(),
            x0: a.dx.into(),
            y0: a.dy.into(),
        }
    }
    pub fn scaling(x: f64, y: f64) -> Self {
        Self {
            xx: x,
            yy: y,
            ..Self::IDENTITY
        }
    }
}
/// Contexte de dessin possédé sur une surface empruntée pendant `'a`.
pub(in crate::platform::linux) struct Context<'a>(NonNull<RawContext>, PhantomData<&'a RawSurface>);
impl<'a> Context<'a> {
    /// Contexte sur `surface`, empruntée tant que le contexte dessine.
    pub fn new<S: Surface + ?Sized>(surface: &'a S) -> crate::Result<Self> {
        // SAFETY : surface valide (invariant de `Surface`) ; cairo_create la référence.
        let raw = required(unsafe { cairo_create(surface.raw()) }, "cairo_create")?;
        let context = Self(raw, PhantomData);
        if context.status() != STATUS_SUCCESS {
            return Err(failure("cairo_create"));
        }
        Ok(context)
    }
}
impl Context<'_> {
    pub fn raw(&self) -> *mut RawContext {
        self.0.as_ptr()
    }
    pub fn status(&self) -> c_int {
        // SAFETY : contexte valide (invariant du type).
        unsafe { cairo_status(self.raw()) }
    }
}
impl Drop for Context<'_> {
    fn drop(&mut self) {
        // SAFETY : contexte possédé, détruit une fois.
        unsafe { cairo_destroy(self.raw()) };
    }
}
/// Appels sans pointeur à valider : l'invariant de `Context` suffit.
macro_rules! forward {
    ($($name:ident => $native:ident($($arg:ident: $ty:ty),*);)*) => {
        impl Context<'_> {
            $(
                pub fn $name(&self, $($arg: $ty),*) {
                    // SAFETY : contexte valide (invariant du type), arguments par valeur.
                    unsafe { $native(self.raw(), $($arg),*) }
                }
            )*
        }
    };
}
forward! {
    save => cairo_save();
    restore => cairo_restore();
    set_operator => cairo_set_operator(op: c_int);
    set_source_rgba => cairo_set_source_rgba(r: f64, g: f64, b: f64, a: f64);
    paint => cairo_paint();
    paint_with_alpha => cairo_paint_with_alpha(alpha: f64);
    fill => cairo_fill();
    stroke => cairo_stroke();
    clip => cairo_clip();
    new_path => cairo_new_path();
    set_fill_rule => cairo_set_fill_rule(rule: c_int);
    set_line_width => cairo_set_line_width(width: f64);
    set_line_cap => cairo_set_line_cap(cap: c_int);
    set_line_join => cairo_set_line_join(join: c_int);
    set_miter_limit => cairo_set_miter_limit(limit: f64);
    move_to => cairo_move_to(x: f64, y: f64);
    line_to => cairo_line_to(x: f64, y: f64);
    curve_to => cairo_curve_to(x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64);
    close_path => cairo_close_path();
    rectangle => cairo_rectangle(x: f64, y: f64, width: f64, height: f64);
    identity_matrix => cairo_identity_matrix();
    push_group => cairo_push_group();
    push_group_with_content => cairo_push_group_with_content(content: c_int);
    pop_group_to_source => cairo_pop_group_to_source();
}
impl Context<'_> {
    pub fn set_source(&self, pattern: &Pattern<'_>) {
        // SAFETY : contexte et motif valides ; Cairo référence le motif (et sa surface).
        unsafe { cairo_set_source(self.raw(), pattern.raw()) };
    }
    pub fn mask(&self, pattern: &Pattern<'_>) {
        // SAFETY : idem.
        unsafe { cairo_mask(self.raw(), pattern.raw()) };
    }
    pub fn transform(&self, matrix: Matrix) {
        // SAFETY : matrice locale lue pendant l'appel.
        unsafe { cairo_transform(self.raw(), &matrix) };
    }
    /// Boîte de la découpe courante dans le repère courant : gauche, haut, droite, bas.
    pub fn clip_extents(&self) -> [f64; 4] {
        let mut b = [0f64; 4];
        let [x1, y1, x2, y2] = &mut b;
        // SAFETY : sorties locales valides en écriture.
        unsafe { cairo_clip_extents(self.raw(), x1, y1, x2, y2) };
        b
    }
    /// Ferme le groupe ouvert par `push_group*` et renvoie son contenu en motif (qui
    /// possède sa propre surface).
    pub fn pop_group(&self) -> crate::Result<Pattern<'static>> {
        // SAFETY : un groupe est ouvert d'après l'appelant ; motif possédé.
        Pattern::own(unsafe { cairo_pop_group(self.raw()) }, "cairo_pop_group")
    }
}
/// Motif possédé : dégradé ou groupe (`'static`), ou surface empruntée pendant `'a`.
pub(in crate::platform::linux) struct Pattern<'a>(NonNull<RawPattern>, PhantomData<&'a RawSurface>);
impl<'a> Pattern<'a> {
    fn own(raw: *mut RawPattern, operation: &'static str) -> crate::Result<Self> {
        let pattern = Self(required(raw, operation)?, PhantomData);
        // SAFETY : motif valide ; un motif en erreur est libéré par `Drop`.
        if unsafe { cairo_pattern_status(pattern.raw()) } != STATUS_SUCCESS {
            return Err(failure(operation));
        }
        Ok(pattern)
    }
    /// Motif d'une surface, empruntée tant que le motif existe.
    pub fn surface<S: Surface + ?Sized>(surface: &'a S) -> crate::Result<Self> {
        // SAFETY : surface valide (invariant de `Surface`) ; le motif la référence.
        Self::own(
            unsafe { cairo_pattern_create_for_surface(surface.raw()) },
            "cairo_pattern_create_for_surface",
        )
    }
}
impl Pattern<'static> {
    pub fn linear(x0: f64, y0: f64, x1: f64, y1: f64) -> crate::Result<Self> {
        // SAFETY : constructeur sans pointeur.
        Self::own(
            unsafe { cairo_pattern_create_linear(x0, y0, x1, y1) },
            "cairo_pattern_create_linear",
        )
    }
    /// Dégradé radial du cercle de centre `(cx, cy)` et de rayon 0 à celui de rayon `r`.
    pub fn radial(cx: f64, cy: f64, r: f64) -> crate::Result<Self> {
        // SAFETY : constructeur sans pointeur.
        Self::own(
            unsafe { cairo_pattern_create_radial(cx, cy, 0., cx, cy, r) },
            "cairo_pattern_create_radial",
        )
    }
}
impl Pattern<'_> {
    pub fn raw(&self) -> *mut RawPattern {
        self.0.as_ptr()
    }
    pub fn add_stop(&self, offset: f64, [r, g, b, a]: [f64; 4]) {
        // SAFETY : motif valide (invariant du type).
        unsafe { cairo_pattern_add_color_stop_rgba(self.raw(), offset, r, g, b, a) };
    }
    pub fn set_extend(&self, extend: c_int) {
        // SAFETY : idem.
        unsafe { cairo_pattern_set_extend(self.raw(), extend) };
    }
    pub fn set_filter(&self, filter: c_int) {
        // SAFETY : idem.
        unsafe { cairo_pattern_set_filter(self.raw(), filter) };
    }
    /// Matrice du motif : du repère utilisateur vers celui du motif.
    pub fn set_matrix(&self, matrix: Matrix) {
        // SAFETY : matrice locale lue pendant l'appel.
        unsafe { cairo_pattern_set_matrix(self.raw(), &matrix) };
    }
}
impl Drop for Pattern<'_> {
    fn drop(&mut self) {
        // SAFETY : référence possédée, rendue une fois.
        unsafe { cairo_pattern_destroy(self.raw()) };
    }
}
/// Options des polices : niveaux de gris, sans hinting des contours ni des métriques,
/// pour que la mise en page ne dépende pas de l'échelle du dessin (comme Core Text).
pub(in crate::platform::linux) struct FontOptions(NonNull<RawFontOptions>);
impl FontOptions {
    pub fn unhinted() -> crate::Result<Self> {
        // SAFETY : constructeur sans pointeur.
        let options = Self(required(
            unsafe { cairo_font_options_create() },
            "cairo_font_options_create",
        )?);
        // SAFETY : options valides.
        unsafe {
            cairo_font_options_set_antialias(options.raw(), ANTIALIAS_GRAY);
            cairo_font_options_set_hint_style(options.raw(), HINT_STYLE_NONE);
            cairo_font_options_set_hint_metrics(options.raw(), HINT_METRICS_OFF);
        }
        Ok(options)
    }
    pub fn raw(&self) -> *mut RawFontOptions {
        self.0.as_ptr()
    }
}
impl Drop for FontOptions {
    fn drop(&mut self) {
        // SAFETY : options possédées, détruites une fois.
        unsafe { cairo_font_options_destroy(self.raw()) };
    }
}
