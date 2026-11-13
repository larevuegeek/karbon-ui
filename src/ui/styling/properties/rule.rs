//! Déclarations d'une règle ([`Properties`]) : validation et superposition.
use super::{BackgroundImages, ControlPart, ImageFit, StateEffect, Typography};
use crate::ui::styling::{
    ColorExpression, LayoutProperties, VariableValue, tokens::DynamicDeclaration,
};
use crate::ui::{element::Insets, theme::Color};

#[derive(Clone, Debug, Default, PartialEq)]
/// Style declarations of one rule (a class or a state block), as produced by
/// [`styles!`](crate::styles) and [`css_file!`](crate::css_file).
///
/// Every property is optional; unset properties leave the element unchanged.
pub struct Properties {
    /// Visual filters (blur, brightness…).
    pub filters: Option<crate::ui::rendering::filter::Filters>,
    /// Layout declarations (size, margins, flex…).
    pub layout: LayoutProperties,
    /// Box shadows.
    pub shadows: Option<crate::ui::rendering::component_shadow::Shadows>,
    /// Per-corner radii in pixels (non-negative), ordered top-left, top-right,
    /// bottom-right, bottom-left; overrides [`radius`](Self::radius).
    pub corner_radii: Option<[f32; 4]>,
    /// Image fitting mode.
    pub image_fit: Option<ImageFit>,
    /// Image position within its box (CSS `object-position`), as horizontal and
    /// vertical fractions of the free space (`0.5` centers).
    pub image_position: Option<[f32; 2]>,
    /// Font size given as an expression (`em`, `rem`, `vw`, `calc()`…), evaluated
    /// against the inherited font size and the viewport.
    pub font_size_expression: Option<crate::ui::layout::ComputedLength>,
    /// Text properties, inherited by descendants.
    pub text: Typography,
    /// Background color.
    pub background: Option<Color>,
    /// Border color.
    pub border: Option<Color>,
    /// Border width in pixels (non-negative).
    pub border_width: Option<f32>,
    /// Uniform corner radius in pixels (non-negative).
    pub radius: Option<f32>,
    /// Padding on all edges, in pixels (non-negative).
    pub padding: Option<Insets>,
    /// Accent color used by controls (checkbox, slider…).
    pub accent: Option<Color>,
    /// Colours evaluated while styles resolve (custom properties, `currentColor`),
    /// indexed by [`ColorTarget`]; they replace the matching constant colour.
    pub color_expressions: [Option<&'static ColorExpression>; 4],
    /// Custom properties declared by the rule (`--h: 210`), inherited by descendants.
    pub custom: Option<&'static [(&'static str, VariableValue)]>,
    /// Opacity and/or transform applied while the rule matches (state rules other than a
    /// plain `:hover`/`:active`, such as `.group:hover .x`); animated by the element's
    /// transition.
    pub effect: Option<StateEffect>,
    /// Border width of each side (top, right, bottom, left), overriding
    /// [`border_width`](Self::border_width).
    pub border_edges: [Option<f32>; 4],
    /// Border colour of each side (top, right, bottom, left), overriding
    /// [`border`](Self::border).
    pub border_colors: [Option<Color>; 4],
    /// Outline width in pixels, drawn outside the border box without affecting layout.
    pub outline_width: Option<f32>,
    /// Outline colour; the text colour when unset.
    pub outline_color: Option<Color>,
    /// Distance between the border box and the outline (may be negative).
    pub outline_offset: Option<f32>,
    /// Filters applied to what is painted behind the element (`backdrop-filter`).
    pub backdrop: Option<crate::ui::rendering::filter::Filters>,
    /// Paint the background through the text only (`background-clip: text`).
    pub text_clip: Option<bool>,
    /// Pointer shape over the element and its subtree (`cursor`).
    pub cursor: Option<crate::ui::runtime::Cursor>,
    /// Declarations of a framework stylesheet evaluated while styles resolve, because
    /// their value depends on custom properties known only at run time; applied over the
    /// constant fields of the same block.
    pub dynamic: Option<&'static [DynamicDeclaration]>,
    /// Colour of a text input's placeholder (`::placeholder { color }`); the theme's
    /// muted colour when unset.
    pub placeholder_color: Option<Color>,
    /// SVG image layers painted over the background colour, with their placement
    /// (`background-image`, `background-position`, `background-size`,
    /// `background-repeat` of framework stylesheets).
    pub background_images: BackgroundImages,
    /// `appearance: none` (`Some(true)`): a checkbox, switch, slider or select drops its
    /// native drawing and is painted from its CSS box instead (background, border,
    /// background image, slider parts). Behaviour and accessibility stay native.
    pub appearance_none: Option<bool>,
    /// Radius of single corners (top-left, top-right, bottom-right, bottom-left), in
    /// pixels, laid over the radii of the block and of the blocks below it
    /// (`border-top-right-radius: 0` of a button inside a group).
    pub corner_edges: [Option<f32>; 4],
    /// Thumb of a slider drawn with `appearance: none` (`::-webkit-slider-thumb`).
    pub slider_thumb: ControlPart,
    /// Track of a slider drawn with `appearance: none`
    /// (`::-webkit-slider-runnable-track`).
    pub slider_track: ControlPart,
    /// Transition evaluated from a framework stylesheet (`transition`, possibly a composed
    /// list read through `var()`); replaces the element's own transition while set.
    pub transition: Option<crate::ui::animation::Transition>,
    /// Animation longhands of a state or media rule (`.track.reverse
    /// { animation-direction: reverse }`, `.paused .bar { animation-play-state: paused }`),
    /// replacing those of the element's animation while the rule matches.
    pub animation: crate::ui::animation::AnimationOverride,
}
/// Property receiving a [`ColorExpression`] of
/// [`Properties::color_expressions`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColorTarget {
    /// Text colour (`color`).
    Text = 0,
    /// Background colour.
    Background = 1,
    /// Border colour.
    Border = 2,
    /// Accent colour.
    Accent = 3,
}
impl Properties {
    /// Checks that every set value is finite and within range; returns an
    /// [`Error::InvalidInput`](crate::Error::InvalidInput) error otherwise.
    pub fn validate(&self) -> crate::Result<()> {
        if self.filters.is_some_and(|f| !f.valid()) || self.backdrop.is_some_and(|f| !f.valid()) {
            return Err(crate::Error::invalid_input(
                "invalid native filter parameters",
            ));
        }
        if self.shadows.as_ref().is_some_and(|s| !s.valid()) {
            return Err(crate::Error::invalid_input("invalid component shadows"));
        }
        if !self.background_images.valid()
            || !self.slider_thumb.valid()
            || !self.slider_track.valid()
        {
            return Err(crate::Error::invalid_input(
                "invalid background images or control parts",
            ));
        }
        if self
            .corner_radii
            .is_some_and(|a| a.iter().any(|n| !n.is_finite() || *n < 0.))
            || self
                .corner_edges
                .iter()
                .flatten()
                .any(|n| !n.is_finite() || *n < 0.)
        {
            return Err(crate::Error::invalid_input("invalid corner radius"));
        }
        if self
            .image_position
            .is_some_and(|p| p.iter().any(|v| !v.is_finite()))
        {
            return Err(crate::Error::invalid_input("invalid image position"));
        }
        if self.font_size_expression.is_some_and(|v| !v.valid()) {
            return Err(crate::Error::invalid_input("invalid font expression"));
        }
        if !self.layout.spacing.valid() {
            return Err(crate::Error::invalid_input("invalid spacing expression"));
        }
        if self.color_expressions.iter().flatten().any(|e| !e.valid())
            || self
                .custom
                .is_some_and(|c| c.iter().any(|(n, _)| !n.starts_with("--") || n.len() < 3))
        {
            return Err(crate::Error::invalid_input("invalid colour expression"));
        }
        self.text.validate()?;
        if [self.radius, self.border_width, self.outline_width]
            .into_iter()
            .chain(self.border_edges)
            .flatten()
            .any(|n| !n.is_finite() || n < 0.)
            || self.outline_offset.is_some_and(|n| !n.is_finite())
            || self.padding.is_some_and(|p| {
                [p.left, p.top, p.right, p.bottom]
                    .iter()
                    .any(|n| !n.is_finite() || *n < 0.)
            })
        {
            return Err(crate::Error::invalid_input("invalid native appearance"));
        }
        Ok(())
    }
    /// Layers `self` over `base`: each property set on `self` wins. Shorthands
    /// set on `self` (`padding`, `radius`, font size) also discard the matching
    /// longhands from `base`.
    pub fn over(self, base: Self) -> Self {
        let mut base_layout = base.layout;
        if self.padding.is_some() {
            base_layout.padding_edges = [None; 4];
            base_layout.spacing.padding = [None; 4];
        }
        Self {
            filters: self.filters.or(base.filters),
            layout: self.layout.over(base_layout),
            shadows: self.shadows.or(base.shadows),
            corner_radii: {
                let own = self.corner_radii.or(if self.radius.is_some() {
                    None
                } else {
                    base.corner_radii
                });
                if self.corner_edges.iter().any(Option::is_some) {
                    // Coins isolés posés sur les rayons en vigueur.
                    let mut corners = own
                        .or(self.radius.or(base.radius).map(|r| [r; 4]))
                        .unwrap_or([0.; 4]);
                    for (corner, edge) in corners.iter_mut().zip(self.corner_edges) {
                        if let Some(edge) = edge {
                            *corner = edge;
                        }
                    }
                    Some(corners)
                } else {
                    own
                }
            },
            corner_edges: [None; 4],
            image_fit: self.image_fit.or(base.image_fit),
            image_position: self.image_position.or(base.image_position),
            font_size_expression: self.font_size_expression.or(if self.text.size.is_some() {
                None
            } else {
                base.font_size_expression
            }),
            text: self.text.over(base.text),
            background: self.background.or(base.background),
            border: self.border.or(base.border),
            border_width: self.border_width.or(base.border_width),
            radius: self.radius.or(base.radius),
            padding: self.padding.or(base.padding),
            accent: self.accent.or(base.accent),
            // Une couleur constante de `self` remplace l'expression de `base`.
            color_expressions: {
                let constant = [
                    self.text.color.is_some(),
                    self.background.is_some(),
                    self.border.is_some(),
                    self.accent.is_some(),
                ];
                std::array::from_fn(|i| {
                    self.color_expressions[i].or(if constant[i] {
                        None
                    } else {
                        base.color_expressions[i]
                    })
                })
            },
            custom: self.custom.or(base.custom),
            effect: match (self.effect, base.effect) {
                (Some(a), Some(b)) => Some(a.over(b)),
                (a, b) => a.or(b),
            },
            // Les raccourcis `border-width` / `border-color` remplacent les côtés de `base`.
            border_edges: std::array::from_fn(|i| {
                self.border_edges[i].or(if self.border_width.is_some() {
                    None
                } else {
                    base.border_edges[i]
                })
            }),
            border_colors: std::array::from_fn(|i| {
                self.border_colors[i].or(if self.border.is_some() {
                    None
                } else {
                    base.border_colors[i]
                })
            }),
            outline_width: self.outline_width.or(base.outline_width),
            outline_color: self.outline_color.or(base.outline_color),
            outline_offset: self.outline_offset.or(base.outline_offset),
            backdrop: self.backdrop.or(base.backdrop),
            text_clip: self.text_clip.or(base.text_clip),
            cursor: self.cursor.or(base.cursor),
            dynamic: self.dynamic.or(base.dynamic),
            placeholder_color: self.placeholder_color.or(base.placeholder_color),
            background_images: self.background_images.over(base.background_images),
            appearance_none: self.appearance_none.or(base.appearance_none),
            slider_thumb: self.slider_thumb.over(base.slider_thumb),
            slider_track: self.slider_track.over(base.slider_track),
            transition: self.transition.or(base.transition),
            animation: self.animation.over(base.animation),
        }
    }
    /// Sides drawn individually: one side has its own width or colour.
    pub fn border_sides(&self) -> bool {
        self.border_edges.iter().any(Option::is_some)
            || self.border_colors.iter().any(Option::is_some)
    }
}
