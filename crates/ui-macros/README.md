# karbon-ui-macros

Procedural macros of [Karbon UI](https://github.com/larevuegeek/karbon-ui), the native
desktop UI library for Rust: the compiler that turns HTML templates (`.ui` files, `ui!`)
and stylesheets (`css_file!`, `styles!`, Tailwind utilities, Bootstrap) into Rust code at
build time.

**Do not depend on this crate directly.** Use
[`karbon-ui`](https://crates.io/crates/karbon-ui), which re-exports the macros under
their public names (`ui!`, `ui_file!`, `styles!`, `css_file!`) and provides the runtime
the generated code calls. The entry points of this crate are an implementation detail and
may change in any release; both crates are published together, at the same version.

```rust
use karbon_ui::ui::prelude::*;

let title = "Hello";
let page: Element = ui! {
    <section style="padding: 24px; gap: 16px">
        <h1>{title}</h1>
        <p>A <b>native</b> page, no browser.</p>
    </section>
};
```

The compiler has no dependency: it reads `proc_macro` tokens and source text by hand
(no `syn`, no `quote`). Its design is described, in French, in
[docs/compiler.md](https://github.com/larevuegeek/karbon-ui/blob/main/docs/compiler.md).

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
