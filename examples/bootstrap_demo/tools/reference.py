"""Build the plain HTML equivalent of page.ui, rendered by a browser as the reference.

    python -I examples/bootstrap_demo/tools/reference.py OUT.html [--theme dark] [--open dropdown,modal,collapse] [--switch]

The markup is the template's own: Rust attributes are removed (onclick={...}), Rust
values replaced by the demo's initial ones (checked={checked}, value={range},
data-bs-theme={theme}),
`switch` becomes role="switch". The open states of --open are written as Bootstrap's
JavaScript leaves them (show classes, data-bs-popper on the menu, modal backdrop), so
that no script runs in the reference page.
"""
import argparse
import re
from pathlib import Path

HERE = Path(__file__).resolve().parent
PAGE = HERE.parent / "page.ui"
CSS = HERE.parents[2] / "tests" / "fixtures" / "bootstrap" / "bootstrap.css"


def convert(text: str, theme: str, opened: set[str], tab: str | None, switch: bool = False) -> str:
    text = text.replace("data-bs-theme={theme}", f'data-bs-theme="{theme}"')
    # Barre de titre de la fenêtre native : absente des exports comme de la référence.
    text = re.sub(r'\s*\{#if chrome\}.*?\{/if\}', "", text, flags=re.S)
    text = re.sub(r'\s+on\w+=\{[^}]*\}', "", text)
    # Valeurs initiales de la démo (bootstrap_demo.rs : checked, switched, range).
    text = text.replace("checked={checked}", "checked")
    text = re.sub(r'\s*checked=\{switched\}', " checked" if switch else "", text)
    text = text.replace("value={range}", 'value="50"')
    text = re.sub(r'(type="checkbox") switch', r'\1 role="switch"', text)
    # Croix des boutons de fermeture : image de fond SVG dans le navigateur.
    text = text.replace('aria-label="Close">×</button>', 'aria-label="Close"></button>')
    if "collapse" in opened:
        text = text.replace('class="collapse mb-3" id="collapseExample"', 'class="collapse mb-3 show" id="collapseExample"')
    if "dropdown" in opened:
        text = text.replace('<ul class="dropdown-menu">', '<ul class="dropdown-menu show" data-bs-popper="static">')
    if tab:
        text = text.replace('class="nav-link active" data-bs-toggle="tab"', 'class="nav-link" data-bs-toggle="tab"')
        text = text.replace(f'class="nav-link" data-bs-toggle="tab" data-bs-target="#{tab}"', f'class="nav-link active" data-bs-toggle="tab" data-bs-target="#{tab}"')
        text = text.replace('class="tab-pane fade show active"', 'class="tab-pane fade"')
        text = text.replace(f'class="tab-pane fade" id="{tab}"', f'class="tab-pane fade show active" id="{tab}"')
    if "offcanvas" in opened:
        text = text.replace('class="offcanvas offcanvas-start"', 'class="offcanvas offcanvas-start show"')
        text = text.replace("</body>", '  <div class="offcanvas-backdrop fade show"></div>\n</body>')
    if "modal" in opened:
        text = text.replace('class="modal fade" id="exampleModal" tabindex="-1"', 'class="modal fade show" id="exampleModal" tabindex="-1" style="display:block"')
        text = text.replace("</body>", '  <div class="modal-backdrop fade show"></div>\n</body>')
    css = CSS.as_uri()
    return (
        '<!doctype html>\n<html lang="en">\n<head>\n<meta charset="utf-8">\n'
        f'<link rel="stylesheet" href="{css}">\n</head>\n{text}</html>\n'
    )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("out")
    parser.add_argument("--theme", default="light", choices=["light", "dark"])
    parser.add_argument("--open", default="")
    parser.add_argument("--tab")
    parser.add_argument("--switch", action="store_true", help="the switch is checked (demo --switch)")
    args = parser.parse_args()
    opened = {o for o in args.open.split(",") if o}
    html = convert(PAGE.read_text(encoding="utf-8"), args.theme, opened, args.tab, args.switch)
    Path(args.out).write_text(html, encoding="utf-8")


if __name__ == "__main__":
    main()
