//! Substitution des `var()` : registre des propriétés globales d'une feuille, recherche
//! dans la portée et calcul des propriétés personnalisées.
use super::token::{Token, Unit, group_end};
use crate::ui::{element::Length, styling::CssVariables, styling::VariableValue};

/// Custom properties of a framework stylesheet's `:root` and `@property` initial values:
/// the lowest-priority source of `var()` values.
#[derive(Debug)]
pub struct Registry {
    /// Values sorted by name.
    pub values: &'static [(&'static str, &'static [Token])],
    /// Properties registered with `inherits: false` (sorted).
    pub non_inherited: &'static [&'static str],
}
/// Registry of a framework stylesheet.
pub type Globals = &'static Registry;
const DEPTH: usize = 32;
/// Jetons d'une valeur de variable typée de l'application.
pub(crate) fn typed_tokens(value: &VariableValue, out: &mut Vec<Token>) -> bool {
    match value {
        VariableValue::Color(c) => out.push(Token::Color(*c)),
        VariableValue::Number(n) => out.push(Token::Number(*n)),
        VariableValue::Length(Length::Px(n)) => out.push(Token::Dimension(*n, Unit::Px)),
        VariableValue::Length(Length::Percent(n)) => out.push(Token::Percentage(*n)),
        VariableValue::Length(Length::Auto) => out.push(Token::Ident("auto")),
        VariableValue::Length(_) => return false,
        VariableValue::Family(f) => out.push(Token::String(f)),
        VariableValue::Insets(i) => {
            for n in [i.top, i.right, i.bottom, i.left] {
                out.push(Token::Dimension(n, Unit::Px));
            }
        }
        VariableValue::Tokens(t) => out.extend_from_slice(t),
        VariableValue::Static(t) => out.extend_from_slice(t),
    }
    true
}
/// Remplace les `var()` de `tokens` ; `false` si une variable manque sans repli (valeur
/// invalide au moment du calcul) ou en cas de cycle.
pub(crate) fn substitute(
    tokens: &[Token],
    lookup: &mut dyn FnMut(&'static str, &mut Vec<Token>, usize) -> bool,
    out: &mut Vec<Token>,
    depth: usize,
) -> bool {
    if depth > DEPTH || out.len() > 4096 {
        return false;
    }
    let mut i = 0;
    while i < tokens.len() {
        if let Token::Var(name) = tokens[i] {
            let end = group_end(tokens, i + 1);
            let start = out.len();
            if !lookup(name, out, depth + 1) {
                out.truncate(start);
                // Repli : jetons après la virgule.
                if tokens.get(i + 1) == Some(&Token::Comma) {
                    if !substitute(&tokens[i + 2..end], lookup, out, depth + 1) {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            i = end + 1;
            continue;
        }
        out.push(tokens[i]);
        i += 1;
    }
    true
}
/// Valeur de `name` dans la portée puis dans les variables globales de la feuille.
pub(crate) fn lookup(
    variables: Option<&CssVariables>,
    globals: Option<Globals>,
    name: &'static str,
    out: &mut Vec<Token>,
    depth: usize,
) -> bool {
    if let Some(value) = variables.and_then(|v| v.get(name)) {
        // Une valeur déclarée contient déjà ses var() substituées, sauf les jetons
        // statiques des globales.
        return match value {
            VariableValue::Static(t) => substitute(
                t,
                &mut |n, o, d| lookup(variables, globals, n, o, d),
                out,
                depth,
            ),
            other => typed_tokens(other, out),
        };
    }
    if let Some(globals) = globals
        && let Ok(index) = globals.values.binary_search_by(|(n, _)| (*n).cmp(name))
    {
        let tokens = globals.values[index].1;
        return substitute(
            tokens,
            &mut |n, o, d| lookup(variables, Some(globals), n, o, d),
            out,
            depth,
        );
    }
    false
}
/// Lecture des déclarations de l'élément : `None` si l'élément ne déclare pas la variable.
type OwnLookup<'a> = dyn FnMut(&'static str, &mut Vec<Token>, usize) -> Option<bool> + 'a;
/// Valeur calculée d'une propriété personnalisée : ses `var()` substituées dans la
/// portée de l'élément. `own` donne les déclarations de l'élément lui-même.
pub(crate) fn compute_custom(
    value: &VariableValue,
    own: &mut OwnLookup,
    variables: Option<&CssVariables>,
    globals: Option<Globals>,
) -> Option<VariableValue> {
    let VariableValue::Static(tokens) = value else {
        return Some(value.clone());
    };
    if !tokens.iter().any(|t| matches!(t, Token::Var(_))) {
        return Some(value.clone());
    }
    let mut out = Vec::new();
    let mut look = |n: &'static str, o: &mut Vec<Token>, d: usize| match own(n, o, d) {
        Some(found) => found,
        None => lookup(variables, globals, n, o, d),
    };
    substitute(tokens, &mut look, &mut out, 0).then(|| VariableValue::Tokens(out.into()))
}
