//! Mémoire des mesures d'une passe de layout : sur des arbres tirés au hasard (lignes,
//! colonnes et grilles, flex CSS ou historique, retour à la ligne, tailles fixes, en
//! pourcentage ou souples, limites fixes et relatives, ligne de base, positionnés,
//! textes), la passe mémoïsée laisse exactement le même arbre qu'une passe complète :
//! rectangles, découpes, tailles naturelles et limites résolues.
use crate::ui::tests::support::*;

/// Générateur congruentiel : arbres reproductibles sans dépendance.
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) as u32
    }
    fn below(&mut self, n: u32) -> u32 {
        self.next() % n
    }
    fn length(&mut self) -> Length {
        match self.below(6) {
            0 => Length::Px(20. + self.below(200) as f32 + 0.25),
            1 => Length::Percent(10. + self.below(80) as f32),
            2 => Length::Fill(1. + self.below(3) as f32),
            _ => Length::Auto,
        }
    }
}
const WORDS: [&str; 8] = [
    "Rumours",
    "Abbey Road",
    "The Dark Side of the Moon",
    "OK",
    "Kind of Blue — édition remasterisée",
    "Interminablement",
    "Blue Train et autres titres",
    "",
];

fn tree(rng: &mut Lcg, id: &mut u64, depth: u32) -> Element {
    *id += 1;
    let me = *id;
    if depth == 0 || rng.below(4) == 0 {
        let words = (0..1 + rng.below(4))
            .map(|_| WORDS[rng.below(WORDS.len() as u32) as usize])
            .collect::<Vec<_>>()
            .join(" ");
        let mut leaf = if rng.below(5) == 0 {
            box_node(me, rng.length(), Length::Px(10. + rng.below(40) as f32))
        } else {
            widgets::text(me, words)
        };
        if rng.below(4) == 0 {
            leaf = leaf.width(rng.length());
        }
        return leaf;
    }
    let count = 1 + rng.below(5);
    let children = (0..count)
        .map(|_| tree(rng, id, depth - 1))
        .collect::<Vec<_>>();
    let mut node = match rng.below(5) {
        0 => Element::grid(me, 1 + rng.below(3) as usize, children),
        1 | 2 => Element::row(me, children),
        _ => Element::column(me, children),
    }
    .gap(rng.below(12) as f32)
    .padding(rng.below(10) as f32);
    node = node.width(rng.length());
    if rng.below(3) == 0 {
        node = node.height(rng.length());
    }
    if rng.below(4) == 0 {
        node = node.min_width(rng.below(120) as f32);
    }
    if rng.below(4) == 0 {
        node = node.max_width(80. + rng.below(300) as f32);
    }
    node = node.align(match rng.below(5) {
        0 => Align::Start,
        1 => Align::Center,
        2 => Align::End,
        3 => Align::Baseline,
        _ => Align::Stretch,
    });
    if rng.below(2) == 0 && !matches!(node.layout, Layout::Grid { .. }) {
        let rules = node.layout_rules();
        rules.css_flex = true;
        if rng.below(3) == 0 {
            rules.flex_wrap = FlexWrap::Wrap;
        }
    }
    for child in &mut node.children {
        if rng.below(4) == 0 {
            let rules = child.layout_rules();
            rules.flex_grow = Some(rng.below(3) as f32);
            rules.flex_shrink = Some(rng.below(2) as f32);
            if rng.below(2) == 0 {
                rules.flex_basis = Some(Length::Px(rng.below(150) as f32));
            }
        }
        if rng.below(10) == 0 {
            // `max-width: 40%` : limite relative résolue à chaque mesure.
            child.layout_rules().relative_limits[2] =
                Some(Length::Percent(30. + rng.below(50) as f32));
        }
        if rng.below(14) == 0 {
            let rules = child.layout_rules();
            rules.position = Position::Absolute;
            rules.offsets = [Some(Length::Px(4.)), None, None, Some(Length::Px(6.))];
        }
        if rng.below(12) == 0 {
            child.layout_rules().align_self = Some(Align::Baseline);
        }
    }
    node
}

/// État laissé par la passe sur chaque élément.
fn dump(node: &Element, out: &mut Vec<[u32; 16]>) {
    let r = |r: RectF| [r.left, r.top, r.right, r.bottom].map(f32::to_bits);
    let [a, b, c, d] = r(node.rect);
    let [e, f, g, h] = r(node.clip);
    let s = &node.style;
    out.push([
        node.id.0 as u32,
        a,
        b,
        c,
        d,
        e,
        f,
        g,
        h,
        node.intrinsic.width.to_bits(),
        node.intrinsic.height.to_bits(),
        s.min_width.to_bits(),
        s.max_width.to_bits(),
        s.min_height.to_bits(),
        s.max_height.to_bits(),
        node.children.len() as u32,
    ]);
    for child in &node.children {
        dump(child, out);
    }
}

/// Mesure puis place `root` comme le runtime, mémoire active ou non, à plusieurs tailles
/// de suite (l'état d'une passe sert d'entrée à la suivante).
fn laid_out(seed: u64, memo: bool) -> crate::Result<Vec<Vec<[u32; 16]>>> {
    let mut rng = Lcg(seed);
    let mut id = 0;
    let mut root = Element::column(0, vec![tree(&mut rng, &mut id, 4)]);
    let mut text = Measure::default();
    let mut passes = Vec::new();
    for (width, height) in [(640., 480.), (517.3, 480.), (903.7, 333.), (517.3, 480.)] {
        layout::validate(&root)?;
        let pass = layout::LayoutPass::begin(&mut root, memo);
        let available = SizeF { width, height };
        let bounds = RectF::new(0., 0., width, height);
        layout::measure(&mut root, available, &mut text)?;
        layout::arrange(&mut root, bounds, bounds, &mut text)?;
        drop(pass);
        let mut out = Vec::new();
        dump(&root, &mut out);
        passes.push(out);
    }
    Ok(passes)
}

#[test]
fn memoized_measures_leave_the_same_tree_as_a_full_layout() {
    let mut compared = 0;
    for seed in 0..400 {
        let full = laid_out(seed, false);
        let memo = laid_out(seed, true);
        match (full, memo) {
            (Ok(full), Ok(memo)) => {
                for (pass, (a, b)) in full.iter().zip(&memo).enumerate() {
                    if let Some(i) = (0..a.len()).find(|&i| a[i] != b[i]) {
                        panic!("seed {seed}, pass {pass}: {:?} != {:?}", a[i], b[i]);
                    }
                }
                compared += 1;
            }
            (Err(_), Err(_)) => {}
            (full, memo) => panic!("seed {seed}: {:?} / {:?}", full.is_ok(), memo.is_ok()),
        }
    }
    assert!(compared > 300, "{compared} trees laid out");
}
