use super::*;
/// One drawing instruction of a [`Scene`], in logical units.
///
/// `Push*` commands open a scope that must be closed by the matching `Pop*` command;
/// [`Scene::validate`] checks the nesting and the numeric invariants. Corner radii arrays are
/// ordered top-left, top-right, bottom-right, bottom-left.
#[derive(Clone)]
pub enum DrawCommand {
    /// Clip subsequent commands to a rounded rectangle until [`DrawCommand::PopShapeClip`].
    PushShapeClip {
        /// Clip rectangle.
        rect: RectF,
        /// Corner radii (non-negative).
        radii: [f32; 4],
    },
    /// Close the scope opened by [`DrawCommand::PushShapeClip`].
    PopShapeClip,
    /// Render subsequent commands offscreen and apply CSS-like filters until
    /// [`DrawCommand::PopFilter`]. At most 32 filter scopes per scene and 8 nested.
    PushFilter {
        /// Element owning the scope; must be unique within the scene.
        id: crate::ui::element::ElementId,
        /// Area affected by the filters.
        bounds: RectF,
        /// Filters to apply, in order.
        filters: crate::ui::rendering::filter::Filters,
    },
    /// Close the scope opened by [`DrawCommand::PushFilter`].
    PopFilter,
    /// Rounded rectangle with per-corner radii, either filled or stroked (not both).
    Shape {
        /// Shape bounds.
        rect: RectF,
        /// Corner radii (non-negative).
        radii: [f32; 4],
        /// Fill background; must be `None` when `stroke` is set.
        fill: Option<crate::ui::rendering::brush::Background>,
        /// Stroke width and colour; must be `None` when `fill` is set.
        stroke: Option<(f32, Color)>,
    },
    /// Rounded rectangle filled with a radial gradient.
    RadialGradient {
        /// Shape bounds.
        rect: RectF,
        /// Corner radius (non-negative).
        radius: f32,
        /// Gradient definition.
        gradient: Arc<crate::ui::rendering::brush::RadialGradient>,
    },
    /// Text laid out with a full [`TextFormat`](crate::ui::styling::text::TextFormat).
    FormattedText {
        /// Layout box.
        rect: RectF,
        /// Text to draw.
        text: Arc<str>,
        /// Font, size, alignment and decoration.
        format: crate::ui::styling::text::TextFormat,
        /// Text colour.
        color: Color,
        /// Wrap lines to the box width.
        wrap: bool,
    },
    /// Paragraph whose ranges carry their own style (weight, italics, decoration,
    /// colour, size, family), laid out as one block.
    RichText {
        /// Layout box.
        rect: RectF,
        /// Text, base format and styled ranges.
        paragraph: Arc<crate::ui::styling::text::RichParagraph>,
        /// Colour of ranges without their own colour.
        color: Color,
        /// Wrap lines to the box width.
        wrap: bool,
    },
    /// Rounded rectangle filled with a linear gradient.
    Gradient {
        /// Shape bounds.
        rect: RectF,
        /// Corner radius (non-negative).
        radius: f32,
        /// Gradient definition.
        gradient: Arc<crate::ui::rendering::brush::LinearGradient>,
    },
    /// Apply a transform and group opacity to subsequent commands until
    /// [`DrawCommand::PopEffect`].
    PushEffect {
        /// Transform applied to the scope's content.
        transform: crate::ui::rendering::transform::Affine,
        /// Group opacity in `0.0..=1.0`.
        opacity: f32,
        /// Untransformed bounds of the affected content.
        bounds: RectF,
    },
    /// Close the scope opened by [`DrawCommand::PushEffect`].
    PopEffect,
    /// Vector icon scaled to `rect`.
    Vector {
        /// Destination rectangle.
        rect: RectF,
        /// Icon geometry.
        icon: Arc<crate::ui::vector::VectorIcon>,
        /// Fill (and outline) colour.
        color: Color,
        /// Extra outline width in icon units; `0.0` draws no outline.
        outline: f32,
    },
    /// Compiled SVG drawing placed in `rect` (viewBox scaled per `preserveAspectRatio`):
    /// filled and stroked shapes, open or closed contours, caps, joins and fill rules.
    Svg {
        /// Destination rectangle.
        rect: RectF,
        /// Drawing, validated by [`Scene::validate`].
        image: &'static crate::ui::rendering::svg::SvgImage,
        /// Value of `currentColor`.
        color: Color,
    },
    /// Text drawn with an explicit [`Font`](crate::ui::font::Font).
    FontText {
        /// Layout box.
        rect: RectF,
        /// Text to draw.
        text: Arc<str>,
        /// Font size in logical pixels.
        size: f32,
        /// Font family and weight.
        font: crate::ui::font::Font,
        /// Text colour.
        color: Color,
        /// Wrap lines to the box width.
        wrap: bool,
    },
    /// Rounded rectangle filled with a solid colour.
    Fill {
        /// Shape bounds.
        rect: RectF,
        /// Corner radius.
        radius: f32,
        /// Fill colour.
        color: Color,
    },
    /// Outline of a rounded rectangle.
    Stroke {
        /// Shape bounds.
        rect: RectF,
        /// Corner radius.
        radius: f32,
        /// Stroke width.
        width: f32,
        /// Stroke colour.
        color: Color,
    },
    /// Straight line segment.
    Line {
        /// Start point.
        start: PointF,
        /// End point.
        end: PointF,
        /// Line width.
        width: f32,
        /// Line colour.
        color: Color,
    },
    /// Polyline through `points`; ignored with fewer than two points.
    Path {
        /// Vertices in drawing order.
        points: Vec<PointF>,
        /// `true` fills the closed polygon; `false` strokes an open 2 px polyline.
        closed: bool,
        /// Fill or stroke colour.
        color: Color,
    },
    /// Text drawn with the default system font.
    Text {
        /// Layout box.
        rect: RectF,
        /// Text to draw.
        text: Arc<str>,
        /// Font size in logical pixels.
        size: f32,
        /// Use the bold weight.
        bold: bool,
        /// Text colour.
        color: Color,
        /// Wrap lines to the box width.
        wrap: bool,
    },
    /// Bitmap stretched to `rect`.
    Image {
        /// Destination rectangle.
        rect: RectF,
        /// Premultiplied BGRA pixels.
        image: Arc<Image>,
    },
    /// Multiply the alpha of subsequent commands by the alpha of a linear gradient laid
    /// over `bounds`, until [`DrawCommand::PopMask`] (CSS `mask-image`).
    PushMask {
        /// Area covered by the gradient; content outside keeps the end colours' alpha.
        bounds: RectF,
        /// Mask gradient; only its alpha matters.
        gradient: Arc<crate::ui::rendering::brush::LinearGradient>,
    },
    /// Close the scope opened by [`DrawCommand::PushMask`].
    PopMask,
    /// Paint the text of subsequent commands with `fill` laid over `rect`, until
    /// [`DrawCommand::PopTextFill`] (CSS `background-clip: text`).
    PushTextFill {
        /// Area covered by the fill (the element's border box).
        rect: RectF,
        /// Colour or gradient used for glyphs.
        fill: crate::ui::rendering::brush::Background,
    },
    /// Close the scope opened by [`DrawCommand::PushTextFill`].
    PopTextFill,
    /// Filter what is already painted under a rounded rectangle and paint the result back
    /// inside it (CSS `backdrop-filter`). Ignored inside a filter scope.
    Backdrop {
        /// Area whose backdrop is filtered.
        rect: RectF,
        /// Corner radii (non-negative).
        radii: [f32; 4],
        /// Filters applied to the backdrop.
        filters: crate::ui::rendering::filter::Filters,
    },
    /// Clip subsequent commands to an axis-aligned rectangle until [`DrawCommand::PopClip`].
    PushClip(RectF),
    /// Close the scope opened by [`DrawCommand::PushClip`].
    PopClip,
}
