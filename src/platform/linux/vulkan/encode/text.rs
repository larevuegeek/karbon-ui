//! Texte : mise en page Pango partagée avec le dessin Cairo (mêmes lignes, runs, positions
//! et couleurs par portion), glyphes tirés de l'atlas et dessinés en quads teintés,
//! décorations en rectangles. Copie de `macos/metal/encode/text.rs`.
//!
//! Sous `background-clip: text`, la couverture de chaque glyphe sert de masque au
//! remplissage (couleur, dégradé linéaire ou radial posé sur la zone de la portée et
//! prolongé au-delà), comme le groupe d'alpha de `Canvas::text_fill`. Un glyphe en couleur
//! (émoji) y sert par son alpha : Cairo le dessine dans ce groupe sans couleurs.
//!
//! Sous un repère droit à échelle uniforme, chaque glyphe est posé au pixel et à la phase
//! où Cairo le poserait (`graphics/gpu.rs`) : mêmes pixels de couverture. Sous une
//! rotation ou une échelle non uniforme, le glyphe rastérisé à une échelle approchée
//! (huitième près) est transformé par le shader (échantillonnage bilinéaire).
use super::{Bind, Builder, shapes::Fill};
use crate::{
    platform::{
        linux::graphics::gpu::{device_position, quarter},
        shared::{
            gpu::{
                Instance, PAINT_LINEAR, PAINT_MASK, PAINT_RADIAL, PAINT_TEXTURE, SHAPE_NONE,
                color as array, pixel,
            },
            linear_points,
        },
    },
    ui::{
        rendering::{brush::Background, transform::Affine},
        scene::{PointF, RectF},
        styling::text::RichParagraph,
        theme::Color,
    },
};

/// Peinture des glyphes sous un remplissage, dans le repère de leurs quads.
struct GlyphPaint {
    kind: u32,
    paint: [f32; 4],
    stops: [u32; 2],
    color: [f32; 4],
}

impl Builder<'_> {
    pub(super) fn text(
        &mut self,
        paragraph: &RichParagraph,
        rect: RectF,
        color: Color,
        wrap: bool,
    ) -> crate::Result<()> {
        let glyphs = self.cairo.text_glyphs(paragraph, rect, color, wrap)?;
        let fill = self.text_fills.last().cloned();
        let saved = self.state;
        if glyphs.clip {
            self.clip_rect(rect);
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
        let corner = PointF {
            x: rect.left,
            y: rect.top,
        };
        for run in glyphs.runs.iter() {
            let ink = array(run.color.unwrap_or(color));
            let font = (self.resources.atlas.font(&run.font), &run.font);
            let start = run.start(corner);
            // Sous un remplissage, un émoji est rastérisé en alpha seul.
            let colors = run.colored && fill.is_none();
            for (&glyph, offset) in run.glyphs.iter().zip(&run.offsets) {
                let (quad, m, entry) = if straight {
                    let s = f64::from(t.m11);
                    let at = |start: f64, offset: f32, translate: f32| {
                        let v = device_position(start, f64::from(offset), s, f64::from(translate));
                        quarter(v, run.colored)
                    };
                    let ((x, fx), (y, fy)) =
                        (at(start[0], offset.x, t.dx), at(start[1], offset.y, t.dy));
                    let Some(e) = self.resources.atlas.glyph(
                        &mut self.resources.staging,
                        font,
                        glyph,
                        scale,
                        (fx, fy),
                        colors,
                    )?
                    else {
                        continue;
                    };
                    let (left, top) = ((x + e.left) as f32, (y + e.top) as f32);
                    let quad = RectF {
                        left,
                        top,
                        right: left + e.width as f32,
                        bottom: top + e.height as f32,
                    };
                    (quad, Affine::IDENTITY, e)
                } else {
                    let Some(e) = self.resources.atlas.glyph(
                        &mut self.resources.staging,
                        font,
                        glyph,
                        scale,
                        (0, 0),
                        colors,
                    )?
                    else {
                        continue;
                    };
                    let p = PointF {
                        x: start[0] as f32 + offset.x,
                        y: start[1] as f32 + offset.y,
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
                let bind = Bind::Texture(self.resources.atlas.view(entry.page));
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
