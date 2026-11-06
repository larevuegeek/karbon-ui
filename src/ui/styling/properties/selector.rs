//! Conditions d'application d'un bloc de style (sélecteurs, requêtes) et leur spécificité.
use crate::ui::styling::{ContainerQuery, MediaQuery};

#[derive(Clone, Copy, Debug, PartialEq)]
/// Condition under which a style block applies (CSS pseudo-classes, classes
/// and media queries).
pub enum StyleState {
    /// Matches when any of the conditions match (`:is()`).
    Any(&'static [StyleState]),
    /// Matches when the viewport satisfies the query (`@media`).
    Media(MediaQuery),
    /// Matches when the nearest matching ancestor container satisfies the query
    /// (`@container`).
    Container(ContainerQuery),
    /// Matches when all of the conditions match (compound selector).
    All(&'static [StyleState]),
    /// Matches when the condition does not match (`:not()`).
    Not(&'static StyleState),
    /// Matches elements carrying this class.
    Class(&'static str),
    /// Matches when the condition matches, without adding to the specificity
    /// (`:where()`).
    Where(&'static StyleState),
    /// Some ancestor matches (descendant combinator, `.a .b`).
    Ancestor(&'static StyleState),
    /// The parent matches (child combinator, `.a > .b`).
    Parent(&'static StyleState),
    /// The immediately preceding sibling matches (`.a + .b`).
    PreviousSibling(&'static StyleState),
    /// Some preceding sibling matches (`.a ~ .b`).
    AnyPreviousSibling(&'static StyleState),
    /// `:first-child`.
    FirstChild,
    /// `:last-child`.
    LastChild,
    /// `:only-child`.
    OnlyChild,
    /// `:empty`: no children and no accessible name.
    Empty,
    /// `:nth-child(an+b)`, given as `(a, b)`.
    NthChild(i32, i32),
    /// `:hover`: the pointer is over the element.
    Hover,
    /// `:active`: the element or a descendant is pressed.
    Active,
    /// `:focus`.
    Focus,
    /// `:focus-visible`: focused via the keyboard.
    FocusVisible,
    /// `:focus-within`: the element or a descendant is focused.
    FocusWithin,
    /// `:disabled`.
    Disabled,
    /// `:enabled`.
    Enabled,
    /// `:checked`: a checked toggle or selected radio button.
    Checked,
    /// `:indeterminate`: a toggle in the mixed state.
    Indeterminate,
    /// `:read-only`: a read-only text input or slider.
    ReadOnly,
    /// `:read-write`: an editable text input.
    ReadWrite,
    /// `:placeholder-shown`: an empty text input showing its placeholder.
    PlaceholderShown,
    /// Matches elements written with this HTML tag (framework stylesheets).
    Tag(&'static str),
    /// Attribute selector (`[data-bs-theme=dark]`, `[type=checkbox]`, `#id`).
    Attribute(&'static AttributeMatch),
    /// `:root`: the outermost element written with an HTML tag.
    Root,
    /// `:first-of-type`.
    FirstOfType,
    /// `:last-of-type`.
    LastOfType,
    /// `:only-of-type`.
    OnlyOfType,
    /// `:nth-last-child(an+b)`, given as `(a, b)`.
    NthLastChild(i32, i32),
    /// `:nth-of-type(an+b)`, given as `(a, b)`.
    NthOfType(i32, i32),
    /// `:nth-last-of-type(an+b)`, given as `(a, b)`.
    NthLastOfType(i32, i32),
}
/// Comparison of an [`AttributeMatch`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AttributeOperator {
    /// `[name]`.
    Exists,
    /// `[name=value]`.
    Equals,
    /// `[name~=value]`: one of the space-separated words.
    Includes,
    /// `[name|=value]`: equal, or starting with `value-`.
    DashMatch,
    /// `[name^=value]`.
    Prefix,
    /// `[name$=value]`.
    Suffix,
    /// `[name*=value]`.
    Substring,
}
/// Attribute selector of a framework stylesheet, matched against the attributes the
/// template wrote on the element.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AttributeMatch {
    /// Attribute name, lower case.
    pub name: &'static str,
    /// Comparison.
    pub operator: AttributeOperator,
    /// Compared value.
    pub value: &'static str,
    /// ASCII case-insensitive comparison (`i` flag).
    pub case_insensitive: bool,
}
impl AttributeMatch {
    /// Returns `true` when an attribute of value `actual` satisfies the selector.
    pub fn matches(&self, actual: &str) -> bool {
        let fold = |s: &str| {
            if self.case_insensitive {
                s.to_ascii_lowercase()
            } else {
                s.to_owned()
            }
        };
        let (v, w) = (fold(actual), fold(self.value));
        match self.operator {
            AttributeOperator::Exists => true,
            AttributeOperator::Equals => v == w,
            AttributeOperator::Includes => v.split_whitespace().any(|p| p == w),
            AttributeOperator::DashMatch => v == w || v.starts_with(&format!("{w}-")),
            AttributeOperator::Prefix => !w.is_empty() && v.starts_with(&w),
            AttributeOperator::Suffix => !w.is_empty() && v.ends_with(&w),
            AttributeOperator::Substring => !w.is_empty() && v.contains(&w),
        }
    }
}
impl StyleState {
    /// Returns the selector specificity used to order matching blocks; media and
    /// container queries count for zero.
    pub fn specificity(self) -> u32 {
        match self {
            Self::All(states) => states.iter().map(|s| s.specificity()).sum(),
            Self::Any(states) => states.iter().map(|s| s.specificity()).max().unwrap_or(0),
            Self::Media(_) | Self::Container(_) | Self::Where(_) => 0,
            Self::Not(state)
            | Self::Ancestor(state)
            | Self::Parent(state)
            | Self::PreviousSibling(state)
            | Self::AnyPreviousSibling(state) => state.specificity(),
            _ => 1,
        }
    }
}
