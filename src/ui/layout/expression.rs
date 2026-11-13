//! Compiled dimension expressions. The parser runs in the proc macro, never per frame.
/// A `calc()`-style length expression tree, built at compile time by the template and style
/// macros. Evaluation never panics; it may yield a non-finite value (e.g. division by zero).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Expression {
    /// A CSS custom property reference, `var(--name, fallback)`.
    Variable {
        /// Property name, including the leading `--`.
        name: &'static str,
        /// Expression used when the variable is unset or has an unusable value.
        fallback: &'static Expression,
        /// Whether the value is a length (numbers are then scaled like pixels).
        length: bool,
    },
    /// Unitless number.
    Number(f32),
    /// Logical pixels, multiplied by [`UnitContext::scale`].
    Px(f32),
    /// Percentage of the reference length.
    Percent(f32),
    /// Multiple of the current font size.
    Em(f32),
    /// Multiple of the root font size.
    Rem(f32),
    /// Percentage of the viewport width.
    Vw(f32),
    /// Percentage of the viewport height.
    Vh(f32),
    /// Sum of two expressions.
    Add(&'static Expression, &'static Expression),
    /// Difference of two expressions.
    Sub(&'static Expression, &'static Expression),
    /// Product of two expressions.
    Mul(&'static Expression, &'static Expression),
    /// Quotient of two expressions.
    Div(&'static Expression, &'static Expression),
    /// Smallest of a non-empty list, `min(...)`.
    Min(&'static [Expression]),
    /// Largest of a non-empty list, `max(...)`.
    Max(&'static [Expression]),
    /// `clamp(min, preferred, max)`.
    Clamp(
        &'static Expression,
        &'static Expression,
        &'static Expression,
    ),
}
/// Values that relative units resolve against. The default uses 16 px fonts, an empty
/// viewport and a scale of 1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnitContext {
    /// Current font size, for `em`.
    pub font: f32,
    /// Root font size, for `rem`.
    pub root_font: f32,
    /// Viewport width, for `vw`.
    pub viewport_width: f32,
    /// Viewport height, for `vh`.
    pub viewport_height: f32,
    /// Scale factor applied to pixel values.
    pub scale: f32,
    /// Physical pixels per logical pixel, to which border widths snap.
    pub pixel_scale: f32,
}
impl Default for UnitContext {
    fn default() -> Self {
        Self {
            font: 16.,
            root_font: 16.,
            viewport_width: 0.,
            viewport_height: 0.,
            scale: 1.,
            pixel_scale: 1.,
        }
    }
}
impl Expression {
    /// Evaluates the expression; percentages resolve against `reference` and variables are
    /// looked up in `variables`. The result may be non-finite: `NaN` when nesting exceeds 64
    /// levels, when the evaluation visits more than 4096 nodes (a variable that refers to
    /// itself several times) or a `min`/`max`/`clamp` operand is not finite, infinity on
    /// division by zero.
    pub fn evaluate(
        self,
        c: UnitContext,
        reference: f32,
        variables: Option<&crate::ui::styling::CssVariables>,
    ) -> f32 {
        fn eval(
            e: Expression,
            c: UnitContext,
            reference: f32,
            vars: Option<&crate::ui::styling::CssVariables>,
            depth: usize,
            budget: &mut u32,
        ) -> f32 {
            // Budget de nœuds : une variable qui se référence deux fois
            // (`calc(var(--a) + var(--a))`) doublerait le travail à chaque niveau.
            if depth > 64 || *budget == 0 {
                *budget = 0;
                return f32::NAN;
            }
            *budget -= 1;
            let mut value = |e: &Expression| eval(*e, c, reference, vars, depth + 1, budget);
            match e {
                Expression::Variable {
                    name,
                    fallback,
                    length,
                } => {
                    use crate::ui::{element::Length, styling::VariableValue};
                    match vars.and_then(|v| v.get(name)) {
                        Some(VariableValue::Number(n)) => {
                            if length {
                                *n * c.scale
                            } else {
                                *n
                            }
                        }
                        Some(VariableValue::Length(length_value)) if length => match length_value {
                            Length::Px(n) => n * c.scale,
                            Length::Percent(n) => n * reference / 100.,
                            Length::Computed(v) => eval(
                                *v.expression,
                                UnitContext {
                                    scale: c.scale * v.scale,
                                    ..c
                                },
                                reference,
                                vars,
                                depth + 1,
                                budget,
                            ),
                            _ => value(fallback),
                        },
                        _ => value(fallback),
                    }
                }
                Expression::Number(n) => n,
                Expression::Px(n) => n * c.scale,
                Expression::Percent(n) => n * reference / 100.,
                Expression::Em(n) => n * c.font,
                Expression::Rem(n) => n * c.root_font,
                Expression::Vw(n) => n * c.viewport_width / 100.,
                Expression::Vh(n) => n * c.viewport_height / 100.,
                Expression::Add(a, b) => value(a) + value(b),
                Expression::Sub(a, b) => value(a) - value(b),
                Expression::Mul(a, b) => value(a) * value(b),
                Expression::Div(a, b) => value(a) / value(b),
                Expression::Min(items) | Expression::Max(items) => {
                    let min = matches!(e, Expression::Min(_));
                    let mut result = if min {
                        f32::INFINITY
                    } else {
                        f32::NEG_INFINITY
                    };
                    for item in items {
                        let n = value(item);
                        if !n.is_finite() {
                            return f32::NAN;
                        }
                        result = if min { result.min(n) } else { result.max(n) };
                    }
                    result
                }
                Expression::Clamp(a, b, d) => {
                    let (a, b, d) = (value(a), value(b), value(d));
                    if [a, b, d].iter().all(|n| n.is_finite()) {
                        a.max(b.min(d))
                    } else {
                        f32::NAN
                    }
                }
            }
        }
        eval(self, c, reference, variables, 0, &mut 4096)
    }
    /// Returns `true` if all numbers are finite, variable names start with `--`, lists are
    /// non-empty and the tree stays within size limits (depth 32, 256 nodes).
    pub fn valid(self) -> bool {
        fn walk(e: Expression, depth: usize, count: &mut usize) -> bool {
            *count += 1;
            if depth > 32 || *count > 256 {
                return false;
            }
            match e {
                Expression::Variable { name, fallback, .. } => {
                    name.starts_with("--") && walk(*fallback, depth + 1, count)
                }
                Expression::Number(n)
                | Expression::Px(n)
                | Expression::Percent(n)
                | Expression::Em(n)
                | Expression::Rem(n)
                | Expression::Vw(n)
                | Expression::Vh(n) => n.is_finite(),
                Expression::Add(a, b)
                | Expression::Sub(a, b)
                | Expression::Mul(a, b)
                | Expression::Div(a, b) => walk(*a, depth + 1, count) && walk(*b, depth + 1, count),
                Expression::Clamp(a, b, c) => {
                    walk(*a, depth + 1, count)
                        && walk(*b, depth + 1, count)
                        && walk(*c, depth + 1, count)
                }
                Expression::Min(items) | Expression::Max(items) => {
                    !items.is_empty() && items.iter().all(|e| walk(*e, depth + 1, count))
                }
            }
        }
        walk(self, 0, &mut 0)
    }
    /// Returns `true` if the expression contains a percentage, which needs a definite
    /// reference length.
    pub fn relative(self) -> bool {
        match self {
            Expression::Percent(_) => true,
            Expression::Variable { fallback, .. } => fallback.relative(),
            Expression::Add(a, b)
            | Expression::Sub(a, b)
            | Expression::Mul(a, b)
            | Expression::Div(a, b) => a.relative() || b.relative(),
            Expression::Clamp(a, b, c) => a.relative() || b.relative() || c.relative(),
            Expression::Min(items) | Expression::Max(items) => items.iter().any(|e| e.relative()),
            _ => false,
        }
    }
}
/// A length defined by an [`Expression`], with an additional scale factor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComputedLength {
    /// The expression to evaluate.
    pub expression: &'static Expression,
    /// Extra multiplier applied to pixel values; must be finite and positive.
    pub scale: f32,
}
impl ComputedLength {
    /// Wraps `expression` with a scale of 1.
    pub const fn new(expression: &'static Expression) -> Self {
        Self {
            expression,
            scale: 1.,
        }
    }
    /// Evaluates the expression with this length's scale applied (see [`Expression::evaluate`]).
    pub fn evaluate(
        self,
        mut context: UnitContext,
        reference: f32,
        variables: Option<&crate::ui::styling::CssVariables>,
    ) -> f32 {
        context.scale *= self.scale;
        self.expression.evaluate(context, reference, variables)
    }
    /// Returns `true` if the scale is finite and positive and the expression is valid.
    pub fn valid(self) -> bool {
        self.scale.is_finite() && self.scale > 0. && self.expression.valid()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::{element::Length, styling::CssVariables};

    #[test]
    fn a_variable_referencing_itself_twice_stops_early() {
        // `--a: calc(var(--a) + var(--a))` : sans budget, 2^32 nœuds avant la profondeur.
        static A: Expression = Expression::Variable {
            name: "--a",
            fallback: &Expression::Px(1.),
            length: true,
        };
        static SUM: Expression = Expression::Add(&A, &A);
        let mut vars = CssVariables::default();
        vars.set_length("--a", Length::Computed(ComputedLength::new(&SUM)));
        let start = std::time::Instant::now();
        let n = SUM.evaluate(UnitContext::default(), 100., Some(&vars));
        assert!(n.is_nan());
        assert!(start.elapsed() < std::time::Duration::from_secs(1));
    }
}
