//! `@keyframes` d'une feuille de framework compilées en animations du moteur, avec
//! les étapes extrêmes absentes complétées par la valeur neutre.
use super::{Sheet, values};
use crate::css::{animation, parser};
use std::collections::{BTreeMap, BTreeSet};

impl Sheet {
    /// Animations `@keyframes` compilées (nom → code `&[Keyframe]`) ; une étape `0%` ou
    /// `100%` absente reprend la valeur neutre (`transform: none`, `opacity: 1`), comme
    /// la valeur de base d'un élément sans transformation.
    pub fn animations(&self) -> BTreeMap<String, String> {
        self.animations_with("1")
    }
    /// Comme [`animations`](Self::animations), l'opacité absente d'une étape `0%` ou
    /// `100%` valant `opacity` : celle de l'élément animé, comme dans un navigateur
    /// (`.spinner-grow` est à `opacity: 0` hors de son étape `50%`).
    pub fn animations_with(&self, opacity: &str) -> BTreeMap<String, String> {
        use parser::{Declaration, Item};
        let mut out = BTreeMap::new();
        for (_, text) in &self.keyframes {
            let Ok(items) = parser::parse(&values::rem_to_px(text)) else {
                continue;
            };
            for item in items {
                let Item::Keyframes {
                    name, mut frames, ..
                } = item
                else {
                    continue;
                };
                let mut offsets = BTreeSet::new();
                let mut animated = BTreeSet::new();
                for frame in &frames {
                    if let Item::Rule {
                        selectors,
                        declarations,
                        ..
                    } = frame
                    {
                        for s in selectors {
                            offsets.insert(match s.as_str() {
                                "from" => "0%".to_owned(),
                                "to" => "100%".to_owned(),
                                other => other.to_owned(),
                            });
                        }
                        for d in declarations {
                            if d.name == "opacity" || d.name == "transform" {
                                animated.insert(d.name.clone());
                            }
                        }
                    }
                }
                let neutral = |n: &str| if n == "opacity" { opacity } else { "none" }.to_owned();
                for edge in ["0%", "100%"] {
                    if !offsets.contains(edge) {
                        frames.push(Item::Rule {
                            selectors: vec![edge.into()],
                            declarations: animated
                                .iter()
                                .map(|n| Declaration {
                                    name: n.clone(),
                                    value: neutral(n),
                                    at: 0,
                                })
                                .collect(),
                            at: 0,
                        });
                    }
                }
                // Étape extrême qui omet une propriété animée ailleurs : valeur de
                // l'élément (`0% { transform: scale(0) }` sans opacité).
                for frame in &mut frames {
                    if let Item::Rule {
                        selectors,
                        declarations,
                        ..
                    } = frame
                        && selectors
                            .iter()
                            .all(|s| matches!(s.as_str(), "from" | "to" | "0%" | "100%"))
                    {
                        for n in &animated {
                            if !declarations.iter().any(|d| &d.name == n) {
                                declarations.push(Declaration {
                                    name: n.clone(),
                                    value: neutral(n),
                                    at: 0,
                                });
                            }
                        }
                    }
                }
                if let Ok(code) = animation::frames(&frames, &BTreeMap::new()) {
                    out.insert(name, code);
                }
            }
        }
        out
    }
}
