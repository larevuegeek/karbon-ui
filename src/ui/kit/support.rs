pub(super) use crate::ui::template::Slots;

/// Gabarit d'un composant du kit : feuille du kit, variables du thème.
macro_rules! kit_ui {
    ($cx:ident, $slots:ident; $($markup:tt)*) => {{
        let variables = super::theme::scope($cx.variables);
        let cx = $cx.with_variables(&variables);
        crate::ui! { @context(cx); @slots($slots); @styles(css); $($markup)* }
    }};
    ($cx:ident; $($markup:tt)*) => {{
        let variables = super::theme::scope($cx.variables);
        let cx = $cx.with_variables(&variables);
        crate::ui! { @context(cx); @styles(css); $($markup)* }
    }};
}

/// Le slot `name` a du contenu.
pub(super) fn has_slot(slots: &Slots, name: &str) -> bool {
    slots.unused_names().any(|used| used == name)
}
/// Texte du contenu du slot par défaut.
pub(super) fn slot_text(slots: &mut Slots) -> String {
    slots
        .take("")
        .iter()
        .filter_map(|element| element.widget.as_ref())
        .map(|widget| widget.semantics().name.to_string())
        .collect::<Vec<_>>()
        .join(" ")
}
