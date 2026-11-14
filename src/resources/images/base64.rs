//! Décodage base64 (alphabet standard, `=` final facultatif, blancs ignorés) des adresses
//! `data:image/…;base64,…`.

/// Octets de `text`, ou `None` si ce n'est pas du base64 valide.
pub(super) fn decode(text: &str) -> Option<Vec<u8>> {
    let value = |c: u8| -> Option<u32> {
        Some(u32::from(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        }))
    };
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let (mut buffer, mut bits) = (0u32, 0u32);
    let mut padding = false;
    for c in text.bytes() {
        if c.is_ascii_whitespace() {
            continue;
        }
        if c == b'=' {
            padding = true;
            continue;
        }
        if padding {
            return None;
        }
        buffer = (buffer << 6) | value(c)?;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }
    // Un seul caractère restant ne porte pas un octet entier.
    (bits < 6).then_some(out)
}

#[cfg(test)]
mod tests {
    #[test]
    fn decodes_standard_base64() {
        assert_eq!(super::decode("aGVsbG8=").unwrap(), b"hello");
        assert_eq!(super::decode("aGVsbG8").unwrap(), b"hello");
        assert_eq!(super::decode("aGVs\nbG8h").unwrap(), b"hello!");
        assert_eq!(super::decode("").unwrap(), b"");
        assert!(super::decode("a$b=").is_none());
        assert!(super::decode("a").is_none());
    }
}
