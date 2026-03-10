use i18n_convert::formats::json5_format;
use i18n_convert::formats::{Confidence, FormatParser, FormatWriter};
use i18n_convert::ir::*;
use indexmap::IndexMap;

fn parser() -> json5_format::Parser {
    json5_format::Parser
}

fn writer() -> json5_format::Writer {
    json5_format::Writer
}

fn fixture(name: &str) -> Vec<u8> {
    let path = format!("{}/tests/fixtures/json5/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read(&path).unwrap_or_else(|e| panic!("Failed to read fixture {path}: {e}"))
}

// ──────────────────────────────────────────────
// Detection tests
// ──────────────────────────────────────────────

#[test]
fn detect_json5_extension_valid_content() {
    let content = b"{ greeting: 'Hello' }";
    assert_eq!(parser().detect(".json5", content), Confidence::Definite);
}

#[test]
fn detect_json5_extension_invalid_content() {
    let content = b"this is not valid json5 {{{";
    assert_eq!(parser().detect(".json5", content), Confidence::Low);
}

#[test]
fn detect_non_json5_extension() {
    let content = b"{ greeting: 'Hello' }";
    assert_eq!(parser().detect(".json", content), Confidence::None);
    assert_eq!(parser().detect(".toml", content), Confidence::None);
    assert_eq!(parser().detect(".xml", content), Confidence::None);
}

#[test]
fn detect_standard_json_as_json5() {
    let content = b"{\"greeting\": \"Hello\"}";
    assert_eq!(parser().detect(".json5", content), Confidence::Definite);
}

// ──────────────────────────────────────────────
// Parsing tests
// ──────────────────────────────────────────────

#[test]
fn parse_flat_json5() {
    let content = fixture("flat.json5");
    let resource = parser().parse(&content).expect("parse should succeed");

    assert_eq!(resource.metadata.source_format, FormatId::Json5);
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
fn parse_nested_json5() {
    let content = fixture("nested.json5");
    let resource = parser().parse(&content).expect("parse should succeed");

    assert_eq!(resource.entries.len(), 6);

    assert_eq!(
        resource.entries["common.greeting"].value,
        EntryValue::Simple("Hello".to_string())
    );
    assert_eq!(
        resource.entries["common.farewell"].value,
        EntryValue::Simple("Goodbye".to_string())
    );
    assert_eq!(
        resource.entries["pages.home.title"].value,
        EntryValue::Simple("Home Page".to_string())
    );
    assert_eq!(
        resource.entries["pages.home.description"].value,
        EntryValue::Simple("Welcome to our website".to_string())
    );
    assert_eq!(
        resource.entries["pages.about.title"].value,
        EntryValue::Simple("About Us".to_string())
    );
    assert_eq!(
        resource.entries["simple_key"].value,
        EntryValue::Simple("A top-level value".to_string())
    );
}

#[test]
fn parse_json5_with_comments() {
    let content = fixture("comments.json5");
    let resource = parser().parse(&content).expect("parse should succeed");

    // Comments are stripped during parsing but should not cause errors
    assert_eq!(resource.entries.len(), 4);

    assert_eq!(
        resource.entries["greeting"].value,
        EntryValue::Simple("Hello, World!".to_string())
    );
    assert_eq!(
        resource.entries["farewell"].value,
        EntryValue::Simple("Goodbye!".to_string())
    );
    assert_eq!(
        resource.entries["nested.welcome"].value,
        EntryValue::Simple("Welcome to our app".to_string())
    );
    assert_eq!(
        resource.entries["nested.items"].value,
        EntryValue::Simple("You have {count} items".to_string())
    );
}

#[test]
fn parse_json5_with_arrays() {
    let content = fixture("arrays.json5");
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
        resource.entries["nested.menu_items"].value,
        EntryValue::Array(vec![
            "Home".to_string(),
            "About".to_string(),
            "Contact".to_string(),
        ])
    );
}

#[test]
fn parse_empty_json5_object() {
    let content = b"{}";
    let resource = parser().parse(content).expect("parse should succeed");
    assert_eq!(resource.entries.len(), 0);
    assert_eq!(resource.metadata.source_format, FormatId::Json5);
}

#[test]
fn parse_invalid_json5_returns_error() {
    let content = b"not json5 at all";
    let result = parser().parse(content);
    assert!(result.is_err());
}

#[test]
fn parse_non_object_root_returns_error() {
    let content = b"['an', 'array']";
    let result = parser().parse(content);
    assert!(result.is_err());
}

#[test]
fn parse_single_quoted_strings() {
    let content = b"{ key: 'single quoted value' }";
    let resource = parser().parse(content).expect("parse should succeed");
    assert_eq!(
        resource.entries["key"].value,
        EntryValue::Simple("single quoted value".to_string())
    );
}

#[test]
fn parse_unquoted_keys() {
    let content = b"{ unquotedKey: 'value' }";
    let resource = parser().parse(content).expect("parse should succeed");
    assert_eq!(
        resource.entries["unquotedKey"].value,
        EntryValue::Simple("value".to_string())
    );
}

#[test]
fn parse_trailing_commas() {
    let content = b"{ a: 'one', b: 'two', }";
    let resource = parser().parse(content).expect("parse should succeed");
    assert_eq!(resource.entries.len(), 2);
}

#[test]
fn parse_numeric_values() {
    let content = b"{ count: 42, ratio: 3.14, flag: true }";
    let resource = parser().parse(content).expect("parse should succeed");

    assert_eq!(resource.entries.len(), 3);
    assert_eq!(
        resource.entries["count"].value,
        EntryValue::Simple("42".to_string())
    );
    assert_eq!(
        resource.entries["flag"].value,
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
            source_format: FormatId::Json5,
            ..Default::default()
        },
        entries,
    };

    let output = writer().write(&resource).expect("write should succeed");
    let output_str = std::str::from_utf8(&output).expect("valid UTF-8");

    // Output is standard JSON (valid JSON5)
    let parsed: serde_json::Value = serde_json::from_str(output_str).expect("valid JSON");
    assert_eq!(parsed["greeting"], "Hello");
    assert_eq!(parsed["farewell"], "Goodbye");
}

#[test]
fn write_nested_entries() {
    let mut entries = IndexMap::new();
    entries.insert(
        "common.greeting".to_string(),
        I18nEntry {
            key: "common.greeting".to_string(),
            value: EntryValue::Simple("Hello".to_string()),
            ..Default::default()
        },
    );
    entries.insert(
        "pages.home.title".to_string(),
        I18nEntry {
            key: "pages.home.title".to_string(),
            value: EntryValue::Simple("Home".to_string()),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Json5,
            ..Default::default()
        },
        entries,
    };

    let output = writer().write(&resource).expect("write should succeed");
    let output_str = std::str::from_utf8(&output).expect("valid UTF-8");

    let parsed: serde_json::Value = serde_json::from_str(output_str).expect("valid JSON");
    assert_eq!(parsed["common"]["greeting"], "Hello");
    assert_eq!(parsed["pages"]["home"]["title"], "Home");
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
            source_format: FormatId::Json5,
            ..Default::default()
        },
        entries,
    };

    let output = writer().write(&resource).expect("write should succeed");
    let output_str = std::str::from_utf8(&output).expect("valid UTF-8");

    let parsed: serde_json::Value = serde_json::from_str(output_str).expect("valid JSON");
    let arr = parsed["colors"].as_array().expect("should be array");
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0], "red");
    assert_eq!(arr[1], "green");
    assert_eq!(arr[2], "blue");
}

#[test]
fn write_empty_resource() {
    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Json5,
            ..Default::default()
        },
        entries: IndexMap::new(),
    };

    let output = writer().write(&resource).expect("write should succeed");
    let output_str = std::str::from_utf8(&output).expect("valid UTF-8");

    let parsed: serde_json::Value = serde_json::from_str(output_str).expect("valid JSON");
    assert_eq!(parsed, serde_json::json!({}));
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
fn roundtrip_flat_json5() {
    let content = fixture("flat.json5");
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
fn roundtrip_nested_json5() {
    let content = fixture("nested.json5");
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
fn roundtrip_comments_json5() {
    // Comments are lost on round-trip, but values survive
    let content = fixture("comments.json5");
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
fn roundtrip_arrays_json5() {
    let content = fixture("arrays.json5");
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
    // Writer outputs standard JSON, which json5 parser can read
    let content = fixture("nested.json5");
    let resource = parser().parse(&content).expect("parse should succeed");
    let output = writer().write(&resource).expect("write should succeed");
    let output_str = std::str::from_utf8(&output).expect("valid UTF-8");

    let written: serde_json::Value = serde_json::from_str(output_str).expect("valid JSON");

    assert_eq!(written["common"]["greeting"], "Hello");
    assert_eq!(written["common"]["farewell"], "Goodbye");
    assert_eq!(written["pages"]["home"]["title"], "Home Page");
    assert_eq!(written["pages"]["about"]["title"], "About Us");
    assert_eq!(written["simple_key"], "A top-level value");
}

// ──────────────────────────────────────────────
// Edge cases
// ──────────────────────────────────────────────

#[test]
fn parse_deeply_nested_json5() {
    let content = b"{ a: { b: { c: { d: 'deep value' } } } }";
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
    let parsed: serde_json::Value = serde_json::from_str(output_str).expect("valid JSON");

    assert_eq!(parsed["a"]["b"]["c"]["d"], "deep value");
}

#[test]
fn roundtrip_mixed_flat_and_nested() {
    let content = b"{ top_level: 'value', nested: { key: 'nested value' } }";
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
    let content = "{ japanese: '\u{3053}\u{3093}\u{306b}\u{3061}\u{306f}', accented: 'caf\u{e9}' }";
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
    let content = fixture("nested.json5");
    let resource = parser().parse(&content).expect("parse should succeed");

    for (map_key, entry) in &resource.entries {
        assert_eq!(
            map_key, &entry.key,
            "Map key should match entry.key for '{map_key}'"
        );
    }
}

#[test]
fn metadata_has_json5_format_extension() {
    let content = fixture("flat.json5");
    let resource = parser().parse(&content).expect("parse should succeed");

    assert!(matches!(
        resource.metadata.format_ext,
        Some(FormatExtension::Json5(Json5Ext { .. }))
    ));
}

#[test]
fn writer_output_is_valid_json5() {
    // Since we write standard JSON, it should be parseable by both json5 and serde_json
    let mut entries = IndexMap::new();
    entries.insert(
        "key".to_string(),
        I18nEntry {
            key: "key".to_string(),
            value: EntryValue::Simple("value".to_string()),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata::default(),
        entries,
    };

    let output = writer().write(&resource).expect("write should succeed");
    let output_str = std::str::from_utf8(&output).expect("valid UTF-8");

    // Valid as standard JSON
    assert!(serde_json::from_str::<serde_json::Value>(output_str).is_ok());
    // Valid as JSON5
    assert!(json5::from_str::<serde_json::Value>(output_str).is_ok());
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
