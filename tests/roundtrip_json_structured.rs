use i18n_convert::formats::json_structured;
use i18n_convert::formats::{Confidence, FormatParser, FormatWriter};
use i18n_convert::ir::*;
use indexmap::IndexMap;

fn parser() -> json_structured::Parser {
    json_structured::Parser
}

fn writer() -> json_structured::Writer {
    json_structured::Writer
}

fn fixture(name: &str) -> Vec<u8> {
    let path = format!(
        "{}/tests/fixtures/json_structured/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read(&path).unwrap_or_else(|e| panic!("Failed to read fixture {path}: {e}"))
}

// ──────────────────────────────────────────────
// Parsing tests
// ──────────────────────────────────────────────

#[test]
fn parse_flat_json() {
    let content = fixture("flat.json");
    let resource = parser().parse(&content).unwrap();

    assert_eq!(resource.metadata.source_format, FormatId::JsonStructured);
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
fn parse_nested_json() {
    let content = fixture("nested.json");
    let resource = parser().parse(&content).unwrap();

    // Nested objects are flattened to dot-separated keys
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
fn parse_icu_json() {
    let content = fixture("icu.json");
    let resource = parser().parse(&content).unwrap();

    assert_eq!(resource.entries.len(), 6);

    // ICU message syntax is preserved as-is in simple string values
    assert_eq!(
        resource.entries["greeting"].value,
        EntryValue::Simple("Hello, {name}!".to_string())
    );
    assert_eq!(
        resource.entries["items_count"].value,
        EntryValue::Simple("{count, plural, one {# item} other {# items}}".to_string())
    );
    assert_eq!(
        resource.entries["gender_msg"].value,
        EntryValue::Simple(
            "{gender, select, male {He} female {She} other {They}} liked this.".to_string()
        )
    );
    assert_eq!(
        resource.entries["nested.welcome"].value,
        EntryValue::Simple("Welcome, {user}!".to_string())
    );
}

#[test]
fn parse_empty_object() {
    let content = b"{}";
    let resource = parser().parse(content).unwrap();
    assert_eq!(resource.entries.len(), 0);
    assert_eq!(resource.metadata.source_format, FormatId::JsonStructured);
}

#[test]
fn parse_invalid_json_returns_error() {
    let content = b"not json at all";
    let result = parser().parse(content);
    assert!(result.is_err());
}

#[test]
fn parse_non_object_root_returns_error() {
    let content = b"[\"an\", \"array\"]";
    let result = parser().parse(content);
    assert!(result.is_err());
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
            source_format: FormatId::JsonStructured,
            ..Default::default()
        },
        entries,
    };

    let output = writer().write(&resource).unwrap();
    let output_str = std::str::from_utf8(&output).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(output_str).unwrap();
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
        "common.farewell".to_string(),
        I18nEntry {
            key: "common.farewell".to_string(),
            value: EntryValue::Simple("Goodbye".to_string()),
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
            source_format: FormatId::JsonStructured,
            ..Default::default()
        },
        entries,
    };

    let output = writer().write(&resource).unwrap();
    let output_str = std::str::from_utf8(&output).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(output_str).unwrap();
    assert_eq!(parsed["common"]["greeting"], "Hello");
    assert_eq!(parsed["common"]["farewell"], "Goodbye");
    assert_eq!(parsed["pages"]["home"]["title"], "Home");
}

#[test]
fn write_empty_resource() {
    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::JsonStructured,
            ..Default::default()
        },
        entries: IndexMap::new(),
    };

    let output = writer().write(&resource).unwrap();
    let output_str = std::str::from_utf8(&output).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(output_str).unwrap();
    assert_eq!(parsed, serde_json::json!({}));
}

#[test]
fn write_output_ends_with_newline() {
    let resource = I18nResource {
        metadata: ResourceMetadata::default(),
        entries: IndexMap::new(),
    };
    let output = writer().write(&resource).unwrap();
    assert!(output.ends_with(b"\n"));
}

// ──────────────────────────────────────────────
// Round-trip tests
// ──────────────────────────────────────────────

#[test]
fn roundtrip_flat_json() {
    let content = fixture("flat.json");
    let resource = parser().parse(&content).unwrap();
    let output = writer().write(&resource).unwrap();
    let reparsed = parser().parse(&output).unwrap();

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, entry) in &resource.entries {
        let reparsed_entry = &reparsed.entries[key];
        assert_eq!(entry.value, reparsed_entry.value, "Mismatch for key: {key}");
    }
}

#[test]
fn roundtrip_nested_json() {
    let content = fixture("nested.json");
    let resource = parser().parse(&content).unwrap();
    let output = writer().write(&resource).unwrap();
    let reparsed = parser().parse(&output).unwrap();

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, entry) in &resource.entries {
        let reparsed_entry = &reparsed.entries[key];
        assert_eq!(entry.value, reparsed_entry.value, "Mismatch for key: {key}");
    }
}

#[test]
fn roundtrip_icu_json() {
    let content = fixture("icu.json");
    let resource = parser().parse(&content).unwrap();
    let output = writer().write(&resource).unwrap();
    let reparsed = parser().parse(&output).unwrap();

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, entry) in &resource.entries {
        let reparsed_entry = &reparsed.entries[key];
        assert_eq!(entry.value, reparsed_entry.value, "Mismatch for key: {key}");
    }
}

#[test]
fn roundtrip_preserves_nested_structure() {
    // Parse nested JSON, write it back out, and verify the JSON structure is correct
    let content = fixture("nested.json");
    let resource = parser().parse(&content).unwrap();
    let output = writer().write(&resource).unwrap();
    let output_str = std::str::from_utf8(&output).unwrap();

    let original: serde_json::Value = serde_json::from_slice(&content).unwrap();
    let written: serde_json::Value = serde_json::from_str(output_str).unwrap();

    assert_eq!(original, written);
}

#[test]
fn roundtrip_preserves_icu_structure() {
    // Parse ICU JSON, write it back out, and verify the JSON structure is correct
    let content = fixture("icu.json");
    let resource = parser().parse(&content).unwrap();
    let output = writer().write(&resource).unwrap();
    let output_str = std::str::from_utf8(&output).unwrap();

    let original: serde_json::Value = serde_json::from_slice(&content).unwrap();
    let written: serde_json::Value = serde_json::from_str(output_str).unwrap();

    assert_eq!(original, written);
}

// ──────────────────────────────────────────────
// Detection tests
// ──────────────────────────────────────────────

#[test]
fn detect_json_file() {
    let content = b"{\"greeting\": \"Hello\"}";
    assert_eq!(parser().detect(".json", content), Confidence::Low);
}

#[test]
fn detect_ignores_non_json_extension() {
    let content = b"{\"greeting\": \"Hello\"}";
    assert_eq!(parser().detect(".xml", content), Confidence::None);
}

#[test]
fn detect_excludes_arb() {
    let content = b"{\"@@locale\": \"en\", \"greeting\": \"Hello\"}";
    assert_eq!(parser().detect(".json", content), Confidence::None);
}

#[test]
fn detect_excludes_xcstrings() {
    let content = b"{\"sourceLanguage\": \"en\", \"strings\": {}}";
    assert_eq!(parser().detect(".json", content), Confidence::None);
}

#[test]
fn detect_excludes_i18next() {
    let content = b"{\"key_one\": \"item\", \"key_other\": \"items\"}";
    assert_eq!(parser().detect(".json", content), Confidence::None);
}

#[test]
fn detect_ignores_non_object() {
    let content = b"[1, 2, 3]";
    assert_eq!(parser().detect(".json", content), Confidence::None);
}

// ──────────────────────────────────────────────
// Capabilities test
// ──────────────────────────────────────────────

#[test]
fn capabilities_reports_nested_keys() {
    let caps = parser().capabilities();
    assert!(caps.nested_keys);
    assert!(!caps.plurals);
    assert!(!caps.comments);
    assert!(!caps.arrays);
}

// ──────────────────────────────────────────────
// Edge cases
// ──────────────────────────────────────────────

#[test]
fn parse_deeply_nested_json() {
    let content = br#"{
        "a": {
            "b": {
                "c": {
                    "d": "deep value"
                }
            }
        }
    }"#;
    let resource = parser().parse(content).unwrap();
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

    let output = writer().write(&resource).unwrap();
    let output_str = std::str::from_utf8(&output).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(output_str).unwrap();

    assert_eq!(parsed["a"]["b"]["c"]["d"], "deep value");
}

#[test]
fn roundtrip_mixed_flat_and_nested() {
    let content = br#"{
  "top_level": "value",
  "nested": {
    "key": "nested value"
  }
}"#;
    let resource = parser().parse(content).unwrap();
    assert_eq!(resource.entries.len(), 2);
    assert!(resource.entries.contains_key("top_level"));
    assert!(resource.entries.contains_key("nested.key"));

    let output = writer().write(&resource).unwrap();
    let reparsed = parser().parse(&output).unwrap();

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, entry) in &resource.entries {
        assert_eq!(entry.value, reparsed.entries[key].value);
    }
}

#[test]
fn parse_unicode_values() {
    let content = br#"{
  "japanese": "\u3053\u3093\u306b\u3061\u306f",
  "emoji": "\ud83d\ude0a",
  "accented": "caf\u00e9"
}"#;
    let resource = parser().parse(content).unwrap();
    assert_eq!(resource.entries.len(), 3);
    assert_eq!(
        resource.entries["accented"].value,
        EntryValue::Simple("caf\u{e9}".to_string())
    );
}

#[test]
fn entry_keys_match_entry_key_field() {
    let content = fixture("nested.json");
    let resource = parser().parse(&content).unwrap();

    for (map_key, entry) in &resource.entries {
        assert_eq!(
            map_key, &entry.key,
            "Map key should match entry.key for '{map_key}'"
        );
    }
}
