# Karbon UI

Native desktop UI library for Rust. Views are written like the web — HTML templates and
a CSS subset — but everything is **compiled to Rust** and drawn by the operating system's
own APIs. No webview, no JavaScript, no runtime parser.

| | Windows | macOS | Linux |
|---|---|---|---|
| Windows & input | Win32, DirectComposition | AppKit | Wayland, or X11 (XCB) |
| Text | DirectWrite | Core Text | Pango / HarfBuzz |
| Rendering | Direct2D (GPU or WARP) | Metal, Core Graphics fallback | Vulkan, Cairo fallback |
| Accessibility | UI Automation | NSAccessibility | AT-SPI |
| Input methods | IME | NSTextInputClient | text-input-v3, IBus |

Dependencies: only the vendors' official bindings (`windows`/`windows-sys`, the `objc2`
family, `libc`). On Linux the desktop libraries are bound by hand-written `extern "C"`.

> Karbon UI is pre-alpha (0.1). It is the native UI layer meant to replace Tauri in
> RustMusic (a desktop music player), and a sibling of the
> [Karbon](https://github.com/larevuegeek/karbon) web framework and
> [KarbonJS](https://github.com/larevuegeek/karbonjs). The API will change.

## A minimal application

```rust
use karbon_ui::{platform, ui::prelude::*, window::WindowOptions};

#[derive(Clone)]
enum Msg {
    NameChanged(String),
    Save,
}

#[derive(Default)]
struct App {
    name: String,
    saved: bool,
}
impl Application for App {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        column("page", [
            Text::new("My application").size(24.).bold().element("title"),
            input("name", self.name.as_str(), Msg::NameChanged),
            button("save", "Save", Msg::Save),
            text("status", if self.saved { "Saved" } else { "" }),
        ])
        .padding(24)
        .gap(12.)
    }
    fn update(&mut self, message: Msg) -> bool {
        match message {
            Msg::NameChanged(name) => self.name = name,
            Msg::Save => self.saved = true,
        }
        true // rebuild the view
    }
}

fn main() -> karbon_ui::Result<()> {
    platform::run(WindowOptions::DEFAULT, App::default())
}
```

The same view as a compiled template (`.ui` file or `ui!` macro): the markup is
translated to native calls at compile time, Rust stays between braces.

```html
<main class="page">
  <h1>My application</h1>
  <input type="text" value={self.name.as_str()} oninput={Msg::NameChanged}>
  <button onclick={Msg::Save}>Save</button>
  {#if self.saved}<p>Saved, <b>{self.name}</b>!</p>{/if}
</main>
```

Each window has its own `Message` type. Simple actions (`button`, menu items, `on_dismiss`…)
receive a message cloned on every activation; value-carrying widgets receive a function, so
a tuple variant is enough (`Msg::NameChanged`). Fallible APIs return `karbon_ui::Result<T>`
with a typed `karbon_ui::Error` that converts to and from `std::io::Error`.

## Ready-made components

`ui::kit` provides styled components written like HTML, grouped by type: layout,
typography, buttons, forms, data, feedback, navigation, pagination, carousel, animation
and overlays. They follow the light or dark colour
scheme and are customised with a theme, attributes or your own classes.

```rust,ignore
use karbon_ui::ui::kit::prelude::*;

ui! {
    <Stack gap="lg" padding="xl">
        <Input label="Name" value={&self.name} oninput={Msg::Name} />
        <Button primary icon="plus" onclick={Msg::Add}>Add</Button>
    </Stack>
}
```

`cargo run --release --example kit_gallery` shows them all; see
[docs/kit.md](docs/kit.md) (in French).

## How it works

1. `Application::view()` builds a tree of `Element`s (Rust constructors or templates).
2. The runtime resolves the CSS cascade (inheritance, `:hover`/`:focus` states, media
   queries, variables), then lays the tree out — only when geometry changed.
3. Widgets paint into a `Scene`, an OS-independent list of draw commands.
4. The platform backend turns the scene into pixels and presents the surface.
5. Native input comes back as `UiEvent`s; the runtime finds the handler in the current
   tree, builds a message and calls `update`. Returning `true` rebuilds the view.

Replacing the tree replaces its handlers: no global registry, no stale identifiers.
The engine keeps three memoisations (layout measures within a pass, text measures across
frames, pointer facts for the cascade) that are verified not to change the result.
[docs/architecture.md](docs/architecture.md) is the map of the code; the Linux backend's
design is detailed there too.

## Examples

```sh
cargo run --release --example rustmusic_html   # RustMusic home, translated from Svelte + Tailwind
cargo run --release --example bootstrap_demo   # Bootstrap 5.3 compiled as a framework stylesheet
cargo run --example simple_ui                  # the Rust API
cargo run --example multi_window               # several windows
cargo run --example background_task            # a background task and its wake-up
```

`rustmusic_html`, `bootstrap_demo` and `simple_ui` accept window options after `--`
(`--renderer=gpu|software`, `--transparent`, `--profile`…); the first two also export an
image without opening a window (`--export x.png --size 1280x900`).

## Building

Rust 1.88 or later. Windows and macOS need nothing else. Linux links against the desktop
libraries at build time (Debian/Ubuntu):

```sh
sudo apt install libcairo2-dev libpango1.0-dev libgdk-pixbuf-2.0-dev libfontconfig-dev \
                 libfreetype-dev libwayland-dev libxkbcommon-dev libdbus-1-dev
```

XCB (X11), Vulkan and the cursor and XInput libraries are loaded at run time with
`dlopen`: a system without X11 or without a Vulkan driver still starts.

## Checking

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo test --doc --workspace
cargo doc --no-deps --workspace
```

The engine's contract (about 300 unit tests, including the window-less `HeadlessApp`
harness) runs on every target; each backend adds its own minimal suite, and the template
and CSS crates in `tests/` add about 150 integration tests. Two reference exports
(`rustmusic_html` at 1280×900 and 960×700, software renderer) must stay byte-identical
across refactors. From Windows, `cargo clippy --target x86_64-unknown-linux-gnu` and
`--target x86_64-apple-darwin` check the other backends.

## Documentation

The documentation is written in French, in [docs/](docs/): templates
([templates.md](docs/templates.md), [html-syntax.md](docs/html-syntax.md)), CSS coverage
([css.md](docs/css.md), [tailwind.md](docs/tailwind.md),
[css-frameworks.md](docs/css-frameworks.md)), components, windows, tasks, images,
drag-and-drop, accessibility, headless testing ([headless.md](docs/headless.md)), the test
organisation ([tests.md](docs/tests.md)) and the resume point ([next-session.md](docs/next-session.md)).
[tools/vscode-karbon-ui](tools/vscode-karbon-ui/README.md) adds `.ui` syntax highlighting,
indentation and snippets to VS Code.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option. Unless you explicitly state otherwise, any
contribution intentionally submitted for inclusion in the work by you, as defined in the
Apache-2.0 license, shall be dual licensed as above, without any additional terms or
conditions.
