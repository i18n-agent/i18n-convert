use i18n_convert::formats::*;

#[test]
fn registry_has_all_tier1_formats() {
    let registry = FormatRegistry::new();
    let ids = [
        "android-xml",
        "xcstrings",
        "ios-strings",
        "stringsdict",
        "arb",
        "json",
        "i18next",
        "xliff",
        "po",
        "yaml-rails",
    ];
    for id in ids {
        assert!(registry.get(id).is_some(), "Missing format: {id}");
    }
}

#[test]
fn registry_list_formats_returns_all() {
    let registry = FormatRegistry::new();
    let formats = registry.list();
    assert_eq!(formats.len(), 32);
}

#[test]
fn format_entry_has_name_and_extensions() {
    let registry = FormatRegistry::new();
    let android = registry.get("android-xml").unwrap();
    assert_eq!(android.name, "Android XML");
    assert!(android.extensions.contains(&".xml"));
}

#[test]
fn detect_android_xml_by_content() {
    let registry = FormatRegistry::new();
    let content = br#"<?xml version="1.0"?><resources><string name="a">b</string></resources>"#;
    let results = registry.detect(".xml", content);
    assert_eq!(results[0].0, "android-xml");
    assert_eq!(results[0].1, Confidence::Definite);
}

#[test]
fn detect_xliff_not_android_xml() {
    let registry = FormatRegistry::new();
    let content = br#"<?xml version="1.0"?><xliff version="1.2"><file></file></xliff>"#;
    let results = registry.detect(".xml", content);
    assert_eq!(results[0].0, "xliff");
    assert_eq!(results[0].1, Confidence::Definite);
}

#[test]
fn detect_arb_by_locale_field() {
    let registry = FormatRegistry::new();
    let content = br#"{"@@locale": "en", "greeting": "Hello"}"#;
    let results = registry.detect(".arb", content);
    assert_eq!(results[0].0, "arb");
}

#[test]
fn detect_i18next_vs_json() {
    let registry = FormatRegistry::new();
    let content = br#"{"common": {"greeting_one": "item", "greeting_other": "items"}}"#;
    let results = registry.detect(".json", content);
    // i18next should rank higher than structured json due to _one/_other suffixes
    assert!(results.iter().any(|(id, _)| *id == "i18next"));
}

#[test]
fn detect_unknown_returns_empty() {
    let registry = FormatRegistry::new();
    let results = registry.detect(".xyz", b"random content");
    assert!(results.is_empty());
}

#[test]
fn detect_po_by_extension() {
    let registry = FormatRegistry::new();
    let results = registry.detect(".po", b"");
    assert_eq!(results[0].0, "po");
    assert_eq!(results[0].1, Confidence::Definite);
}

#[test]
fn detect_yaml_by_extension() {
    let registry = FormatRegistry::new();
    let content = b"en:\n  greeting: Hello";
    let results = registry.detect(".yml", content);
    assert!(results.iter().any(|(id, _)| *id == "yaml-rails"));
}

#[test]
fn detect_xcstrings_by_extension() {
    let registry = FormatRegistry::new();
    let results = registry.detect(".xcstrings", b"{}");
    assert_eq!(results[0].0, "xcstrings");
    assert_eq!(results[0].1, Confidence::Definite);
}

#[test]
fn detect_ios_strings_by_extension() {
    let registry = FormatRegistry::new();
    let results = registry.detect(".strings", b"");
    assert_eq!(results[0].0, "ios-strings");
    assert_eq!(results[0].1, Confidence::Definite);
}

#[test]
fn detect_stringsdict_by_extension() {
    let registry = FormatRegistry::new();
    let results = registry.detect(".stringsdict", b"");
    assert_eq!(results[0].0, "stringsdict");
    assert_eq!(results[0].1, Confidence::Definite);
}

#[test]
fn capabilities_android_xml_supports_plurals_and_arrays() {
    let registry = FormatRegistry::new();
    let android = registry.get("android-xml").unwrap();
    let caps = android.parser.capabilities();
    assert!(caps.plurals);
    assert!(caps.arrays);
    assert!(caps.comments);
    assert!(caps.translatable_flag);
    assert!(!caps.source_string);
    assert!(!caps.context);
}

#[test]
fn capabilities_xliff_supports_bilingual() {
    let registry = FormatRegistry::new();
    let xliff = registry.get("xliff").unwrap();
    let caps = xliff.parser.capabilities();
    assert!(caps.source_string);
    assert!(caps.translation_state);
    assert!(caps.comments);
    assert!(caps.context);
    assert!(caps.max_width);
    assert!(caps.alternatives);
}

#[test]
fn capabilities_ios_strings_minimal() {
    let registry = FormatRegistry::new();
    let ios = registry.get("ios-strings").unwrap();
    let caps = ios.parser.capabilities();
    assert!(!caps.plurals);
    assert!(!caps.arrays);
    assert!(caps.comments);
    assert!(!caps.context);
    assert!(!caps.source_string);
}

#[test]
fn parser_and_writer_capabilities_match() {
    let registry = FormatRegistry::new();
    for entry in registry.list() {
        let parser_caps = entry.parser.capabilities();
        let writer_caps = entry.writer.capabilities();
        assert_eq!(
            parser_caps, writer_caps,
            "Parser and writer capabilities mismatch for format: {}",
            entry.id
        );
    }
}
