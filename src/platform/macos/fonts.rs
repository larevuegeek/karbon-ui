//! Polices Core Text : polices embarquées de l'application (TTF, OTF, WOFF, WOFF2, gardées
//! privées au processus), familles système et familles génériques CSS.
//!
//! Les familles Windows du moteur (`Segoe UI`, `Consolas`) et les génériques CSS
//! désignent les polices d'interface de macOS (SF Pro, SF Mono/Menlo).
use super::ffi::{cf_string, required};
use crate::ui::font::FontAsset;
use objc2_core_foundation::{
    CFArray, CFDictionary, CFNumber, CFRetained, CFString, CFType, CGAffineTransform, CGFloat,
};
use objc2_core_text::{
    CTFont, CTFontDescriptor, CTFontManagerCreateFontDescriptorsFromData, CTFontSymbolicTraits,
    CTFontUIFontType, kCTFontFamilyNameAttribute, kCTFontFeatureSettingsAttribute,
    kCTFontOpenTypeFeatureTag, kCTFontOpenTypeFeatureValue, kCTFontSymbolicTrait,
    kCTFontTraitsAttribute, kCTFontWeightTrait,
};
use std::collections::HashMap;

/// Identifiant OpenType de l'axe de graisse (`wght`).
const WEIGHT_AXIS: i64 = 0x7767_6874;
/// Graisses CSS et graisses Core Text (`NSFontWeight*`) correspondantes.
const WEIGHTS: [(f64, f64); 9] = [
    (100., -0.8),
    (200., -0.6),
    (300., -0.4),
    (400., 0.),
    (500., 0.23),
    (600., 0.3),
    (700., 0.4),
    (800., 0.56),
    (900., 0.62),
];
/// Graisse Core Text (`-1..=1`) d'une graisse CSS, par interpolation linéaire.
fn native_weight(css: u16) -> f64 {
    interpolate(f64::from(css), |(c, _)| c, |(_, n)| n)
}
/// Graisse CSS d'une graisse Core Text.
fn css_weight(native: f64) -> f64 {
    interpolate(native, |(_, n)| n, |(c, _)| c)
}
fn interpolate(x: f64, from: fn((f64, f64)) -> f64, to: fn((f64, f64)) -> f64) -> f64 {
    let first = WEIGHTS[0];
    if x <= from(first) {
        return to(first);
    }
    for pair in WEIGHTS.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if x <= from(b) {
            let t = (x - from(a)) / (from(b) - from(a));
            return to(a) + t * (to(b) - to(a));
        }
    }
    to(WEIGHTS[WEIGHTS.len() - 1])
}
/// Famille générique ou alias Windows : police d'interface macOS correspondante.
fn generic(family: &str) -> Option<CTFontUIFontType> {
    match family.to_ascii_lowercase().as_str() {
        "segoe ui"
        | "segoe ui variable"
        | "segoe ui variable text"
        | "system-ui"
        | "-apple-system"
        | "blinkmacsystemfont"
        | "sans-serif"
        | "ui-sans-serif" => Some(CTFontUIFontType::System),
        "monospace" | "ui-monospace" | "consolas" | "cascadia code" | "cascadia mono"
        | "sfmono-regular" => Some(CTFontUIFontType::UserFixedPitch),
        _ => None,
    }
}
/// Instance d'une police embarquée.
struct Face {
    descriptor: CFRetained<CTFontDescriptor>,
    weight: f64,
    italic: bool,
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Key {
    family: &'static str,
    weight: u16,
    italic: bool,
    size: u32,
    tabular: bool,
}
#[derive(Default)]
pub(super) struct Fonts {
    /// Polices de l'application, par nom de famille en minuscules.
    embedded: HashMap<String, Vec<Face>>,
    fonts: HashMap<Key, CFRetained<CTFont>>,
    /// Familles système existantes (ou non), par nom en minuscules.
    system: HashMap<String, bool>,
    resolved: HashMap<&'static str, &'static str>,
}
impl Fonts {
    /// Enregistre les polices embarquées (sans les installer pour le système).
    pub fn load(&mut self, assets: &[FontAsset]) -> crate::Result<()> {
        let mut embedded = HashMap::<String, Vec<Face>>::new();
        for asset in assets {
            if asset.family.is_empty()
                || asset.family.contains('\0')
                || asset.data.is_empty()
                || asset.data.len() > 16 * 1024 * 1024
            {
                return Err(crate::Error::invalid_input("invalid font asset"));
            }
            let data = objc2_core_foundation::CFData::from_static_bytes(asset.data);
            // SAFETY : données CF valides ; renvoie un tableau de CTFontDescriptor.
            let descriptors = unsafe { CTFontManagerCreateFontDescriptorsFromData(&data) };
            // SAFETY : tableau typé par la documentation de Core Text.
            let descriptors: &CFArray<CTFontDescriptor> = unsafe { descriptors.cast_unchecked() };
            if descriptors.is_empty() {
                return Err(crate::Error::invalid_input("unsupported font data"));
            }
            let faces = embedded
                .entry(asset.family.to_ascii_lowercase())
                .or_default();
            for descriptor in descriptors {
                let (weight, italic) = traits(&descriptor);
                faces.push(Face {
                    descriptor,
                    weight,
                    italic,
                });
            }
        }
        self.embedded = embedded;
        self.fonts.clear();
        self.resolved.clear();
        Ok(())
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
        let found = system_font(family, 400, false, 12.).is_some();
        self.system.insert(lower, found);
        found
    }
    /// Police Core Text de `family` (liste CSS admise) à cette graisse et cette taille.
    pub fn font(
        &mut self,
        family: &'static str,
        weight: u16,
        italic: bool,
        size: f32,
        tabular: bool,
    ) -> crate::Result<CFRetained<CTFont>> {
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
            tabular,
        };
        if let Some(font) = self.fonts.get(&key) {
            return Ok(font.clone());
        }
        let size = CGFloat::from(size);
        let mut descriptor = match self.embedded.get(&family.to_ascii_lowercase()) {
            Some(faces) => embedded_descriptor(faces, weight, italic),
            None => match generic(family) {
                Some(kind) => ui_descriptor(kind, weight, size)?,
                None => match system_font(family, weight, italic, size) {
                    // SAFETY : police valide.
                    Some(font) => unsafe { font.font_descriptor() },
                    None => ui_descriptor(CTFontUIFontType::System, weight, size)?,
                },
            },
        };
        if tabular {
            descriptor = tabular_descriptor(&descriptor);
        }
        // SAFETY : descripteur valide ; matrice nulle (identité).
        let mut font = unsafe { CTFont::with_font_descriptor(&descriptor, size, std::ptr::null()) };
        // SAFETY : police valide.
        if italic && !unsafe { font.symbolic_traits() }.contains(CTFontSymbolicTraits::ItalicTrait)
        {
            // Oblique synthétique, comme DirectWrite sans italique dans la famille.
            let skew = CGAffineTransform {
                a: 1.,
                b: 0.,
                c: 0.21,
                d: 1.,
                tx: 0.,
                ty: 0.,
            };
            // SAFETY : descripteur valide ; matrice locale lue pendant l'appel.
            font = unsafe { CTFont::with_font_descriptor(&descriptor, size, &skew) };
        }
        if self.fonts.len() >= 512 {
            self.fonts.clear();
        }
        self.fonts.insert(key, font.clone());
        Ok(font)
    }
}
fn unquote(name: &str) -> &str {
    name.trim().trim_matches(|c| c == '"' || c == '\'').trim()
}
/// Graisse CSS et italique d'un descripteur.
fn traits(descriptor: &CTFontDescriptor) -> (f64, bool) {
    // SAFETY : clé constante de Core Text.
    let Some(value) = (unsafe { descriptor.attribute(kCTFontTraitsAttribute) }) else {
        return (400., false);
    };
    let Ok(traits) = value.downcast::<CFDictionary>() else {
        return (400., false);
    };
    // SAFETY : dictionnaire de traits : clés CFString, valeurs CFNumber.
    let traits: &CFDictionary<CFString, CFNumber> = unsafe { traits.cast_unchecked() };
    // SAFETY : clés constantes de Core Text.
    let weight = traits
        .get(unsafe { kCTFontWeightTrait })
        .and_then(|n| n.as_f64())
        .map_or(400., css_weight);
    // SAFETY : idem.
    let symbolic = traits
        .get(unsafe { kCTFontSymbolicTrait })
        .and_then(|n| n.as_i64())
        .unwrap_or(0) as u32;
    (weight, symbolic & CTFontSymbolicTraits::ItalicTrait.0 != 0)
}
/// Instance embarquée la plus proche, à la graisse exacte si la police est variable.
fn embedded_descriptor(faces: &[Face], weight: u16, italic: bool) -> CFRetained<CTFontDescriptor> {
    let face = faces
        .iter()
        .min_by(|a, b| {
            let score = |f: &Face| {
                (f.weight - f64::from(weight)).abs() + if f.italic == italic { 0. } else { 1000. }
            };
            score(a).total_cmp(&score(b))
        })
        .expect("a loaded family has at least one face");
    // SAFETY : descripteur valide ; un axe absent laisse le descripteur inchangé.
    unsafe {
        face.descriptor
            .copy_with_variation(&CFNumber::new_i64(WEIGHT_AXIS), f64::from(weight))
    }
}
/// Police d'interface (`System` ou `UserFixedPitch`) à la graisse demandée.
fn ui_descriptor(
    kind: CTFontUIFontType,
    weight: u16,
    size: CGFloat,
) -> crate::Result<CFRetained<CTFontDescriptor>> {
    // SAFETY : type de police d'interface valide ; langue par défaut.
    let font = required(
        unsafe { CTFont::new_ui_font_for_language(kind, size, None) },
        "CTFontCreateUIFontForLanguage",
    )?;
    // SAFETY : police valide.
    let base = unsafe { font.font_descriptor() };
    if weight == 400 {
        return Ok(base);
    }
    let attributes = weight_attributes(weight, None);
    // SAFETY : descripteur et dictionnaire d'attributs valides.
    Ok(unsafe { base.copy_with_attributes(attributes.as_opaque()) })
}
/// Attributs `{traits: {weight, symbolic?}}` (et famille) d'une recherche de police.
fn weight_attributes(
    weight: u16,
    family: Option<(&CFString, bool)>,
) -> CFRetained<CFDictionary<CFString, CFType>> {
    let weight = CFNumber::new_f64(native_weight(weight));
    let italic = CFNumber::new_i64(i64::from(CTFontSymbolicTraits::ItalicTrait.0));
    // SAFETY : clés constantes de Core Text.
    let (weight_key, symbolic_key) = unsafe { (kCTFontWeightTrait, kCTFontSymbolicTrait) };
    let traits = match family {
        Some((_, true)) => CFDictionary::<CFString, CFType>::from_slices(
            &[weight_key, symbolic_key],
            &[weight.as_ref(), italic.as_ref()],
        ),
        _ => CFDictionary::<CFString, CFType>::from_slices(&[weight_key], &[weight.as_ref()]),
    };
    // SAFETY : clés constantes de Core Text.
    let (traits_key, family_key) = unsafe { (kCTFontTraitsAttribute, kCTFontFamilyNameAttribute) };
    match family {
        Some((name, _)) => {
            CFDictionary::from_slices(&[traits_key, family_key], &[traits.as_ref(), name.as_ref()])
        }
        None => CFDictionary::from_slices(&[traits_key], &[traits.as_ref()]),
    }
}
/// Police installée de cette famille ; `None` si Core Text lui substitue une autre famille.
fn system_font(
    family: &str,
    weight: u16,
    italic: bool,
    size: CGFloat,
) -> Option<CFRetained<CTFont>> {
    let name = cf_string(family);
    let attributes = weight_attributes(weight, Some((&name, italic)));
    // SAFETY : dictionnaire d'attributs valide ; matrice nulle (identité).
    let font = unsafe {
        let descriptor = CTFontDescriptor::with_attributes(attributes.as_opaque());
        CTFont::with_font_descriptor(&descriptor, size, std::ptr::null())
    };
    // SAFETY : police valide.
    let found = unsafe { font.family_name() }.to_string();
    found.eq_ignore_ascii_case(family).then_some(font)
}
/// Chiffres tabulaires (`tnum`).
fn tabular_descriptor(descriptor: &CTFontDescriptor) -> CFRetained<CTFontDescriptor> {
    let tag = cf_string("tnum");
    let on = CFNumber::new_i64(1);
    // SAFETY : clés constantes de Core Text.
    let (tag_key, value_key, settings_key) = unsafe {
        (
            kCTFontOpenTypeFeatureTag,
            kCTFontOpenTypeFeatureValue,
            kCTFontFeatureSettingsAttribute,
        )
    };
    let setting = CFDictionary::<CFString, CFType>::from_slices(
        &[tag_key, value_key],
        &[tag.as_ref(), on.as_ref()],
    );
    let settings = CFArray::<CFDictionary<CFString, CFType>>::from_objects(&[&setting]);
    let attributes =
        CFDictionary::<CFString, CFType>::from_slices(&[settings_key], &[settings.as_ref()]);
    // SAFETY : descripteur et dictionnaire valides.
    unsafe { descriptor.copy_with_attributes(attributes.as_opaque()) }
}
