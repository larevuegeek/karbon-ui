//! Exercise the compiler as a downstream crate, including dependency renaming and
//! Cargo's tracking of nested .ui files. No test framework dependency is required.
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn write(root: &Path, path: &str, contents: &str) {
    // Les dates des fichiers suivent une horloge grossière (quelques ms) : sans attente, une
    // réécriture juste après un `cargo check` peut porter la date de ses fichiers de
    // dépendances, et cargo la croit déjà compilée (échec observé sous forte charge).
    std::thread::sleep(std::time::Duration::from_millis(50));
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

fn check(root: &Path) -> Output {
    Command::new(env!("CARGO"))
        .args([
            "check",
            "--offline",
            "--bins",
            "--keep-going",
            "--quiet",
            "--manifest-path",
        ])
        .arg(root.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/template-downstream"))
        .output()
        .unwrap()
}

#[test]
fn downstream_hygiene_incremental_includes_and_diagnostics() {
    let package = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = package.join(format!("target/template-fixture-{}", std::process::id()));
    write(
        &root,
        "Cargo.toml",
        &format!(
            "[package]\nname=\"template-fixture\"\nversion=\"0.0.0\"\nedition=\"2024\"\n[workspace]\n[dependencies]\nrenamed={{package=\"karbon-ui\",path={:?}}}\n",
            package.to_string_lossy()
        ),
    );
    write(
        &root,
        "src/main.rs",
        r#"
use renamed::ui::prelude::*;
styles! { CARD {padding:4;align:{Align::Center};} }
css_file!(pub mod css,"views/main.css");
fn main() {
    let title="Caller scope";
    let width=42;
    let cx=TemplateContext::default();
    let _=ui_file!("views/page.ui",context=cx);
    let _=ui!{@styles(css);<div class="card"><span class="title">CSS</span></div>};
}
"#,
    );
    write(
        &root,
        "src/views/page.ui",
        r#"<div class={CARD}>{@include "nested/part.ui"}</div>"#,
    );
    let fragment = r#"<p style:width={width}>Hello from {title}, l'include &amp; co.</p>"#;
    write(
        &root,
        "src/views/main.css",
        "@import \"palette.css\";\n.card {padding:4px; background-color:var(--bg);}\n.title{font-size:18px;color:#fff;}",
    );
    let palette = ":root {--bg:#102030;}";
    write(&root, "src/views/palette.css", palette);
    write(&root, "src/views/nested/part.ui", fragment);
    let good = check(&root);
    assert!(
        good.status.success(),
        "{}",
        String::from_utf8_lossy(&good.stderr)
    );
    // Only this nested include changes; the Rust source stays untouched.
    write(
        &root,
        "src/views/nested/part.ui",
        "<UnknownIncrementalComponent />",
    );
    let changed = check(&root);
    let error = String::from_utf8_lossy(&changed.stderr);
    assert!(
        !changed.status.success() && error.contains("UnknownIncrementalComponent"),
        "{error}"
    );
    write(&root, "src/views/nested/part.ui", fragment);

    // Only the CSS @import changes. Cargo must also rebuild the parent stylesheet.
    write(
        &root,
        "src/views/palette.css",
        ":root {--bg:#102030;} .bad { mix-blend-mode:screen; }",
    );
    let changed = check(&root);
    let error = String::from_utf8_lossy(&changed.stderr);
    assert!(
        !changed.status.success()
            && error.contains("CSS property 'mix-blend-mode'")
            && error.contains("palette.css:1:"),
        "{error}"
    );
    write(&root, "src/views/palette.css", palette);
    let cases = [
        (
            "attribute",
            r#"<div mystery="1"></div>"#,
            "Unknown attribute 'mystery' on <div>",
        ),
        ("closing", r#"<div></section>"#, "Mismatched closing tag"),
        (
            "duplicate",
            r#"<div id="a" id="b"></div>"#,
            "Duplicate attribute",
        ),
        (
            "key",
            r#"<div>{#each [1,2] as n}<span>{n}</span>{/each}</div>"#,
            "stable key",
        ),
        (
            "binding",
            r#"<div>{#each [1] (1)}<span>a</span>{/each}</div>"#,
            "Expected {#each items as item (key)}",
        ),
        (
            "unclosed_block",
            r#"<div>{#if true}<span>a</span></div>"#,
            "Unexpected closing tag </div>",
        ),
        (
            "select_children",
            r#"<select onchange={|_| ()}><div></div></select>"#,
            "<select> can only contain <option>",
        ),
        (
            "wrong_input",
            r#"<input type="range" value="1" oninput={|_| ()} placeholder="ignored">"#,
            "Unknown attribute 'placeholder' on <input>",
        ),
        (
            "checkbox_event",
            r#"<input type="checkbox" oninput={|_| ()}>"#,
            "Event 'oninput' does not apply to <input type=\"checkbox\">; valid events: onchange",
        ),
        (
            "radio_name",
            r#"<div><input type="radio" onchange={()}> A</div>"#,
            "needs name=",
        ),
        (
            "wrong_style",
            r#"<span class={LAYOUT}>Invalid container style</span>"#,
            "ContainerStyle",
        ),
        (
            "unsupported",
            r#"<table></table>"#,
            "<table> is not supported",
        ),
        (
            "textarea_content",
            r#"<textarea oninput={|_: String| ()}>Texte</textarea>"#,
            "takes its text from value=",
        ),
        (
            "textarea_event",
            r#"<textarea oninput={|_: String| ()} onchange={()}></textarea>"#,
            "Event 'onchange' does not apply to <textarea>",
        ),
        ("typo", r#"<dvi></dvi>"#, "Did you mean <div>?"),
        ("entity", r#"<p>&bogus;</p>"#, "Unknown HTML entity &bogus;"),
        (
            "exit",
            r#"<div data-exit="fade-ot"></div>"#,
            "did you mean 'fade-out'?",
        ),
        (
            "stagger",
            r#"<ul data-stagger="fast"></ul>"#,
            "Invalid data-stagger 'fast'",
        ),
        (
            "inline_block",
            r#"<p>Text <div></div></p>"#,
            "<div> cannot appear inside <p>",
        ),
        (
            "number",
            r#"<input type="range" min="abc" oninput={|_| ()}>"#,
            "min=\"abc\" is not a number",
        ),
        ("href", r#"<p><a href="x">y</a></p>"#, "Links emit messages"),
        (
            "cycle",
            r#"<div>{@include "cycle.ui"}</div>"#,
            "Circular template include",
        ),
        (
            "missing",
            r#"<div>{@include "missing.ui"}</div>"#,
            "missing.ui",
        ),
        (
            "svg_text",
            r#"<svg viewBox="0 0 24 24"><text>x</text></svg>"#,
            "<text> is not supported inside inline SVG",
        ),
        (
            "svg_path",
            r#"<svg viewBox="0 0 24 24"><path d="M0 0 X"/></svg>"#,
            "Unknown path command 'X'",
        ),
        (
            "svg_size",
            r#"<svg><path d="M0 0"/></svg>"#,
            "<svg> needs a viewBox",
        ),
        (
            "svg_gradient",
            r#"<svg viewBox="0 0 24 24"><path fill="url(#g)" d="M0 0"/></svg>"#,
            "SVG gradients and patterns are not supported",
        ),
        (
            "svg_f32_range",
            r#"<svg viewBox="0 0 24 24"><path d="M0 0 L1e39 0"/></svg>"#,
            "SVG number 1e39 is out of range",
        ),
        (
            "svg_dash",
            r#"<svg viewBox="0 0 24 24"><path stroke-dasharray="2" d="M0 0"/></svg>"#,
            "Dashed strokes are not supported",
        ),
        (
            "role_unknown",
            r#"<div role="grid"></div>"#,
            "role=\"grid\" is not supported",
        ),
        (
            "aria_checked_role",
            r#"<button aria-checked="true" onclick={()}>Choix</button>"#,
            "aria-checked needs role=",
        ),
        (
            "aria_current_token",
            r#"<button aria-current="yes" onclick={()}>Accueil</button>"#,
            "aria-current expects page",
        ),
        (
            "haspopup",
            r#"<button aria-haspopup="menu" onclick={()}>Menu</button>"#,
            "aria-haspopup needs aria-expanded",
        ),
        (
            "placement",
            r#"<div><button id="a" onclick={()}>A</button><k-popover anchor="a" aria-label="M" onclose={()} placement="middle"></k-popover></div>"#,
            "placement=\"middle\"",
        ),
        (
            "combobox_menu",
            r#"<div><input id="a" type="text" value="" oninput={|_: String| ()}><k-popover anchor="a" aria-label="M" onclose={()} menu combobox></k-popover></div>"#,
            "either a menu, a listbox or a combobox",
        ),
        (
            "cross_offset",
            r#"<div><button id="a" onclick={()}>A</button><k-popover anchor="a" aria-label="M" onclose={()} cross-offset="400"></k-popover></div>"#,
            "cross-offset=\"400\" must be a shift",
        ),
        (
            "reserved_prop",
            r#"<Comp __ui_root="1" />"#,
            "'__ui_root' cannot be a component prop name",
        ),
        (
            "svg_expression",
            r#"<svg viewBox="0 0 24 24"><path d={path}/></svg>"#,
            "must be a literal value",
        ),
    ];
    write(
        &root,
        "src/bin/cycle.ui",
        r#"<p>x</p>{@include "cycle.ui"}"#,
    );
    for (name, markup, _) in cases {
        write(
            &root,
            &format!("src/bin/{name}.rs"),
            &format!(
                "use renamed::ui::prelude::*;\nstyles!{{TYPO{{font_size:14;}} LAYOUT{{columns:2;}}}}\nfn main(){{let _=ui!{{{markup}}};}}"
            ),
        );
    }
    let css_cases = [
        (
            "css_unknown",
            ".card { mix-blend-mode: screen; }",
            "CSS property 'mix-blend-mode'",
        ),
        (
            "css_selector",
            ".card Text {color:red;}",
            "Use class selectors on native template elements",
        ),
        (
            "css_variable",
            ":root{--a:var(--b);--b:var(--a);} .card {padding:var(--a);}",
            "Circular CSS variable",
        ),
        (
            "css_media",
            "@media (unknown-feature:600px){.card{gap:2px;}}",
            "Unsupported media feature",
        ),
        (
            "css_unit",
            ".card {padding:5%;}",
            "percentages are not supported for spacing",
        ),
        (
            "css_hover",
            ".card:hover {overflow:hidden;}",
            "State property 'overflow'",
        ),
        (
            "css_container_style",
            "@container style(--dark: 1) {.card{gap:2px;}}",
            "Style queries",
        ),
        (
            "css_container_type",
            ".card {container-type: block-size;}",
            "container-type: block-size is not supported",
        ),
        (
            "css_container_state",
            "@media (min-width: 10px) {.card {container-type: size;}}",
            "State property 'container_type'",
        ),
        (
            "css_webkit_thumb",
            ".card::-webkit-scrollbar-thumb {background: red;}",
            "use scrollbar-color for the thumb",
        ),
        (
            "css_webkit_color",
            ".card::-webkit-scrollbar {background: red;}",
            "supports width and height only",
        ),
        (
            "css_webkit_hover",
            ".card:hover::-webkit-scrollbar {width: 4px;}",
            "::-webkit-scrollbar in a state",
        ),
        (
            "css_webkit_axes",
            ".card::-webkit-scrollbar {width: 8px; height: 6px;}",
            "width and height must be equal",
        ),
        // Animations : erreurs à la position de la déclaration fautive.
        (
            "css_keyframes_property",
            "@keyframes k { 0% { opacity: 0 }\n 100% { padding: 10px } }\n.card { animation: k 1s; }",
            "css_keyframes_property.css:2:9: Keyframe property 'padding' is not animatable",
        ),
        (
            "css_animation_unknown",
            "@keyframes k { to { opacity: 0 } }\n.card {\n  animation: k 1s, missing 2s; }",
            "css_animation_unknown.css:3:3: Unknown @keyframes 'missing'",
        ),
        (
            "css_animation_iterations",
            "@keyframes k { to { opacity: 0 } }\n.card { animation: k 1s 1.5; }",
            "css_animation_iterations.css:2:9: Animation iterations must be a positive integer",
        ),
    ];
    for (name, css, _) in css_cases {
        write(&root, &format!("src/bin/{name}.css"), css);
        write(
            &root,
            &format!("src/bin/{name}.rs"),
            &format!("use renamed::ui::prelude::*;css_file!(mod css,\"{name}.css\");fn main(){{}}"),
        );
    }
    // Classes littérales : feuille seule, puis feuille qui importe Tailwind.
    write(&root, "src/bin/plain.css", ".card {padding:2px;}");
    write(
        &root,
        "src/bin/tw.css",
        "@import \"tailwindcss\";\n@theme { --color-brand: #123456; }\n.card {padding:2px;}",
    );
    let class_cases = [
        (
            "class_sheet",
            "plain",
            r#"<div class="crad"></div>"#,
            "did you mean 'card'?",
        ),
        (
            "class_hint",
            "plain",
            r#"<div class="items-center"></div>"#,
            "add @import \"tailwindcss\"",
        ),
        (
            "class_tw",
            "tw",
            r#"<div class="card itms-center"></div>"#,
            "did you mean 'items-center'?",
        ),
        (
            "class_color",
            "tw",
            r#"<div class="bg-brnd/50"></div>"#,
            "did you mean 'brand'?",
        ),
        (
            "class_variant",
            "tw",
            r#"<div class="hovr:flex"></div>"#,
            "did you mean 'hover:'?",
        ),
        (
            "class_gap",
            "tw",
            r#"<div class="has-checked:flex"></div>"#,
            ":has() is not implemented by the engine",
        ),
    ];
    for (name, sheet, markup, _) in class_cases {
        write(
            &root,
            &format!("src/bin/{name}.rs"),
            &format!(
                "use renamed::ui::prelude::*;css_file!(mod css,\"{sheet}.css\");fn main(){{let _:Element=ui!{{@styles(css);{markup}}};}}"
            ),
        );
    }
    let bad = check(&root);
    let errors = String::from_utf8_lossy(&bad.stderr);
    assert!(!bad.status.success());
    assert!(!errors.contains("proc macro panicked"), "{errors}");
    for (_, _, expected) in cases {
        assert!(errors.contains(expected), "Missing {expected:?}: {errors}");
    }
    for (_, _, expected) in css_cases {
        assert!(errors.contains(expected), "Missing {expected:?}: {errors}");
    }
    for (_, _, _, expected) in class_cases {
        assert!(errors.contains(expected), "Missing {expected:?}: {errors}");
    }
}
