//! Lecteur CSS tolérant des feuilles de frameworks : il suit la grammaire générique de
//! CSS Syntax 3 (règles, at-rules, blocs, déclarations, `!important`, imbrication) sans
//! juger les valeurs. Les constructions illisibles sont sautées et signalées par
//! l'appelant ; rien ici ne fait échouer la compilation.

/// Déclaration `nom: valeur [!important]`.
#[derive(Clone, Debug)]
pub struct Declaration {
    pub name: String,
    pub value: String,
    pub important: bool,
    pub at: usize,
}
/// Nœud d'une feuille.
#[derive(Clone, Debug)]
pub enum Node {
    /// Règle qualifiée : prélude (sélecteurs), déclarations et règles imbriquées.
    Rule {
        prelude: String,
        declarations: Vec<Declaration>,
        children: Vec<Node>,
        at: usize,
    },
    /// At-rule : nom sans `@`, prélude, et bloc éventuel (déclarations et règles).
    At {
        name: String,
        prelude: String,
        block: Option<Block>,
        at: usize,
    },
    /// Texte illisible (accolades déséquilibrées…), sauté.
    Invalid { at: usize, reason: String },
}
/// Contenu d'un bloc : déclarations et règles imbriquées, dans l'ordre de la source.
#[derive(Clone, Debug, Default)]
pub struct Block {
    pub declarations: Vec<Declaration>,
    pub children: Vec<Node>,
}

/// Remplace les commentaires par des espaces (positions conservées).
fn strip_comments(source: &str) -> String {
    let mut bytes = source.as_bytes().to_vec();
    let mut i = 0;
    let mut quote = None;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            i += 2;
            continue;
        }
        if let Some(q) = quote {
            if bytes[i] == q || bytes[i] == b'\n' {
                quote = None;
            }
            i += 1;
            continue;
        }
        if bytes[i] == b'"' || bytes[i] == b'\'' {
            quote = Some(bytes[i]);
            i += 1;
            continue;
        }
        if bytes.get(i..i + 2) == Some(b"/*") {
            let start = i;
            i += 2;
            while i + 1 < bytes.len() && &bytes[i..i + 2] != b"*/" {
                i += 1;
            }
            i = (i + 2).min(bytes.len());
            for byte in &mut bytes[start..i] {
                // Les octets non ASCII d'un commentaire deviennent aussi des espaces :
                // le texte reste de l'UTF-8 valide.
                if *byte != b'\n' {
                    *byte = b' ';
                }
            }
        } else {
            i += 1;
        }
    }
    String::from_utf8(bytes).unwrap_or_default()
}

/// Lit une feuille complète.
pub fn parse(source: &str) -> Vec<Node> {
    let text = strip_comments(source);
    let mut cursor = 0;
    let block = block(&text, &mut cursor, true);
    // Au niveau de la feuille, une « déclaration » isolée est du texte invalide.
    let mut nodes = block.children;
    for d in block.declarations {
        nodes.push(Node::Invalid {
            at: d.at,
            reason: format!("stray declaration '{}' outside a rule", d.name),
        });
    }
    nodes.sort_by_key(|n| match n {
        Node::Rule { at, .. } | Node::At { at, .. } | Node::Invalid { at, .. } => *at,
    });
    nodes
}

/// Position du prochain caractère parmi `stops` au niveau 0 (hors chaînes, parenthèses
/// et crochets), ou la fin du texte.
fn scan(text: &str, start: usize, stops: &[u8]) -> usize {
    let bytes = text.as_bytes();
    let mut i = start;
    let mut depth = 0usize;
    let mut quote = None;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'\\' {
            i += 2;
            continue;
        }
        if let Some(q) = quote {
            if b == q || b == b'\n' {
                quote = None;
            }
            i += 1;
            continue;
        }
        match b {
            b'"' | b'\'' => quote = Some(b),
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth = depth.saturating_sub(1),
            _ if depth == 0 && stops.contains(&b) => return i,
            _ => {}
        }
        i += 1;
    }
    bytes.len()
}
fn skip_space(text: &str, mut i: usize) -> usize {
    let bytes = text.as_bytes();
    while i < bytes.len() && (bytes[i].is_ascii_whitespace() || bytes[i] == b';') {
        i += 1;
    }
    i
}

/// Contenu d'un bloc jusqu'à `}` (consommée) ou la fin du texte.
fn block(text: &str, cursor: &mut usize, top: bool) -> Block {
    let mut out = Block::default();
    let bytes = text.as_bytes();
    loop {
        *cursor = skip_space(text, *cursor);
        if *cursor >= bytes.len() {
            return out;
        }
        if bytes[*cursor] == b'}' {
            *cursor += 1;
            if top {
                out.children.push(Node::Invalid {
                    at: *cursor - 1,
                    reason: "unbalanced '}'".into(),
                });
                continue;
            }
            return out;
        }
        let at = *cursor;
        if bytes[at] == b'@' {
            let name_end = text[at + 1..]
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
                .map_or(bytes.len(), |n| at + 1 + n);
            let name = text[at + 1..name_end].to_ascii_lowercase();
            let stop = scan(text, name_end, b"{;}");
            let prelude = text[name_end..stop].trim().to_owned();
            if stop < bytes.len() && bytes[stop] == b'{' {
                *cursor = stop + 1;
                let inner = block(text, cursor, false);
                out.children.push(Node::At {
                    name,
                    prelude,
                    block: Some(inner),
                    at,
                });
            } else {
                *cursor = if stop < bytes.len() && bytes[stop] == b';' {
                    stop + 1
                } else {
                    stop
                };
                out.children.push(Node::At {
                    name,
                    prelude,
                    block: None,
                    at,
                });
            }
            continue;
        }
        // Déclaration (fin à `;` ou `}`) ou règle imbriquée (prélude jusqu'à `{`).
        let stop = scan(text, at, b"{;}");
        if stop < bytes.len() && bytes[stop] == b'{' {
            // Un `{` dans la valeur d'une propriété personnalisée (`--x: {…}`) n'existe
            // pas dans les feuilles visées : c'est une règle.
            let prelude = text[at..stop].trim().to_owned();
            *cursor = stop + 1;
            let inner = block(text, cursor, false);
            out.children.push(Node::Rule {
                prelude,
                declarations: inner.declarations,
                children: inner.children,
                at,
            });
            continue;
        }
        let raw = &text[at..stop];
        *cursor = if stop < bytes.len() && bytes[stop] == b';' {
            stop + 1
        } else {
            stop
        };
        match declaration(raw, at) {
            Some(d) => out.declarations.push(d),
            None => out.children.push(Node::Invalid {
                at,
                reason: format!("unreadable declaration '{}'", raw.trim()),
            }),
        }
    }
}

/// `nom: valeur [!important]`.
fn declaration(raw: &str, at: usize) -> Option<Declaration> {
    let colon = scan(raw, 0, b":");
    if colon >= raw.len() {
        return None;
    }
    let name = raw[..colon].trim();
    if name.is_empty()
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return None;
    }
    let name = if name.starts_with("--") {
        name.to_owned()
    } else {
        name.to_ascii_lowercase()
    };
    let mut value = raw[colon + 1..].trim();
    let mut important = false;
    if let Some(bang) = value.rfind('!') {
        let flag = value[bang + 1..].trim();
        if flag.eq_ignore_ascii_case("important") {
            important = true;
            value = value[..bang].trim_end();
        }
    }
    Some(Declaration {
        name,
        value: value.split_whitespace().collect::<Vec<_>>().join(" "),
        important,
        at,
    })
}

/// Ligne et colonne (1-based) d'une position.
pub fn line_column(text: &str, at: usize) -> (usize, usize) {
    let prefix = &text[..at.min(text.len())];
    let line = prefix.bytes().filter(|b| *b == b'\n').count() + 1;
    let column = prefix.rsplit('\n').next().unwrap_or("").chars().count() + 1;
    (line, column)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_rules_at_rules_and_important() {
        let nodes = parse(
            "@charset \"UTF-8\";/* x */ :root{--a:1px; color : red !important}@media (min-width:1px){.a{b:c}}@layer x, y;",
        );
        assert_eq!(nodes.len(), 4);
        match &nodes[1] {
            Node::Rule {
                prelude,
                declarations,
                ..
            } => {
                assert_eq!(prelude, ":root");
                assert_eq!(declarations[0].name, "--a");
                assert!(declarations[1].important);
                assert_eq!(declarations[1].value, "red");
            }
            other => panic!("{other:?}"),
        }
        assert!(
            matches!(&nodes[2], Node::At { name, block: Some(b), .. } if name == "media" && b.children.len() == 1)
        );
        assert!(
            matches!(&nodes[3], Node::At { name, prelude, block: None, .. } if name == "layer" && prelude == "x, y")
        );
    }
    #[test]
    fn nested_rules_and_strings() {
        let nodes = parse(".a{content:\"}{;\";&:hover{color:red}.b{x:y}}");
        let Node::Rule {
            declarations,
            children,
            ..
        } = &nodes[0]
        else {
            panic!()
        };
        assert_eq!(declarations[0].value, "\"}{;\"");
        assert_eq!(children.len(), 2);
    }
}
