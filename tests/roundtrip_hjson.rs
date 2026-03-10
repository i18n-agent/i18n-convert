use i18n_convert::formats::{Confidence, FormatParser, FormatWriter};
use i18n_convert::formats::hjson;
use i18n_convert::ir::*;
use indexmap::IndexMap;

fn parser() -> hjson::Parser {
    hjson::Parser
}

fn writer() -> hjson::Writer {
    hjson::Writer
}

fn fixture(name: &str) -> Vec<u8> {
    let path = format!(
        "{}/tests/fixtures/hjson/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read(&path).unwrap_or_else(|e| panic!("Failed to read fixture {path}: {e}"))
}

// ──────────────────────────────────────────────
// Detection tests
// ──────────────────────────────────────────────

#[test]
fn detect_hjson_by_extension() {
    let content = b"{greeting: Hello}";
    assert_eq!(parser().detect(".hjson", content), Confidence::Definite);
}

#[test]
fn detect_ignores_non_hjson_extension() {
    let content = b"{greeting: Hello}";
    assert_eq!(parser().detect(".json", content), Confidence::None);
}

#[test]
fn detect_hjson_content_with_comments() {
    let content = b"{\n  # This is a comment\n  greeting: Hello\n}";
    assert_eq!(parser().detect(".txt", content), Confidence::Low);
}

// ──────────────────────────────────────────────
// Parsing tests
// ──────────────────────────────────────────────

#[test]
fn parse_flat_hjson() {
    let content = fixture("flat.hjson");
    let resource = parser().parse(&content).unwrap();

    assert_eq!(resource.metadata.source_format, FormatId::Hjson);
    assert_eq!(resource.entries.len(), 5);

    assert_eq!(
        resource.entries["greeting"].value,
        EntryValue::Simple("Hello, World!".to_string())
    );
    assert_eq!(
        resource.entries["farewell"].value,
        EntryValue::Simple("Goodbye!".to_string())
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
fn parse_nested_hjson() {
    let content = fixture("nested.hjson");
    let resource = parser().parse(&content).unwrap();

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
fn parse_quoted_hjson() {
    let content = fixture("quoted.hjson");
    let resource = parser().parse(&content).unwrap();

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
        resource.entries["with_escape"].value,
        EntryValue::Simple("Line one\nLine two".to_string())
    );
    assert_eq!(
        resource.entries["with_colon"].value,
        EntryValue::Simple("Time: 12:00".to_string())
    );
}

#[test]
fn parse_multiline_hjson() {
    let content = fixture("multiline.hjson");
    let resource = parser().parse(&content).unwrap();

    assert_eq!(resource.entries.len(), 2);

    assert_eq!(
        resource.entries["description"].value,
        EntryValue::Simple("This is a\nmulti-line string".to_string())
    );
    assert_eq!(
        resource.entries["simple"].value,
        EntryValue::Simple("Hello".to_string())
    );
}

#[test]
fn parse_arrays_hjson() {
    let content = fixture("arrays.hjson");
    let resource = parser().parse(&content).unwrap();

    assert_eq!(resource.entries.len(), 2);

    assert_eq!(
        resource.entries["colors"].value,
        EntryValue::Array(vec![
            "Red".to_string(),
            "Green".to_string(),
            "Blue".to_string(),
        ])
    );
    assert_eq!(
        resource.entries["greeting"].value,
        EntryValue::Simple("Hello".to_string())
    );
}

#[test]
fn parse_empty_object() {
    let content = b"{}";
    let resource = parser().parse(content).unwrap();
    assert_eq!(resource.entries.len(), 0);
    assert_eq!(resource.metadata.source_format, FormatId::Hjson);
}

#[test]
fn parse_invalid_hjson_returns_error() {
    let content = b"not hjson at all";
    let result = parser().parse(content);
    assert!(result.is_err());
}

#[test]
fn parse_hash_comments() {
    let content = b"{\n  # This is a comment\n  key: value\n}";
    let resource = parser().parse(content).unwrap();
    assert_eq!(resource.entries.len(), 1);
    assert_eq!(
        resource.entries["key"].value,
        EntryValue::Simple("value".to_string())
    );
}

#[test]
fn parse_slash_comments() {
    let content = b"{\n  // This is a comment\n  key: value\n}";
    let resource = parser().parse(content).unwrap();
    assert_eq!(resource.entries.len(), 1);
}

#[test]
fn parse_block_comments() {
    let content = b"{\n  /* Block comment */\n  key: value\n}";
    let resource = parser().parse(content).unwrap();
    assert_eq!(resource.entries.len(), 1);
}

#[test]
fn parse_trailing_comma() {
    let content = b"{\n  a: one,\n  b: two,\n}";
    let resource = parser().parse(content).unwrap();
    assert_eq!(resource.entries.len(), 2);
    assert_eq!(
        resource.entries["a"].value,
        EntryValue::Simple("one".to_string())
    );
    assert_eq!(
        resource.entries["b"].value,
        EntryValue::Simple("two".to_string())
    );
}

#[test]
fn parse_no_commas() {
    let content = b"{\n  a: one\n  b: two\n}";
    let resource = parser().parse(content).unwrap();
    assert_eq!(resource.entries.len(), 2);
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
            source_format: FormatId::Hjson,
            ..Default::default()
        },
        entries,
    };

    let output = writer().write(&resource).unwrap();
    let output_str = std::str::from_utf8(&output).unwrap();

    // Output is valid JSON (JSON is a subset of HJSON)
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
        "pages.home.title".to_string(),
        I18nEntry {
            key: "pages.home.title".to_string(),
            value: EntryValue::Simple("Home".to_string()),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Hjson,
            ..Default::default()
        },
        entries,
    };

    let output = writer().write(&resource).unwrap();
    let output_str = std::str::from_utf8(&output).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(output_str).unwrap();
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
                "Red".to_string(),
                "Green".to_string(),
                "Blue".to_string(),
            ]),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Hjson,
            ..Default::default()
        },
        entries,
    };

    let output = writer().write(&resource).unwrap();
    let output_str = std::str::from_utf8(&output).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(output_str).unwrap();
    assert_eq!(
        parsed["colors"],
        serde_json::json!(["Red", "Green", "Blue"])
    );
}

#[test]
fn write_empty_resource() {
    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Hjson,
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
fn roundtrip_flat_hjson() {
    let content = fixture("flat.hjson");
    let resource = parser().parse(&content).unwrap();
    let output = writer().write(&resource).unwrap();

    // Writer outputs JSON, which the HJSON parser can also parse
    // But for true roundtrip we re-parse with the JSON parser since output is JSON
    let reparsed = parser().parse(&output).unwrap();

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, entry) in &resource.entries {
        let reparsed_entry = &reparsed.entries[key];
        assert_eq!(entry.value, reparsed_entry.value, "Mismatch for key: {key}");
    }
}

#[test]
fn roundtrip_nested_hjson() {
    let content = fixture("nested.hjson");
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
fn roundtrip_quoted_hjson() {
    let content = fixture("quoted.hjson");
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
fn roundtrip_multiline_hjson() {
    let content = fixture("multiline.hjson");
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
fn roundtrip_arrays_hjson() {
    let content = fixture("arrays.hjson");
    let resource = parser().parse(&content).unwrap();
    let output = writer().write(&resource).unwrap();
    let reparsed = parser().parse(&output).unwrap();

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, entry) in &resource.entries {
        let reparsed_entry = &reparsed.entries[key];
        assert_eq!(entry.value, reparsed_entry.value, "Mismatch for key: {key}");
    }
}

// ──────────────────────────────────────────────
// Capabilities test
// ──────────────────────────────────────────────

#[test]
fn capabilities_reports_nested_keys_and_arrays() {
    let caps = parser().capabilities();
    assert!(caps.nested_keys);
    assert!(caps.arrays);
    assert!(!caps.plurals);
    assert!(!caps.comments);
    assert!(!caps.custom_properties);
}

// ──────────────────────────────────────────────
// Edge cases
// ──────────────────────────────────────────────

#[test]
fn parse_deeply_nested_hjson() {
    let content = b"{\n  a: {\n    b: {\n      c: deep value\n    }\n  }\n}";
    let resource = parser().parse(content).unwrap();
    assert_eq!(resource.entries.len(), 1);
    assert_eq!(
        resource.entries["a.b.c"].value,
        EntryValue::Simple("deep value".to_string())
    );
}

#[test]
fn entry_keys_match_entry_key_field() {
    let content = fixture("nested.hjson");
    let resource = parser().parse(&content).unwrap();

    for (map_key, entry) in &resource.entries {
        assert_eq!(
            map_key, &entry.key,
            "Map key should match entry.key for '{map_key}'"
        );
    }
}

#[test]
fn parse_single_quoted_string() {
    let content = b"{\n  key: 'single quoted value'\n}";
    let resource = parser().parse(content).unwrap();
    assert_eq!(
        resource.entries["key"].value,
        EntryValue::Simple("single quoted value".to_string())
    );
}

#[test]
fn parse_mixed_comment_styles() {
    let content = b"{\n  # hash comment\n  a: one\n  // slash comment\n  b: two\n  /* block\n     comment */\n  c: three\n}";
    let resource = parser().parse(content).unwrap();
    assert_eq!(resource.entries.len(), 3);
    assert_eq!(
        resource.entries["a"].value,
        EntryValue::Simple("one".to_string())
    );
    assert_eq!(
        resource.entries["b"].value,
        EntryValue::Simple("two".to_string())
    );
    assert_eq!(
        resource.entries["c"].value,
        EntryValue::Simple("three".to_string())
    );
}
