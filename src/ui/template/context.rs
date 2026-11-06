use crate::ui::{
    element::{ElementId, Insets, Length},
    font::Font,
};

/// Borrowed resources and logical scale; copied without cloning application resources.
///
/// Templates read lengths through this context so that pixel values are
/// multiplied by [`scale`](Self::scale).
pub struct TemplateContext<'a, R = ()> {
    /// CSS variables in scope, if any.
    pub variables: Option<&'a crate::ui::styling::CssVariables>,
    /// Application resources (images, fonts…).
    pub resources: &'a R,
    /// Identifier of the element being built; children derive theirs from it.
    pub id: ElementId,
    /// Logical scale applied to pixel lengths (finite and positive, `1.` by default).
    pub scale: f32,
    /// Default font for text.
    pub font: Font,
}
impl<R> Copy for TemplateContext<'_, R> {}
impl<R> Clone for TemplateContext<'_, R> {
    fn clone(&self) -> Self {
        *self
    }
}
impl Default for TemplateContext<'static, ()> {
    fn default() -> Self {
        Self::new(&(), ElementId(0))
    }
}
impl<'a, R> TemplateContext<'a, R> {
    /// Returns `value` with its pixel parameters scaled by the context scale.
    pub fn filters(
        &self,
        mut value: crate::ui::rendering::filter::Filters,
    ) -> crate::ui::rendering::filter::Filters {
        value.scale *= self.scale;
        value
    }
    /// Reads variable `name` as insets; a number or pixel length applies to every
    /// edge. Returns `fallback` when unset or of another type.
    pub fn variable_insets(&self, name: &str, fallback: impl Into<Insets>) -> Insets {
        match self.variables.and_then(|v| v.get(name)) {
            Some(crate::ui::styling::VariableValue::Insets(v)) => *v,
            Some(crate::ui::styling::VariableValue::Number(v))
            | Some(crate::ui::styling::VariableValue::Length(Length::Px(v))) => Insets::all(*v),
            _ => fallback.into(),
        }
    }
    /// Reads variable `name` as margins (top, right, bottom, left; `None` is
    /// `auto`). Returns `fallback` when unset or of another type.
    pub fn variable_margins(&self, name: &str, fallback: [Option<f32>; 4]) -> [Option<f32>; 4] {
        match self.variables.and_then(|v| v.get(name)) {
            Some(crate::ui::styling::VariableValue::Insets(v)) => {
                [Some(v.top), Some(v.right), Some(v.bottom), Some(v.left)]
            }
            Some(crate::ui::styling::VariableValue::Number(v))
            | Some(crate::ui::styling::VariableValue::Length(Length::Px(v))) => [Some(*v); 4],
            Some(crate::ui::styling::VariableValue::Length(Length::Auto)) => [None; 4],
            _ => fallback,
        }
    }
    /// Reads variable `name` as a single margin (`None` is `auto`); see
    /// [`variable_margins`](Self::variable_margins).
    pub fn variable_margin(&self, name: &str, fallback: Option<f32>) -> Option<f32> {
        self.variable_margins(name, [fallback; 4])[0]
    }
    /// Returns a copy of the context using `variables`.
    pub fn with_variables<'b>(
        self,
        variables: &'b crate::ui::styling::CssVariables,
    ) -> TemplateContext<'b, R>
    where
        'a: 'b,
    {
        TemplateContext {
            resources: self.resources,
            id: self.id,
            scale: self.scale,
            font: self.font,
            variables: Some(variables),
        }
    }
    /// Reads color variable `name`, or returns `fallback`.
    pub fn variable_color(
        &self,
        name: &str,
        fallback: crate::ui::theme::Color,
    ) -> crate::ui::theme::Color {
        if let Some(crate::ui::styling::VariableValue::Color(v)) =
            self.variables.and_then(|v| v.get(name))
        {
            *v
        } else {
            fallback
        }
    }
    /// Reads variable `name` as a length (a number is taken as pixels), or
    /// returns `fallback`.
    pub fn variable_length(&self, name: &str, fallback: impl Into<Length>) -> Length {
        match self.variables.and_then(|v| v.get(name)) {
            Some(crate::ui::styling::VariableValue::Length(v)) => *v,
            Some(crate::ui::styling::VariableValue::Number(v)) => Length::Px(*v),
            _ => fallback.into(),
        }
    }
    /// Reads variable `name` as a number (a pixel length is accepted), or
    /// returns `fallback`.
    pub fn variable_number(&self, name: &str, fallback: f32) -> f32 {
        match self.variables.and_then(|v| v.get(name)) {
            Some(crate::ui::styling::VariableValue::Number(v))
            | Some(crate::ui::styling::VariableValue::Length(Length::Px(v))) => *v,
            _ => fallback,
        }
    }
    /// Reads font family variable `name`, or returns `fallback`.
    pub fn variable_family(&self, name: &str, fallback: &'static str) -> &'static str {
        if let Some(crate::ui::styling::VariableValue::Family(v)) =
            self.variables.and_then(|v| v.get(name))
        {
            v
        } else {
            fallback
        }
    }
    /// Returns `value` with its pixel parameters scaled by the context scale.
    pub fn shadows(
        &self,
        mut value: crate::ui::rendering::component_shadow::Shadows,
    ) -> crate::ui::rendering::component_shadow::Shadows {
        value.scale *= self.scale;
        value
    }
    /// Returns `value` with its pixel terms scaled by the context scale.
    pub fn computed_length(
        &self,
        mut value: crate::ui::layout::ComputedLength,
    ) -> crate::ui::layout::ComputedLength {
        value.scale *= self.scale;
        value
    }
    /// Scales a pixel line height by the context scale; other variants are unchanged.
    pub fn line_height(
        &self,
        value: crate::ui::styling::LineHeight,
    ) -> crate::ui::styling::LineHeight {
        match value {
            crate::ui::styling::LineHeight::Px(n) => {
                crate::ui::styling::LineHeight::Px(n * self.scale)
            }
            other => other,
        }
    }
    /// Creates a context for element `id`, with no variables, scale `1.` and the
    /// regular system font.
    pub fn new(resources: &'a R, id: impl Into<ElementId>) -> Self {
        Self {
            variables: None,
            resources,
            id: id.into(),
            scale: 1.,
            font: Font::system(false),
        }
    }
    /// Returns a copy of the context for element `id`.
    pub fn with_id(&self, id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            ..*self
        }
    }
    /// Returns a copy of the context with logical scale `scale`.
    ///
    /// # Panics
    ///
    /// Panics if `scale` is not finite and positive.
    pub fn with_scale(self, scale: f32) -> Self {
        assert!(
            scale.is_finite() && scale > 0.,
            "Template scale must be finite and positive"
        );
        Self { scale, ..self }
    }
    /// Returns a copy of the context with default font `font`.
    pub fn with_font(self, font: impl Into<Font>) -> Self {
        Self {
            font: font.into(),
            ..self
        }
    }
    /// Returns a copy of the context for child element `id`.
    pub fn child(&self, id: ElementId) -> Self {
        self.with_id(id)
    }
    /// Scales pixel and computed lengths by the context scale; relative lengths
    /// are unchanged.
    pub fn length(&self, value: impl Into<Length>) -> Length {
        match value.into() {
            Length::Computed(mut value) => {
                value.scale *= self.scale;
                Length::Computed(value)
            }
            Length::Px(n) => Length::Px(n * self.scale),
            other => other,
        }
    }
    /// Scales every edge of `value` by the context scale.
    pub fn insets(&self, value: impl Into<Insets>) -> Insets {
        let p = value.into();
        Insets::edges(
            p.top * self.scale,
            p.right * self.scale,
            p.bottom * self.scale,
            p.left * self.scale,
        )
    }
    /// Scales a pixel value by the context scale.
    pub fn px(&self, value: f32) -> f32 {
        value * self.scale
    }
    /// Scales a translation `(x, y)` in pixels by the context scale.
    pub fn translation(&self, value: (f32, f32)) -> (f32, f32) {
        (value.0 * self.scale, value.1 * self.scale)
    }
    /// Scales the translation of `value` by the context scale.
    pub fn effect(&self, mut value: crate::ui::animation::Effect) -> crate::ui::animation::Effect {
        value.x *= self.scale;
        value.y *= self.scale;
        value
    }
    /// Scales the distances of `value` by the context scale.
    pub fn animation(
        &self,
        value: crate::ui::animation::Animation,
    ) -> crate::ui::animation::Animation {
        value.scaled_distance(self.scale)
    }
}
/// Copies a template context; used by code generated by [`ui!`](macro@crate::ui).
pub fn copy_context<'a, R>(cx: &TemplateContext<'a, R>) -> TemplateContext<'a, R> {
    *cx
}
/// Key type accepted by keyed template loops (integers, strings, [`ElementId`]).
pub trait TemplateKey {
    /// Derives a stable child identifier of `parent` from this key.
    fn scope(self, parent: ElementId) -> ElementId;
}
impl TemplateKey for u64 {
    fn scope(self, parent: ElementId) -> ElementId {
        parent.item(self)
    }
}
impl TemplateKey for usize {
    fn scope(self, parent: ElementId) -> ElementId {
        parent.item(self as u64)
    }
}
impl TemplateKey for u32 {
    fn scope(self, parent: ElementId) -> ElementId {
        parent.item(self as u64)
    }
}
impl TemplateKey for i32 {
    fn scope(self, parent: ElementId) -> ElementId {
        parent.item(self as u64)
    }
}
impl TemplateKey for i64 {
    fn scope(self, parent: ElementId) -> ElementId {
        parent.item(self as u64)
    }
}
impl TemplateKey for &str {
    fn scope(self, parent: ElementId) -> ElementId {
        parent.child(self)
    }
}
impl TemplateKey for &String {
    fn scope(self, parent: ElementId) -> ElementId {
        parent.child(self)
    }
}
impl TemplateKey for String {
    fn scope(self, parent: ElementId) -> ElementId {
        parent.child(&self)
    }
}
impl TemplateKey for ElementId {
    fn scope(self, parent: ElementId) -> ElementId {
        parent.item(self.0)
    }
}
/// Derives the identifier of a keyed child of `parent`, stable across rebuilds.
pub fn keyed(parent: ElementId, key: impl TemplateKey) -> ElementId {
    key.scope(parent)
}

/// Prop conversions are statically dispatched, including optional component fields.
pub trait IntoProp<T> {
    /// Converts the value into the prop type `T`.
    fn into_prop(self) -> T;
}
impl<T> IntoProp<T> for T {
    fn into_prop(self) -> T {
        self
    }
}
impl<T> IntoProp<Option<T>> for T {
    fn into_prop(self) -> Option<T> {
        Some(self)
    }
}
impl IntoProp<String> for &str {
    fn into_prop(self) -> String {
        self.into()
    }
}
impl IntoProp<std::sync::Arc<str>> for &str {
    fn into_prop(self) -> std::sync::Arc<str> {
        self.into()
    }
}
impl IntoProp<f32> for i32 {
    fn into_prop(self) -> f32 {
        self as f32
    }
}
impl IntoProp<f64> for i32 {
    fn into_prop(self) -> f64 {
        self as f64
    }
}
impl IntoProp<Option<f32>> for i32 {
    fn into_prop(self) -> Option<f32> {
        Some(self as f32)
    }
}
/// Converts a template attribute value into a component prop of type `T`.
pub fn prop<T, V: IntoProp<T>>(value: V) -> T {
    value.into_prop()
}
