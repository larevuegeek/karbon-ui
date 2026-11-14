//! Vérification d'un paquet : signature minisign (Ed25519) par la clé publique de
//! l'application, puis la version signée (commentaire de confiance `version:…`, que
//! `tauri signer` écrit) contre la version annoncée par le serveur, dont la réponse n'est
//! pas signée : sans ce contrôle, un serveur compromis pourrait annoncer une version neuve
//! et servir un ancien paquet, correctement signé.
use crate::{Error, version::Version};
use minisign_verify::{PublicKey, Signature};

/// Vérifie `data`. La clé et la signature sont acceptées sous trois formes : le fichier
/// minisign encodé en base64 (configuration de Tauri), le texte du fichier, ou la seule
/// ligne de données (clé `RW…`).
pub(crate) fn verify(
    data: &[u8],
    signature: &str,
    public_key: &str,
    announced: &str,
) -> Result<(), Error> {
    let key_text = minisign_text(public_key);
    let key = PublicKey::decode(&key_text)
        .or_else(|_| PublicKey::from_base64(key_text.trim()))
        .map_err(|e| Error::Signature(format!("public key: {e}")))?;
    let signature = Signature::decode(&minisign_text(signature))
        .map_err(|e| Error::Signature(format!("signature: {e}")))?;
    key.verify(data, &signature, true)
        .map_err(|e| Error::Signature(e.to_string()))?;
    // Lu seulement maintenant : la signature globale de minisign couvre ce commentaire.
    signed_version_matches(signature.trusted_comment(), announced)
}

/// La version du commentaire de confiance (`timestamp:…\tfile:…\tversion:0.5.0`) est celle
/// annoncée ; une signature sans version (signée par minisign lui-même) est acceptée.
fn signed_version_matches(trusted_comment: &str, announced: &str) -> Result<(), Error> {
    let signed = trusted_comment
        .split('\t')
        .find_map(|field| field.strip_prefix("version:"));
    match signed {
        Some(signed) if Version::parse(signed) != Version::parse(announced) => {
            Err(Error::Signature(format!(
                "signed for version {signed}, announced as {announced}"
            )))
        }
        _ => Ok(()),
    }
}

/// Texte minisign : décodé du base64 s'il l'est, sinon tel quel.
fn minisign_text(value: &str) -> String {
    let value = value.trim();
    if value.starts_with("untrusted comment:") {
        return value.to_owned();
    }
    base64(value)
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .filter(|text| text.starts_with("untrusted comment:"))
        .unwrap_or_else(|| value.to_owned())
}

/// Base64 standard (RFC 4648), remplissage facultatif, blancs ignorés.
pub(crate) fn base64(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() * 3 / 4);
    let (mut buffer, mut bits) = (0_u32, 0);
    for byte in text.bytes() {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' | b' ' | b'\n' | b'\r' | b'\t' => continue,
            _ => return None,
        };
        buffer = (buffer << 6) | u32::from(value);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Vecteur des tests de minisign-verify 0.2 : clé publique et signature du texte `test`.
    const KEY: &str = "untrusted comment: minisign public key E7620F1842B4E81F\n\
                       RWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3";
    const SIGNATURE: &str = "untrusted comment: signature from minisign secret key\n\
        RWQf6LRCGA9i59SLOFxz6NxvASXDJeRtuZykwQepbDEGt87ig1BNpWaVWuNrm73YiIiJbq71Wi+dP9eKL8OC351vwIasSSbXxwA=\n\
        trusted comment: timestamp:1555779966\tfile:test\n\
        QtKMXWyYcwdpZAlPF7tE2ENJkRd1ujvKjlj1m9RtHTBnZPa5WKU5uWRs5GoP5M/VqE81QFuMKI5k/SfNQUaOAA==";

    #[test]
    fn the_signed_version_must_be_the_announced_one() {
        let comment = "timestamp:1794571844\tfile:App_0.4.4_x64-setup.exe\tversion:0.4.4";
        assert!(signed_version_matches(comment, "0.4.4").is_ok());
        assert!(signed_version_matches(comment, "v0.4.4").is_ok());
        assert!(matches!(
            signed_version_matches(comment, "0.5.0"),
            Err(Error::Signature(_))
        ));
        // Signé par minisign, sans version : accepté.
        assert!(signed_version_matches("timestamp:1555779966\tfile:test", "9.9.9").is_ok());
    }

    #[test]
    fn base64_decodes_with_and_without_padding() {
        assert_eq!(base64("aGVsbG8=").unwrap(), b"hello");
        assert_eq!(base64("aGVsbG8").unwrap(), b"hello");
        assert_eq!(base64("aGVs\nbG8h").unwrap(), b"hello!");
        assert!(base64("a$b").is_none());
    }

    #[test]
    fn keys_and_signatures_are_accepted_in_every_form() {
        let encode = |text: &str| -> String {
            const TABLE: &[u8] =
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
            let bytes = text.as_bytes();
            let mut out = String::new();
            for chunk in bytes.chunks(3) {
                let n = (u32::from(chunk[0]) << 16)
                    | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
                    | u32::from(*chunk.get(2).unwrap_or(&0));
                for i in 0..4 {
                    if i <= chunk.len() {
                        out.push(TABLE[(n >> (18 - 6 * i)) as usize & 63] as char);
                    } else {
                        out.push('=');
                    }
                }
            }
            out
        };
        let data = b"test";
        assert!(verify(data, SIGNATURE, KEY, "1.0.0").is_ok());
        assert!(verify(data, &encode(SIGNATURE), &encode(KEY), "1.0.0").is_ok());
        let bare_key = KEY.lines().nth(1).unwrap().trim();
        assert!(verify(data, SIGNATURE, bare_key, "1.0.0").is_ok());
        assert!(matches!(
            verify(b"tampered", SIGNATURE, KEY, "1.0.0"),
            Err(Error::Signature(_))
        ));
        assert!(matches!(
            verify(data, SIGNATURE, "not a key", "1.0.0"),
            Err(Error::Signature(_))
        ));
    }
}
