//! Polices Pango/fontconfig : polices embarquées de l'application (TTF, OTF, WOFF, WOFF2,
//! gardées privées au processus), familles système et familles génériques CSS.
//!
//! Une police embarquée est écrite dans un fichier anonyme en mémoire (`memfd`) que
//! fontconfig, FreeType et HarfBuzz ouvrent par `/proc/self/fd/N` ; un conteneur WOFF ou
//! WOFF2 est d'abord décodé par FreeType (HarfBuzz ne lit que les polices brutes). Elle
//! est enregistrée dans une configuration fontconfig propre au renderer, sous une famille
//! privée : une police installée du même nom (Inter, JetBrains Mono…) ne la remplace pas,
//! et le nom interne du fichier (« Figtree Light ») n'a pas d'importance.
//!
//! Les familles Windows du moteur (`Segoe UI`, `Consolas`) et les génériques CSS
//! désignent les génériques de fontconfig (`system-ui`, `monospace`…) ; `system-ui` et
//! `monospace` désignent d'abord les polices du bureau (`desktop.rs`, GNOME : « Inter 10 »
//! → Inter) quand elles sont installées, pour les fenêtres seulement (`follow_desktop`) :
//! aperçus et exports ne dépendent pas des réglages de la session.
use super::ffi::{
    c_string,
    fontconfig::{self, Config},
    pango::{FontDescription, FontMap},
};
use crate::ui::font::FontAsset;
use std::{collections::HashMap, ffi::CString, io::Write, os::fd::AsRawFd};

/// Famille générique ou alias Windows/macOS : générique fontconfig correspondant.
fn generic(family: &str) -> Option<&'static str> {
    Some(match family.to_ascii_lowercase().as_str() {
        "segoe ui"
        | "segoe ui variable"
        | "segoe ui variable text"
        | "system-ui"
        | "-apple-system"
        | "blinkmacsystemfont"
        | "ui-sans-serif" => "system-ui",
        "sans-serif" => "sans-serif",
        "serif" | "ui-serif" => "serif",
        "monospace" | "ui-monospace" | "consolas" | "cascadia code" | "cascadia mono"
        | "sfmono-regular" | "menlo" => "monospace",
        "cursive" => "cursive",
        "fantasy" => "fantasy",
        "emoji" => "emoji",
        _ => return None,
    })
}
/// Famille embarquée : nom privé enregistré dans fontconfig.
struct Embedded {
    private: CString,
    /// Une instance au moins est variable : la graisse demandée devient l'axe `wght`.
    variable: bool,
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Key {
    family: &'static str,
    weight: u16,
    italic: bool,
    size: u32,
}
pub(super) struct Fonts {
    map: FontMap,
    /// Configuration privée des polices embarquées (`None` : celle du système).
    config: Option<Config>,
    /// Fichiers en mémoire des polices embarquées, ouverts tant que la carte existe.
    _files: Vec<std::fs::File>,
    embedded: HashMap<String, Embedded>,
    descriptions: HashMap<Key, FontDescription>,
    /// Familles système existantes (ou non), par nom en minuscules.
    system: HashMap<String, bool>,
    resolved: HashMap<&'static str, &'static str>,
    /// `system-ui` et `monospace` suivent les polices du bureau (fenêtres).
    desktop: bool,
}
impl Fonts {
    pub fn new() -> crate::Result<Self> {
        Ok(Self {
            map: FontMap::new(None)?,
            config: None,
            _files: Vec::new(),
            embedded: HashMap::new(),
            descriptions: HashMap::new(),
            system: HashMap::new(),
            resolved: HashMap::new(),
            desktop: false,
        })
    }
    /// Les familles `system-ui` et `monospace` désignent désormais les polices du bureau ;
    /// les descriptions déjà résolues sont oubliées.
    pub fn follow_desktop(&mut self) {
        self.desktop = true;
        self.descriptions.clear();
        self.resolved.clear();
    }
    /// Carte des polices et contexte de mise en page Pango.
    pub fn map(&self) -> &FontMap {
        &self.map
    }
    /// Enregistre les polices embarquées (sans les installer pour le système) dans une
    /// nouvelle configuration ; les mises en page existantes doivent être oubliées.
    pub fn load(&mut self, assets: &[FontAsset]) -> crate::Result<()> {
        if assets.is_empty() {
            // Retour à la configuration du système ; le suivi des polices du bureau
            // (`follow_desktop`) est conservé : les boucles l'activent avant de charger
            // les polices de l'application.
            let desktop = self.desktop;
            *self = Self::new()?;
            self.desktop = desktop;
            return Ok(());
        }
        let config = Config::system()?;
        let mut files = Vec::new();
        let mut embedded = HashMap::<String, Embedded>::new();
        for asset in assets {
            if asset.family.is_empty()
                || asset.family.contains('\0')
                || asset.data.is_empty()
                || asset.data.len() > 16 * 1024 * 1024
            {
                return Err(crate::Error::invalid_input("invalid font asset"));
            }
            let raw = match asset.data.get(..4) {
                Some(b"wOFF" | b"wOF2") => fontconfig::sfnt(asset.data)
                    .ok_or_else(|| crate::Error::invalid_input("unsupported font data"))?,
                _ => asset.data.to_vec(),
            };
            let file = memory_file(&raw)?;
            let path = c_string(&format!("/proc/self/fd/{}", file.as_raw_fd()))?;
            let lower = asset.family.to_ascii_lowercase();
            let private = c_string(&format!("karbon-embedded-{lower}"))?;
            let variable = config
                .add_font(&path, &private)
                .ok_or_else(|| crate::Error::invalid_input("unsupported font data"))?;
            files.push(file);
            embedded
                .entry(lower)
                .and_modify(|e| e.variable |= variable)
                .or_insert(Embedded { private, variable });
        }
        self.map = FontMap::new(Some(config.raw()))?;
        self.config = Some(config);
        self._files = files;
        self.embedded = embedded;
        self.descriptions.clear();
        self.system.clear();
        self.resolved.clear();
        Ok(())
    }
    /// Famille du bureau pour le générique fontconfig `generic` (`system-ui`, `monospace`),
    /// si elle est installée ; sinon le générique lui-même.
    fn desktop<'a>(&mut self, generic: &'a str) -> &'a str {
        if !self.desktop {
            return generic;
        }
        let fonts = super::desktop::fonts();
        let family = match generic {
            "system-ui" => fonts.interface.as_deref(),
            "monospace" => fonts.monospace.as_deref(),
            _ => None,
        };
        match family {
            Some(family) if self.available(family) => family,
            _ => generic,
        }
    }
    /// Première famille disponible d'une liste CSS (`"Inter, Segoe UI"`) : polices de
    /// l'application, génériques puis polices système ; sinon la dernière de la liste.
    pub fn resolve(&mut self, family: &'static str) -> &'static str {
        if !family.contains(',') {
            return unquote(family);
        }
        if let Some(found) = self.resolved.get(family) {
            return found;
        }
        let mut chosen = None;
        let mut last = family;
        for candidate in family.split(',').map(unquote).filter(|s| !s.is_empty()) {
            last = candidate;
            if self.available(candidate) {
                chosen = Some(candidate);
                break;
            }
        }
        let found = chosen.unwrap_or(last);
        self.resolved.insert(family, found);
        found
    }
    fn available(&mut self, family: &str) -> bool {
        let lower = family.to_ascii_lowercase();
        if self.embedded.contains_key(&lower) || generic(family).is_some() {
            return true;
        }
        if let Some(found) = self.system.get(&lower) {
            return *found;
        }
        let found = CString::new(family)
            .is_ok_and(|name| fontconfig::has_family(self.config.as_ref(), &name));
        self.system.insert(lower, found);
        found
    }
    /// Description Pango de `family` (liste CSS admise) à cette graisse et cette taille.
    pub fn font(
        &mut self,
        family: &'static str,
        weight: u16,
        italic: bool,
        size: f32,
    ) -> crate::Result<FontDescription> {
        if !(1..=999).contains(&weight)
            || family.is_empty()
            || family.contains('\0')
            || !size.is_finite()
            || size <= 0.
        {
            return Err(crate::Error::invalid_input(
                "invalid font family, weight or size",
            ));
        }
        let family = self.resolve(family);
        let key = Key {
            family,
            weight,
            italic,
            size: size.to_bits(),
        };
        if let Some(desc) = self.descriptions.get(&key) {
            return Ok(desc.clone());
        }
        let lower = family.to_ascii_lowercase();
        let desc = match self.embedded.get(&lower) {
            Some(face) => {
                let axis = face
                    .variable
                    .then(|| c_string(&format!("wght={weight}")))
                    .transpose()?;
                FontDescription::new(&face.private, weight, italic, size, axis.as_deref())?
            }
            None => {
                let name = match generic(family) {
                    Some(generic) => self.desktop(generic),
                    None if self.available(family) => family,
                    // Famille absente : police d'interface, comme sous macOS.
                    None => self.desktop("system-ui"),
                };
                FontDescription::new(&c_string(name)?, weight, italic, size, None)?
            }
        };
        if self.descriptions.len() >= 512 {
            self.descriptions.clear();
        }
        self.descriptions.insert(key, desc.clone());
        Ok(desc)
    }
}
fn unquote(name: &str) -> &str {
    name.trim().trim_matches(|c| c == '"' || c == '\'').trim()
}
/// Fichier anonyme en mémoire contenant `bytes`, fermé à l'exécution d'un autre programme.
fn memory_file(bytes: &[u8]) -> crate::Result<std::fs::File> {
    use std::os::fd::FromRawFd;
    // SAFETY : nom statique terminé par un octet nul.
    let fd = unsafe { libc::memfd_create(c"karbon-font".as_ptr(), libc::MFD_CLOEXEC) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    // SAFETY : descripteur fraîchement créé, possédé par le fichier.
    let mut file = unsafe { std::fs::File::from_raw_fd(fd) };
    file.write_all(bytes)?;
    Ok(file)
}
