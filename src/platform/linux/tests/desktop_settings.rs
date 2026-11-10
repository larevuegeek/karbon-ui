#[test]
fn desktop_font_settings_name_families_without_size_or_style() {
    use super::super::ffi::pango::family_of;
    assert_eq!(family_of("Inter 10").as_deref(), Some("Inter"));
    assert_eq!(family_of("Cantarell Bold 11").as_deref(), Some("Cantarell"));
    assert_eq!(
        family_of("Noto Sans CJK JP, 10").as_deref(),
        Some("Noto Sans CJK JP")
    );
    assert_eq!(family_of("").as_deref(), None);
}
