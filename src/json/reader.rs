//! Analyse d'un texte JSON (RFC 8259) : objets, tableaux, chaînes avec échappements et
//! paires de substitution UTF-16, nombres, `true`, `false`, `null`. Une marque d'ordre
//! des octets en tête est ignorée ; l'imbrication est bornée à 128 niveaux.
use super::Value;

const MAX_DEPTH: usize = 128;

/// Valeur du texte entier ; `None` s'il n'est pas un JSON valide.
pub(crate) fn parse(text: &str) -> Option<Value> {
    let mut reader = Reader {
        bytes: text.trim_start_matches('\u{feff}').as_bytes(),
        at: 0,
    };
    let value = reader.value(0)?;
    reader.skip_space();
    (reader.at == reader.bytes.len()).then_some(value)
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn skip_space(&mut self) {
        while self
            .bytes
            .get(self.at)
            .is_some_and(|b| b" \t\r\n".contains(b))
        {
            self.at += 1;
        }
    }
    fn eat(&mut self, byte: u8) -> bool {
        self.skip_space();
        if self.bytes.get(self.at) == Some(&byte) {
            self.at += 1;
            true
        } else {
            false
        }
    }
    fn expect(&mut self, byte: u8) -> Option<()> {
        self.eat(byte).then_some(())
    }
    fn word(&mut self, word: &[u8]) -> bool {
        if self.bytes[self.at..].starts_with(word) {
            self.at += word.len();
            true
        } else {
            false
        }
    }
    fn value(&mut self, depth: usize) -> Option<Value> {
        if depth > MAX_DEPTH {
            return None;
        }
        self.skip_space();
        match *self.bytes.get(self.at)? {
            b'{' => {
                self.at += 1;
                let mut members = Vec::new();
                if !self.eat(b'}') {
                    loop {
                        self.skip_space();
                        let key = self.string()?;
                        self.expect(b':')?;
                        members.push((key, self.value(depth + 1)?));
                        if self.eat(b'}') {
                            break;
                        }
                        self.expect(b',')?;
                    }
                }
                Some(Value::Object(members))
            }
            b'[' => {
                self.at += 1;
                let mut items = Vec::new();
                if !self.eat(b']') {
                    loop {
                        items.push(self.value(depth + 1)?);
                        if self.eat(b']') {
                            break;
                        }
                        self.expect(b',')?;
                    }
                }
                Some(Value::Array(items))
            }
            b'"' => self.string().map(Value::String),
            b't' => self.word(b"true").then_some(Value::Bool(true)),
            b'f' => self.word(b"false").then_some(Value::Bool(false)),
            b'n' => self.word(b"null").then_some(Value::Null),
            _ => self.number().map(Value::Number),
        }
    }
    fn number(&mut self) -> Option<f64> {
        let start = self.at;
        while self
            .bytes
            .get(self.at)
            .is_some_and(|b| b"+-.eE".contains(b) || b.is_ascii_digit())
        {
            self.at += 1;
        }
        let text = std::str::from_utf8(&self.bytes[start..self.at]).ok()?;
        // La grammaire JSON est plus stricte que `f64::from_str` : pas de `+` en tête,
        // pas de `.5` ni de `5.`, pas de zéro de tête.
        let digits = text.strip_prefix('-').unwrap_or(text);
        let first = digits.bytes().next()?;
        if !first.is_ascii_digit()
            || (first == b'0' && digits.as_bytes().get(1).is_some_and(u8::is_ascii_digit))
            || digits.contains(".e")
            || digits.contains(".E")
            || digits.ends_with('.')
        {
            return None;
        }
        text.parse().ok().filter(|n: &f64| n.is_finite())
    }
    fn hex4(&mut self) -> Option<u32> {
        let digits = std::str::from_utf8(self.bytes.get(self.at..self.at + 4)?).ok()?;
        self.at += 4;
        u32::from_str_radix(digits, 16).ok()
    }
    fn string(&mut self) -> Option<String> {
        if self.bytes.get(self.at) != Some(&b'"') {
            return None;
        }
        self.at += 1;
        let mut out = Vec::new();
        loop {
            let byte = *self.bytes.get(self.at)?;
            self.at += 1;
            match byte {
                b'"' => return String::from_utf8(out).ok(),
                b'\\' => {
                    let escape = *self.bytes.get(self.at)?;
                    self.at += 1;
                    let c = match escape {
                        b'"' => '"',
                        b'\\' => '\\',
                        b'/' => '/',
                        b'b' => '\u{8}',
                        b'f' => '\u{c}',
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'u' => {
                            let high = self.hex4()?;
                            let code = if (0xd800..0xdc00).contains(&high) {
                                // Paire de substitution UTF-16 : `🎵`.
                                if self.bytes.get(self.at..self.at + 2) != Some(b"\\u") {
                                    return None;
                                }
                                self.at += 2;
                                let low = self.hex4()?;
                                if !(0xdc00..0xe000).contains(&low) {
                                    return None;
                                }
                                0x10000 + ((high - 0xd800) << 10) + (low - 0xdc00)
                            } else {
                                high
                            };
                            char::from_u32(code)?
                        }
                        _ => return None,
                    };
                    let mut buffer = [0; 4];
                    out.extend_from_slice(c.encode_utf8(&mut buffer).as_bytes());
                }
                0..0x20 => return None,
                _ => out.push(byte),
            }
        }
    }
}
