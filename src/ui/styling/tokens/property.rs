//! Propriétés évaluables pendant la résolution des styles et leurs déclarations.
use super::token::Token;

/// Property set by a [`DynamicDeclaration`].
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DynamicProperty {
    Color,
    BackgroundColor,
    Background,
    BorderColor,
    BorderTopColor,
    BorderRightColor,
    BorderBottomColor,
    BorderLeftColor,
    AccentColor,
    OutlineColor,
    Outline,
    OutlineWidth,
    OutlineOffset,
    Border,
    BorderTop,
    BorderRight,
    BorderBottom,
    BorderLeft,
    BorderWidth,
    BorderTopWidth,
    BorderRightWidth,
    BorderBottomWidth,
    BorderLeftWidth,
    BorderRadius,
    Padding,
    PaddingTop,
    PaddingRight,
    PaddingBottom,
    PaddingLeft,
    Margin,
    MarginTop,
    MarginRight,
    MarginBottom,
    MarginLeft,
    Width,
    Height,
    MinWidth,
    MinHeight,
    MaxWidth,
    MaxHeight,
    FlexBasis,
    FlexGrow,
    FlexShrink,
    Gap,
    RowGap,
    ColumnGap,
    FontSize,
    FontWeight,
    LineHeight,
    LetterSpacing,
    Opacity,
    BoxShadow,
    Transform,
    Translate,
    Scale,
    Rotate,
    BorderTopLeftRadius,
    BorderTopRightRadius,
    BorderBottomRightRadius,
    BorderBottomLeftRadius,
    BackgroundImage,
    BackgroundPosition,
    BackgroundSize,
    BackgroundRepeat,
    Appearance,
    /// `background-color` of `::-webkit-slider-thumb`.
    ThumbBackground,
    /// `width` of `::-webkit-slider-thumb`.
    ThumbWidth,
    /// `height` of `::-webkit-slider-thumb`.
    ThumbHeight,
    /// `border-radius` of `::-webkit-slider-thumb`.
    ThumbRadius,
    /// `border` of `::-webkit-slider-thumb`.
    ThumbBorder,
    /// `box-shadow` of `::-webkit-slider-thumb`.
    ThumbShadow,
    /// `background-color` of `::-webkit-slider-runnable-track`.
    TrackBackground,
    /// `height` of `::-webkit-slider-runnable-track`.
    TrackHeight,
    /// `border-radius` of `::-webkit-slider-runnable-track`.
    TrackRadius,
    /// `border` of `::-webkit-slider-runnable-track`.
    TrackBorder,
    /// `transition`: a composed list (`color .15s ease-in-out, box-shadow .15s …`), often
    /// read through `var()`. The engine keeps one timing per element: that of the first
    /// item it can animate (colours, opacity and transform, `width` / `height`); the other
    /// properties of the list change at once.
    Transition,
}
impl DynamicProperty {
    /// Property of CSS name `name` that can be evaluated at run time.
    pub fn from_name(name: &str) -> Option<Self> {
        use DynamicProperty::*;
        Some(match name {
            "color" => Color,
            "background-color" => BackgroundColor,
            "background" => Background,
            "border-color" => BorderColor,
            "border-top-color" => BorderTopColor,
            "border-right-color" => BorderRightColor,
            "border-bottom-color" => BorderBottomColor,
            "border-left-color" => BorderLeftColor,
            "accent-color" => AccentColor,
            "outline-color" => OutlineColor,
            "outline" => Outline,
            "outline-width" => OutlineWidth,
            "outline-offset" => OutlineOffset,
            "border" => Border,
            "border-top" => BorderTop,
            "border-right" => BorderRight,
            "border-bottom" => BorderBottom,
            "border-left" => BorderLeft,
            "border-width" => BorderWidth,
            "border-top-width" => BorderTopWidth,
            "border-right-width" => BorderRightWidth,
            "border-bottom-width" => BorderBottomWidth,
            "border-left-width" => BorderLeftWidth,
            "border-radius" => BorderRadius,
            "padding" => Padding,
            "padding-top" => PaddingTop,
            "padding-right" => PaddingRight,
            "padding-bottom" => PaddingBottom,
            "padding-left" => PaddingLeft,
            "margin" => Margin,
            "margin-top" => MarginTop,
            "margin-right" => MarginRight,
            "margin-bottom" => MarginBottom,
            "margin-left" => MarginLeft,
            "width" => Width,
            "height" => Height,
            "min-width" => MinWidth,
            "min-height" => MinHeight,
            "max-width" => MaxWidth,
            "max-height" => MaxHeight,
            "flex-basis" => FlexBasis,
            "flex-grow" => FlexGrow,
            "flex-shrink" => FlexShrink,
            "gap" => Gap,
            "row-gap" => RowGap,
            "column-gap" => ColumnGap,
            "font-size" => FontSize,
            "font-weight" => FontWeight,
            "line-height" => LineHeight,
            "letter-spacing" => LetterSpacing,
            "opacity" => Opacity,
            "box-shadow" => BoxShadow,
            "transform" => Transform,
            "translate" => Translate,
            "scale" => Scale,
            "rotate" => Rotate,
            "border-top-left-radius" => BorderTopLeftRadius,
            "border-top-right-radius" => BorderTopRightRadius,
            "border-bottom-right-radius" => BorderBottomRightRadius,
            "border-bottom-left-radius" => BorderBottomLeftRadius,
            "background-image" => BackgroundImage,
            "background-position" => BackgroundPosition,
            "background-size" => BackgroundSize,
            "background-repeat" => BackgroundRepeat,
            "appearance" => Appearance,
            "transition" => Transition,
            _ => return None,
        })
    }
}
/// Declaration whose value is evaluated while styles resolve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DynamicDeclaration {
    /// Target property.
    pub property: DynamicProperty,
    /// Pre-tokenized value, possibly containing `var()` groups.
    pub value: &'static [Token],
}
