//! Zones modifiées d'une image à l'autre, pour ne redessiner que ce qui change.
//!
//! Chaque élément peint reçoit une empreinte (sa commande et l'état qui l'entoure :
//! transformations, opacité, découpes, masques) et ses bornes en pixels de la cible.
//! Les éléments apparus, disparus ou modifiés entre deux scènes forment la zone à
//! redessiner ; un groupe filtré compte pour un seul élément. Un `backdrop-filter`
//! touché par la zone l'agrandit à toute la surface qu'il relit.
use super::{TextCache, Typesetter, plain, simple_format};
use crate::ui::{
    rendering::{
        filter::{Filter, Filters},
        transform::Affine,
    },
    scene::{DrawCommand, RectF, Scene, signature::Signature},
};
use std::collections::HashMap;

/// Rectangle entier en pixels : gauche, haut, droite, bas.
pub(in crate::platform) type Pixels = [i32; 4];
/// Inventaire d'une scène peinte.
#[derive(Clone)]
pub(in crate::platform) struct Damage {
    items: HashMap<(u64, Pixels), u32>,
    backdrops: Vec<Pixels>,
    target: (usize, usize, u32, u32),
}
/// Ce qui doit être redessiné.
pub(in crate::platform) enum Change {
    /// Rien n'a changé.
    None,
    /// Toute la surface.
    Full,
    /// Seulement ces rectangles, disjoints.
    Region(Vec<Pixels>),
}
#[derive(Clone, Copy)]
struct State {
    transform: Affine,
    clip: RectF,
    hash: u64,
}
fn mix(a: u64, b: u64) -> u64 {
    (a.rotate_left(5) ^ b).wrapping_mul(0x9e37_79b9_7f4a_7c15)
}
fn digest(commands: &[DrawCommand]) -> u64 {
    Signature::new(commands, RectF::new(0., 0., 0., 0.), Filters::new(&[]), 0.).digest()
}
fn intersect(a: RectF, b: RectF) -> RectF {
    RectF {
        left: a.left.max(b.left),
        top: a.top.max(b.top),
        right: a.right.min(b.right),
        bottom: a.bottom.min(b.bottom),
    }
}
fn grow(r: RectF, by: f32) -> RectF {
    RectF {
        left: r.left - by,
        top: r.top - by,
        right: r.right + by,
        bottom: r.bottom + by,
    }
}
fn pixels(r: RectF) -> Option<Pixels> {
    // Un pixel de marge pour l'anticrénelage.
    let p = [
        r.left.floor() as i32 - 1,
        r.top.floor() as i32 - 1,
        r.right.ceil() as i32 + 1,
        r.bottom.ceil() as i32 + 1,
    ];
    (r.right > r.left && r.bottom > r.top).then_some(p)
}
fn union(a: Option<Pixels>, b: Pixels) -> Pixels {
    match a {
        Some(a) => [
            a[0].min(b[0]),
            a[1].min(b[1]),
            a[2].max(b[2]),
            a[3].max(b[3]),
        ],
        None => b,
    }
}
fn overlaps(a: Pixels, b: Pixels) -> bool {
    a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3]
}
fn contains(a: Pixels, b: Pixels) -> bool {
    a[0] <= b[0] && a[1] <= b[1] && a[2] >= b[2] && a[3] >= b[3]
}
/// Index de la commande qui ferme la portée ouverte en `start`.
fn scope_end(commands: &[DrawCommand], start: usize) -> usize {
    let mut depth = 0isize;
    for (index, command) in commands.iter().enumerate().skip(start) {
        depth += match command {
            DrawCommand::PushClip(_)
            | DrawCommand::PushShapeClip { .. }
            | DrawCommand::PushEffect { .. }
            | DrawCommand::PushFilter { .. }
            | DrawCommand::PushMask { .. }
            | DrawCommand::PushTextFill { .. } => 1,
            DrawCommand::PopClip
            | DrawCommand::PopShapeClip
            | DrawCommand::PopEffect
            | DrawCommand::PopFilter
            | DrawCommand::PopMask
            | DrawCommand::PopTextFill => -1,
            _ => 0,
        };
        if depth == 0 {
            return index;
        }
    }
    commands.len().saturating_sub(1)
}
/// Bornes logiques de ce que peint un texte : sa boîte, et ses lignes si elles en
/// débordent (texte non découpé, italique, ellipse).
fn text_bounds<T: Typesetter>(
    fonts: &mut T,
    cache: &mut TextCache<T>,
    paragraph: &crate::ui::styling::text::RichParagraph,
    rect: RectF,
    wrap: bool,
) -> crate::Result<RectF> {
    let layout = cache.layout(fonts, paragraph, rect.right - rect.left, wrap)?;
    let size = paragraph.format.size;
    let (left, laid) = (T::left(&layout), T::size(&layout));
    let ink = RectF {
        left: rect.left + left,
        top: rect.top,
        right: rect.right.max(rect.left + left + laid.width),
        bottom: rect.bottom.max(rect.top + laid.height),
    };
    let ink = grow(ink, size * 0.5);
    Ok(if paragraph.format.clip {
        intersect(ink, grow(rect, 1.))
    } else {
        ink
    })
}
impl Damage {
    /// Inventaire de `scene` peinte à `scale` sur une cible de `width` × `height` pixels.
    pub fn new<T: Typesetter>(
        scene: &Scene,
        scale: f32,
        (width, height): (usize, usize),
        fonts: &mut T,
        cache: &mut TextCache<T>,
    ) -> crate::Result<Self> {
        let clear = u32::from_le_bytes([
            scene.clear.red,
            scene.clear.green,
            scene.clear.blue,
            scene.clear.alpha,
        ]);
        let mut damage = Self {
            items: HashMap::new(),
            backdrops: Vec::new(),
            target: (width, height, scale.to_bits(), clear),
        };
        let mut state = State {
            transform: Affine {
                m11: scale,
                m22: scale,
                ..Affine::IDENTITY
            },
            clip: RectF::new(0., 0., width as f32, height as f32),
            hash: 0,
        };
        let mut stack = Vec::new();
        let commands = &scene.commands;
        let mut index = 0;
        while index < commands.len() {
            let command = &commands[index];
            let own = || digest(std::slice::from_ref(command));
            let local = match command {
                DrawCommand::PushFilter {
                    bounds, filters, ..
                } => {
                    let end = scope_end(commands, index);
                    let content =
                        Signature::new(&commands[index + 1..end], *bounds, *filters, scale);
                    damage.item(state, mix(own(), content.digest()), filters.bounds(*bounds));
                    index = end + 1;
                    continue;
                }
                DrawCommand::PushEffect {
                    transform, opacity, ..
                } => {
                    if *opacity <= 1. / 512. {
                        index = scope_end(commands, index) + 1;
                        continue;
                    }
                    stack.push(state);
                    state.transform = transform.then(state.transform);
                    state.hash = mix(state.hash, own());
                    None
                }
                // Une découpe rectangulaire n'entre pas dans l'empreinte : son effet est
                // déjà dans les bornes découpées des éléments (une découpe ajoutée autour
                // de la page à l'ouverture d'un menu ne change rien à l'écran). Les coins
                // d'une découpe arrondie, eux, changent des pixels.
                DrawCommand::PushClip(rect) => {
                    stack.push(state);
                    state.clip = intersect(state.clip, state.transform.bounds(*rect));
                    None
                }
                DrawCommand::PushShapeClip { rect, .. } => {
                    stack.push(state);
                    state.clip = intersect(state.clip, state.transform.bounds(*rect));
                    state.hash = mix(state.hash, own());
                    None
                }
                DrawCommand::PushMask { .. } | DrawCommand::PushTextFill { .. } => {
                    stack.push(state);
                    state.hash = mix(state.hash, own());
                    None
                }
                DrawCommand::PopClip
                | DrawCommand::PopShapeClip
                | DrawCommand::PopEffect
                | DrawCommand::PopFilter
                | DrawCommand::PopMask
                | DrawCommand::PopTextFill => {
                    state = stack.pop().unwrap_or(state);
                    None
                }
                DrawCommand::Backdrop { rect, filters, .. } => {
                    let blur = filters
                        .values
                        .iter()
                        .map(|f| match f {
                            Filter::Blur(n) => *n,
                            _ => 0.,
                        })
                        .sum::<f32>()
                        * filters.scale;
                    let read = intersect(
                        state.transform.bounds(grow(*rect, (blur * 3.).ceil())),
                        state.clip,
                    );
                    damage.backdrops.extend(pixels(read));
                    Some(*rect)
                }
                DrawCommand::Shape { rect, stroke, .. } => {
                    Some(grow(*rect, stroke.map_or(0., |(w, _)| w)))
                }
                DrawCommand::Stroke { rect, width, .. } => Some(grow(*rect, *width)),
                DrawCommand::Fill { rect, .. }
                | DrawCommand::Gradient { rect, .. }
                | DrawCommand::RadialGradient { rect, .. }
                | DrawCommand::Image { rect, .. }
                | DrawCommand::Vector { rect, .. }
                | DrawCommand::Svg { rect, .. } => Some(*rect),
                DrawCommand::Line {
                    start, end, width, ..
                } => Some(grow(
                    RectF {
                        left: start.x.min(end.x),
                        top: start.y.min(end.y),
                        right: start.x.max(end.x),
                        bottom: start.y.max(end.y),
                    },
                    *width,
                )),
                DrawCommand::Path { points, .. } => points.first().map(|first| {
                    let r = points.iter().fold(
                        RectF {
                            left: first.x,
                            top: first.y,
                            right: first.x,
                            bottom: first.y,
                        },
                        |r, p| RectF {
                            left: r.left.min(p.x),
                            top: r.top.min(p.y),
                            right: r.right.max(p.x),
                            bottom: r.bottom.max(p.y),
                        },
                    );
                    grow(r, 2.)
                }),
                DrawCommand::FormattedText {
                    rect,
                    text: value,
                    format,
                    wrap,
                    ..
                } => Some(text_bounds(
                    fonts,
                    cache,
                    &plain(value, *format),
                    *rect,
                    *wrap,
                )?),
                DrawCommand::RichText {
                    rect,
                    paragraph,
                    wrap,
                    ..
                } => Some(text_bounds(fonts, cache, paragraph, *rect, *wrap)?),
                DrawCommand::Text {
                    rect,
                    text: value,
                    size,
                    bold,
                    wrap,
                    ..
                } => {
                    let format = simple_format(crate::ui::font::Font::system(*bold), *size);
                    Some(text_bounds(
                        fonts,
                        cache,
                        &plain(value, format),
                        *rect,
                        *wrap,
                    )?)
                }
                DrawCommand::FontText {
                    rect,
                    text: value,
                    size,
                    font,
                    wrap,
                    ..
                } => {
                    let format = simple_format(*font, *size);
                    Some(text_bounds(
                        fonts,
                        cache,
                        &plain(value, format),
                        *rect,
                        *wrap,
                    )?)
                }
            };
            if let Some(local) = local {
                damage.item(state, own(), local);
            }
            index += 1;
        }
        Ok(damage)
    }
    fn item(&mut self, state: State, hash: u64, local: RectF) {
        let device = intersect(state.transform.bounds(local), state.clip);
        if let Some(p) = pixels(device) {
            *self.items.entry((mix(state.hash, hash), p)).or_default() += 1;
        }
    }
    /// Zone à redessiner pour passer de `previous` à `self`.
    pub fn change(&self, previous: &Self) -> Change {
        if self.target != previous.target {
            return Change::Full;
        }
        let mut dirty = Vec::new();
        for (key, count) in &self.items {
            if previous.items.get(key) != Some(count) {
                dirty.push(key.1);
            }
        }
        for (key, count) in &previous.items {
            if self.items.get(key) != Some(count) {
                dirty.push(key.1);
            }
        }
        if dirty.is_empty() {
            return Change::None;
        }
        merge(&mut dirty);
        // Un `backdrop-filter` relit les pixels sous lui : s'il touche une zone, tout ce
        // qu'il relit est redessiné, jusqu'à stabilité.
        loop {
            let mut grown = false;
            for b in self.backdrops.iter().chain(&previous.backdrops) {
                if dirty.iter().any(|d| overlaps(*d, *b)) && !dirty.iter().any(|d| contains(*d, *b))
                {
                    dirty.push(*b);
                    grown = true;
                }
            }
            if !grown {
                break;
            }
            merge(&mut dirty);
        }
        let (width, height) = (self.target.0 as i32, self.target.1 as i32);
        let mut zones: Vec<Pixels> = dirty
            .into_iter()
            .map(|d| [d[0].max(0), d[1].max(0), d[2].min(width), d[3].min(height)])
            .filter(|d| d[2] > d[0] && d[3] > d[1])
            .collect();
        merge(&mut zones);
        let area: i64 = zones
            .iter()
            .map(|d| i64::from(d[2] - d[0]) * i64::from(d[3] - d[1]))
            .sum();
        if area == 0 {
            Change::None
        } else if area * 10 > i64::from(width) * i64::from(height) * 6 {
            Change::Full
        } else {
            Change::Region(zones)
        }
    }
}
/// Réunit les zones qui se chevauchent ou se touchent presque (16 pixels) ; au-delà de 24
/// zones, une seule zone englobante.
fn merge(zones: &mut Vec<Pixels>) {
    let near = |a: Pixels, b: Pixels| {
        a[0] < b[2] + 16 && b[0] < a[2] + 16 && a[1] < b[3] + 16 && b[1] < a[3] + 16
    };
    'restart: loop {
        for i in 0..zones.len() {
            for j in i + 1..zones.len() {
                if near(zones[i], zones[j]) {
                    let b = zones.swap_remove(j);
                    zones[i] = union(Some(zones[i]), b);
                    continue 'restart;
                }
            }
        }
        break;
    }
    if zones.len() > 24 {
        let all = zones.iter().copied().fold(None, |a, b| Some(union(a, b)));
        *zones = all.into_iter().collect();
    }
}
