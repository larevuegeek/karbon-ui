//! Native computed styles. Declaration values are shared by CSS and Rust.
//! Styles resolve when the tree or interaction state changes, before layout.
/// Colours evaluated while styles resolve (`currentColor`, custom properties).
pub mod color;
mod layout;
mod media;
/// Painting helpers that honour per-corner radii from computed styles.
pub mod paint;
mod properties;
mod spacing;
pub(crate) mod variables;
pub use color::{ColorExpression, MixSpace, color_mix};
pub use layout::LayoutProperties;
pub use media::{Container, ContainerQuery, ContainerType, MediaQuery};
pub use spacing::SpacingExpressions;
pub use variables::{CssVariables, VariableValue};
pub(crate) mod resolve;
/// Text formatting, measurement and drawing driven by computed typography.
pub mod text;
/// Pre-tokenized values of framework stylesheets evaluated while styles resolve.
pub mod tokens;
pub use properties::*;
pub use resolve::{
    AnimatedSizes, ElementColors, ElementSizes, StyleChanges, StyleEnvironment, resolve,
};

use crate::ui::element::Style;
#[derive(Default)]
pub(crate) struct Styles {
    /// Variables in scope: those of the template context, overlaid with the custom
    /// properties inherited from ancestors once styles resolve.
    pub variables: Option<CssVariables>,
    /// Variables of the template context, saved before the first inherited overlay.
    pub app_variables: Option<Option<CssVariables>>,
    /// Custom properties declared by the template (`style:--h={expr}`).
    pub custom: Vec<(&'static str, VariableValue)>,
    /// Custom properties declared by stylesheet classes.
    pub class_custom: Vec<(&'static str, VariableValue)>,
    /// Alpha mask of the element and its subtree (`mask-image`).
    pub mask: Option<std::sync::Arc<crate::ui::rendering::brush::LinearGradient>>,
    pub shadow_cache: crate::ui::rendering::component_shadow::Cache,
    pub classes: Vec<&'static str>,
    pub normal: Properties,
    pub states: Vec<(StyleState, Properties, u32)>,
    /// Blocks applied to the children that match (`.list > :not(:last-child)`),
    /// under the children's own declarations.
    pub child_states: Vec<(StyleState, Properties, u32)>,
    pub base: Option<Style>,
    pub base_layout: Option<crate::ui::element::Layout>,
    pub base_surface: Option<Option<std::sync::Arc<crate::ui::style::SurfaceStyle>>>,
    pub base_border: Option<crate::ui::element::Insets>,
    pub base_rules: Option<Option<Box<crate::ui::element::LayoutRules>>>,
    /// Transition du template, gardée quand une règle de la feuille la remplace.
    pub base_transition: Option<Option<crate::ui::animation::Transition>>,
    /// Animation du template, gardée quand une règle d'état en remplace des réglages, et
    /// celle affichée par la dernière résolution (une autre valeur vient d'une nouvelle
    /// vue : elle devient la base).
    pub base_animation: Option<(
        Option<crate::ui::animation::Animation>,
        Option<crate::ui::animation::Animation>,
    )>,
    pub computed: ComputedStyle,
    pub ready: bool,
    /// Query container type declared by `container-type` (`@container`).
    pub container_type: Option<ContainerType>,
    /// Container names declared by `container-name`.
    pub container_names: &'static [&'static str],
    /// Content box size of the container seen by the last style resolution.
    pub container_seen: Option<crate::ui::element::SizeF>,
    /// `pointer-events`: `Some(false)` for `none`, `Some(true)` for `auto` (inherited).
    pub pointer_events: Option<bool>,
    /// HTML tag written in the template (framework stylesheets), empty otherwise.
    pub tag: &'static str,
    /// Attributes written in the template, for attribute selectors.
    pub attributes: Vec<(&'static str, std::borrow::Cow<'static, str>)>,
    /// Custom properties of the framework stylesheet's `:root`, read last by `var()`.
    pub globals: Option<tokens::Globals>,
    /// Template scale applied to the pixel values of dynamic declarations (`0`: 1).
    pub scale: f32,
    /// Pixels physiques par pixel logique vus par la dernière résolution des styles
    /// (`0` : pas encore résolu) ; les boîtes des feuilles de frameworks sont peintes
    /// calées sur ces pixels, comme dans un navigateur.
    pub pixel_scale: f32,
}
impl Styles {
    /// Boîte de bordure peinte : celle d'une feuille de framework (`globals`) a ses bords
    /// arrondis aux pixels physiques, comme les navigateurs (une bordure de 1 px à 150 %
    /// reste un trait net d'un pixel, sur ses quatre côtés). Les autres feuilles gardent
    /// la boîte du layout.
    pub(crate) fn painted_rect(
        &self,
        rect: crate::ui::geometry::RectF,
    ) -> crate::ui::geometry::RectF {
        let s = self.pixel_scale;
        if self.globals.is_none() || !(s.is_finite() && s > 0.) {
            return rect;
        }
        // Demi-pixel arrondi vers le bas et la droite, avec une tolérance d'un 1/64 de
        // pixel (l'unité de layout des navigateurs) : un bord à 1210,4999 px après un
        // défilement reste au même pixel qu'à 1210,5.
        let snap = |v: f32| (v * s + 0.5 + 1. / 64.).floor() / s;
        crate::ui::geometry::RectF {
            left: snap(rect.left),
            top: snap(rect.top),
            right: snap(rect.right),
            bottom: snap(rect.bottom),
        }
    }
}
