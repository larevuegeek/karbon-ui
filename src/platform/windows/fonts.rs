//! Collection DirectWrite privée ; WOFF/WOFF2 décompressés par Windows.
use super::graphics::Checked;
use crate::ui::font::FontAsset;
use std::ffi::c_void;
use windows::Win32::Graphics::DirectWrite::*;
use windows::core::{HSTRING, Interface, PCWSTR};
pub(super) struct FontCollection {
    pub collection: Option<IDWriteFontCollection>,
    loader: IDWriteInMemoryFontFileLoader,
    factory: IDWriteFactory6,
}
impl Drop for FontCollection {
    fn drop(&mut self) {
        drop(self.collection.take());
        // SAFETY: chargeur enregistré dans `new` auprès de cette même fabrique.
        let _ = unsafe { self.factory.UnregisterFontFileLoader(&self.loader) };
    }
}
impl FontCollection {
    pub fn new(base: &IDWriteFactory, assets: &[FontAsset]) -> crate::Result<Self> {
        let factory: IDWriteFactory6 = base.cast().check("QueryInterface")?;
        // SAFETY: fabrique DirectWrite valide.
        let loader = unsafe { factory.CreateInMemoryFontFileLoader() }
            .check("CreateInMemoryFontFileLoader")?;
        // SAFETY: chargeur valide ; désenregistré dans Drop.
        unsafe { factory.RegisterFontFileLoader(&loader) }.check("RegisterFontFileLoader")?;
        let mut owned = Self {
            collection: None,
            loader,
            factory,
        };
        // IDWriteFactory6::CreateFontSetBuilder ; AddFontFile est celui d'IDWriteFontSetBuilder1.
        // SAFETY: fabrique DirectWrite valide.
        let builder =
            unsafe { owned.factory.CreateFontSetBuilder() }.check("CreateFontSetBuilder1")?;
        let builder: &IDWriteFontSetBuilder1 = &builder;
        for asset in assets {
            if asset.data.is_empty() || asset.data.len() > 16 * 1024 * 1024 {
                return Err(crate::Error::invalid_input(
                    "font asset empty or larger than 16 MiB",
                ));
            }
            let (data, size) = (
                asset.data.as_ptr().cast::<c_void>(),
                asset.data.len() as u32,
            );
            // SAFETY: tampon lu pendant l'appel seulement ; taille bornée ci-dessus.
            let kind = unsafe { owned.factory.AnalyzeContainerType(data, size) };
            let font = if kind == DWRITE_CONTAINER_TYPE_UNKNOWN {
                owned.reference(asset.data)?
            } else {
                // SAFETY: idem ; le flux décompressé est une référence possédée.
                let stream = unsafe { owned.factory.UnpackFontFile(kind, data, size) }
                    .check("UnpackFontFile")?;
                // SAFETY: flux valide.
                let length = unsafe { stream.GetFileSize() }.check("GetFileSize")?;
                if length == 0 || length > 16 * 1024 * 1024 {
                    return Err(crate::Error::invalid_input(
                        "decompressed font empty or larger than 16 MiB",
                    ));
                }
                let mut data = std::ptr::null_mut();
                let mut context = std::ptr::null_mut();
                // SAFETY: lecture du flux entier ; fragment libéré par `Fragment` ci-dessous.
                unsafe { stream.ReadFileFragment(&mut data, 0, length, &mut context) }
                    .check("ReadFileFragment")?;
                struct Fragment<'a>(&'a IDWriteFontFileStream, *mut c_void);
                impl Drop for Fragment<'_> {
                    fn drop(&mut self) {
                        // SAFETY: contexte rendu par le ReadFileFragment réussi, libéré une fois.
                        unsafe { self.0.ReleaseFileFragment(self.1) }
                    }
                }
                let _fragment = Fragment(&stream, context);
                if data.is_null() {
                    return Err(crate::Error::Platform {
                        operation: "ReadFileFragment",
                        code: windows::Win32::Foundation::E_POINTER.0,
                    });
                }
                // SAFETY: fragment de `length` octets valide jusqu'à ReleaseFileFragment ;
                // CreateInMemoryFontFileReference en fait une copie.
                owned.reference(unsafe {
                    std::slice::from_raw_parts(data.cast::<u8>(), length as usize)
                })?
            };
            // SAFETY: constructeur et fichier de police valides.
            unsafe { builder.AddFontFile(&font) }.check("AddFontFile")?;
        }
        // SAFETY: constructeur valide.
        let set = unsafe { builder.CreateFontSet() }.check("CreateFontSet")?;
        // SAFETY: ensemble valide ; modèle de familles typographique.
        let collection = unsafe {
            owned
                .factory
                .CreateFontCollectionFromFontSet(&set, DWRITE_FONT_FAMILY_MODEL_TYPOGRAPHIC)
        }
        .check("CreateFontCollectionFromFontSet")?;
        owned.collection = Some(collection.into());
        for asset in assets {
            if !owned.contains(asset.family)? {
                return Err(crate::Error::invalid_input(format!(
                    "font family `{}` not found in its asset (available: {:?})",
                    asset.family,
                    owned.family_names()?
                )));
            }
        }
        Ok(owned)
    }
    fn reference(&self, data: &[u8]) -> crate::Result<IDWriteFontFile> {
        // SAFETY: sans objet propriétaire, le chargeur copie les données pendant l'appel.
        unsafe {
            self.loader.CreateInMemoryFontFileReference(
                &self.factory,
                data.as_ptr().cast(),
                data.len() as u32,
                None,
            )
        }
        .check("CreateInMemoryFontFileReference")
    }
    pub fn contains(&self, family: &str) -> crate::Result<bool> {
        let family = HSTRING::from(family);
        let collection = self.collection.as_ref().unwrap();
        let (mut index, mut found) = (0, false.into());
        // SAFETY: nom terminé par NUL et sorties valides pendant l'appel.
        unsafe { collection.FindFamilyName(&family, &mut index, &mut found) }
            .check("FindFamilyName")?;
        Ok(found.as_bool())
    }
    pub fn format(
        &self,
        family: &HSTRING,
        size: f32,
        weight: u16,
        locale: PCWSTR,
    ) -> crate::Result<IDWriteTextFormat> {
        let axis = DWRITE_FONT_AXIS_VALUE {
            axisTag: DWRITE_FONT_AXIS_TAG_WEIGHT,
            value: weight as f32,
        };
        // SAFETY: chaînes terminées par NUL vivantes ; axe lu pendant l'appel.
        let format = unsafe {
            self.factory
                .CreateTextFormat(family, self.collection.as_ref(), &[axis], size, locale)
        }
        .check("CreateTextFormat (wght axis)")?;
        Ok(format.into())
    }
    fn family_names(&self) -> crate::Result<Vec<String>> {
        let c = self.collection.as_ref().unwrap();
        let mut result = Vec::new();
        // SAFETY: collection valide.
        for index in 0..unsafe { c.GetFontFamilyCount() } {
            // SAFETY: index borné par GetFontFamilyCount.
            let family = unsafe { c.GetFontFamily(index) }.check("GetFontFamily")?;
            // SAFETY: famille valide.
            let names = unsafe { family.GetFamilyNames() }.check("GetFamilyNames")?;
            // SAFETY: liste de noms valide, premier élément.
            let len = unsafe { names.GetStringLength(0) }.check("GetStringLength")?;
            let mut buf = vec![0; len as usize + 1];
            // SAFETY: tampon de len + 1 unités, NUL compris.
            unsafe { names.GetString(0, &mut buf) }.check("GetString")?;
            result.push(String::from_utf16_lossy(&buf[..len as usize]));
        }
        Ok(result)
    }
}
/// Première famille disponible d'une liste CSS (`"Inter,Segoe UI"`) : polices de
/// l'application puis polices système. Une famille seule est rendue telle quelle ;
/// si aucune n'existe, la dernière est gardée (DirectWrite applique son repli).
pub(super) fn resolve(
    factory: &IDWriteFactory,
    fonts: Option<&FontCollection>,
    family: &'static str,
) -> crate::Result<&'static str> {
    type Cache = std::collections::HashMap<(usize, usize, usize), &'static str>;
    thread_local! {
        static CACHE: std::cell::RefCell<Cache> = Default::default();
    }
    if !family.contains(',') {
        return Ok(family);
    }
    let key = (
        family.as_ptr() as usize,
        family.len(),
        fonts.map_or(0, |f| f as *const FontCollection as usize),
    );
    if let Some(found) = CACHE.with(|c| c.borrow().get(&key).copied()) {
        return Ok(found);
    }
    let mut system = None;
    // SAFETY: sortie valide pendant l'appel ; collection système partagée.
    unsafe { factory.GetSystemFontCollection(&mut system, false) }
        .check("GetSystemFontCollection")?;
    let mut chosen = None;
    let mut last = family;
    for candidate in family.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        last = candidate;
        if let Some(fonts) = fonts
            && fonts.contains(candidate)?
        {
            chosen = Some(candidate);
            break;
        }
        if let Some(system) = &system {
            let name = HSTRING::from(candidate);
            let (mut index, mut found) = (0, false.into());
            // SAFETY: nom terminé par NUL et sorties valides pendant l'appel.
            unsafe { system.FindFamilyName(&name, &mut index, &mut found) }
                .check("FindFamilyName system")?;
            if found.as_bool() {
                chosen = Some(candidate);
                break;
            }
        }
    }
    let chosen = chosen.unwrap_or(last);
    CACHE.with(|c| {
        let mut cache = c.borrow_mut();
        if cache.len() >= 256 {
            cache.clear();
        }
        cache.insert(key, chosen);
    });
    Ok(chosen)
}
