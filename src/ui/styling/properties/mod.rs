//! Types des déclarations de style : typographie, fonds, apparence, règles, sélecteurs et
//! style calculé.
mod appearance;
mod background;
mod computed;
mod rule;
mod selector;
mod typography;

pub use appearance::{ControlPart, ImageFit, StateEffect};
pub use background::{
    BACKGROUND_LAYERS, BackgroundImages, BackgroundLength, BackgroundPosition, BackgroundSize,
};
pub use computed::ComputedStyle;
pub use rule::{ColorTarget, Properties};
pub use selector::{AttributeMatch, AttributeOperator, StyleState};
pub use typography::{
    LetterSpacing, LineHeight, TextAlign, TextDecoration, TextOverflow, TextTransform, Typography,
    WhiteSpace,
};
