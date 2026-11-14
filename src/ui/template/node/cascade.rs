//! Données de cascade portées par le nœud : classes, faits HTML, règles d'état et
//! d'enfants, propriétés personnalisées, conteneurs de requête.
use super::Node;
use crate::ui::{element::*, template::TemplateContext};
impl<W: Widget + 'static> Node<W> {
    /// Captures the CSS variables of `cx` so styles can resolve `var()` references.
    pub fn style_context<R>(mut self, cx: &TemplateContext<'_, R>) -> Self {
        self.element
            .styles
            .get_or_insert_with(Default::default)
            .variables = cx.variables.cloned();
        self
    }
    /// Sets the custom properties read last by `var()` (a framework stylesheet's
    /// `:root`), and the template scale applied to its dynamic declarations.
    pub fn style_globals<R>(
        mut self,
        globals: crate::ui::styling::tokens::Globals,
        cx: &TemplateContext<'_, R>,
    ) -> Self {
        let styles = self.element.styles.get_or_insert_with(Default::default);
        styles.globals = Some(globals);
        styles.scale = cx.scale;
        self
    }
    /// Adds a CSS class (ignored if already present).
    pub fn css_class(mut self, name: &'static str) -> Self {
        let classes = &mut self
            .element
            .styles
            .get_or_insert_with(Default::default)
            .classes;
        if !classes.contains(&name) {
            classes.push(name);
        }
        self
    }
    /// Records the HTML tag and literal attributes written in the template, matched by
    /// the type and attribute selectors of framework stylesheets.
    pub fn html_facts(
        mut self,
        tag: &'static str,
        attributes: &'static [(&'static str, &'static str)],
    ) -> Self {
        let styles = self.element.styles.get_or_insert_with(Default::default);
        styles.tag = tag;
        for (name, value) in attributes {
            styles
                .attributes
                .push((name, std::borrow::Cow::Borrowed(*value)));
        }
        self
    }
    /// Records an attribute whose value is computed by the template
    /// (`data-bs-theme={theme}`); `None` leaves the attribute absent.
    pub fn html_attribute(mut self, name: &'static str, value: Option<String>) -> Self {
        if let Some(value) = value {
            self.element
                .styles
                .get_or_insert_with(Default::default)
                .attributes
                .push((name, std::borrow::Cow::Owned(value)));
        }
        self
    }
    /// Adds a style block applied while `state` matches, after the existing blocks.
    pub fn state_style(
        self,
        state: crate::ui::styling::StyleState,
        value: crate::ui::styling::Properties,
    ) -> Self {
        let order = self
            .element
            .styles
            .as_ref()
            .map_or(0, |s| s.states.len() as u32);
        self.state_rule(state, value, order)
    }
    /// Adds a style block applied while `state` matches. Blocks are applied by
    /// increasing specificity, then by `order`.
    pub fn state_rule(
        mut self,
        state: crate::ui::styling::StyleState,
        value: crate::ui::styling::Properties,
        order: u32,
    ) -> Self {
        let states = &mut self
            .element
            .styles
            .get_or_insert_with(Default::default)
            .states;
        states.push((state, value, order));
        states.sort_by_key(|(state, _, order)| (state.specificity(), *order));
        self
    }
    /// Adds a style block applied to the children of the node that match `state`
    /// (evaluated on each child, e.g. `Not(LastChild)` for `.list > :not(:last-child)`),
    /// beneath the children's own declarations (`:where()` specificity, as Tailwind's
    /// `space-y-*` and `divide-y`).
    pub fn child_rule(
        mut self,
        state: crate::ui::styling::StyleState,
        value: crate::ui::styling::Properties,
        order: u32,
    ) -> Self {
        // Les enfants portent des styles : leur base est restaurée quand la règle cesse
        // de s'appliquer.
        for child in &mut self.element.children {
            child.styles.get_or_insert_with(Default::default);
        }
        let rules = &mut self
            .element
            .styles
            .get_or_insert_with(Default::default)
            .child_states;
        rules.push((state, value, order));
        rules.sort_by_key(|(state, _, order)| (state.specificity(), *order));
        self
    }
    /// Declares custom properties of a stylesheet class (`--h: 210`), inherited by
    /// descendants; template declarations ([`custom_property`](Self::custom_property))
    /// and active state rules take precedence.
    pub fn custom_properties(
        mut self,
        values: &'static [(&'static str, crate::ui::styling::VariableValue)],
    ) -> Self {
        let styles = self.element.styles.get_or_insert_with(Default::default);
        styles.class_custom.extend(values.iter().cloned());
        self
    }
    /// Declares custom property `name` (`style:--h={expr}`), inherited by descendants
    /// and visible to `var()` in their colours and computed lengths.
    ///
    /// # Panics
    ///
    /// Panics if `name` does not start with `--` or is just `--`.
    pub fn custom_property(
        mut self,
        name: &'static str,
        value: impl Into<crate::ui::styling::VariableValue>,
    ) -> Self {
        assert!(
            name.starts_with("--") && name.len() > 2,
            "CSS custom property names start with --"
        );
        let custom = &mut self
            .element
            .styles
            .get_or_insert_with(Default::default)
            .custom;
        custom.retain(|(n, _)| *n != name);
        custom.push((name, value.into()));
        self
    }
    /// Sets a colour evaluated while styles resolve (custom properties, `currentColor`);
    /// it replaces the constant colour of `target`.
    pub fn color_expression(
        mut self,
        target: crate::ui::styling::ColorTarget,
        value: &'static crate::ui::styling::ColorExpression,
    ) -> Self {
        self.properties().color_expressions[target as usize] = Some(value);
        self
    }
    /// Sets background layers whose colours are evaluated while styles resolve (a
    /// gradient tinted by `var(--h)`), frontmost first; they replace the gradient layers.
    pub fn background_expressions(
        mut self,
        value: &'static [crate::ui::styling::BackgroundExpression],
    ) -> Self {
        self.properties().background_expressions = Some(value);
        self
    }
    /// Sets box shadows whose colours are evaluated while styles resolve.
    pub fn shadow_expressions(mut self, value: crate::ui::styling::ShadowExpressions) -> Self {
        self.properties().shadow_expressions = Some(value);
        self.properties().shadows = None;
        self
    }
    /// Declares the element as a query container (`container-type`); `None` removes it.
    pub fn container_type(mut self, value: Option<crate::ui::styling::ContainerType>) -> Self {
        self.element
            .styles
            .get_or_insert_with(Default::default)
            .container_type = value;
        self
    }
    /// Names the query container (`container-name`), used by `@container name (…)`.
    pub fn container_name(mut self, names: &'static [&'static str]) -> Self {
        self.element
            .styles
            .get_or_insert_with(Default::default)
            .container_names = names;
        self
    }
    /// Applies a reusable view style.
    pub fn style(mut self, value: crate::ui::view::ViewStyle) -> Self {
        self.element = self.element.style(value);
        self
    }
    /// Sets the cursor shown over the element and its subtree (`cursor`).
    pub fn cursor(mut self, value: crate::ui::runtime::Cursor) -> Self {
        self.properties().cursor = Some(value);
        self
    }
    /// `pointer-events`: `false` makes the element and its subtree transparent to the
    /// pointer (no hit, hover or cursor), `true` restores it inside such a subtree.
    pub fn pointer_events(mut self, enabled: bool) -> Self {
        self.element
            .styles
            .get_or_insert_with(Default::default)
            .pointer_events = Some(enabled);
        self
    }
}
