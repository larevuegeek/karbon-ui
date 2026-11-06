"""Copy the covers of the Tauri reference state from RustMusic's cover cache.

Reads (never writes) %APPDATA%/com.larevuegeek.rustmusic/covers and writes resized
JPEGs to examples/rustmusic_html/assets/covers/. The hashes were identified by
matching the reference captures against the cache (see docs/rustmusic-html.md).

    python -I examples/rustmusic_html/tools/prepare-covers.py
"""
import os
import sys
from pathlib import Path

from PIL import Image

CACHE = Path(os.environ["APPDATA"]) / "com.larevuegeek.rustmusic" / "covers"
OUT = Path(__file__).resolve().parent.parent / "assets" / "covers"

# nom -> (fichier du cache, côté en pixels). 360 px : pochette du hero (220 × 1,5).
COVERS = {
    # Lecteur, hero et file « Ensuite »
    "zeppelin4": ("albums/2x/0f1451c2c3afe60d8c6be61aa58d9129.jpg", 360),
    "man": ("albums/2x/8a7b50ba15a17f6da8e93a3af2e7ecec.jpg", 132),
    "radiohead_best2": ("albums/2x/49c834ea3a5f6352dc01a51df8bbbde0.jpg", 132),
    "iam_anthologie": ("albums/2x/67b64e6d9726e3322e7df03ec9753a03.jpg", 132),
    "stp_purple": ("albums/2x/c2a4fc8a51cd8310c9d7a6613ea36e12.jpg", 132),
    # Mix « Pour vous »
    "beatles_past1": ("albums/2x/92451622653de009a1c1c3640c5308af.jpg", 240),
    "adele25": ("albums/2x/207eed35a126dd68f617c6a7ed5202e7.jpg", 240),
    "divide": ("albums/2x/0d7e5caf5bb84874e6d566bfbc131147.jpg", 240),
    "overexposed": ("albums/2x/f1f2117e94efa9e6441ad3fcb72e6789.jpg", 240),
    "rush": ("albums/2x/a2dea529eb4c330843ce0bbd306c8a16.jpg", 240),
    "no_line": ("albums/2x/905ce23da5009c1c871a68e349fcaec0.jpg", 240),
    "lights_out": ("albums/2x/5507ccc44112619288c2098a4ae85413.jpg", 240),
    "u2_best": ("albums/2x/68f53de6293240c1bc5ce0cec72ad9fd.jpg", 240),
    "slim_shady": ("albums/2x/224992ee8b23cf3ab8ba30d30f724217.jpg", 240),
    "ballo": ("albums/2x/dc2d2a2bd03b938ea43f0e644d8adc41.jpg", 240),
    "top50": ("albums/2x/e4a65f8531621cea5a30a7cbabaceea0.jpg", 240),
    "stp100": ("albums/2x/89f74e102163220b3b7ac28cb9f0c268.jpg", 240),
    "zimmer_live": ("albums/2x/736d44076e47618592b6b8799dfa400f.jpg", 240),
    "bohemian": ("albums/2x/1ef4589d6bf6823688214d12cee0349d.jpg", 240),
    "flower_moon": ("albums/2x/af45239dacc8b869b2a1ccdb086cce36.jpg", 240),
    "peaky": ("albums/2x/43859e75dc315e88ff9f098443e46bae.jpg", 240),
    # Barre latérale
    "pablo_honey": ("albums/2x/d7dc3c0dce99af6dc0dfaac12ceeb852.jpg", 64),
    "facelift": ("albums/2x/cc8f3f23a1273fcc7fa65b0234abfffb.jpg", 64),
    "ten": ("albums/2x/578fbbc3eb5c959dd6f17a84867cfdd5.jpg", 64),
    "bssm": ("albums/2x/339d7a408e21613ed115a61ff7a4c113.jpg", 64),
    "reggatta": ("albums/2x/13ed815c972df41ebfcb0580e65a905a.jpg", 64),
    "iz_it_a_crime": ("albums/2x/030b48a23f1a45ffb0d2ec73d364fa59.jpg", 64),
    "aaliyah": ("albums/2x/d394513158f9e0657ccff66357766649.jpg", 64),
    # Suggestions de recherche (« que », capture menu-search) : vignettes de 40 px
    "s_kings_30stm": ("albums/2x/e3859cee3fbc9dd15dcae07f5418e7a1.jpg", 60),
    "s_aaliyah_age": ("albums/2x/049907358beeabe7f476982b75359fb9.jpg", 60),
    "s_aerosmith": ("albums/2x/6ca8c9537df1fc8a046a48b61f2a2b20.jpg", 60),
    "s_meteque": ("albums/2x/2205bb63f54a6e3e1c44f91846e30288.jpg", 60),
    "s_angele": ("albums/2x/b3622c3c2e565d543f20e5c30ae67a96.jpg", 60),
    "s_alizee": ("albums/2x/f596b29ecc8d7052760b3af74d60c584.jpg", 60),
    "podcast_gt": ("podcasts/1x/2.jpg", 64),
    "podcast_legend": ("podcasts/1x/3.jpg", 64),
    "podcast_crime": ("podcasts/1x/7.jpg", 64),
    "podcast_hondelatte": ("podcasts/1x/1.jpg", 64),
    "radio_logo": ("radios/d1b5ff80edc930cb463ea861b596987a.png", 64),
}


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    for name, (source, side) in COVERS.items():
        path = CACHE / source
        if not path.exists():
            # Certaines pochettes n'ont que la vignette 1x ou l'original.
            path = next(p for p in (path.parent.parent / d / path.name for d in ("full", "1x")) if p.exists())
        image = Image.open(path)
        if name == "radio_logo":
            # Logo transparent posé sur blanc par RadioLogo.svelte : on garde l'alpha.
            image.convert("RGBA").resize((side, side), Image.LANCZOS).save(OUT / f"{name}.png", optimize=True)
            continue
        image = image.convert("RGB").resize((side, side), Image.LANCZOS)
        image.save(OUT / f"{name}.jpg", quality=88, optimize=True)
    print(f"{len(COVERS)} covers -> {OUT}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
