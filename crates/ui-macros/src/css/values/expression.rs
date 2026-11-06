//! Expressions numériques (`calc()`, `min()`, `max()`, `clamp()`, variables) traduites
//! en `Expression` / `ComputedLength` du moteur.
use super::*;
const EXPR: &str = "__ui_root::ui::layout::Expression";
struct Value {
    code: String,
    length: bool,
    scalar: Option<f32>,
}
struct Reader<'a> {
    vars: &'a BTreeMap<String, String>,
    stack: Vec<String>,
    text: &'a str,
    at: usize,
    nodes: usize,
}
impl<'a> Reader<'a> {
    fn variable(&mut self, depth: usize) -> Result<Value> {
        let start = self.at;
        let mut end = start;
        let mut nested = 1;
        for b in self.text[start..].bytes() {
            match b {
                b'(' => nested += 1,
                b')' => {
                    nested -= 1;
                    if nested == 0 {
                        break;
                    }
                }
                _ => {}
            }
            end += 1;
        }
        if nested != 0 {
            return Err("Unclosed var() in dimension".into());
        }
        let args = split(&self.text[start..end], ',')?;
        let name = args[0].trim();
        if !name.starts_with("--") || args.len() > 2 {
            return Err("Use var(--name, fallback)".into());
        }
        if self.stack.iter().any(|s| s == name) || self.stack.len() > 32 {
            return Err("Circular or deeply nested dimension variable".into());
        }
        let selected = self
            .vars
            .get(name)
            .map(String::as_str)
            .or_else(|| args.get(1).copied())
            .ok_or_else(|| format!("Unknown CSS variable '{name}'"))?;
        self.at = end + 1;
        let mut stack = self.stack.clone();
        stack.push(name.into());
        let mut reader = Reader {
            text: selected,
            at: 0,
            nodes: self.nodes,
            vars: self.vars,
            stack,
        };
        let fallback = reader.sum(depth + 1)?;
        reader.space();
        if reader.at != selected.len() {
            return Err("Invalid dimension variable default".into());
        }
        self.nodes = reader.nodes;
        Ok(Value {
            code: format!(
                "{EXPR}::Variable{{name:{name:?},fallback:&{},length:{}}}",
                fallback.code, fallback.length
            ),
            length: fallback.length,
            scalar: None,
        })
    }
    fn space(&mut self) {
        while self
            .text
            .as_bytes()
            .get(self.at)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.at += 1;
        }
    }
    fn take(&mut self, c: u8) -> bool {
        self.space();
        if self.text.as_bytes().get(self.at) == Some(&c) {
            self.at += 1;
            true
        } else {
            false
        }
    }
    fn sum(&mut self, depth: usize) -> Result<Value> {
        let mut a = self.product(depth)?;
        loop {
            let op = if self.take(b'+') {
                "Add"
            } else if self.take(b'-') {
                "Sub"
            } else {
                break;
            };
            let b = self.product(depth)?;
            if a.length != b.length {
                return Err("calc addition/subtraction requires compatible dimensions".into());
            }
            a = Value {
                code: format!("{EXPR}::{op}(&{},&{})", a.code, b.code),
                length: a.length,
                scalar: None,
            };
        }
        Ok(a)
    }
    fn product(&mut self, depth: usize) -> Result<Value> {
        let mut a = self.atom(depth)?;
        loop {
            let multiply = if self.take(b'*') {
                true
            } else if self.take(b'/') {
                false
            } else {
                break;
            };
            let b = self.atom(depth)?;
            if multiply && a.length && b.length || !multiply && b.length {
                return Err("Multiply a length by a number; divide by a nonzero number".into());
            }
            if !multiply && b.scalar == Some(0.) {
                return Err("Division by zero in calc".into());
            }
            let scalar = match (a.scalar, b.scalar) {
                (Some(a), Some(b)) => Some(if multiply { a * b } else { a / b }),
                _ => None,
            };
            a = Value {
                code: format!(
                    "{EXPR}::{}(&{},&{})",
                    if multiply { "Mul" } else { "Div" },
                    a.code,
                    b.code
                ),
                length: a.length || b.length,
                scalar,
            };
        }
        Ok(a)
    }
    fn atom(&mut self, depth: usize) -> Result<Value> {
        self.nodes += 1;
        if depth > 32 || self.nodes > 128 {
            return Err("Dimension expression is too complex".into());
        }
        self.space();
        if self.take(b'-') {
            let v = self.atom(depth + 1)?;
            return Ok(Value {
                code: format!("{EXPR}::Mul(&{EXPR}::Number(-1.),&{})", v.code),
                length: v.length,
                scalar: v.scalar.map(|v| -v),
            });
        }
        if self.take(b'+') {
            return self.atom(depth + 1);
        }
        if self.take(b'(') {
            let v = self.sum(depth + 1)?;
            if !self.take(b')') {
                return Err("Missing ) in expression".into());
            }
            return Ok(v);
        }
        let start = self.at;
        while self
            .text
            .as_bytes()
            .get(self.at)
            .is_some_and(u8::is_ascii_alphabetic)
        {
            self.at += 1;
        }
        if self.at > start {
            let name = &self.text[start..self.at];
            if !self.take(b'(') {
                return Err(format!("Expected function after '{name}'"));
            }
            if name == "var" {
                return self.variable(depth);
            }
            if name == "calc" {
                let v = self.sum(depth + 1)?;
                if !self.take(b')') {
                    return Err("Malformed calc()".into());
                }
                return Ok(v);
            }
            if !["min", "max", "clamp"].contains(&name) {
                return Err(format!("Unsupported dimension function '{name}'"));
            }
            let mut args = vec![self.sum(depth + 1)?];
            while self.take(b',') {
                args.push(self.sum(depth + 1)?);
            }
            if !self.take(b')') || args.iter().any(|v| v.length != args[0].length) {
                return Err("Malformed min/max/clamp or incompatible arguments".into());
            }
            if name == "clamp" && args.len() != 3 {
                return Err("clamp needs three arguments".into());
            }
            let code = if name == "clamp" {
                format!(
                    "{EXPR}::Clamp(&{},&{},&{})",
                    args[0].code, args[1].code, args[2].code
                )
            } else {
                format!(
                    "{EXPR}::{}(&[{}])",
                    if name == "min" { "Min" } else { "Max" },
                    args.iter()
                        .map(|v| v.code.as_str())
                        .collect::<Vec<_>>()
                        .join(",")
                )
            };
            return Ok(Value {
                code,
                length: args[0].length,
                scalar: None,
            });
        }
        while self
            .text
            .as_bytes()
            .get(self.at)
            .is_some_and(|b| b.is_ascii_digit() || *b == b'.')
        {
            self.at += 1;
        }
        if start == self.at {
            return Err("Expected a dimension in expression".into());
        }
        let n = number(&self.text[start..self.at])?;
        let unit_start = self.at;
        while self
            .text
            .as_bytes()
            .get(self.at)
            .is_some_and(|b| b.is_ascii_alphabetic() || *b == b'%')
        {
            self.at += 1;
        }
        let unit = &self.text[unit_start..self.at];
        let ty = match unit {
            "" => "Number",
            "px" => "Px",
            "%" => "Percent",
            "em" => "Em",
            "rem" => "Rem",
            "vw" => "Vw",
            "vh" => "Vh",
            _ => return Err(format!("Unsupported unit '{unit}'")),
        };
        Ok(Value {
            code: format!("{EXPR}::{ty}({n:?})"),
            length: !unit.is_empty(),
            scalar: if unit.is_empty() { Some(n) } else { None },
        })
    }
}
/// Expression numérique d'un canal de couleur (nombres, pourcentages, `var()`,
/// `calc()`), en code `Expression` sans enveloppe de longueur.
pub(super) fn parse_number_with_vars(
    text: &str,
    vars: &BTreeMap<String, String>,
) -> Result<String> {
    let mut r = Reader {
        vars,
        stack: Vec::new(),
        text,
        at: 0,
        nodes: 0,
    };
    let value = r.sum(0)?;
    r.space();
    if r.at != text.len() {
        return Err(format!(
            "Unexpected tokens in color channel: '{}'",
            &text[r.at..]
        ));
    }
    Ok(format!("const{{&{}}}", value.code))
}
pub(super) fn parse(text: &str) -> Result<String> {
    parse_with_vars(text, &BTreeMap::new())
}
pub(super) fn parse_with_vars(text: &str, vars: &BTreeMap<String, String>) -> Result<String> {
    let mut r = Reader {
        vars,
        stack: Vec::new(),
        text,
        at: 0,
        nodes: 0,
    };
    let mut value = r.sum(0)?;
    r.space();
    if r.at != text.len() {
        return Err(format!(
            "Unexpected tokens in dimension: '{}'",
            &text[r.at..]
        ));
    }
    if !value.length {
        if value.scalar == Some(0.) {
            value.code = format!("{EXPR}::Px(0.)");
        } else {
            return Err("A CSS dimension needs a length unit".into());
        }
    }
    Ok(format!(
        "__ui_root::ui::layout::ComputedLength::new(const{{&{}}})",
        value.code
    ))
}
