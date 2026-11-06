//! Portée des propriétés personnalisées déclarées par un élément.
use crate::ui::styling::{CssVariables, VariableValue, tokens};

/// Portée enfant de `declared` contenant les propriétés personnalisées de l'élément,
/// leurs `var()` calculées (déclarations de l'élément d'abord, puis ancêtres et
/// variables globales de la feuille).
pub(super) fn declare(
    declared: Option<&CssVariables>,
    own: &[(&'static str, VariableValue)],
    globals: Option<tokens::Globals>,
) -> CssVariables {
    fn own_value(
        own: &[(&'static str, VariableValue)],
        name: &'static str,
        declared: Option<&CssVariables>,
        globals: Option<tokens::Globals>,
        out: &mut Vec<tokens::Token>,
        depth: usize,
    ) -> Option<bool> {
        let (_, value) = own.iter().rev().find(|(n, _)| *n == name)?;
        Some(match value {
            VariableValue::Static(t) => tokens::substitute(
                t,
                &mut |n, o, d| {
                    own_value(own, n, declared, globals, o, d)
                        .unwrap_or_else(|| tokens::lookup(declared, globals, n, o, d))
                },
                out,
                depth,
            ),
            other => tokens::typed_tokens(other, out),
        })
    }
    let mut scope = CssVariables::child(declared);
    for (name, value) in own {
        let computed = tokens::compute_custom(
            value,
            &mut |n, o, d| own_value(own, n, declared, globals, o, d),
            declared,
            globals,
        );
        match computed {
            Some(v) => scope.set_static(name, v),
            // Valeur invalide au moment du calcul : la propriété garde sa valeur héritée.
            None => {
                std::sync::Arc::make_mut(&mut scope.values).remove(*name);
            }
        }
    }
    scope
}
