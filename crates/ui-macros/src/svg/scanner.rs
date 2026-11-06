//! Lecture des nombres SVG : séparateurs, exposants, nombres collés (`.5.5`,
//! `-1-1`) et drapeaux d'arc ; listes de nombres (`viewBox`, `points`, transformations).
// -- Données de chemin ---------------------------------------------------------
pub(super) struct Scanner<'a> {
    pub(super) bytes: &'a [u8],
    pub(super) at: usize,
}
impl<'a> Scanner<'a> {
    pub(super) fn new(text: &'a str) -> Self {
        Self {
            bytes: text.as_bytes(),
            at: 0,
        }
    }
    pub(super) fn done(&self) -> bool {
        self.at >= self.bytes.len()
    }
    pub(super) fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }
    pub(super) fn separators(&mut self) {
        while self
            .peek()
            .is_some_and(|b| b.is_ascii_whitespace() || b == b',')
        {
            self.at += 1;
        }
    }
    pub(super) fn starts_number(&self) -> bool {
        self.peek()
            .is_some_and(|b| b.is_ascii_digit() || matches!(b, b'-' | b'+' | b'.'))
    }
    pub(super) fn number(&mut self) -> std::result::Result<f64, String> {
        let start = self.at;
        if matches!(self.peek(), Some(b'-' | b'+')) {
            self.at += 1;
        }
        let mut digits = false;
        while self.peek().is_some_and(|b| b.is_ascii_digit()) {
            self.at += 1;
            digits = true;
        }
        if self.peek() == Some(b'.') {
            self.at += 1;
            while self.peek().is_some_and(|b| b.is_ascii_digit()) {
                self.at += 1;
                digits = true;
            }
        }
        if digits && matches!(self.peek(), Some(b'e' | b'E')) {
            let save = self.at;
            self.at += 1;
            if matches!(self.peek(), Some(b'-' | b'+')) {
                self.at += 1;
            }
            if self.peek().is_some_and(|b| b.is_ascii_digit()) {
                while self.peek().is_some_and(|b| b.is_ascii_digit()) {
                    self.at += 1;
                }
            } else {
                self.at = save;
            }
        }
        let text = std::str::from_utf8(&self.bytes[start..self.at]).unwrap_or("");
        if !digits {
            return Err(format!("Expected a number at offset {start}"));
        }
        text.parse::<f64>()
            .ok()
            .filter(|n| n.is_finite())
            .ok_or_else(|| format!("Invalid number '{text}'"))
    }
    /// Drapeau d'arc : `0` ou `1`, éventuellement collé au suivant (`a1 1 0 01…`).
    pub(super) fn flag(&mut self) -> std::result::Result<bool, String> {
        self.separators();
        match self.peek() {
            Some(b'0') => {
                self.at += 1;
                Ok(false)
            }
            Some(b'1') => {
                self.at += 1;
                Ok(true)
            }
            _ => Err(format!(
                "Expected an arc flag (0 or 1) at offset {}",
                self.at
            )),
        }
    }
}
// -- Transformations ---------------------------------------------------------
pub(super) fn numbers(text: &str) -> std::result::Result<Vec<f64>, String> {
    let mut scanner = Scanner::new(text);
    let mut out = Vec::new();
    loop {
        scanner.separators();
        if scanner.done() {
            return Ok(out);
        }
        out.push(scanner.number()?);
    }
}
