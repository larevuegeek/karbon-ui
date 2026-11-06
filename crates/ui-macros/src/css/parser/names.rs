//! Noms de classes CSS et identifiants Rust des fonctions de style générées.
pub fn class_name(value: &str) -> std::result::Result<String, String> {
    let mut chars = value.chars();
    if !chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        || !chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(format!(
            "Unsupported class name '{value}'; use letters, digits, _ and -"
        ));
    }
    Ok(value.replace('-', "_").to_ascii_uppercase())
}
pub fn class_symbol(value: &str) -> std::result::Result<String, String> {
    class_name(value)?;
    Ok(format!(
        "__class_{}",
        value
            .bytes()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    ))
}
