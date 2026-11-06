//! Fonts and covers, decoded once at load. Templates only clone `Arc` handles. The
//! icons are compiled SVG (see `icons.rs`).
use karbon_ui::{
    application::Resources,
    ui::{font::FontAsset, scene::Image},
};
use std::{collections::HashMap, sync::Arc};

pub static FONTS: [FontAsset; 3] = [
    FontAsset {
        family: "Figtree Light",
        data: include_bytes!("assets/fonts/figtree.woff2"),
    },
    FontAsset {
        family: "Inter",
        data: include_bytes!("assets/fonts/inter.woff2"),
    },
    FontAsset {
        family: "JetBrains Mono",
        data: include_bytes!("assets/fonts/jetbrains-mono.woff2"),
    },
];

macro_rules! covers {
    ($($name:literal),* $(,)?) => {
        [$(($name, include_bytes!(concat!("assets/covers/", $name, ".png")).as_slice())),*]
    };
}
/// Pochettes de l'état de référence (captures Tauri), tirées du cache de RustMusic par
/// `tools/prepare-covers.py`.
macro_rules! reference_covers {
    ($($name:literal),* $(,)?) => {
        [$(($name, include_bytes!(concat!("assets/covers/", $name, ".jpg")).as_slice())),*]
    };
}

pub struct Assets {
    covers: HashMap<&'static str, Arc<Image>>,
}
impl Assets {
    pub fn load(resources: &dyn Resources) -> karbon_ui::Result<Self> {
        let mut covers = HashMap::new();
        for (name, bytes) in covers!(
            "queen",
            "jurassic",
            "unplugged",
            "it",
            "katy",
            "bowie",
            "nrj",
            "johnny",
            "incesticide",
            "now",
            "lotr",
            "snoop",
            "adele",
            "sting",
            "hurts",
            "aerosmith",
            "madmax",
            "bonjovi",
            "coldplay",
        ) {
            covers.insert(name, resources.decode_image(bytes)?);
        }
        for (name, bytes) in reference_covers!(
            "zeppelin4",
            "man",
            "radiohead_best2",
            "iam_anthologie",
            "stp_purple",
            "beatles_past1",
            "adele25",
            "divide",
            "overexposed",
            "rush",
            "no_line",
            "lights_out",
            "u2_best",
            "slim_shady",
            "ballo",
            "top50",
            "stp100",
            "zimmer_live",
            "bohemian",
            "flower_moon",
            "peaky",
            "pablo_honey",
            "facelift",
            "ten",
            "bssm",
            "reggatta",
            "iz_it_a_crime",
            "aaliyah",
            "s_kings_30stm",
            "s_aaliyah_age",
            "s_aerosmith",
            "s_meteque",
            "s_angele",
            "s_alizee",
            "podcast_gt",
            "podcast_legend",
            "podcast_crime",
            "podcast_hondelatte",
        ) {
            covers.insert(name, resources.decode_image(bytes)?);
        }
        // Logo transparent de la dernière radio écoutée (RadioLogo le pose sur blanc).
        covers.insert(
            "radio_logo",
            resources.decode_image(include_bytes!("assets/covers/radio_logo.png"))?,
        );
        Ok(Self { covers })
    }
    /// Cover by its asset name (`"queen"`, `"katy"`…). Unknown names are a programming error.
    pub fn cover(&self, name: &str) -> Arc<Image> {
        Arc::clone(
            self.covers
                .get(name)
                .unwrap_or_else(|| panic!("unknown cover {name}")),
        )
    }
}
