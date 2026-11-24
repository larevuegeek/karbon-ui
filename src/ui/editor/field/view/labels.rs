//! Les noms accessibles des champs numériques : le Narrateur lit le nom donné
//! (« Position X », « Intensité ») et la valeur montrée (« 2,5 m », ou « — »).
use super::{NumberField, VectorField};
use std::sync::Arc;

/// Les lettres des axes, après le nom d'un champ de trois nombres.
const AXES: [&str; 3] = ["X", "Y", "Z"];

impl<M: 'static> NumberField<M> {
    /// The name assistive technologies (Narrator) read for this field, with its shown
    /// value: the label of its row in a property grid.
    pub fn aria_label(mut self, label: impl Into<Arc<str>>) -> Self {
        self.label = Some(label.into());
        self
    }
}

impl<M: 'static> VectorField<M> {
    /// The name read for each of the three fields: `label` followed by its axis
    /// (« Position X », « Position Y », « Position Z »).
    pub fn aria_label(self, label: &str) -> Self {
        let [x, y, z] = self.fields;
        let named = |field: NumberField<M>, axis: usize| {
            field.aria_label(format!("{label} {}", AXES[axis]))
        };
        Self {
            fields: [named(x, 0), named(y, 1), named(z, 2)],
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ui::{
        editor::{FieldMsg, FieldState, NumberField, VectorField, testing::window},
        element::{Element, Role},
        prelude::{Application, ViewContext},
    };

    struct Fields;
    impl Application for Fields {
        type Message = FieldMsg;
        fn view(&self, _: &ViewContext) -> Element {
            let state = FieldState::default();
            let power = NumberField::new(1, 2.5, &state, |m| m)
                .unit("m")
                .aria_label("Intensité");
            let place = VectorField::new(2, [1., 2., 3.], &state, |m| m).aria_label("Position");
            let mixed = NumberField::new(3, 7., &state, |m| m)
                .mixed(true)
                .aria_label("Portée");
            let bare = NumberField::new(4, 1., &state, |m| m);
            Element::column(
                9,
                vec![
                    power.element(),
                    place.element(),
                    mixed.element(),
                    bare.element(),
                ],
            )
        }
        fn update(&mut self, _: FieldMsg) -> bool {
            false
        }
    }

    #[test]
    fn narrator_reads_the_name_and_the_shown_value_of_number_fields() {
        let app = window(Fields, 400., 300.);
        let read: Vec<(String, Option<String>)> = app
            .accessibility()
            .into_iter()
            .filter(|a| a.role == Role::Slider)
            .map(|a| (a.name, a.value))
            .collect();
        let named = |name: &str| {
            read.iter()
                .find(|(n, _)| n == name)
                .and_then(|(_, v)| v.clone())
        };
        assert_eq!(named("Intensité").as_deref(), Some("2.5m"), "{read:?}");
        assert_eq!(named("Position Y").as_deref(), Some("2.0"));
        assert_eq!(named("Portée").as_deref(), Some("—"));
        // Témoin : sans nom donné, le champ est lu sans nom, sa valeur seule.
        assert!(
            read.iter()
                .any(|(n, v)| n.is_empty() && v.as_deref() == Some("1.0")),
            "{read:?}"
        );
    }
}
