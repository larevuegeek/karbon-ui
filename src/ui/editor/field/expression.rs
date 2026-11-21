//! Petites expressions tapées dans un champ numérique : nombres (point ou virgule
//! décimale), `+ - * /`, parenthèses et signe ; « 2*3 », « 1,5 + 0.25 », « -(4/2) ».

/// Value of `text`, or `None` when it is not a valid expression. A unit written after
/// the number (`"12 m"`, `"90°"`) is ignored when it is `unit`.
pub(super) fn evaluate(text: &str, unit: &str) -> Option<f64> {
    let (text, unit) = (text.trim(), unit.trim());
    let text = text
        .strip_suffix(unit)
        .filter(|_| !unit.is_empty())
        .unwrap_or(text);
    let mut parser = Parser {
        bytes: text.as_bytes(),
        at: 0,
        depth: 0,
    };
    let value = parser.sum()?;
    parser.skip_spaces();
    (parser.at == parser.bytes.len() && value.is_finite()).then_some(value)
}

struct Parser<'a> {
    bytes: &'a [u8],
    at: usize,
    depth: u32,
}

impl Parser<'_> {
    fn skip_spaces(&mut self) {
        while self.bytes.get(self.at).is_some_and(u8::is_ascii_whitespace) {
            self.at += 1;
        }
    }

    fn eat(&mut self, byte: u8) -> bool {
        self.skip_spaces();
        let found = self.bytes.get(self.at) == Some(&byte);
        if found {
            self.at += 1;
        }
        found
    }

    /// Somme et différence de produits.
    fn sum(&mut self) -> Option<f64> {
        let mut value = self.product()?;
        loop {
            if self.eat(b'+') {
                value += self.product()?;
            } else if self.eat(b'-') {
                value -= self.product()?;
            } else {
                return Some(value);
            }
        }
    }

    /// Produit et quotient de facteurs.
    fn product(&mut self) -> Option<f64> {
        let mut value = self.factor()?;
        loop {
            if self.eat(b'*') {
                value *= self.factor()?;
            } else if self.eat(b'/') {
                value /= self.factor()?;
            } else {
                return Some(value);
            }
        }
    }

    /// Nombre, expression entre parenthèses, ou facteur précédé d'un signe.
    fn factor(&mut self) -> Option<f64> {
        if self.eat(b'-') {
            return self.factor().map(|v| -v);
        }
        if self.eat(b'+') {
            return self.factor();
        }
        if self.eat(b'(') {
            // Profondeur bornée : une suite de « ( » ne fait pas déborder la pile.
            self.depth += 1;
            if self.depth > 64 {
                return None;
            }
            let value = self.sum()?;
            self.depth -= 1;
            return self.eat(b')').then_some(value);
        }
        self.number()
    }

    fn number(&mut self) -> Option<f64> {
        self.skip_spaces();
        let start = self.at;
        while self
            .bytes
            .get(self.at)
            .is_some_and(|b| b.is_ascii_digit() || *b == b'.' || *b == b',')
        {
            self.at += 1;
        }
        let digits = std::str::from_utf8(&self.bytes[start..self.at]).ok()?;
        digits.replace(',', ".").parse().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::evaluate;

    #[test]
    fn simple_expressions_are_evaluated() {
        assert_eq!(evaluate("2*3", ""), Some(6.));
        assert_eq!(evaluate(" 1,5 + 0.25 ", ""), Some(1.75));
        assert_eq!(evaluate("-(4/2)", ""), Some(-2.));
        assert_eq!(evaluate("2+3*4", ""), Some(14.));
        assert_eq!(evaluate("(2+3)*4", ""), Some(20.));
        assert_eq!(evaluate("12 m", " m"), Some(12.));
        assert_eq!(evaluate("12m", " m"), Some(12.));
        assert_eq!(evaluate("90°", "°"), Some(90.));
        for wrong in ["", "2*", "abc", "1/0", "((1)", "1.2.3", "2 3"] {
            assert_eq!(evaluate(wrong, ""), None, "{wrong}");
        }
        assert_eq!(evaluate(&"(".repeat(100), ""), None);
    }
}
