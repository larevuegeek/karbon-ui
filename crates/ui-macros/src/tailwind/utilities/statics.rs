//! Utilitaires sans valeur (`flex`, `items-center`, `truncate`…) : table des
//! déclarations de chacun, lue d'un seul tenant.
use super::{
    Out,
    composed::{FILTER, SHADOW, TRANSITION_COLORS, TRANSITION_DEFAULT, transition},
    decls, out,
};
/// Utilitaires sans valeur (`flex`, `items-center`, `truncate`…).
pub(super) fn statics(name: &str) -> Option<Out> {
    let d = |list: &[(&str, &str)]| Some(out(list));
    match name {
        "block" => d(&[("display", "block")]),
        "flex" => d(&[("display", "flex")]),
        "inline-flex" => d(&[("display", "inline-flex")]),
        "grid" => d(&[("display", "grid")]),
        "inline-grid" => d(&[("display", "inline-grid")]),
        "hidden" => d(&[("display", "none")]),
        "inline-block" => d(&[("display", "inline-block")]),
        "inline" => d(&[("display", "inline")]),
        "contents" => d(&[("display", "contents")]),
        "flow-root" => d(&[("display", "flow-root")]),
        "static" | "fixed" | "absolute" | "relative" | "sticky" => d(&[("position", name)]),
        "visible" | "invisible" | "collapse" => d(&[(
            "visibility",
            if name == "invisible" { "hidden" } else { name },
        )]),
        "isolate" | "isolation-auto" => d(&[]),
        "flex-row" => d(&[("flex-direction", "row")]),
        "flex-row-reverse" => d(&[("flex-direction", "row-reverse")]),
        "flex-col" => d(&[("flex-direction", "column")]),
        "flex-col-reverse" => d(&[("flex-direction", "column-reverse")]),
        "flex-wrap" => d(&[("flex-wrap", "wrap")]),
        "flex-nowrap" => d(&[("flex-wrap", "nowrap")]),
        "flex-wrap-reverse" => d(&[("flex-wrap", "wrap-reverse")]),
        "flex-auto" => d(&[("flex", "auto")]),
        "flex-initial" => d(&[("flex", "0 auto")]),
        "flex-none" => d(&[("flex", "none")]),
        "grow" => d(&[("flex-grow", "1")]),
        "shrink" => d(&[("flex-shrink", "1")]),
        "order-first" => d(&[("order", "-9999")]),
        "order-last" => d(&[("order", "9999")]),
        "order-none" => d(&[("order", "0")]),
        "items-start" => d(&[("align-items", "flex-start")]),
        "items-end" => d(&[("align-items", "flex-end")]),
        "items-center" => d(&[("align-items", "center")]),
        "items-baseline" => d(&[("align-items", "baseline")]),
        "items-stretch" => d(&[("align-items", "stretch")]),
        "self-auto" => d(&[("align-self", "auto")]),
        "self-start" => d(&[("align-self", "flex-start")]),
        "self-end" => d(&[("align-self", "flex-end")]),
        "self-center" => d(&[("align-self", "center")]),
        "self-stretch" => d(&[("align-self", "stretch")]),
        "self-baseline" => d(&[("align-self", "baseline")]),
        "justify-start" => d(&[("justify-content", "flex-start")]),
        "justify-end" => d(&[("justify-content", "flex-end")]),
        "justify-center" => d(&[("justify-content", "center")]),
        "justify-between" => d(&[("justify-content", "space-between")]),
        "justify-around" => d(&[("justify-content", "space-around")]),
        "justify-evenly" => d(&[("justify-content", "space-evenly")]),
        "justify-stretch" => d(&[("justify-content", "stretch")]),
        "justify-normal" => d(&[("justify-content", "normal")]),
        "justify-self-auto" => d(&[("justify-self", "auto")]),
        "justify-self-start" => d(&[("justify-self", "start")]),
        "justify-self-end" => d(&[("justify-self", "end")]),
        "justify-self-center" => d(&[("justify-self", "center")]),
        "justify-self-stretch" => d(&[("justify-self", "stretch")]),
        "justify-items-start"
        | "justify-items-end"
        | "justify-items-center"
        | "justify-items-stretch" => d(&[("justify-items", &name[14..])]),
        "place-items-start"
        | "place-items-end"
        | "place-items-center"
        | "place-items-stretch"
        | "place-items-baseline" => d(&[("place-items", &name[12..])]),
        "place-self-auto" | "place-self-start" | "place-self-end" | "place-self-center"
        | "place-self-stretch" => d(&[("align-self", &name[11..]), ("justify-self", &name[11..])]),
        "place-content-center"
        | "place-content-start"
        | "place-content-end"
        | "place-content-between"
        | "place-content-stretch" => d(&[("place-content", &name[14..])]),
        "content-center" | "content-start" | "content-end" | "content-between"
        | "content-around" | "content-evenly" | "content-stretch" | "content-normal" => {
            d(&[("align-content", &name[8..])])
        }
        "content-none" => d(&[("--tw-content", "none"), ("content", "none")]),
        "grid-flow-row" => d(&[]),
        "grid-flow-col" | "grid-flow-dense" | "grid-flow-row-dense" | "grid-flow-col-dense" => {
            d(&[(
                "grid-auto-flow",
                &name[10..].replace("col", "column").replace('-', " "),
            )])
        }
        "box-border" => d(&[("box-sizing", "border-box")]),
        "box-content" => d(&[("box-sizing", "content-box")]),
        "overflow-auto" | "overflow-hidden" | "overflow-clip" | "overflow-visible"
        | "overflow-scroll" => d(&[("overflow", &name[9..])]),
        "overflow-x-auto" | "overflow-x-hidden" | "overflow-x-clip" | "overflow-x-visible"
        | "overflow-x-scroll" => d(&[("overflow-x", &name[11..])]),
        "overflow-y-auto" | "overflow-y-hidden" | "overflow-y-clip" | "overflow-y-visible"
        | "overflow-y-scroll" => d(&[("overflow-y", &name[11..])]),
        "truncate" => d(&[
            ("overflow", "hidden"),
            ("text-overflow", "ellipsis"),
            ("white-space", "nowrap"),
        ]),
        "text-ellipsis" => d(&[("text-overflow", "ellipsis")]),
        "text-clip" => d(&[("text-overflow", "clip")]),
        "text-left" | "text-center" | "text-right" | "text-justify" | "text-start" | "text-end" => {
            d(&[("text-align", &name[5..])])
        }
        "text-wrap" => d(&[("text-wrap", "wrap")]),
        "text-nowrap" => d(&[("text-wrap", "nowrap")]),
        // Équilibrage de lignes : simple retour à la ligne pour le moteur.
        "text-balance" | "text-pretty" => d(&[("text-wrap", "wrap")]),
        "uppercase" | "lowercase" | "capitalize" => d(&[("text-transform", name)]),
        "normal-case" => d(&[("text-transform", "none")]),
        "italic" => d(&[("font-style", "italic")]),
        "not-italic" => d(&[("font-style", "normal")]),
        "underline" | "overline" | "line-through" => d(&[("text-decoration-line", name)]),
        "no-underline" => d(&[("text-decoration-line", "none")]),
        "overscroll-auto" => d(&[("overscroll-behavior", "auto")]),
        "overscroll-contain" => d(&[("overscroll-behavior", "contain")]),
        "overscroll-none" => d(&[("overscroll-behavior", "none")]),
        "overscroll-x-auto" => d(&[("overscroll-behavior-x", "auto")]),
        "overscroll-x-contain" => d(&[("overscroll-behavior-x", "contain")]),
        "overscroll-x-none" => d(&[("overscroll-behavior-x", "none")]),
        "overscroll-y-auto" => d(&[("overscroll-behavior-y", "auto")]),
        "overscroll-y-contain" => d(&[("overscroll-behavior-y", "contain")]),
        "overscroll-y-none" => d(&[("overscroll-behavior-y", "none")]),
        "whitespace-normal"
        | "whitespace-nowrap"
        | "whitespace-pre"
        | "whitespace-pre-wrap"
        | "whitespace-pre-line"
        | "whitespace-break-spaces" => d(&[("white-space", &name[11..])]),
        "break-words" | "wrap-break-word" => d(&[("overflow-wrap", "break-word")]),
        "wrap-anywhere" => d(&[("overflow-wrap", "anywhere")]),
        "break-all" => d(&[("word-break", "break-all")]),
        "break-normal" => d(&[("overflow-wrap", "normal"), ("word-break", "normal")]),
        "tabular-nums" | "proportional-nums" => d(&[
            ("--tw-numeric-spacing", name),
            (
                "font-variant-numeric",
                "var(--tw-ordinal,) var(--tw-slashed-zero,) var(--tw-numeric-figure,) var(--tw-numeric-spacing,) var(--tw-numeric-fraction,)",
            ),
        ]),
        "normal-nums" => d(&[("font-variant-numeric", "normal")]),
        "lining-nums" | "oldstyle-nums" | "slashed-zero" | "ordinal" | "diagonal-fractions"
        | "stacked-fractions" => d(&[("font-variant-numeric", name)]),
        // Indications de rendu sans effet sur le moteur (DirectWrite lisse toujours).
        "antialiased"
        | "subpixel-antialiased"
        | "transform-gpu"
        | "transform-cpu"
        | "transform"
        | "appearance-none"
        | "appearance-auto"
        | "will-change-auto"
        | "will-change-scroll"
        | "will-change-contents"
        | "will-change-transform" => d(&[]),
        // Le texte statique n'est pas sélectionnable dans le moteur.
        "select-none" | "select-auto" => d(&[]),
        "select-text" | "select-all" => d(&[("user-select", &name[7..])]),
        "sr-only" => d(&[
            ("position", "absolute"),
            ("width", "1px"),
            ("height", "1px"),
            ("padding", "0"),
            ("margin", "-1px"),
            ("overflow", "hidden"),
            ("white-space", "nowrap"),
            ("border-width", "0"),
        ]),
        "not-sr-only" => d(&[
            ("position", "static"),
            ("width", "auto"),
            ("height", "auto"),
            ("padding", "0"),
            ("margin", "0"),
            ("overflow", "visible"),
            ("white-space", "normal"),
        ]),
        "object-contain" | "object-cover" | "object-fill" | "object-none" | "object-scale-down" => {
            d(&[("object-fit", &name[7..])])
        }
        "object-center" | "object-top" | "object-bottom" | "object-left" | "object-right" => {
            d(&[("object-position", &name[7..])])
        }
        "object-left-top"
        | "object-left-bottom"
        | "object-right-top"
        | "object-right-bottom"
        | "object-top-left"
        | "object-top-right"
        | "object-bottom-left"
        | "object-bottom-right" => d(&[("object-position", &name[7..].replace('-', " "))]),
        "aspect-auto" => d(&[("aspect-ratio", "auto")]),
        "aspect-square" => d(&[("aspect-ratio", "1 / 1")]),
        "outline-none" => d(&[("--tw-outline-style", "none"), ("outline-style", "none")]),
        "outline-hidden" => d(&[("outline", "none")]),
        "outline-solid" | "outline-dashed" | "outline-dotted" | "outline-double" => d(&[
            ("--tw-outline-style", &name[8..]),
            ("outline-style", &name[8..]),
        ]),
        "border-solid" | "border-dashed" | "border-dotted" | "border-double" | "border-hidden"
        | "border-none" => d(&[
            ("--tw-border-style", &name[7..]),
            ("border-style", &name[7..]),
        ]),
        "ring-inset" => d(&[("--tw-ring-inset", "inset")]),
        "bg-none" => d(&[("background-image", "none")]),
        "bg-clip-text" => d(&[("background-clip", "text")]),
        "bg-clip-border" | "bg-clip-padding" | "bg-clip-content" => {
            d(&[("background-clip", &format!("{}-box", &name[8..]))])
        }
        "transition" => Some(transition(TRANSITION_DEFAULT)),
        "transition-all" => Some(transition("all")),
        "transition-colors" => Some(transition(TRANSITION_COLORS)),
        "transition-opacity" => Some(transition("opacity")),
        "transition-shadow" => Some(transition("box-shadow")),
        "transition-transform" => Some(transition("transform, translate, scale, rotate")),
        "transition-none" => d(&[("transition-property", "none")]),
        "ease-linear" => d(&[
            ("--tw-ease", "linear"),
            ("transition-timing-function", "linear"),
        ]),
        "ease-initial" => d(&[(
            "transition-timing-function",
            "var(--default-transition-timing-function)",
        )]),
        "filter-none" => d(&[("filter", "none")]),
        "backdrop-filter-none" => d(&[("backdrop-filter", "none")]),
        "grayscale" | "invert" | "sepia" => Some(decls(vec![
            (format!("--tw-{name}"), format!("{name}(100%)")),
            ("filter".to_owned(), FILTER.into()),
        ])),
        "shadow-none" | "inset-shadow-none" => {
            let var = if name == "shadow-none" {
                "--tw-shadow"
            } else {
                "--tw-inset-shadow"
            };
            d(&[(var, "0 0 #0000"), ("box-shadow", SHADOW)])
        }
        "translate-none" => d(&[("translate", "none")]),
        "scale-none" => d(&[("scale", "none")]),
        "rotate-none" => d(&[("rotate", "none")]),
        "@container" => d(&[("container-type", "inline-size")]),
        "@container-normal" => d(&[("container-type", "normal")]),
        "@container-size" => d(&[("container-type", "size")]),
        "pointer-events-none" | "pointer-events-auto" => d(&[("pointer-events", &name[15..])]),
        "resize" => d(&[("resize", "both")]),
        "resize-none" => d(&[("resize", "none")]),
        "resize-x" => d(&[("resize", "horizontal")]),
        "resize-y" => d(&[("resize", "vertical")]),
        "list-none" | "list-disc" | "list-decimal" | "list-inside" | "list-outside" => {
            d(&[("list-style-type", &name[5..])])
        }
        "underline-offset-auto" => d(&[("text-underline-offset", "auto")]),
        _ => {
            if let Some(cursor) = name.strip_prefix("cursor-")
                && !cursor.starts_with('[')
                && !cursor.starts_with('(')
            {
                return d(&[("cursor", cursor)]);
            }
            None
        }
    }
}
