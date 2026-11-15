//! Lecture : types, imbrication, échappements, et refus des textes invalides.
use super::{Value, parse};

#[test]
fn reads_every_kind_of_value() {
    let value = parse(
        "\u{feff}{\"a\": {\"b\": [1, -2.5e3, true, false, null]}, \"s\": \"\\u00e9\\ud83c\\udfb5\\t\"}",
    )
    .unwrap();
    assert_eq!(
        value.get("a").and_then(|a| a.get("b")),
        Some(&Value::Array(vec![
            Value::Number(1.),
            Value::Number(-2500.),
            Value::Bool(true),
            Value::Bool(false),
            Value::Null,
        ]))
    );
    assert_eq!(value.get("s").and_then(Value::as_str), Some("é🎵\t"));
    assert_eq!(parse(" [] "), Some(Value::Array(Vec::new())));
    assert_eq!(parse("{}"), Some(Value::Object(Vec::new())));
}

#[test]
fn a_repeated_key_keeps_its_last_value() {
    let value = parse("{\"k\": \"1\", \"k\": \"2\"}").unwrap();
    assert_eq!(value.get("k").and_then(Value::as_str), Some("2"));
}

#[test]
fn rejects_invalid_texts() {
    for text in [
        "",
        "{",
        "{\"a\": }",
        "[1,]",
        "{\"a\" 1}",
        "01",
        ".5",
        "5.",
        "+1",
        "tru",
        "\"\n\"",
        "\"\\x\"",
        "\"\\ud83c\"",
        "{} x",
        "nan",
    ] {
        assert_eq!(parse(text), None, "{text:?}");
    }
    let deep = "[".repeat(200) + &"]".repeat(200);
    assert_eq!(parse(&deep), None);
}
