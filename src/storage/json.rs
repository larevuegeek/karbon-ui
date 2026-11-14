//! Le fichier d'un `LocalStorage` : un objet JSON de chaînes (`{"clé": "valeur"}`), lu et
//! écrit à la main, au format qu'écrivent les navigateurs et `serde_json` (indentation de
//! deux espaces), pour qu'un fichier existant se relise tel quel.
use std::collections::BTreeMap;

/// Objet `{"clé": "valeur", …}` ; `None` si le texte n'est pas un tel objet.
pub(super) fn parse(text: &str) -> Option<BTreeMap<String, String>> {
    let mut reader = Reader {
        bytes: text.trim_start_matches('\u{feff}').as_bytes(),
        at: 0,
    };
    let mut map = BTreeMap::new();
    reader.expect(b'{')?;
    if !reader.eat(b'}') {
        loop {
            let key = reader.string()?;
            reader.expect(b':')?;
            let value = reader.string()?;
            map.insert(key, value);
            if reader.eat(b'}') {
                break;
            }
            reader.expect(b',')?;
        }
    }
    reader.skip_space();
    (reader.at == reader.bytes.len()).then_some(map)
}

/// Objet indenté de deux espaces, clés triées.
pub(super) fn write(map: &BTreeMap<String, String>) -> String {
    if map.is_empty() {
        return "{}".into();
    }
    let mut out = String::from("{\n");
    for (i, (key, value)) in map.iter().enumerate() {
        out += "  ";
        quote(&mut out, key);
        out += ": ";
        quote(&mut out, value);
        out += if i + 1 < map.len() { ",\n" } else { "\n" };
    }
    out.push('}');
    out
}

fn quote(out: &mut String, text: &str) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
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
    fn hex4(&mut self) -> Option<u32> {
        let digits = std::str::from_utf8(self.bytes.get(self.at..self.at + 4)?).ok()?;
        self.at += 4;
        u32::from_str_radix(digits, 16).ok()
    }
    fn string(&mut self) -> Option<String> {
        self.expect(b'"')?;
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
                                // Paire de substitution UTF-16 : `😀`.
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
