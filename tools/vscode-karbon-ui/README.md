# Karbon UI for Visual Studio Code

Syntax highlighting, indentation and snippets for the `.ui` templates of
[karbon-ui](https://github.com/larevuegeek/karbon-ui), the native desktop UI library
for Rust: templates written like HTML, compiled to native widgets, with Rust
expressions between braces.

```html
<ul data-stagger="40ms">
  {#each self.tracks as track (track.id)}
    <li class:active={track.id == self.current} onclick={Msg::Play(track.id)}>{track.title}</li>
  {:else}
    <p class="muted">No track yet &mdash; add a folder.</p>
  {/each}
</ul>
```

## Highlighting

- HTML tags, engine tags (`k-split`, `k-popover`, `k-virtual-list`), Rust components
  (`<Card>`, `<widgets::Card />`) and `<slot />`, each with its own scope;
- attributes: quoted, unquoted and boolean values, Rust values `attr={expr}`, events
  (`onclick`, `oninput`, `onpan`…), directives `class:name={cond}`, `style:prop={expr}`,
  `style:--var={expr}`, `data-*` and `aria-*`;
- CSS declarations inside `style="…"`, entities and `<!-- … -->` comments;
- control blocks: `{#if}`, `{#if let}`, `{:else if}`, `{:else}`, `{/if}`,
  `{#each … as … (key)}`, `{/each}`, `{@const}`, `{@include "file.ui"}`, `{@element}`;
- Rust between braces, in text and in attributes, through VS Code's Rust grammar:
  nested braces, strings, comments and closures (`{|v| Msg::Set(3, v)}`), over several
  lines if needed.

## Editing

- block comments (`Shift+Alt+A`);
- auto-closing of brackets, quotes and `<!--`;
- indentation after opening tags and `{#if}` / `{#each}` / `{:else}`, outdent on closing
  ones;
- folding regions `<!-- #region -->` / `<!-- #endregion -->`.

## Snippets

`div`, `flex`, `button`, `input`, `checkbox`, `switch`, `select`, `img`, `dialog`, `if`,
`ifelse`, `each`, `eachi`, `include`, `slot`, `component`, `k-popover`, `k-split`,
`k-virtual-list` and more; each also exists with the `ui-` prefix.

## `.ui` files from Qt

Qt Designer also uses the `.ui` extension (XML files). In a Qt project, map them back to
XML in `.vscode/settings.json`:

```json
"files.associations": { "*.ui": "xml" }
```

## Scope

The extension is declarative: no code runs. Template errors (unsupported tag or event,
missing `{#each}` key…) are reported by the compiler (`cargo check`). Completion,
go-to-definition and formatting are not provided yet.

Template syntax reference:
[docs/html-syntax.md](https://github.com/larevuegeek/karbon-ui/blob/main/docs/html-syntax.md).

## License

MIT or Apache-2.0, at your option.
