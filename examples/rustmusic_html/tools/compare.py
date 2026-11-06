"""Compare a Karbon layout dump with a Tauri reference capture.

    cargo run --release --example rustmusic_html -- --dump k.json --export k.png --size 1280x900
    python -I examples/rustmusic_html/tools/compare.py REF.json k.json [--ref-png REF.png --png k.png]
           [--out DIR] [--crop x,y,w,h ...] [--all]

REF.json is a capture of the Tauri page (one object per visible element: tag, path, text,
x, y, w, h in logical px, font, ls, color, bg, radius, border...). k.json comes from
`--dump`. Elements are matched by text (case and spacing ignored, truncated texts by
prefix) and the chrome without text by the class pairs of CHROME below. The report
lists position, size, font, letter-spacing and radius deltas, worst first; `--crop`
writes side-by-side crops (reference left, Karbon right) of logical rectangles.
"""
import argparse
import json
import re
import statistics
import sys
from pathlib import Path

# Boîtes sans texte : classe Karbon -> fragment du chemin de la référence (classes Tailwind).
# Appariées dans l'ordre du document.
CHROME = [
    ("titlebar", "main.w-screen.h-screen.flex > div.relative.z-30.flex"),
    ("search-field", "div.group.relative.flex"),
    ("search-kbd", "kbd.hidden.lg:inline-flex"),
    ("theme-switch", "button.inline-flex.items-center.rounded-full.p-1"),
    # Composants convertis en Tailwind : classe propre à l'élément dans le dump.
    ("pr-2.5", "button.group.flex.items-center.gap-2.rounded-full"),
    ("w-7.5", "span.relative.grid.place-items-center"),
    ("nav-arrow", "div.hidden.sm:flex.items-center > button"),
    ("round-action", "button.flex.items-center.justify-center.rounded-full.h-8.w-8"),
    ("sidebar", "aside.sidebar"),
    ("source-switcher", "button.w-full.flex.items-center.gap-2.5"),
    ("sb-picker-icon", "span.relative.w-9.h-9"),
    ("open-button", "button.flex-1.h-9"),
    ("open-divider", "div.h-px.bg-linear-to-r"),
    ("sb-tile", "a.flex.flex-col.items-center"),
    ("library-selector", "button.flex-1.min-w-0.flex.items-center"),
    ("sb-list", "div.flex.flex-col.border-t"),
    ("sb-row-thumb", "span.shrink-0.overflow-hidden.rounded-md"),
    ("sb-pied-carte", "div.sb-pied-carte"),
    ("sb-footer-tab", "button.w-10.h-4"),
    ("sb-avatar", "span.sb-avatar"),
    ("hero-reprise", "section.hero-reprise"),
    ("hero-cover", "button.group.relative.w-36"),
    ("hero-bar", "div.flex-1.h-[5px]"),
    ("hero-play", "button.h-[52px].px-[26px]"),
    # favori (classe d'app.css) est rangé après les utilitaires dans le dump : apparié par son nom.
    ("favori", "button.favori.shrink-0.w-[52px]"),
    ("hero-album", "button.hidden.@3xl:flex.@7xl:hidden"),
    ("hero-album-label", "button.flex.@3xl:hidden.@7xl:flex"),
    ("ensuite-panneau", "div.ensuite-panneau"),
    ("ensuite-card", "button.group.grid.grid-cols-[44px_minmax(0,1fr)_auto]"),
    ("ensuite-thumb", "span.relative.w-11.h-11"),
    ("mix-cover", "div.relative.aspect-square.overflow-hidden"),
    ("lecteur", "div.lecteur.relative"),
    ("lecteur-pochette", "div.lecteur-pochette"),
    ("lecteur-centre", "div.justify-self-center.w-full.max-w-155"),
    ("lecteur-lecture", "button.lecteur-lecture"),
    ("gros", "button.w-11.h-11.shrink-0"),
    ("ib", "button.relative.w-9.5.h-9.5"),
    ("bg-(--lc-rail)", "div.absolute.inset-0.rounded-full.overflow-hidden"),
    ("volume", "div.group/vol.relative"),
    ("volume-rail", "div.relative.w-full.h-1.25"),
    ("volume-thumb", "div.absolute.top-1/2.w-3"),
    ("statut", "div.flex.items-center.gap-2.5.h-8"),
    ("pastille", "button.shrink-0.h-5.5.px-2"),
]
# Menus et panneaux (captures menu-*.json) : appariés seulement avec --only-new ; le
# troisième champ nomme les menus (--menu, noms de --open) où la boîte existe.
MENU_CHROME = [
    ("w-56", "div.absolute.top-full.right-0 > div.rounded-xl.border.overflow-hidden", "profile"),
    ("mx-3", "div.h-px.mx-3", "profile"),
    # absolute top-11 right-0 du Svelte : placement du k-popover, la boîte porte w-64.
    ("w-64", "div.absolute.top-11.right-0", "sleep"),
    ("veille-tete-icone", "div.flex.items-center.justify-center.w-8.h-8.rounded-lg", "sleep"),
    # Classe de la feuille (app.css), rangée après les utilitaires dans le dump.
    ("rg-field", "div.rg-field.mt-1.5.flex", "sleep"),
    ("source-menu", "div.fixed.z-[9999] > div.p-1.5.flex.flex-col", "source"),
    ("source-option", "button.flex.items-center.gap-3", "source"),
    ("source-sep", "div.my-1.h-px", "source"),
    # fixed z-[9999] du Svelte : placement du k-popover, la boîte porte w-66 (comme w-56).
    ("w-66", "div.fixed.z-[9999].w-66", "open display"),
    ("zone-placement", "button.flex-1.flex.flex-col", "open"),
    ("zone-ligne", "button.w-full.flex.items-center", "open display"),
    ("zone-pastille", "span.w-7.h-7.rounded-lg", "open display"),
    ("zone-interrupteur", "span.w-8.h-4.5.rounded-full", "open display"),
    ("zone-sep", "div.h-px.mx-2", "open display"),
    ("biblio-menu", "div.fixed.z-[9999] > div.rounded-xl.border.overflow-hidden", "library"),
    ("biblio-icone", "div.relative.w-7.h-7", "library"),
    ("biblio-gerer", "button.shrink-0.w-7.h-7", "library"),
    ("mix-menu", "div.absolute.top-full.mt-2", "mix"),
    ("mix-menu-item", "button.w-full.flex.items-center", "mix"),
    # absolute bottom-full right-2 du Svelte : placement du k-popover, la boîte porte w-100 / w-96.
    ("w-100", "div.absolute.bottom-full.right-2", "output"),
    ("w-96", "div.absolute.bottom-full.right-2", "network"),
    ("sortie-ligne", "div.relative.flex.items-stretch", "output"),
    ("reseau-ligne", "div.flex.items-center.gap-3", "network"),
    ("reseau-icone", "span.w-9.h-9.shrink-0", "network"),
    ("interrupteur", "button.relative.w-8.5.h-5", "output network"),
]


def norm(text):
    text = text.replace("‎", "").replace(" ", " ").replace("\xa0", " ")
    text = re.sub(r"\s+", " ", text).strip().casefold()
    return text.rstrip("…").rstrip(".").strip()


def px(value):
    m = re.search(r"(-?[\d.e+]+)px", value or "")
    return float(m.group(1)) if m else None


def font(e):
    m = re.match(r"(\d+) ([\d.]+)px/([\d.]+px|normal)", e.get("font") or "")
    return (int(m.group(1)), float(m.group(2))) if m else (None, None)


def radius(e):
    r = px(e.get("radius"))
    return None if r is None else min(r, min(e["w"], e["h"]) / 2)


def fragment_classes(frag):
    """Classes du dernier composé d'un fragment de chemin (`button.group.w-9.5` ->
    {group, w-9.5}) : un point suivi d'un chiffre appartient à la classe."""
    last = frag.split(" > ")[-1]
    parts = re.split(r"\.(?!\d)", last)
    return parts[1:]


def load(path):
    return json.load(open(path, encoding="utf-8"))


def match(ref, kar, only_new=False, menu=None):
    pairs = []
    used = set()
    # Textes : texte propre d'une feuille, ou texte des descendants d'un conteneur (un
    # <button> de la référence porte le texte de son contenu). Le plus proche en position
    # et en taille l'emporte.
    k_text = [
        (i, norm(e.get("text") or e.get("deep") or ""))
        for i, e in enumerate(kar)
        if (e.get("text") or e.get("deep") or "").strip()
    ]
    for r in ref:
        t = norm(r.get("text", ""))
        if len(t) < 1 or r["tag"] in ("svg", "path", "tspan", "text") or r["w"] < 2:
            continue
        cands = []
        if r.get("bg") or r.get("border"):
            # Bouton à fond ou bordure dont le texte propre précède celui d'un enfant
            # (« 15 » puis « min ») : la boîte Karbon dont le texte des descendants
            # commence par ce texte.
            cands = [
                i for i, e in enumerate(kar)
                if i not in used and e.get("deep") and not e.get("text")
                and norm(e["deep"]).startswith(t + " ")
                and abs(e["x"] - r["x"]) + abs(e["y"] - r["y"]) < 30
                and abs(e["w"] - r["w"]) + abs(e["h"] - r["h"]) < 30
            ]
        cands = cands or [
            i for i, kt in k_text
            if i not in used and (kt == t or (min(len(kt), len(t)) >= 6 and (kt.startswith(t) or t.startswith(kt))))
        ]
        if not cands:
            pairs.append((r, None, "text"))
            continue
        # À égalité (un <div> et le paragraphe anonyme de son texte ont la même boîte),
        # l'élément qui porte le texte lui-même.
        best = min(cands, key=lambda i: abs(kar[i]["x"] - r["x"]) + abs(kar[i]["y"] - r["y"])
                   + abs(kar[i]["w"] - r["w"]) + abs(kar[i]["h"] - r["h"])
                   + (0 if kar[i].get("text") else 1e-3))
        used.add(best)
        pairs.append((r, kar[best], "text"))
    # Boîtes : élément Karbon qui porte la classe sémantique de la table, ou (templates
    # écrits avec les utilitaires de la source) dont la liste de classes commence par
    # celles du dernier composé du fragment, comme le chemin de la référence ; le plus
    # proche en position l'emporte.
    if only_new:
        menus = [(c, f) for c, f, names in MENU_CHROME if menu is None or menu in names.split()]
    else:
        menus = []
    for cls, frag in CHROME + menus:
        rs = [r for r in ref if r["path"].endswith(frag) or (frag in r["path"].split(" > ")[-1])]
        rs = [r for r in rs if r["path"].split(" > ")[-1].startswith(frag.split(" > ")[-1])]
        wanted = fragment_classes(frag)
        ks = [
            k for k in kar
            if cls in k.get("cls", "").split() or (wanted and k.get("cls", "").split()[:len(wanted)] == wanted)
        ]
        if only_new and not ks:
            # Boîte d'un autre menu (même classes Tailwind) : hors sujet ici.
            continue
        for r in rs:
            if not ks:
                pairs.append((r, None, cls))
                continue
            k = min(ks, key=lambda k: abs(k["x"] - r["x"]) + abs(k["y"] - r["y"]))
            ks.remove(k)
            pairs.append((r, k, cls))
    return pairs


def report(pairs, all_rows, page_bottom=float("inf")):
    rows = []
    missing = []
    for r, k, kind in pairs:
        label = (r["text"] or kind)[:34]
        if k is None and r["y"] >= page_bottom:
            # Contenu de la page sous le lecteur : présent dans le DOM, invisible.
            continue
        if k is None:
            missing.append(f"  missing  {kind:16} {label!r} at {r['x']:.0f},{r['y']:.0f} {r['w']:.0f}x{r['h']:.0f}")
            continue
        d = {
            "x": k["x"] - r["x"], "y": k["y"] - r["y"],
            "w": k["w"] - r["w"], "h": k["h"] - r["h"],
        }
        notes = []
        rw, rs = font(r)
        kw, ks = font(k)
        if rs is not None and ks is not None:
            if abs(rs - ks) > 0.05:
                notes.append(f"size {ks:g}≠{rs:g}")
            if rw != kw:
                notes.append(f"weight {kw}≠{rw}")
            rl, kl = px(r.get("ls")) or 0.0, px(k.get("ls")) or 0.0
            if abs(rl - kl) > 0.05:
                notes.append(f"ls {kl:g}≠{rl:g}")
            rf = (r.get("font") or "").split('"')[1:2]
            kf = (k.get("font") or "").split('"')[1:2]
            if rf and kf and re.split(r"[ ,]", rf[0])[0] != re.split(r"[ ,]", kf[0])[0]:
                notes.append(f"family {kf[0]}≠{rf[0]}")
        rr, kr = radius(r), radius(k)
        if rr is not None or kr is not None:
            if abs((rr or 0) - (kr or 0)) > 0.5:
                notes.append(f"radius {kr or 0:g}≠{rr or 0:g}")
        # Pour un texte, la largeur dépend du conteneur (bloc ou ligne) : seule la
        # position et la hauteur comptent dans l'écart principal.
        # Un texte nu prend la largeur de son bloc ici et celle de sa ligne là-bas : sa
        # largeur ne compte pas ; celle d'une boîte (bouton, pastille) compte.
        loose = kind == "text" and k.get("text") and not r.get("bg") and not r.get("radius")
        worst = max(abs(d["x"]), abs(d["y"]), abs(d["h"]), 0 if loose else abs(d["w"]))
        rows.append((worst, label, kind, r, k, d, notes))
    rows.sort(key=lambda row: -row[0] - (5 if row[6] else 0))
    print(f"{'Δmax':>6} {'dx':>6} {'dy':>6} {'dw':>6} {'dh':>6}  element (ref x,y wxh)  notes")
    for worst, label, kind, r, k, d, notes in rows:
        if not all_rows and worst <= 1.5 and not notes:
            continue
        print(f"{worst:6.1f} {d['x']:6.1f} {d['y']:6.1f} {d['w']:6.1f} {d['h']:6.1f}  "
              f"{label!r} ({r['x']:.0f},{r['y']:.0f} {r['w']:.0f}x{r['h']:.0f})  {'; '.join(notes)}")
    for line in missing:
        print(line)
    worsts = [row[0] for row in rows]
    if worsts:
        within = sum(1 for w in worsts if w <= 2) / len(worsts)
        print(f"\nmatched {len(rows)}, missing {len(missing)}; within 2 px: {within:.0%}; "
              f"median {statistics.median(worsts):.1f}; max {max(worsts):.1f}; "
              f"with style notes: {sum(1 for row in rows if row[6])}")
    return rows


def crops(ref_png, k_png, rects, out, scale=1.5):
    from PIL import Image
    a, b = Image.open(ref_png).convert("RGB"), Image.open(k_png).convert("RGB")
    out.mkdir(parents=True, exist_ok=True)
    for x, y, w, h in rects:
        box = tuple(int(v * scale) for v in (x, y, x + w, y + h))
        ca, cb = a.crop(box), b.crop(box)
        # Côte à côte, ou l'une sous l'autre pour une bande large (référence d'abord).
        if ca.width > ca.height * 1.5:
            sheet = Image.new("RGB", (ca.width, ca.height * 2 + 6), "magenta")
            sheet.paste(cb, (0, ca.height + 6))
        else:
            sheet = Image.new("RGB", (ca.width * 2 + 6, ca.height), "magenta")
            sheet.paste(cb, (ca.width + 6, 0))
        sheet.paste(ca, (0, 0))
        name = out / f"crop-{x}-{y}-{w}x{h}.png"
        sheet.save(name)
        print(f"crop: {name}")


def main():
    sys.stdout.reconfigure(encoding="utf-8")
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("ref")
    p.add_argument("karbon")
    p.add_argument("--ref-png")
    p.add_argument("--png")
    p.add_argument("--out", default=".")
    p.add_argument("--crop", action="append", default=[])
    p.add_argument("--all", action="store_true", help="also list the elements within 1.5 px")
    p.add_argument("--only-new", metavar="HOME.json",
                   help="compare only the reference elements absent from this capture (an open menu)")
    p.add_argument("--menu", help="name of the open menu (--open): only its boxes of MENU_CHROME")
    a = p.parse_args()
    ref = load(a.ref)
    if a.only_new:
        key = lambda e: (e["path"], e.get("text", ""), round(e["x"]), round(e["y"]))
        home = {key(e) for e in load(a.only_new)}
        ref = [e for e in ref if key(e) not in home]
    # Bas visible de la page : haut de la carte du lecteur, moins son retrait (pt-1.5).
    players = [r["y"] for r in ref if r["path"].endswith("div.lecteur.relative.rounded-[18px]")]
    report(match(ref, load(a.karbon), bool(a.only_new), a.menu), a.all,
           min(players) - 6 if players else float("inf"))
    if a.crop:
        rects = [tuple(float(v) for v in c.split(",")) for c in a.crop]
        crops(a.ref_png, a.png, rects, Path(a.out))
    return 0


if __name__ == "__main__":
    sys.exit(main())
