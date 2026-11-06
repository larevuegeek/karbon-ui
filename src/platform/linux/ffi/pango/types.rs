//! Structures publiques de Pango lues par le backend (`PangoLayoutLine`,
//! `PangoGlyphItem`…), reproduites avec leur disposition C, stable depuis Pango 1.44,
//! et constantes employées.
use super::super::glib::GSList;
use std::ffi::{c_int, c_void};

/// Unités Pango par pixel logique.
pub(in crate::platform::linux) const SCALE: f64 = 1024.;
pub(in crate::platform::linux) const WRAP_WORD_CHAR: c_int = 2;
pub(in crate::platform::linux) const ELLIPSIZE_END: c_int = 3;
pub(in crate::platform::linux) const STYLE_NORMAL: c_int = 0;
pub(in crate::platform::linux) const STYLE_ITALIC: c_int = 2;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub(in crate::platform::linux) struct Rectangle {
    pub x: c_int,
    pub y: c_int,
    pub width: c_int,
    pub height: c_int,
}
#[repr(C)]
pub(in crate::platform::linux) struct LayoutLine {
    pub layout: *mut c_void,
    pub start_index: c_int,
    pub length: c_int,
    /// `PangoGlyphItem` dans l'ordre visuel.
    pub runs: *mut GSList,
    pub flags: u32,
}
#[repr(C)]
pub(in crate::platform::linux) struct Analysis {
    pub shape_engine: *mut c_void,
    pub lang_engine: *mut c_void,
    pub font: *mut c_void,
    pub level: u8,
    pub gravity: u8,
    pub flags: u8,
    pub script: u8,
    pub language: *mut c_void,
    pub extra_attrs: *mut GSList,
}
#[repr(C)]
pub(in crate::platform::linux) struct Item {
    /// Octet de début dans le texte de la mise en page.
    pub offset: c_int,
    pub length: c_int,
    pub num_chars: c_int,
    pub analysis: Analysis,
}
#[repr(C)]
pub(in crate::platform::linux) struct GlyphGeometry {
    pub width: i32,
    pub x_offset: i32,
    pub y_offset: i32,
}
#[repr(C)]
pub(in crate::platform::linux) struct GlyphInfo {
    pub glyph: u32,
    pub geometry: GlyphGeometry,
    pub attr: u32,
}
/// Glyphe vide (`PANGO_GLYPH_EMPTY` : caractère sans encre, séparateur de ligne) : rien
/// à rastériser.
pub(in crate::platform::linux) const GLYPH_EMPTY: u32 = 0x0FFF_FFFF;
/// Bit des glyphes inconnus de la police (`PANGO_GLYPH_UNKNOWN_FLAG`) : rien à
/// rastériser non plus.
pub(in crate::platform::linux) const GLYPH_UNKNOWN_FLAG: u32 = 0x1000_0000;
impl GlyphInfo {
    /// Le glyphe a de l'encre à rastériser : ni vide, ni inconnu.
    pub fn has_ink(&self) -> bool {
        self.glyph != GLYPH_EMPTY && self.glyph & GLYPH_UNKNOWN_FLAG == 0
    }
    /// La police dessine ce glyphe en couleur (émoji ; bit `is_color`, Pango 1.50).
    pub fn colored(&self) -> bool {
        self.attr & 1 << 1 != 0
    }
}
#[repr(C)]
pub(in crate::platform::linux) struct GlyphString {
    pub num_glyphs: c_int,
    pub glyphs: *mut GlyphInfo,
    pub log_clusters: *mut c_int,
    space: c_int,
}
#[repr(C)]
pub(in crate::platform::linux) struct GlyphItem {
    pub item: *mut Item,
    pub glyphs: *mut GlyphString,
    pub y_offset: c_int,
    pub start_x_offset: c_int,
    pub end_x_offset: c_int,
}
/// Attributs d'un caractère (`PangoLogAttr`, champs de bits).
#[repr(transparent)]
#[derive(Clone, Copy)]
pub(in crate::platform::linux) struct LogAttr(u32);
impl LogAttr {
    pub fn is_cursor_position(self) -> bool {
        self.0 & 1 << 4 != 0
    }
}
#[repr(C)]
pub(in crate::platform::linux) struct Attribute {
    klass: *const c_void,
    pub start_index: u32,
    pub end_index: u32,
}
