pub(super) use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Combinator {
    Descendant,
    Child,
    Next,
    Subsequent,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttrOp {
    Exists,
    Equals,
    Includes,
    Dash,
    Prefix,
    Suffix,
    Substring,
}
#[derive(Clone, Debug, PartialEq)]
pub struct AttrSel {
    pub name: String,
    pub op: AttrOp,
    pub value: String,
    pub insensitive: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub enum Pseudo {
    /// Pseudo-classe évaluée à l'exécution (variante de `StyleState`).
    State(&'static str),
    /// `:nth-child(an+b)`, `:nth-last-child`, `:nth-of-type`, `:nth-last-of-type`.
    Nth {
        variant: &'static str,
        a: i32,
        b: i32,
    },
    Not(Vec<Complex>),
    Is(Vec<Complex>),
    Where(Vec<Complex>),
    /// `:root` / `:host` : racine du document (premier élément balisé).
    Root,
    /// Pseudo-classe qui ne correspond jamais à un élément natif (`:visited`…).
    Never,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PseudoElement {
    Before,
    After,
    Placeholder,
    /// `::-webkit-slider-thumb` : pouce d'un curseur dessiné par la feuille.
    SliderThumb,
    /// `::-webkit-slider-runnable-track` : piste d'un curseur dessiné par la feuille.
    SliderTrack,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Compound {
    /// Balise en minuscules ; `None` : universel ou absente.
    pub tag: Option<String>,
    pub ids: Vec<String>,
    pub classes: Vec<String>,
    pub attrs: Vec<AttrSel>,
    pub pseudos: Vec<Pseudo>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Complex {
    /// Composés de gauche à droite ; le combinateur relie au composé précédent.
    pub parts: Vec<(Combinator, Compound)>,
    pub element: Option<PseudoElement>,
}
/// Spécificité (id, classes/attributs/pseudo-classes, types/pseudo-éléments).
pub type Specificity = (u32, u32, u32);
pub(super) fn add(a: Specificity, b: Specificity) -> Specificity {
    (a.0 + b.0, a.1 + b.1, a.2 + b.2)
}

impl Complex {
    pub fn subject(&self) -> &Compound {
        &self.parts.last().unwrap().1
    }
    pub fn specificity(&self) -> Specificity {
        let mut s = self
            .parts
            .iter()
            .fold((0, 0, 0), |acc, (_, c)| add(acc, c.specificity()));
        if self.element.is_some() {
            s.2 += 1;
        }
        s
    }
}
impl Compound {
    pub fn specificity(&self) -> Specificity {
        let mut s = (
            self.ids.len() as u32,
            (self.classes.len() + self.attrs.len()) as u32,
            u32::from(self.tag.is_some()),
        );
        for p in &self.pseudos {
            s = add(
                s,
                match p {
                    Pseudo::Where(_) => (0, 0, 0),
                    Pseudo::Not(list) | Pseudo::Is(list) => list
                        .iter()
                        .map(Complex::specificity)
                        .max()
                        .unwrap_or((0, 0, 0)),
                    _ => (0, 1, 0),
                },
            );
        }
        s
    }
}

/// Clé d'index du sujet : la première classe, l'id, un attribut, la balise, ou rien
/// (universel).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Key {
    Class(String),
    Id(String),
    Attr(String),
    Tag(String),
    Universal,
}
pub fn key(c: &Compound) -> Key {
    if let Some(class) = c.classes.first() {
        return Key::Class(class.clone());
    }
    if let Some(id) = c.ids.first() {
        return Key::Id(id.clone());
    }
    if let Some(a) = c.attrs.first() {
        return Key::Attr(a.name.clone());
    }
    if let Some(t) = &c.tag {
        return Key::Tag(t.clone());
    }
    // `:is(.a)` / `:where(.a > .b)` à une seule alternative : clé de cette alternative.
    for p in &c.pseudos {
        if let Pseudo::Is(list) | Pseudo::Where(list) = p
            && let [only] = list.as_slice()
        {
            let k = key(only.subject());
            if k != Key::Universal {
                return k;
            }
        }
    }
    Key::Universal
}
/// Classes nommées n'importe où dans le sélecteur (classes acceptées par les templates).
pub fn classes(x: &Complex, out: &mut BTreeSet<String>) {
    for (_, c) in &x.parts {
        out.extend(c.classes.iter().cloned());
        for p in &c.pseudos {
            if let Pseudo::Not(list) | Pseudo::Is(list) | Pseudo::Where(list) = p {
                for y in list {
                    classes(y, out);
                }
            }
        }
    }
}
