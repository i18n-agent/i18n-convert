use i18n_convert::formats::yaml_plain;
use i18n_convert::formats::{Confidence, FormatParser, FormatWriter};
use i18n_convert::ir::*;
use indexmap::IndexMap;

fn parser() -> yaml_plain::Parser {
    yaml_plain::Parser
}

fn writer() -> yaml_plain::Writer {
    yaml_plain::Writer
}

fn fixture(name: &str) -> Vec<u8> {
    let path = format!(
        "{}/tests/fixtures/yaml_plain/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read(&path).unwrap_or_else(|e| panic!("Failed to read fixture {path}: {e}"))
}

// ──────────────────────────────────────────────
// Detection tests
// ──────────────────────────────────────────────

#[test]
fn detect_yml_extension_plain_content() {
    let content = b"greeting: Hello\nfarewell: Goodbye\n";
    assert_eq!(parser().detect(".yml", content), Confidence::High);
}

#[test]
fn detect_yaml_extension_plain_content() {
    let content = b"greeting: Hello\nfarewell: Goodbye\n";
    assert_eq!(parser().detect(".yaml", content), Confidence::High);
}

#[test]
fn detect_low_confidence_when_locale_root() {
    // Rails-style YAML should get Low confidence from the plain parser
    let content = b"en:\n  greeting: Hello\n";
    assert_eq!(parser().detect(".yml", content), Confidence::Low);
}

#[test]
fn detect_low_confidence_various_locales() {
    assert_eq!(
        parser().detect(".yml", b"ja:\n  key: val\n"),
        Confidence::Low
    );
    assert_eq!(
        parser().detect(".yml", b"zh-Hans:\n  key: val\n"),
        Confidence::Low
    );
    assert_eq!(
        parser().detect(".yml", b"pt-BR:\n  key: val\n"),
        Confidence::Low
    );
}

#[test]
fn detect_high_when_first_key_is_long() {
    // "config:" is not a locale code
    let content = b"config:\n  debug: true\n";
    assert_eq!(parser().detect(".yml", content), Confidence::High);
}

#[test]
fn detect_high_with_comments_before_content() {
    let content = b"# This is a comment\ngreeting: Hello\n";
    assert_eq!(parser().detect(".yml", content), Confidence::High);
}

#[test]
fn detect_none_for_non_yaml_extension() {
    let content = b"greeting: Hello\n";
    assert_eq!(parser().detect(".json", content), Confidence::None);
    assert_eq!(parser().detect(".xml", content), Confidence::None);
    assert_eq!(parser().detect(".txt", content), Confidence::None);
}

// ──────────────────────────────────────────────
// Parse fixture tests
// ──────────────────────────────────────────────

#[test]
fn parse_simple_fixture() {
    let content = fixture("simple.yml");
    let resource = parser().parse(&content).expect("parse should succeed");

    assert_eq!(resource.metadata.source_format, FormatId::YamlPlain);
    assert_eq!(resource.entries.len(), 4);

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
        EntryValue::Simple("Welcome to our app".to_string())
    );
}

#[test]
fn parse_nested_fixture() {
    let content = fixture("nested.yml");
    let resource = parser().parse(&content).expect("parse should succeed");

    assert_eq!(resource.entries.len(), 7);

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
        EntryValue::Simple("Welcome to the home page".to_string())
    );
    assert_eq!(
        resource.entries["pages.about.title"].value,
        EntryValue::Simple("About Us".to_string())
    );
    assert_eq!(
        resource.entries["pages.about.team.lead"].value,
        EntryValue::Simple("Team Lead".to_string())
    );
    assert_eq!(
        resource.entries["pages.about.team.members"].value,
        EntryValue::Simple("Team Members".to_string())
    );
}

#[test]
fn parse_plurals_fixture() {
    let content = fixture("plurals.yml");
    let resource = parser().parse(&content).expect("parse should succeed");

    // items_one + items_other -> "items" plural
    // files_zero + files_one + files_other -> "files" plural
    // simple_key -> simple entry
    assert_eq!(resource.entries.len(), 3);

    match &resource.entries["items"].value {
        EntryValue::Plural(ps) => {
            assert_eq!(ps.one, Some("{count} item".to_string()));
            assert_eq!(ps.other, "{count} items");
        }
        other => panic!("Expected Plural for 'items', got {:?}", other),
    }

    match &resource.entries["files"].value {
        EntryValue::Plural(ps) => {
            assert_eq!(ps.zero, Some("No files".to_string()));
            assert_eq!(ps.one, Some("{count} file".to_string()));
            assert_eq!(ps.other, "{count} files");
        }
        other => panic!("Expected Plural for 'files', got {:?}", other),
    }

    assert_eq!(
        resource.entries["simple_key"].value,
        EntryValue::Simple("not a plural".to_string())
    );
}

#[test]
fn parse_arrays_fixture() {
    let content = fixture("arrays.yml");
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
fn parse_empty_yaml() {
    let content = b"";
    let resource = parser().parse(content).expect("parse should succeed");
    assert_eq!(resource.entries.len(), 0);
    assert_eq!(resource.metadata.source_format, FormatId::YamlPlain);
}

#[test]
fn parse_invalid_yaml_returns_error() {
    let content = b"greeting: [invalid: {yaml";
    let result = parser().parse(content);
    assert!(result.is_err());
}

// ──────────────────────────────────────────────
// Writer tests
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
            source_format: FormatId::YamlPlain,
            ..Default::default()
        },
        entries,
    };

    let output = writer().write(&resource).expect("write should succeed");
    let text = std::str::from_utf8(&output).expect("valid UTF-8");

    // Plain YAML should NOT have a locale root key
    assert!(!text.starts_with("en:"));
    assert!(text.contains("greeting: Hello"));
    assert!(text.contains("farewell: Goodbye"));
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

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::YamlPlain,
            ..Default::default()
        },
        entries,
    };

    let output = writer().write(&resource).expect("write should succeed");
    let text = std::str::from_utf8(&output).expect("valid UTF-8");

    // Should have nested structure, not flat dot-keys
    assert!(text.contains("messages:"), "Expected nested 'messages' key in:\n{}", text);
}

#[test]
fn write_plural_entries_as_suffix_keys() {
    let mut entries = IndexMap::new();
    entries.insert(
        "items".to_string(),
        I18nEntry {
            key: "items".to_string(),
            value: EntryValue::Plural(PluralSet {
                one: Some("{count} item".to_string()),
                other: "{count} items".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::YamlPlain,
            ..Default::default()
        },
        entries,
    };

    let output = writer().write(&resource).expect("write should succeed");
    let text = std::str::from_utf8(&output).expect("valid UTF-8");

    assert!(
        text.contains("items_one:"),
        "Expected 'items_one' suffix key in:\n{}",
        text
    );
    assert!(
        text.contains("items_other:"),
        "Expected 'items_other' suffix key in:\n{}",
        text
    );
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
            source_format: FormatId::YamlPlain,
            ..Default::default()
        },
        entries,
    };

    let output = writer().write(&resource).expect("write should succeed");
    let text = std::str::from_utf8(&output).expect("valid UTF-8");

    // Should produce YAML sequence
    assert!(text.contains("colors:"), "Expected 'colors' key");
    assert!(text.contains("- red"), "Expected '- red' in array");
    assert!(text.contains("- green"), "Expected '- green' in array");
    assert!(text.contains("- blue"), "Expected '- blue' in array");
}

#[test]
fn write_empty_resource() {
    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::YamlPlain,
            ..Default::default()
        },
        entries: IndexMap::new(),
    };

    let output = writer().write(&resource).expect("write should succeed");
    let text = std::str::from_utf8(&output).expect("valid UTF-8");
    // Empty YAML: serde_yaml produces "{}\n" for an empty mapping
    assert!(text.trim() == "{}" || text.trim().is_empty());
}

// ──────────────────────────────────────────────
// Roundtrip tests
// ──────────────────────────────────────────────

#[test]
fn roundtrip_simple() {
    let content = fixture("simple.yml");
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
fn roundtrip_nested() {
    let content = fixture("nested.yml");
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
fn roundtrip_plurals() {
    let content = fixture("plurals.yml");
    let resource = parser().parse(&content).expect("parse should succeed");
    let output = writer().write(&resource).expect("write should succeed");
    let reparsed = parser().parse(&output).expect("reparse should succeed");

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, entry) in &resource.entries {
        let reparsed_entry = reparsed.entries.get(key).unwrap_or_else(|| {
            panic!("Key '{}' missing after round-trip", key);
        });
        assert_eq!(entry.value, reparsed_entry.value, "Mismatch for key: {key}");
    }
}

#[test]
fn roundtrip_arrays() {
    let content = fixture("arrays.yml");
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
    let content = fixture("nested.yml");
    let resource = parser().parse(&content).expect("parse should succeed");
    let output = writer().write(&resource).expect("write should succeed");
    let output_str = std::str::from_utf8(&output).expect("valid UTF-8");

    // Verify nested structure (not flat dot-separated keys)
    assert!(
        output_str.contains("common:"),
        "Expected nested 'common' key"
    );
    assert!(output_str.contains("pages:"), "Expected nested 'pages' key");
}

// ──────────────────────────────────────────────
// Format extension tests
// ──────────────────────────────────────────────

#[test]
fn metadata_has_yaml_plain_format_id() {
    let content = fixture("simple.yml");
    let resource = parser().parse(&content).expect("parse should succeed");
    assert_eq!(resource.metadata.source_format, FormatId::YamlPlain);
}

#[test]
fn metadata_has_yaml_plain_extension() {
    let content = fixture("simple.yml");
    let resource = parser().parse(&content).expect("parse should succeed");
    assert!(matches!(
        resource.metadata.format_ext,
        Some(FormatExtension::YamlPlain(YamlPlainExt { .. }))
    ));
}

#[test]
fn no_locale_in_metadata() {
    // Plain YAML doesn't have a locale root key, so no locale in metadata
    let content = fixture("simple.yml");
    let resource = parser().parse(&content).expect("parse should succeed");
    assert_eq!(resource.metadata.locale, None);
}

// ──────────────────────────────────────────────
// Edge cases
// ──────────────────────────────────────────────

#[test]
fn parse_numeric_values() {
    let content = b"version: 42\npi: 3.14\nenabled: true\n";
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

#[test]
fn parse_unicode_values() {
    let content = "japanese: \u{3053}\u{3093}\u{306b}\u{3061}\u{306f}\naccented: caf\u{e9}\n";
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
fn parse_deeply_nested() {
    let content = b"a:\n  b:\n    c:\n      d: deep value\n";
    let resource = parser().parse(content).expect("parse should succeed");
    assert_eq!(resource.entries.len(), 1);
    assert_eq!(
        resource.entries["a.b.c.d"].value,
        EntryValue::Simple("deep value".to_string())
    );
}

#[test]
fn entry_keys_match_entry_key_field() {
    let content = fixture("nested.yml");
    let resource = parser().parse(&content).expect("parse should succeed");

    for (map_key, entry) in &resource.entries {
        assert_eq!(
            map_key, &entry.key,
            "Map key should match entry.key for '{map_key}'"
        );
    }
}

#[test]
fn capabilities_match_spec() {
    let caps = parser().capabilities();
    assert!(caps.plurals);
    assert!(caps.arrays);
    assert!(caps.comments);
    assert!(caps.nested_keys);
    assert!(!caps.context);
    assert!(!caps.source_string);
    assert!(!caps.translatable_flag);
    assert!(!caps.translation_state);
    assert!(!caps.max_width);
    assert!(!caps.device_variants);
    assert!(!caps.select_gender);
    assert!(!caps.inline_markup);
    assert!(!caps.alternatives);
    assert!(!caps.source_references);
    assert!(!caps.custom_properties);
}

#[test]
fn writer_capabilities_match_parser() {
    let parser_caps = parser().capabilities();
    let writer_caps = writer().capabilities();
    assert_eq!(parser_caps, writer_caps);
}

#[test]
fn roundtrip_mixed_flat_and_nested() {
    let content = b"top_level: value\nnested:\n  key: nested value\n";
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
fn plural_suffix_only_other_is_not_grouped() {
    // A single _other key without corresponding _one etc. should still be grouped
    // since _other alone is a valid (minimal) plural
    let content = b"items_other: many items\n";
    let resource = parser().parse(content).expect("parse should succeed");

    // items_other has the _other suffix, so it should become a plural entry
    assert_eq!(resource.entries.len(), 1);
    match &resource.entries["items"].value {
        EntryValue::Plural(ps) => {
            assert_eq!(ps.other, "many items");
            assert_eq!(ps.one, None);
        }
        other => panic!("Expected Plural, got {:?}", other),
    }
}

#[test]
fn parse_placeholder_extraction() {
    let content = b"greeting: \"Hello, {name}!\"\n";
    let resource = parser().parse(content).expect("parse should succeed");

    let entry = &resource.entries["greeting"];
    assert_eq!(entry.placeholders.len(), 1);
    assert_eq!(entry.placeholders[0].name, "name");
    assert_eq!(entry.placeholders[0].original_syntax, "{name}");
}
