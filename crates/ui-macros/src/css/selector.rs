//! Sélecteurs des feuilles strictes : la classe sujet (dernier composé), ses états,
//! combinateurs et pseudo-éléments deviennent un prédicat `StyleState` évalué à l'exécution.
use super::parser;
type Result<T> = std::result::Result<T, String>;
const STATE: &str = "__ui_root::ui::styling::StyleState";
pub(super) struct Selector {
    pub class: String,
    pub classes: Vec<String>,
    pub state: String,
    pub predicate: Option<String>,
    /// Pseudo-élément généré (`before` ou `after`) ; le prédicat porte sur l'élément hôte.
    pub pseudo: Option<&'static str>,
}
fn pseudo(value: &str, depth: usize) -> Result<String> {
    if depth > 8 {
        return Err("Selector nesting exceeds 8".into());
    }
    if let Some(inner) = value.strip_prefix("not(").and_then(|s| s.strip_suffix(')')) {
        let p = predicate(inner, depth + 1)?;
        return Ok(format!("{STATE}::Not(&{p})"));
    }
    if let Some(inner) = value
        .strip_prefix("nth-child(")
        .and_then(|s| s.strip_suffix(')'))
    {
        let v = inner.replace(' ', "");
        let (a, b) = match v.as_str() {
            "odd" => (2, 1),
            "even" => (2, 0),
            _ => {
                if let Some((a, b)) = v.split_once('n') {
                    let a = match a {
                        "" | "+" => 1,
                        "-" => -1,
                        _ => a
                            .parse::<i32>()
                            .map_err(|_| "Invalid nth-child coefficient")?,
                    };
                    let b = if b.is_empty() {
                        0
                    } else {
                        b.parse::<i32>().map_err(|_| "Invalid nth-child offset")?
                    };
                    (a, b)
                } else {
                    (0, v.parse::<i32>().map_err(|_| "Invalid nth-child")?)
                }
            }
        };
        return Ok(format!("{STATE}::NthChild({a},{b})"));
    }
    let variant = match value {
        "hover" => "Hover",
        "active" => "Active",
        "focus" => "Focus",
        "focus-visible" => "FocusVisible",
        "focus-within" => "FocusWithin",
        "disabled" => "Disabled",
        "enabled" => "Enabled",
        "checked" => "Checked",
        "indeterminate" => "Indeterminate",
        "read-only" => "ReadOnly",
        "read-write" => "ReadWrite",
        "placeholder-shown" => "PlaceholderShown",
        "first-child" => "FirstChild",
        "last-child" => "LastChild",
        "only-child" => "OnlyChild",
        "empty" => "Empty",
        _ => return Err(format!("Pseudo-class :{value} is not implemented")),
    };
    Ok(format!("{STATE}::{variant}"))
}
fn predicate(value: &str, depth: usize) -> Result<String> {
    let value = value.strip_prefix(':').unwrap_or(value);
    let parts = parser::split(value, ':')?;
    if parts.iter().any(|s| s.is_empty()) {
        return Err("Empty pseudo-class".into());
    }
    let parts = parts
        .iter()
        .map(|s| pseudo(s, depth))
        .collect::<Result<Vec<_>>>()?;
    if parts.len() == 1 {
        Ok(parts[0].clone())
    } else {
        Ok(format!("{STATE}::All(&[{}])", parts.join(",")))
    }
}
/// Découpe un sélecteur en composés et combinateurs (` `, `>`, `+`, `~`), hors
/// parenthèses.
fn compounds(value: &str) -> Result<Vec<(char, String)>> {
    let mut out: Vec<(char, String)> = Vec::new();
    let mut current = String::new();
    let mut pending = None;
    let mut depth = 0usize;
    let push = |current: &mut String,
                pending: &mut Option<char>,
                out: &mut Vec<(char, String)>|
     -> Result<()> {
        if current.is_empty() {
            return Ok(());
        }
        let combinator = if out.is_empty() {
            if pending.is_some_and(|c| c != ' ') {
                return Err("A selector cannot start with a combinator".into());
            }
            ' '
        } else {
            pending.unwrap_or(' ')
        };
        out.push((combinator, std::mem::take(current)));
        *pending = None;
        Ok(())
    };
    for c in value.chars() {
        match c {
            '(' | '[' => {
                depth += 1;
                current.push(c);
            }
            ')' | ']' => {
                depth = depth
                    .checked_sub(1)
                    .ok_or("Unbalanced parentheses in selector")?;
                current.push(c);
            }
            c if depth == 0 && (c.is_whitespace() || matches!(c, '>' | '+' | '~')) => {
                push(&mut current, &mut pending, &mut out)?;
                if c != ' ' && !c.is_whitespace() {
                    if pending.is_some_and(|p| p != ' ') {
                        return Err("Two consecutive combinators in selector".into());
                    }
                    pending = Some(c);
                } else if pending.is_none() && !out.is_empty() {
                    pending = Some(' ');
                }
            }
            c => current.push(c),
        }
    }
    if pending.is_some_and(|p| p != ' ') && current.is_empty() {
        return Err("A selector cannot end with a combinator".into());
    }
    push(&mut current, &mut pending, &mut out)?;
    if out.is_empty() {
        return Err("Empty selector".into());
    }
    if out.len() > 8 {
        return Err("Selector chains are limited to 8 compound selectors".into());
    }
    Ok(out)
}
/// Sélecteur d'attribut `[name]`, `[name=value]`, `[name^="v" i]`… en prédicat
/// `StyleState::Attribute` (comparé aux attributs écrits dans le template).
pub(crate) fn attribute(inner: &str) -> Result<String> {
    let inner = inner.trim();
    let ops = [
        ("~=", "Includes"),
        ("|=", "DashMatch"),
        ("^=", "Prefix"),
        ("$=", "Suffix"),
        ("*=", "Substring"),
        ("=", "Equals"),
    ];
    let (name, operator, value) = match ops.iter().find_map(|(op, variant)| {
        inner
            .find(op)
            .map(|at| (&inner[..at], *variant, &inner[at + op.len()..]))
    }) {
        Some((name, operator, value)) => (name.trim(), operator, value.trim()),
        None => (inner, "Exists", ""),
    };
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(format!("Invalid attribute selector '[{inner}]'"));
    }
    // Drapeau `i` : comparaison sans casse.
    let (value, case_insensitive) = match value.rsplit_once(char::is_whitespace) {
        Some((v, flag)) if flag.eq_ignore_ascii_case("i") => (v.trim(), true),
        Some((v, flag)) if flag.eq_ignore_ascii_case("s") => (v.trim(), false),
        _ => (value, false),
    };
    let value = value
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
        .unwrap_or(value);
    Ok(format!(
        "{STATE}::Attribute(&__ui_root::ui::styling::AttributeMatch{{name:{:?},operator:__ui_root::ui::styling::AttributeOperator::{operator},value:{value:?},case_insensitive:{case_insensitive}}})",
        name.to_ascii_lowercase()
    ))
}
/// Composé `a.b:hover`, `div:last-child`, `.a[data-x=y]` : classes, prédicats de
/// balise et d'attributs, pseudo-classes. Les balises et attributs sont ceux écrits
/// dans le template.
fn compound(value: &str) -> Result<(Vec<String>, String)> {
    let mut rest = value;
    let mut extra = Vec::new();
    let tag_len = rest
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '*'))
        .unwrap_or(rest.len());
    if tag_len > 0 {
        let tag = &rest[..tag_len];
        if tag != "*" {
            if !tag.starts_with(|c: char| c.is_ascii_alphabetic()) {
                return Err(format!("Invalid type selector '{tag}'"));
            }
            extra.push(format!("{STATE}::Tag({:?})", tag.to_ascii_lowercase()));
        }
        rest = &rest[tag_len..];
    }
    let mut classes = Vec::new();
    loop {
        if let Some(tail) = rest.strip_prefix('.') {
            let end = tail.find(['.', '[', ':']).unwrap_or(tail.len());
            parser::class_name(&tail[..end])?;
            classes.push(tail[..end].to_owned());
            rest = &tail[end..];
        } else if let Some(tail) = rest.strip_prefix('[') {
            let end = tail.find(']').ok_or("Unclosed attribute selector")?;
            extra.push(attribute(&tail[..end])?);
            rest = &tail[end + 1..];
        } else {
            break;
        }
    }
    let state = match rest {
        "" => String::new(),
        s if s.starts_with("::") => {
            return Err("Pseudo-elements are not implemented in compound selectors".into());
        }
        s if s.starts_with(':') => s[1..].to_owned(),
        s => return Err(format!("Unsupported selector part '{s}'")),
    };
    if classes.is_empty() && extra.is_empty() && state.is_empty() && value != "*" {
        return Err("Empty compound selector".into());
    }
    // Les prédicats de balise et d'attribut passent avec l'état, sous forme de marqueur.
    let mut state = state;
    for p in extra {
        state = if state.is_empty() {
            format!("\u{1}{p}")
        } else {
            format!("{state}\u{1}{p}")
        };
    }
    Ok((classes, state))
}
/// Prédicat d'un composé seul, sans combinateur.
pub(crate) fn compound_predicate(text: &str) -> Result<String> {
    let text = text.trim();
    if text.is_empty() || text.contains(char::is_whitespace) || text.contains(['>', '+', '~']) {
        return Err(format!("'{text}' must be a single compound selector"));
    }
    let (classes, state) = compound(text)?;
    matching(&classes, &state)
}
/// Prédicat « l'élément correspond au composé » (toutes ses classes et pseudo-classes).
fn matching(classes: &[String], state: &str) -> Result<String> {
    // Prédicats déjà compilés (balise, attributs) après le marqueur \u{1}.
    let mut pieces = state.split('\u{1}');
    let state = pieces.next().unwrap_or("");
    let mut parts = classes
        .iter()
        .map(|s| format!("{STATE}::Class({s:?})"))
        .collect::<Vec<_>>();
    parts.extend(pieces.map(str::to_owned));
    if !state.is_empty() {
        parts.push(predicate(state, 0)?);
    }
    Ok(if parts.len() == 1 {
        parts.remove(0)
    } else {
        format!("{STATE}::All(&[{}])", parts.join(","))
    })
}
/// Relation d'un combinateur avec le composé précédent.
fn relation(combinator: char, inner: String) -> String {
    let variant = match combinator {
        '>' => "Parent",
        '+' => "PreviousSibling",
        '~' => "AnyPreviousSibling",
        _ => "Ancestor",
    };
    format!("{STATE}::{variant}(&{inner})")
}
pub(super) fn parse(value: &str) -> Result<Selector> {
    let value = value.trim();
    for (suffix, kind) in [
        ("::before", "before"),
        ("::after", "after"),
        (":before", "before"),
        (":after", "after"),
        // Épaisseur de la barre de défilement de l'hôte (WebView2/Chromium).
        ("::-webkit-scrollbar", "scrollbar"),
        // Texte indicatif d'un champ : couleur seulement.
        ("::placeholder", "placeholder"),
    ] {
        if let Some(host) = value.strip_suffix(suffix) {
            let mut selector = parse(host)?;
            selector.pseudo = Some(kind);
            return Ok(selector);
        }
    }
    if value.contains("::") {
        return Err(
            "Only ::before, ::after and ::-webkit-scrollbar (width, height) pseudo-elements are implemented, at the end of a selector; use scrollbar-color for the thumb"
                .into(),
        );
    }
    let chain = compounds(value)?;
    if chain.len() == 1 {
        return parse_compound(&chain[0].1);
    }
    // Contexte gauche : P(0) = composé 0 ; P(i) = composé i ∧ relation(P(i-1)).
    let mut context: Option<String> = None;
    let mut all_classes = Vec::new();
    let (last, left) = chain.split_last().unwrap();
    for (combinator, text) in left {
        let (classes, state) = compound(text)?;
        let mut predicate = matching(&classes, &state)?;
        if let Some(previous) = context.take() {
            predicate = format!(
                "{STATE}::All(&[{predicate},{}])",
                relation(*combinator, previous)
            );
        }
        all_classes.extend(classes);
        context = Some(predicate);
    }
    let mut subject = parse_compound(&last.1)?;
    let relation = relation(last.0, context.unwrap());
    subject.predicate = Some(match subject.predicate.take() {
        Some(own) => format!("{STATE}::All(&[{own},{relation}])"),
        None => relation,
    });
    // L'état du sujet ne suffit plus pour les effets simples (`css_hover`).
    subject.state = "combined".into();
    subject.classes.extend(all_classes);
    Ok(subject)
}
fn parse_compound(value: &str) -> Result<Selector> {
    // Le sujet est désigné par sa première classe ; balise et attributs (`.a[data-x]`)
    // s'ajoutent au prédicat.
    let (classes, state) = compound(value)?;
    if classes.is_empty() {
        return Err("Use class selectors on native template elements: the subject (last compound) needs a class".into());
    }
    let mut pieces = state.split('\u{1}');
    let own_state = pieces.next().unwrap_or("").to_owned();
    let extra = pieces.map(str::to_owned).collect::<Vec<_>>();
    let class = classes.first().ok_or("Empty selector")?.clone();
    let mut predicates = classes
        .iter()
        .skip(1)
        .map(|s| format!("{STATE}::Class({s:?})"))
        .collect::<Vec<_>>();
    predicates.extend(extra.iter().cloned());
    if !own_state.is_empty() {
        predicates.push(predicate(&own_state, 0)?);
    }
    let predicate = match predicates.len() {
        0 => None,
        1 => Some(predicates.remove(0)),
        _ => Some(format!("{STATE}::All(&[{}])", predicates.join(","))),
    };
    Ok(Selector {
        class,
        classes,
        // Avec une balise ou un attribut, l'état seul ne suffit plus (effets simples).
        state: if extra.is_empty() {
            own_state
        } else {
            "combined".into()
        },
        predicate,
        pseudo: None,
    })
}
