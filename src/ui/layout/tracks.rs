use super::*;
/// Size of one bound of a grid track, like a CSS `grid-template-columns` value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TrackBreadth {
    /// Fixed size in logical pixels (finite, non-negative).
    Px(f32),
    /// Percentage of the grid's width (finite, non-negative).
    Percent(f32),
    /// Flexible share of the remaining space (finite, positive); not allowed as a minimum.
    Fr(f32),
    /// Sized to the content of the track.
    Auto,
    /// Widest content of the track, without sharing the free space (`max-content`).
    MaxContent,
    /// Narrowest width of the content without overflow: its longest word or widest
    /// unbreakable item (`min-content`).
    MinContent,
    /// `fit-content(limit)`: the content width, at most `limit` pixels.
    FitContent(f32),
}
/// A grid column definition with minimum and maximum breadths, like CSS `minmax()`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridTrack {
    /// Minimum breadth; must not be [`TrackBreadth::Fr`].
    pub min: TrackBreadth,
    /// Maximum breadth.
    pub max: TrackBreadth,
}
impl GridTrack {
    /// Track with a single breadth; for `Fr` the minimum becomes 0 px.
    pub const fn new(value: TrackBreadth) -> Self {
        Self {
            min: match value {
                TrackBreadth::Fr(_) => TrackBreadth::Px(0.),
                other => other,
            },
            max: value,
        }
    }
    /// Track with explicit minimum and maximum, like CSS `minmax(min, max)`.
    pub const fn minmax(min: TrackBreadth, max: TrackBreadth) -> Self {
        Self { min, max }
    }
    /// Returns `true` if both breadths are in range and the minimum is not `Fr`.
    pub fn valid(self) -> bool {
        let valid = |v| match v {
            TrackBreadth::Px(n) | TrackBreadth::Percent(n) | TrackBreadth::FitContent(n) => {
                n.is_finite() && n >= 0.
            }
            TrackBreadth::Fr(n) => n.is_finite() && n > 0.,
            TrackBreadth::Auto | TrackBreadth::MaxContent | TrackBreadth::MinContent => true,
        };
        valid(self.min) && valid(self.max) && !matches!(self.min, TrackBreadth::Fr(_))
    }
}
/// Hauteurs des lignes explicites (`grid-template-rows`) appliquées aux hauteurs de
/// contenu ; `available` (hauteur de la grille) partage l'espace libre entre les `fr`.
/// Les lignes implicites restent à la hauteur de leur contenu.
pub(super) fn rows(node: &Element, heights: &mut Vec<f32>, available: Option<f32>, gap: f32) {
    let Some(tracks) = node.rules.as_ref().and_then(|r| r.rows.clone()) else {
        return;
    };
    if heights.len() < tracks.len() {
        heights.resize(tracks.len(), 0.);
    }
    let breadth = |b: TrackBreadth, content: f32| match b {
        TrackBreadth::Px(n) => Some(n),
        TrackBreadth::Percent(n) => Some(available.map_or(0., |a| a * n / 100.)),
        TrackBreadth::Auto | TrackBreadth::MaxContent | TrackBreadth::MinContent => Some(content),
        TrackBreadth::FitContent(n) => Some(content.min(n)),
        TrackBreadth::Fr(_) => None,
    };
    let mut fractions = 0.;
    for (track, height) in tracks.iter().zip(heights.iter_mut()) {
        if !track.valid() {
            continue;
        }
        let content = *height;
        let min = breadth(track.min, content).unwrap_or(0.);
        match track.max {
            TrackBreadth::Fr(n) => {
                fractions += n;
                *height = content.max(min);
            }
            max => {
                *height = content
                    .max(min)
                    .min(breadth(max, content).unwrap_or(content).max(min))
            }
        }
    }
    if let Some(available) = available.filter(|_| fractions > 0.) {
        let fixed = tracks
            .iter()
            .zip(heights.iter())
            .filter(|(t, _)| !matches!(t.max, TrackBreadth::Fr(_)))
            .map(|(_, h)| *h)
            .sum::<f32>()
            + heights.iter().skip(tracks.len()).sum::<f32>()
            + gap * heights.len().saturating_sub(1) as f32;
        let free = (available - fixed).max(0.);
        for (track, height) in tracks.iter().zip(heights.iter_mut()) {
            if let TrackBreadth::Fr(n) = track.max {
                *height = height.max(free * n / fractions);
            }
        }
    }
}
pub(super) fn columns(
    node: &mut Element,
    size: SizeF,
    positions: &[(usize, usize, usize)],
    count: usize,
    text: &mut dyn TextMeasurer,
) -> crate::Result<Vec<f32>> {
    let gap = node.style.horizontal_gap();
    let available = (size.width - gap * count.saturating_sub(1) as f32).max(0.);
    let Some(tracks) = node.rules.as_ref().and_then(|r| r.columns.clone()) else {
        return Ok(vec![available / count as f32; count]);
    };
    if tracks.len() != count || tracks.iter().any(|t| !t.valid()) {
        return Err(invalid("invalid grid tracks"));
    }
    let mut natural = vec![0f32; count];
    let mut minimum = vec![0f32; count];
    for (child, &(_, col, span)) in node.children.iter().zip(positions) {
        if span == 1
            && (matches!(tracks[col].min, TrackBreadth::MinContent)
                || matches!(tracks[col].max, TrackBreadth::MinContent))
        {
            let inherited = node
                .styles
                .as_ref()
                .map(|s| s.computed.text)
                .unwrap_or_default();
            minimum[col] = minimum[col].max(min_content(child, inherited, text)?);
        }
    }
    let content = |b: TrackBreadth| {
        matches!(
            b,
            TrackBreadth::Auto | TrackBreadth::MaxContent | TrackBreadth::FitContent(_)
        )
    };
    for (child, &(_, col, span)) in node.children.iter_mut().zip(positions) {
        if span == 1 && (content(tracks[col].min) || content(tracks[col].max)) {
            let previous = child.style.width;
            if matches!(previous, Length::Fill(_)) {
                child.style.width = Length::Auto;
            }
            let measured = measure(child, size, text);
            child.style.width = previous;
            natural[col] = natural[col].max(measured?.width);
        }
    }
    let length = |v, col: usize| match v {
        TrackBreadth::Px(n) => n,
        TrackBreadth::Percent(n) => size.width * n / 100.,
        TrackBreadth::Auto | TrackBreadth::MaxContent => natural[col],
        TrackBreadth::MinContent => minimum[col],
        TrackBreadth::FitContent(n) => natural[col].min(n),
        TrackBreadth::Fr(_) => 0.,
    };
    let mut widths: Vec<f32> = tracks
        .iter()
        .enumerate()
        .map(|(i, t)| length(t.min, i))
        .collect();
    let mut limits = Vec::with_capacity(count);
    let mut weights = Vec::with_capacity(count);
    for (i, t) in tracks.iter().enumerate() {
        match t.max {
            TrackBreadth::Fr(n) => {
                limits.push(f32::INFINITY);
                weights.push(n);
            }
            TrackBreadth::Auto => {
                widths[i] = widths[i].max(natural[i]);
                limits.push(f32::INFINITY);
                weights.push(0.);
            }
            other => {
                let n = length(other, i).max(widths[i]);
                widths[i] = n;
                limits.push(n);
                weights.push(0.);
            }
        }
    }
    for _ in 0..=count {
        let fixed = widths
            .iter()
            .zip(&weights)
            .filter(|(_, w)| **w == 0.)
            .map(|(v, _)| *v)
            .sum::<f32>();
        let total = weights.iter().sum::<f32>();
        if total <= 0. {
            break;
        }
        let unit = (available - fixed).max(0.) / total;
        let mut frozen = false;
        for i in 0..count {
            if weights[i] > 0. && unit * weights[i] < widths[i] {
                weights[i] = 0.;
                frozen = true;
            }
        }
        if !frozen {
            for i in 0..count {
                if weights[i] > 0. {
                    widths[i] = (unit * weights[i]).min(limits[i]);
                }
            }
            break;
        }
    }
    let free = (available - widths.iter().sum::<f32>()).max(0.);
    let autos = tracks
        .iter()
        .filter(|t| matches!(t.max, TrackBreadth::Auto))
        .count();
    if autos > 0 {
        for (i, t) in tracks.iter().enumerate() {
            if matches!(t.max, TrackBreadth::Auto) {
                widths[i] += free / autos as f32;
            }
        }
    }
    Ok(widths)
}
/// Largeur min-content d'un élément (boîte de marge) : le plus long mot d'un texte
/// (mesuré seul avec la typographie calculée), la largeur naturelle d'une autre feuille,
/// la somme des enfants d'une rangée flex sans retour à la ligne, sinon leur maximum.
/// Une largeur en px est gardée telle quelle.
pub(crate) fn min_content(
    node: &Element,
    inherited: crate::ui::styling::Typography,
    text: &mut dyn TextMeasurer,
) -> crate::Result<f32> {
    let cached = MIN_CONTENT.with(|memo| {
        memo.borrow()
            .as_ref()?
            .get(&node.id)
            .filter(|(typography, _)| *typography == inherited)
            .map(|(_, width)| *width)
    });
    if let Some(width) = cached {
        return Ok(width);
    }
    let width = min_content_uncached(node, inherited, text)?;
    MIN_CONTENT.with(|memo| {
        if let Some(memo) = memo.borrow_mut().as_mut() {
            memo.insert(node.id, (inherited, width));
        }
    });
    Ok(width)
}
type WordFormat = (crate::ui::font::Font, u32, u32, bool, bool);
thread_local! {
    /// Largeur des mots isolés (min-content) : des milliers de mots différents par page,
    /// trop pour le cache de mises en page du moteur de texte, qui recréerait chacune.
    static WORDS: std::cell::RefCell<
        std::collections::HashMap<WordFormat, std::collections::HashMap<Box<str>, f32>>,
    > = Default::default();
}
fn word_width(
    word: &str,
    format: crate::ui::styling::text::TextFormat,
    text: &mut dyn TextMeasurer,
) -> crate::Result<f32> {
    let key = (
        format.font,
        format.size.to_bits(),
        format.letter_spacing.to_bits(),
        format.italic,
        format.tabular_numbers,
    );
    if let Some(width) = WORDS.with(|w| w.borrow().get(&key)?.get(word).copied()) {
        return Ok(width);
    }
    let width = text.measure_formatted(word, format, 1e6)?.width.ceil();
    WORDS.with(|words| {
        let mut words = words.borrow_mut();
        if words.values().map(|w| w.len()).sum::<usize>() >= 16_384 {
            words.clear();
        }
        words.entry(key).or_default().insert(word.into(), width);
    });
    Ok(width)
}
/// Mesure d'une feuille, mémorisée le temps d'une passe de layout pour une même place
/// offerte : un layout flex imbriqué redemande la même mesure à chaque niveau.
pub(crate) fn measure_leaf(
    id: ElementId,
    available: SizeF,
    measure: impl FnOnce() -> crate::Result<SizeF>,
) -> crate::Result<SizeF> {
    let key = (id, available.width.to_bits(), available.height.to_bits());
    if let Some(size) = LEAVES.with(|memo| memo.borrow().as_ref()?.get(&key).copied()) {
        return Ok(size);
    }
    let size = measure()?;
    LEAVES.with(|memo| {
        if let Some(memo) = memo.borrow_mut().as_mut() {
            memo.insert(key, size);
        }
    });
    Ok(size)
}
type LeafKey = (ElementId, u32, u32);
type FastMap<K, V> = crate::ui::core::hash::FastMap<K, V>;
thread_local! {
    static LEAVES: std::cell::RefCell<Option<FastMap<LeafKey, SizeF>>> =
        const { std::cell::RefCell::new(None) };
    /// Largeurs min-content d'une passe de layout du runtime. Elles ne dépendent pas de la
    /// place offerte : une grille mesurée plusieurs fois par ses parents flex ne remesure
    /// plus chaque mot de chaque texte (31 000 mesures par image sur l'accueil RustMusic).
    static MIN_CONTENT: std::cell::RefCell<
        Option<FastMap<ElementId, (crate::ui::styling::Typography, f32)>>,
    > = const { std::cell::RefCell::new(None) };
}
/// Mémoïsation des largeurs min-content et des mesures (voir `memo`) le temps d'une
/// passe de layout de `root` (arbre figé) ; retirée quand la valeur rendue est
/// abandonnée. Sans `measures`, chaque mesure d'élément est recalculée.
pub(crate) struct LayoutPass {
    measures: bool,
}
impl LayoutPass {
    pub(crate) fn begin(root: &mut Element, measures: bool) -> Self {
        Self::set(true, measures.then_some(root));
        Self { measures }
    }
    /// Oublie les mesures : les styles ont été résolus de nouveau (requêtes de conteneur).
    pub(crate) fn reset(&self, root: &mut Element) {
        Self::set(true, self.measures.then_some(root));
    }
    fn set(active: bool, root: Option<&mut Element>) {
        MIN_CONTENT.with(|memo| *memo.borrow_mut() = active.then(Default::default));
        LEAVES.with(|memo| *memo.borrow_mut() = active.then(Default::default));
        super::memo::begin(root);
    }
}
impl Drop for LayoutPass {
    fn drop(&mut self) {
        Self::set(false, None);
    }
}
fn min_content_uncached(
    node: &Element,
    inherited: crate::ui::styling::Typography,
    text: &mut dyn TextMeasurer,
) -> crate::Result<f32> {
    if super::position::hidden(node) {
        return Ok(0.);
    }
    // Typographie de l'élément, sinon celle héritée de ses ancêtres stylés.
    let typography = node
        .styles
        .as_ref()
        .map_or(inherited, |s| s.computed.text.over(inherited));
    let edges = box_edges(node).horizontal() + node.style.margin.horizontal();
    if let Length::Px(n) = node.style.width {
        return Ok(n + node.style.margin.horizontal());
    }
    let inner = if super::containment::skipped(node) {
        super::containment::placeholder(node).width
    } else if matches!(node.layout, Layout::Leaf) {
        match &node.widget {
            Some(widget)
                if matches!(
                    widget.semantics().role,
                    Role::Text | Role::Heading | Role::Link
                ) =>
            {
                let format = crate::ui::styling::text::TextFormat::new(
                    typography,
                    crate::ui::font::Font::system(false),
                    16.,
                );
                let mut widest = 0f32;
                for word in widget.semantics().name.split_whitespace() {
                    widest = widest.max(word_width(word, format, text)?);
                }
                widest
            }
            // Autre feuille (icône, image, contrôle) : mesurée à la place la plus étroite,
            // sans attendre une mesure du layout (sa taille naturelle valait 0 à la
            // première mise en page, puis celle de la colonne qu'elle avait reçue).
            Some(widget) => {
                widget
                    .measure(
                        SizeF {
                            width: 0.,
                            height: f32::INFINITY,
                        },
                        text,
                    )?
                    .width
            }
            None => 0.,
        }
    } else {
        let row = matches!(node.layout, Layout::Row)
            && node
                .rules
                .as_ref()
                .is_none_or(|r| r.flex_wrap == FlexWrap::NoWrap);
        let mut total = 0f32;
        let mut count = 0usize;
        for child in &node.children {
            if super::position::hidden(child) || super::position::out_of_flow(child) {
                continue;
            }
            let width = min_content(child, typography, text)?;
            total = if row { total + width } else { total.max(width) };
            count += 1;
        }
        if row {
            total += node.style.horizontal_gap() * count.saturating_sub(1) as f32;
        }
        total
    };
    let (min, max) = limits(node, true);
    Ok(inner.clamp(min, max.max(min)) + edges)
}
