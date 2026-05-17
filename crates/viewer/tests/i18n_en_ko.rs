#[path = "../src/i18n.rs"]
mod i18n;

#[test]
fn i18n_table_has_en_ko_parity() {
    let keys: Vec<_> = i18n::keys().collect();
    assert!(
        keys.len() >= 200,
        "expected at least 200 keys, got {}",
        keys.len()
    );
    assert_eq!(i18n::Language::ALL.len(), 2);
    assert_eq!(i18n::Language::Ko.label(), "Korean");
    assert!(i18n::missing_for(i18n::Language::En).is_empty());
    assert!(i18n::missing_for(i18n::Language::Ko).is_empty());
}

#[test]
fn i18n_lookup_returns_requested_language() {
    assert_eq!(i18n::t("menu.file"), "File");
    assert_eq!(i18n::translate(i18n::Language::Ko, "menu.file"), "파일");
    assert!(i18n::has_key("dialog.autosave.recover"));
}
