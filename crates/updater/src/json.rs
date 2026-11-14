//! Lecteur JSON minimal pour le manifeste d'une version (objets, tableaux, chaînes avec
//! leurs échappements, nombres, booléens, `null`). Le manifeste est petit et sa forme
//! connue : pas besoin de plus.

/// Valeur JSON lue.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Value>),
    /// Membres dans l'ordre du texte.
    Object(Vec<(String, Value)>),
}

impl Value {
    /// Membre `key` d'un objet.
    pub(crate) fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Self::Object(members) => members.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
    pub(crate) fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(text) => Some(text),
            _ => None,
        }
    }
}

/// Lit un document JSON complet ; le texte de l'erreur sinon.
pub(crate) fn parse(text: &str) -> Result<Value, String> {
    let mut reader = Reader {
        bytes: text.as_bytes(),
        at: 0,
    };
    let value = reader.value(0)?;
    reader.blank();
    if reader.at != reader.bytes.len() {
        return Err(format!("unexpected text at byte {}", reader.at));
    }
    Ok(value)
}

/// Profondeur maximale d'imbrication (un manifeste en a trois).
const DEPTH: usize = 64;

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn blank(&mut self) {
        while self
            .bytes
            .get(self.at)
            .is_some_and(|b| matches!(b, b' ' | b'\t' | b'\n' | b'\r'))
        {
            self.at += 1;
        }
    }
    fn expect(&mut self, byte: u8) -> Result<(), String> {
        self.blank();
        if self.bytes.get(self.at) == Some(&byte) {
            self.at += 1;
            Ok(())
        } else {
            Err(format!("expected `{}` at byte {}", byte as char, self.at))
        }
    }
    fn value(&mut self, depth: usize) -> Result<Value, String> {
        if depth > DEPTH {
            return Err("nesting too deep".into());
        }
        self.blank();
        match self.bytes.get(self.at) {
            Some(b'{') => self.object(depth),
            Some(b'[') => self.array(depth),
            Some(b'"') => self.string().map(Value::String),
            Some(b't') => self.word("true", Value::Bool(true)),
            Some(b'f') => self.word("false", Value::Bool(false)),
            Some(b'n') => self.word("null", Value::Null),
            Some(b'-' | b'0'..=b'9') => self.number(),
            _ => Err(format!("unexpected character at byte {}", self.at)),
        }
    }
    fn word(&mut self, word: &str, value: Value) -> Result<Value, String> {
        if self.bytes[self.at..].starts_with(word.as_bytes()) {
            self.at += word.len();
            Ok(value)
        } else {
            Err(format!("unexpected word at byte {}", self.at))
        }
    }
    fn number(&mut self) -> Result<Value, String> {
        let start = self.at;
        while self
            .bytes
            .get(self.at)
            .is_some_and(|b| matches!(b, b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9'))
        {
            self.at += 1;
        }
        std::str::from_utf8(&self.bytes[start..self.at])
            .ok()
            .and_then(|text| text.parse().ok())
            .map(Value::Number)
            .ok_or_else(|| format!("invalid number at byte {start}"))
    }
    fn array(&mut self, depth: usize) -> Result<Value, String> {
        self.expect(b'[')?;
        let mut items = Vec::new();
        self.blank();
        if self.bytes.get(self.at) == Some(&b']') {
            self.at += 1;
            return Ok(Value::Array(items));
        }
        loop {
            items.push(self.value(depth + 1)?);
            self.blank();
            match self.bytes.get(self.at) {
                Some(b',') => self.at += 1,
                Some(b']') => {
                    self.at += 1;
                    return Ok(Value::Array(items));
                }
                _ => return Err(format!("expected `,` or `]` at byte {}", self.at)),
            }
        }
    }
    fn object(&mut self, depth: usize) -> Result<Value, String> {
        self.expect(b'{')?;
        let mut members = Vec::new();
        self.blank();
        if self.bytes.get(self.at) == Some(&b'}') {
            self.at += 1;
            return Ok(Value::Object(members));
        }
        loop {
            self.blank();
            let key = self.string()?;
            self.expect(b':')?;
            members.push((key, self.value(depth + 1)?));
            self.blank();
            match self.bytes.get(self.at) {
                Some(b',') => self.at += 1,
                Some(b'}') => {
                    self.at += 1;
                    return Ok(Value::Object(members));
                }
                _ => return Err(format!("expected `,` or `}}` at byte {}", self.at)),
            }
        }
    }
    fn string(&mut self) -> Result<String, String> {
        self.expect(b'"')?;
        let mut out = Vec::new();
        loop {
            let Some(&byte) = self.bytes.get(self.at) else {
                return Err("unterminated string".into());
            };
            self.at += 1;
            match byte {
                b'"' => return String::from_utf8(out).map_err(|_| "invalid UTF-8".into()),
                b'\\' => {
                    let Some(&escape) = self.bytes.get(self.at) else {
                        return Err("unterminated escape".into());
                    };
                    self.at += 1;
                    let character = match escape {
                        b'"' => '"',
                        b'\\' => '\\',
                        b'/' => '/',
                        b'b' => '\u{8}',
                        b'f' => '\u{c}',
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'u' => self.unicode()?,
                        _ => return Err(format!("invalid escape at byte {}", self.at)),
                    };
                    out.extend_from_slice(character.encode_utf8(&mut [0; 4]).as_bytes());
                }
                _ => out.push(byte),
            }
        }
    }
    /// `\uXXXX`, paire de demi-codets comprise.
    fn unicode(&mut self) -> Result<char, String> {
        let first = self.hex4()?;
        let code = if (0xD800..0xDC00).contains(&first) {
            if self.bytes[self.at..].starts_with(b"\\u") {
                self.at += 2;
                let second = self.hex4()?;
                0x10000 + ((first - 0xD800) << 10) + (second.wrapping_sub(0xDC00) & 0x3FF)
            } else {
                0xFFFD
            }
        } else {
            first
        };
        Ok(char::from_u32(code).unwrap_or('\u{FFFD}'))
    }
    fn hex4(&mut self) -> Result<u32, String> {
        let digits = self
            .bytes
            .get(self.at..self.at + 4)
            .and_then(|d| std::str::from_utf8(d).ok())
            .and_then(|d| u32::from_str_radix(d, 16).ok())
            .ok_or_else(|| format!("invalid \\u escape at byte {}", self.at))?;
        self.at += 4;
        Ok(digits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_release_manifest() {
        let value = parse(
            r#"{"version":"0.4.4","notes":"Ligne\n« guillemets » été 🎵",
                "pub_date":"2026-11-13T00:00:00Z","n":-1.5e2,"ok":true,"none":null,
                "platforms":{"windows-x86_64":{"url":"https://x/a.exe","signature":"c2ln"}},
                "list":[1,[],{}]}"#,
        )
        .unwrap();
        assert_eq!(value.get("version").and_then(Value::as_str), Some("0.4.4"));
        assert_eq!(
            value.get("notes").and_then(Value::as_str),
            Some("Ligne\n« guillemets » été 🎵")
        );
        assert_eq!(value.get("n"), Some(&Value::Number(-150.)));
        assert_eq!(value.get("ok"), Some(&Value::Bool(true)));
        assert_eq!(value.get("none"), Some(&Value::Null));
        let platform = value.get("platforms").and_then(|p| p.get("windows-x86_64"));
        assert_eq!(
            platform.and_then(|p| p.get("url")).and_then(Value::as_str),
            Some("https://x/a.exe")
        );
    }

    #[test]
    fn rejects_malformed_documents() {
        for text in [
            r#"{"a":}"#,
            r#"{"a":1"#,
            r#"["a" "b"]"#,
            r#""\x""#,
            "tru",
            "{} x",
        ] {
            assert!(parse(text).is_err(), "{text}");
        }
        let deep = "[".repeat(100) + &"]".repeat(100);
        assert!(parse(&deep).is_err());
    }
}
