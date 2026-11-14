//! Réponse du serveur : la dernière version, au format du module de mise à jour de Tauri.
//!
//! - Statique : `{"version", "notes", "pub_date", "platforms": {"<os>-<arch>[-<paquet>]":
//!   {"url", "signature"}}}` ;
//! - dynamique : `{"version", "notes", "pub_date", "url", "signature"}` (le serveur a déjà
//!   choisi le paquet).
//!
//! `version` peut s'appeler `name` ; un `v` initial est accepté.
use crate::{Error, json, version::Version};

/// Paquet téléchargeable et sa signature (minisign, en base64).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Asset {
    pub url: String,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Release {
    pub version: String,
    pub parsed: Version,
    pub notes: Option<String>,
    pub date: Option<String>,
    assets: Assets,
}

#[derive(Debug, Clone, PartialEq)]
enum Assets {
    Static(Vec<(String, Asset)>),
    Dynamic(Asset),
}

impl Release {
    pub(crate) fn parse(text: &str) -> Result<Self, Error> {
        let root = json::parse(text).map_err(Error::Manifest)?;
        fn text_of(value: Option<&json::Value>) -> Option<&str> {
            value.and_then(json::Value::as_str)
        }
        let version = text_of(root.get("version").or_else(|| root.get("name")))
            .ok_or_else(|| Error::Manifest("no `version`".into()))?
            .trim_start_matches('v')
            .to_owned();
        let parsed = Version::parse(&version)
            .ok_or_else(|| Error::Manifest(format!("`{version}` is not a version")))?;
        let asset = |value: &json::Value| -> Result<Asset, Error> {
            Ok(Asset {
                url: text_of(value.get("url"))
                    .ok_or_else(|| Error::Manifest("a package has no `url`".into()))?
                    .to_owned(),
                signature: text_of(value.get("signature"))
                    .ok_or_else(|| Error::Manifest("a package has no `signature`".into()))?
                    .to_owned(),
            })
        };
        let assets = match root.get("platforms") {
            Some(json::Value::Object(platforms)) => Assets::Static(
                platforms
                    .iter()
                    .map(|(key, value)| Ok((key.clone(), asset(value)?)))
                    .collect::<Result<_, Error>>()?,
            ),
            Some(_) => return Err(Error::Manifest("`platforms` is not an object".into())),
            None => Assets::Dynamic(asset(&root)?),
        };
        Ok(Self {
            version,
            parsed,
            notes: text_of(root.get("notes")).map(str::to_owned),
            date: text_of(root.get("pub_date")).map(str::to_owned),
            assets,
        })
    }

    /// Le paquet de la première clé présente (manifeste statique), ou celui du manifeste
    /// dynamique.
    pub(crate) fn asset(&self, keys: &[String]) -> Result<&Asset, Error> {
        match &self.assets {
            Assets::Dynamic(asset) => Ok(asset),
            Assets::Static(platforms) => keys
                .iter()
                .find_map(|key| platforms.iter().find(|(k, _)| k == key).map(|(_, a)| a))
                .ok_or_else(|| Error::NoPackage(keys.to_vec())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_manifests_choose_the_most_specific_package() {
        let release = Release::parse(
            r#"{"version":"v0.5.0","notes":"Notes","pub_date":"2026-12-01T00:00:00Z",
                "platforms":{"windows-x86_64":{"url":"https://x/setup.exe","signature":"AAA"},
                "windows-x86_64-msi":{"url":"https://x/a.msi","signature":"BBB"}}}"#,
        )
        .unwrap();
        assert_eq!(release.version, "0.5.0");
        assert_eq!(release.notes.as_deref(), Some("Notes"));
        assert_eq!(release.date.as_deref(), Some("2026-12-01T00:00:00Z"));
        let keys = |k: &[&str]| k.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let asset = release
            .asset(&keys(&["windows-x86_64-msi", "windows-x86_64"]))
            .unwrap();
        assert_eq!(asset.url, "https://x/a.msi");
        let asset = release
            .asset(&keys(&["windows-x86_64-nsis", "windows-x86_64"]))
            .unwrap();
        assert_eq!(asset.signature, "AAA");
        assert!(matches!(
            release.asset(&keys(&["linux-x86_64"])),
            Err(Error::NoPackage(_))
        ));
    }

    #[test]
    fn dynamic_manifests_carry_their_package() {
        let release =
            Release::parse(r#"{"name":"1.2.0","url":"https://x/p.tar.gz","signature":"S"}"#)
                .unwrap();
        assert_eq!(release.asset(&[]).unwrap().url, "https://x/p.tar.gz");
        assert!(Release::parse(r#"{"version":"1.2.0"}"#).is_err());
        assert!(Release::parse(r#"{"version":"latest","url":"u","signature":"s"}"#).is_err());
    }
}
