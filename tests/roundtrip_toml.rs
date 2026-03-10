use i18n_convert::formats::toml_format;
use i18n_convert::formats::{Confidence, FormatParser, FormatWriter};
use i18n_convert::ir::*;
use indexmap::IndexMap;

fn parser() -> toml_format::Parser {
    toml_format::Parser
}

fn writer() -> toml_format::Writer {
    toml_format::Writer
}

fn fixture(name: &str) -> Vec<u8> {
    let path = format!("{}/tests/fixtures/toml/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read(&path).unwrap_or_else(|e| panic!("Failed to read fixture {path}: {e}"))
}

// ──────────────────────────────────────────────
// Detection tests
// ──────────────────────────────────────────────

#[test]
fn detect_toml_extension_valid_content() {
    let content = b"[messages]\ngreeting = \"Hello\"\n";
    assert_eq!(parser().detect(".toml", content), Confidence::High);
}

#[test]
fn detect_toml_extension_invalid_content() {
    let content = b"this is not valid toml {{{";
    assert_eq!(parser().detect(".toml", content), Confidence::Low);
}

#[test]
fn detect_non_toml_extension() {
    let content = b"[messages]\ngreeting = \"Hello\"\n";
    assert_eq!(parser().detect(".json", content), Confidence::None);
    assert_eq!(parser().detect(".yaml", content), Confidence::None);
    assert_eq!(parser().detect(".xml", content), Confidence::None);
}

// ──────────────────────────────────────────────
// Parsing tests
// ──────────────────────────────────────────────

#[test]
fn parse_flat_toml() {
    let content = fixture("flat.toml");
    let resource = parser().parse(&content).expect("parse should succeed");

    assert_eq!(resource.metadata.source_format, FormatId::Toml);
    assert_eq!(resource.entries.len(), 5);

    assert_eq!(
        resource.entries["greeting"].value,
        EntryValue::Simple("Hello".to_string())
    );
    assert_eq!(
        resource.entries["farewell"].value,
        EntryValue::Simple("Goodbye".to_string())
    );
    assert_eq!(
        resource.entries["app_name"].value,
        EntryValue::Simple("My Application".to_string())
    );
    assert_eq!(
        resource.entries["welcome_message"].value,
        EntryValue::Simple("Welcome to our app!".to_string())
    );
    assert_eq!(
        resource.entries["empty_value"].value,
        EntryValue::Simple("".to_string())
    );
}

#[test]
fn parse_nested_toml() {
    let content = fixture("nested.toml");
    let resource = parser().parse(&content).expect("parse should succeed");

    assert_eq!(resource.entries.len(), 5);

    assert_eq!(
        resource.entries["messages.greeting"].value,
        EntryValue::Simple("Hello, World!".to_string())
    );
    assert_eq!(
        resource.entries["messages.farewell"].value,
        EntryValue::Simple("Goodbye!".to_string())
    );
    assert_eq!(
        resource.entries["messages.nested.welcome"].value,
        EntryValue::Simple("Welcome to our app".to_string())
    );
    assert_eq!(
        resource.entries["errors.not_found"].value,
        EntryValue::Simple("Page not found".to_string())
    );
    assert_eq!(
        resource.entries["errors.server_error"].value,
        EntryValue::Simple("Internal server error".to_string())
    );
}

#[test]
fn parse_arrays_toml() {
    let content = fixture("arrays.toml");
    let resource = parser().parse(&content).expect("parse should succeed");

    assert_eq!(resource.entries.len(), 3);

    assert_eq!(
        resource.entries["colors"].value,
        EntryValue::Array(vec![
            "red".to_string(),
            "green".to_string(),
            "blue".to_string(),
        ])
    );
    assert_eq!(
        resource.entries["sizes"].value,
        EntryValue::Array(vec![
            "small".to_string(),
            "medium".to_string(),
            "large".to_string(),
        ])
    );
    assert_eq!(
        resource.entries["navigation.menu_items"].value,
        EntryValue::Array(vec![
            "Home".to_string(),
            "About".to_string(),
            "Contact".to_string(),
        ])
    );
}

#[test]
fn parse_empty_toml() {
    let content = b"";
    let resource = parser().parse(content).expect("parse should succeed");
    assert_eq!(resource.entries.len(), 0);
    assert_eq!(resource.metadata.source_format, FormatId::Toml);
}

#[test]
fn parse_invalid_toml_returns_error() {
    let content = b"this is not valid [toml {{{";
    let result = parser().parse(content);
    assert!(result.is_err());
}

#[test]
fn parse_numeric_values() {
    let content = b"version = 42\npi = 3.14\nenabled = true\n";
    let resource = parser().parse(content).expect("parse should succeed");

    assert_eq!(resource.entries.len(), 3);
    assert_eq!(
        resource.entries["version"].value,
        EntryValue::Simple("42".to_string())
    );
    assert_eq!(
        resource.entries["pi"].value,
        EntryValue::Simple("3.14".to_string())
    );
    assert_eq!(
        resource.entries["enabled"].value,
        EntryValue::Simple("true".to_string())
    );
}

// ──────────────────────────────────────────────
// Writing tests
// ──────────────────────────────────────────────

#[test]
fn write_flat_entries() {
    let mut entries = IndexMap::new();
    entries.insert(
        "greeting".to_string(),
        I18nEntry {
            key: "greeting".to_string(),
            value: EntryValue::Simple("Hello".to_string()),
            ..Default::default()
        },
    );
    entries.insert(
        "farewell".to_string(),
        I18nEntry {
            key: "farewell".to_string(),
            value: EntryValue::Simple("Goodbye".to_string()),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Toml,
            ..Default::default()
        },
        entries,
    };

    let output = writer().write(&resource).expect("write should succeed");
    let output_str = std::str::from_utf8(&output).expect("valid UTF-8");

    let parsed: toml::Value = toml::from_str(output_str).expect("valid TOML output");
    assert_eq!(parsed["greeting"].as_str(), Some("Hello"));
    assert_eq!(parsed["farewell"].as_str(), Some("Goodbye"));
}

#[test]
fn write_nested_entries() {
    let mut entries = IndexMap::new();
    entries.insert(
        "messages.greeting".to_string(),
        I18nEntry {
            key: "messages.greeting".to_string(),
            value: EntryValue::Simple("Hello".to_string()),
            ..Default::default()
        },
    );
    entries.insert(
        "messages.farewell".to_string(),
        I18nEntry {
            key: "messages.farewell".to_string(),
            value: EntryValue::Simple("Goodbye".to_string()),
            ..Default::default()
        },
    );
    entries.insert(
        "errors.not_found".to_string(),
        I18nEntry {
            key: "errors.not_found".to_string(),
            value: EntryValue::Simple("Not found".to_string()),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Toml,
            ..Default::default()
        },
        entries,
    };

    let output = writer().write(&resource).expect("write should succeed");
    let output_str = std::str::from_utf8(&output).expect("valid UTF-8");

    let parsed: toml::Value = toml::from_str(output_str).expect("valid TOML output");
    assert_eq!(parsed["messages"]["greeting"].as_str(), Some("Hello"));
    assert_eq!(parsed["messages"]["farewell"].as_str(), Some("Goodbye"));
    assert_eq!(parsed["errors"]["not_found"].as_str(), Some("Not found"));
}

#[test]
fn write_array_entries() {
    let mut entries = IndexMap::new();
    entries.insert(
        "colors".to_string(),
        I18nEntry {
            key: "colors".to_string(),
            value: EntryValue::Array(vec![
                "red".to_string(),
                "green".to_string(),
                "blue".to_string(),
            ]),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Toml,
            ..Default::default()
        },
        entries,
    };

    let output = writer().write(&resource).expect("write should succeed");
    let output_str = std::str::from_utf8(&output).expect("valid UTF-8");

    let parsed: toml::Value = toml::from_str(output_str).expect("valid TOML output");
    let arr = parsed["colors"].as_array().expect("should be array");
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0].as_str(), Some("red"));
    assert_eq!(arr[1].as_str(), Some("green"));
    assert_eq!(arr[2].as_str(), Some("blue"));
}

#[test]
fn write_empty_resource() {
    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Toml,
            ..Default::default()
        },
        entries: IndexMap::new(),
    };

    let output = writer().write(&resource).expect("write should succeed");
    let output_str = std::str::from_utf8(&output).expect("valid UTF-8");
    // Empty TOML should still be valid
    let parsed: toml::Value = toml::from_str(output_str).expect("valid TOML output");
    assert!(parsed.as_table().expect("should be table").is_empty());
}

#[test]
fn write_output_ends_with_newline() {
    let resource = I18nResource {
        metadata: ResourceMetadata::default(),
        entries: IndexMap::new(),
    };
    let output = writer().write(&resource).expect("write should succeed");
    assert!(output.ends_with(b"\n"));
}

// ──────────────────────────────────────────────
// Round-trip tests
// ──────────────────────────────────────────────

#[test]
fn roundtrip_flat_toml() {
    let content = fixture("flat.toml");
    let resource = parser().parse(&content).expect("parse should succeed");
    let output = writer().write(&resource).expect("write should succeed");
    let reparsed = parser().parse(&output).expect("reparse should succeed");

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, entry) in &resource.entries {
        let reparsed_entry = &reparsed.entries[key];
        assert_eq!(entry.value, reparsed_entry.value, "Mismatch for key: {key}");
    }
}

#[test]
fn roundtrip_nested_toml() {
    let content = fixture("nested.toml");
    let resource = parser().parse(&content).expect("parse should succeed");
    let output = writer().write(&resource).expect("write should succeed");
    let reparsed = parser().parse(&output).expect("reparse should succeed");

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, entry) in &resource.entries {
        let reparsed_entry = &reparsed.entries[key];
        assert_eq!(entry.value, reparsed_entry.value, "Mismatch for key: {key}");
    }
}

#[test]
fn roundtrip_arrays_toml() {
    let content = fixture("arrays.toml");
    let resource = parser().parse(&content).expect("parse should succeed");
    let output = writer().write(&resource).expect("write should succeed");
    let reparsed = parser().parse(&output).expect("reparse should succeed");

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, entry) in &resource.entries {
        let reparsed_entry = &reparsed.entries[key];
        assert_eq!(entry.value, reparsed_entry.value, "Mismatch for key: {key}");
    }
}

#[test]
fn roundtrip_preserves_nested_structure() {
    let content = fixture("nested.toml");
    let resource = parser().parse(&content).expect("parse should succeed");
    let output = writer().write(&resource).expect("write should succeed");
    let output_str = std::str::from_utf8(&output).expect("valid UTF-8");

    let written: toml::Value = toml::from_str(output_str).expect("valid TOML");

    // Verify the nested structure
    assert_eq!(
        written["messages"]["greeting"].as_str(),
        Some("Hello, World!")
    );
    assert_eq!(
        written["messages"]["nested"]["welcome"].as_str(),
        Some("Welcome to our app")
    );
    assert_eq!(
        written["errors"]["not_found"].as_str(),
        Some("Page not found")
    );
}

// ──────────────────────────────────────────────
// Edge cases
// ──────────────────────────────────────────────

#[test]
fn parse_deeply_nested_toml() {
    let content = b"[a.b.c]\nd = \"deep value\"\n";
    let resource = parser().parse(content).expect("parse should succeed");
    assert_eq!(resource.entries.len(), 1);
    assert_eq!(
        resource.entries["a.b.c.d"].value,
        EntryValue::Simple("deep value".to_string())
    );
}

#[test]
fn write_deeply_nested_entries() {
    let mut entries = IndexMap::new();
    entries.insert(
        "a.b.c.d".to_string(),
        I18nEntry {
            key: "a.b.c.d".to_string(),
            value: EntryValue::Simple("deep value".to_string()),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata::default(),
        entries,
    };

    let output = writer().write(&resource).expect("write should succeed");
    let output_str = std::str::from_utf8(&output).expect("valid UTF-8");
    let parsed: toml::Value = toml::from_str(output_str).expect("valid TOML");

    assert_eq!(parsed["a"]["b"]["c"]["d"].as_str(), Some("deep value"));
}

#[test]
fn roundtrip_mixed_flat_and_nested() {
    let content = b"top_level = \"value\"\n\n[nested]\nkey = \"nested value\"\n";
    let resource = parser().parse(content).expect("parse should succeed");
    assert_eq!(resource.entries.len(), 2);
    assert!(resource.entries.contains_key("top_level"));
    assert!(resource.entries.contains_key("nested.key"));

    let output = writer().write(&resource).expect("write should succeed");
    let reparsed = parser().parse(&output).expect("reparse should succeed");

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, entry) in &resource.entries {
        assert_eq!(entry.value, reparsed.entries[key].value);
    }
}

#[test]
fn parse_unicode_values() {
    let content =
        "japanese = \"\u{3053}\u{3093}\u{306b}\u{3061}\u{306f}\"\naccented = \"caf\u{e9}\"\n";
    let resource = parser()
        .parse(content.as_bytes())
        .expect("parse should succeed");
    assert_eq!(resource.entries.len(), 2);
    assert_eq!(
        resource.entries["accented"].value,
        EntryValue::Simple("caf\u{e9}".to_string())
    );
}

#[test]
fn entry_keys_match_entry_key_field() {
    let content = fixture("nested.toml");
    let resource = parser().parse(&content).expect("parse should succeed");

    for (map_key, entry) in &resource.entries {
        assert_eq!(
            map_key, &entry.key,
            "Map key should match entry.key for '{map_key}'"
        );
    }
}

#[test]
fn metadata_has_toml_format_extension() {
    let content = fixture("flat.toml");
    let resource = parser().parse(&content).expect("parse should succeed");

    assert!(matches!(
        resource.metadata.format_ext,
        Some(FormatExtension::Toml(TomlExt { .. }))
    ));
}

// ──────────────────────────────────────────────
// Capabilities test
// ──────────────────────────────────────────────

#[test]
fn capabilities_match_spec() {
    let caps = parser().capabilities();
    assert!(!caps.plurals);
    assert!(caps.arrays);
    assert!(!caps.comments);
    assert!(!caps.context);
    assert!(!caps.source_string);
    assert!(!caps.translatable_flag);
    assert!(!caps.translation_state);
    assert!(!caps.max_width);
    assert!(!caps.device_variants);
    assert!(!caps.select_gender);
    assert!(caps.nested_keys);
    assert!(!caps.inline_markup);
    assert!(!caps.alternatives);
    assert!(!caps.source_references);
    assert!(!caps.custom_properties);
}
