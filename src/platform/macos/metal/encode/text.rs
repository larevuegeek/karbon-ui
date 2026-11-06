//! Texte : mise en page Core Text partagée avec le chemin Core Graphics (mêmes lignes,
//! runs, positions et couleurs par portion), glyphes tirés de l'atlas et dessinés en
//! quads teintés, décorations en rectangles.
//!
//! Sous `background-clip: text`, la couverture de chaque glyphe sert de masque au
//! remplissage (couleur, dégradé linéaire ou radial posé sur la zone de la portée, hors de
//! laquelle rien n'est peint), comme le masque de glyphes de `Canvas::text_fill`. Un glyphe
//! en couleur (émoji) y sert par sa luminance, comme dans le masque en niveaux de gris de
//! Core Graphics (`atlas::Kind::Luminance`).
use super::{Bind, Builder, shapes::Fill};
use crate::platform::macos::graphics::{colored, linear_points};
use crate::platform::macos::metal::atlas::Kind;
use crate::platform::macos::metal::instance::{
    Instance, PAINT_LINEAR, PAINT_MASK, PAINT_RADIAL, PAINT_TEXTURE, SHAPE_NONE, color as array,
    pixel,
};
use crate::ui::{
    rendering::{brush::Background, transform::Affine},
    scene::{PointF, RectF},
    styling::text::RichParagraph,
    theme::Color,
};

/// Peinture des glyphes sous un remplissage, dans le repère de leurs quads.
struct GlyphPaint {
    kind: u32,
    paint: [f32; 4],
    stops: [u32; 2],
    color: [f32; 4],
}

/// Abscisse en pixels coupée en pixel entier et en `steps`-ièmes de pixel (arrondi vers
/// le bas).
fn split(v: f32, steps: f32) -> (f32, u8) {
    let n = (v * steps).floor();
    let whole = (n / steps).floor();
    (whole, (n - whole * steps).clamp(0., steps - 1.) as u8)
}
impl Builder<'_> {
    pub(super) fn text(
        &mut self,
        paragraph: &RichParagraph,
        rect: RectF,
        color: Color,
        wrap: bool,
    ) -> crate::Result<()> {
        let glyphs = self.cg.text_glyphs(paragraph, rect, color, wrap)?;
        let fill = self.text_fills.last().cloned();
        let saved = self.state;
        if glyphs.clip {
            self.clip_rect(rect);
        }
        if let Some((_, area)) = &fill {
            self.clip_rect(*area);
        }
        let t = self.transform();
        // Repère droit à échelle uniforme : glyphes posés sur les pixels de la cible.
        let straight = t.m12 == 0. && t.m21 == 0. && t.m11 > 0. && t.m11 == t.m22;
        let paint = fill
            .as_ref()
            .map(|(background, area)| self.glyph_paint(background, *area, straight.then_some(t)));
        let scale = if straight {
            t.m11
        } else {
            ((pixel(t).recip() * 8.).round() / 8.).max(0.125)
        };
        for run in glyphs.runs.iter() {
            let ink = array(run.color.unwrap_or(color));
            let kind = match (colored(&run.font), fill.is_some()) {
                (false, _) => Kind::Mask,
                (true, false) => Kind::Color,
                (true, true) => Kind::Luminance,
            };
            let font = self.resources.atlas.font(&run.font);
            let steps = super::super::atlas::steps(&run.font, scale);
            for (&glyph, origin) in run.glyphs.iter().zip(&run.origins) {
                let p = PointF {
                    x: rect.left + origin.x,
                    y: rect.top + origin.y,
                };
                let (quad, m, entry) = if straight {
                    let d = t.point(p);
                    // Core Graphics place la ligne de base sur un pixel entier (arrondi vers
                    // le bas dans son repère natif, y vers le haut). Exception non suivie :
                    // l'image d'un émoji, qu'il pose à sa position verticale fractionnaire
                    // (jusqu'à un pixel d'écart, voir `docs/tests.md`).
                    let ((x, fx), (y, fy)) = (split(d.x, steps), (d.y.ceil(), 0));
                    let Some(e) = self.resources.atlas.glyph(
                        self.gpu,
                        font,
                        &run.font,
                        glyph,
                        scale,
                        (fx, fy),
                        kind,
                    )?
                    else {
                        continue;
                    };
                    let (left, top) = (x + e.left as f32, y + e.top as f32);
                    let quad = RectF {
                        left,
                        top,
                        right: left + e.width as f32,
                        bottom: top + e.height as f32,
                    };
                    (quad, Affine::IDENTITY, e)
                } else {
                    let Some(e) = self.resources.atlas.glyph(
                        self.gpu,
                        font,
                        &run.font,
                        glyph,
                        scale,
                        (0, 0),
                        kind,
                    )?
                    else {
                        continue;
                    };
                    let (left, top) = (p.x + e.left as f32 / scale, p.y + e.top as f32 / scale);
                    let quad = RectF {
                        left,
                        top,
                        right: left + e.width as f32 / scale,
                        bottom: top + e.height as f32 / scale,
                    };
                    (quad, t, e)
                };
                let mut i = Instance::new(
                    quad,
                    m,
                    SHAPE_NONE,
                    if entry.colored {
                        PAINT_TEXTURE
                    } else {
                        PAINT_MASK
                    },
                );
                i.uv = entry.uv;
                i.color = ink;
                if let Some(p) = &paint {
                    i.kind[1] = p.kind;
                    i.paint = p.paint;
                    [i.kind[2], i.kind[3]] = p.stops;
                    i.color = p.color;
                    i.extra[1] = i32::from(p.kind != PAINT_MASK);
                }
                let texture = self.resources.atlas.texture(entry.page).clone();
                let bind = self.bind(&texture);
                self.emit(i, bind);
            }
        }
        for (r, c) in glyphs.decorations {
            if let Some((background, area)) = &fill {
                let fill = match background {
                    Background::Solid(c) => Fill::Solid(*c),
                    Background::Linear(g) => Fill::Linear(g, *area),
                    Background::Radial(g) => Fill::Radial(g, *area),
                };
                self.fill_box(r, [0.; 4], fill);
            } else if r.right > r.left && r.bottom > r.top {
                let mut i = Instance::rounded(r, [0.; 4], t);
                i.color = array(c);
                self.emit(i, Bind::None);
            }
        }
        self.state = saved;
        Ok(())
    }
    /// Remplissage `background` posé sur `area` (repère logique), exprimé dans le repère
    /// des quads des glyphes : pixels de la cible sous un repère droit `straight`, repère
    /// logique sinon.
    fn glyph_paint(
        &mut self,
        background: &Background,
        area: RectF,
        straight: Option<Affine>,
    ) -> GlyphPaint {
        let map = |p: PointF| straight.map_or(p, |t| t.point(p));
        let zoom = straight.map_or(1., |t| t.m11);
        match background {
            Background::Solid(c) => GlyphPaint {
                kind: PAINT_MASK,
                paint: [0.; 4],
                stops: [0, 0],
                color: array(*c),
            },
            Background::Linear(g) => {
                let (a, b) = linear_points(g, area);
                let (a, b) = (map(a), map(b));
                GlyphPaint {
                    kind: PAINT_LINEAR,
                    paint: [a.x, a.y, b.x, b.y],
                    stops: self.stops(&g.stops),
                    color: [0.; 4],
                }
            }
            Background::Radial(g) => {
                let (w, h) = (area.right - area.left, area.bottom - area.top);
                let [rx, ry] = g.radii(w, h);
                let center = map(PointF {
                    x: area.left + g.center[0] * w,
                    y: area.top + g.center[1] * h,
                });
                GlyphPaint {
                    kind: PAINT_RADIAL,
                    paint: [
                        center.x,
                        center.y,
                        rx.max(0.001) * zoom,
                        ry.max(0.001) * zoom,
                    ],
                    stops: self.stops(&g.stops),
                    color: [0.; 4],
                }
            }
        }
    }
}
