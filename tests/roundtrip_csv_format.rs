use i18n_convert::formats::{Confidence, FormatParser, FormatWriter};
use i18n_convert::formats::csv_format::{Parser, Writer};
use i18n_convert::ir::*;
use indexmap::IndexMap;

fn parser() -> Parser {
    Parser
}

fn writer() -> Writer {
    Writer
}

fn fixture(name: &str) -> Vec<u8> {
    let path = format!(
        "{}/tests/fixtures/csv_format/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read(&path).unwrap_or_else(|e| panic!("Failed to read fixture {path}: {e}"))
}

// ──────────────────────────────────────────────
// Detection tests
// ──────────────────────────────────────────────

#[test]
fn detect_csv_extension() {
    assert_eq!(parser().detect(".csv", b"key,en\n"), Confidence::High);
}

#[test]
fn detect_tsv_extension() {
    assert_eq!(parser().detect(".tsv", b"key\ten\n"), Confidence::High);
}

#[test]
fn detect_by_content_with_key_header() {
    let content = b"key,value\ngreeting,Hello\n";
    assert_eq!(parser().detect(".txt", content), Confidence::Low);
}

#[test]
fn detect_no_match() {
    let content = b"just some random text";
    assert_eq!(parser().detect(".json", content), Confidence::None);
}

// ──────────────────────────────────────────────
// Parsing tests
// ──────────────────────────────────────────────

#[test]
fn parse_simple_fixture() {
    let content = fixture("simple.csv");
    let resource = parser().parse(&content).expect("parse should succeed");

    assert_eq!(resource.metadata.source_format, FormatId::Csv);
    assert_eq!(resource.entries.len(), 4);

    assert_eq!(
        resource.entries["greeting"].value,
        EntryValue::Simple("Hello World!".to_string())
    );
    assert_eq!(
        resource.entries["farewell"].value,
        EntryValue::Simple("Goodbye!".to_string())
    );
    assert_eq!(
        resource.entries["app.title"].value,
        EntryValue::Simple("My Application".to_string())
    );
    assert_eq!(
        resource.entries["empty"].value,
        EntryValue::Simple("".to_string())
    );

    // Check comments
    assert_eq!(resource.entries["greeting"].comments.len(), 1);
    assert_eq!(resource.entries["greeting"].comments[0].text, "Main greeting");

    // Empty comment should not be added
    assert!(resource.entries["farewell"].comments.is_empty());
}

#[test]
fn parse_locale_detected() {
    let content = fixture("simple.csv");
    let resource = parser().parse(&content).expect("parse should succeed");

    // The "en" column should be detected as locale
    assert_eq!(resource.metadata.locale, Some("en".to_string()));
}

#[test]
fn parse_quoting_fixture() {
    let content = fixture("quoting.csv");
    let resource = parser().parse(&content).expect("parse should succeed");

    assert_eq!(resource.entries.len(), 4);

    assert_eq!(
        resource.entries["simple"].value,
        EntryValue::Simple("Hello World".to_string())
    );
    assert_eq!(
        resource.entries["with_comma"].value,
        EntryValue::Simple("Hello, World!".to_string())
    );
    assert_eq!(
        resource.entries["with_quotes"].value,
        EntryValue::Simple("She said \"hello\"".to_string())
    );
    assert_eq!(
        resource.entries["with_newline"].value,
        EntryValue::Simple("Line one\nLine two".to_string())
    );
}

#[test]
fn parse_no_comments_fixture() {
    let content = fixture("no_comments.csv");
    let resource = parser().parse(&content).expect("parse should succeed");

    assert_eq!(resource.entries.len(), 3);

    for (_key, entry) in &resource.entries {
        assert!(entry.comments.is_empty());
    }
}

#[test]
fn parse_tsv_fixture() {
    let content = fixture("tab_separated.tsv");
    let resource = parser().parse(&content).expect("parse should succeed");

    assert_eq!(resource.entries.len(), 2);

    assert_eq!(
        resource.entries["greeting"].value,
        EntryValue::Simple("Hello World!".to_string())
    );
    assert_eq!(
        resource.entries["farewell"].value,
        EntryValue::Simple("Goodbye!".to_string())
    );

    // Check delimiter in extension
    match &resource.metadata.format_ext {
        Some(FormatExtension::Csv(ext)) => {
            assert_eq!(ext.delimiter, Some('\t'));
        }
        other => panic!("Expected CsvExt, got {:?}", other),
    }
}

#[test]
fn parse_format_extension_is_set() {
    let content = fixture("simple.csv");
    let resource = parser().parse(&content).expect("parse should succeed");

    match &resource.metadata.format_ext {
        Some(FormatExtension::Csv(ext)) => {
            assert_eq!(ext.delimiter, Some(','));
            assert!(ext.key_column.is_some());
            assert!(ext.value_column.is_some());
        }
        other => panic!("Expected CsvExt, got {:?}", other),
    }
}

#[test]
fn parse_empty_rows_skipped() {
    let content = b"key,value\ngreeting,Hello\n,\nfarewell,Goodbye\n";
    let resource = parser().parse(content).expect("parse should succeed");
    assert_eq!(resource.entries.len(), 2);
}

// ──────────────────────────────────────────────
// Writer tests
// ──────────────────────────────────────────────

#[test]
fn write_simple_entries() {
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
            source_format: FormatId::Csv,
            ..Default::default()
        },
        entries,
    };

    let output = writer().write(&resource).expect("write should succeed");
    let text = String::from_utf8(output).expect("valid UTF-8");

    // Should have header and two data rows
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 3);
    assert!(lines[0].contains("key"));
    assert!(lines[0].contains("value"));
}

#[test]
fn write_with_locale_metadata() {
    let mut entries = IndexMap::new();
    entries.insert(
        "greeting".to_string(),
        I18nEntry {
            key: "greeting".to_string(),
            value: EntryValue::Simple("Hallo".to_string()),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Csv,
            locale: Some("de".to_string()),
            format_ext: Some(FormatExtension::Csv(CsvExt {
                delimiter: Some(','),
                key_column: Some("key".to_string()),
                value_column: Some("de".to_string()),
                has_bom: Some(false),
            })),
            ..Default::default()
        },
        entries,
    };

    let output = writer().write(&resource).expect("write should succeed");
    let text = String::from_utf8(output).expect("valid UTF-8");

    assert!(text.starts_with("key,de\n") || text.starts_with("key,de,"));
}

#[test]
fn write_with_comments() {
    let mut entries = IndexMap::new();
    entries.insert(
        "greeting".to_string(),
        I18nEntry {
            key: "greeting".to_string(),
            value: EntryValue::Simple("Hello".to_string()),
            comments: vec![Comment {
                text: "A greeting".to_string(),
                role: CommentRole::General,
                priority: None,
                annotates: None,
            }],
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Csv,
            ..Default::default()
        },
        entries,
    };

    let output = writer().write(&resource).expect("write should succeed");
    let text = String::from_utf8(output).expect("valid UTF-8");

    assert!(text.contains("comment"));
    assert!(text.contains("A greeting"));
}

#[test]
fn write_quotes_values_with_commas() {
    let mut entries = IndexMap::new();
    entries.insert(
        "key".to_string(),
        I18nEntry {
            key: "key".to_string(),
            value: EntryValue::Simple("Hello, World!".to_string()),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Csv,
            ..Default::default()
        },
        entries,
    };

    let output = writer().write(&resource).expect("write should succeed");
    let text = String::from_utf8(output).expect("valid UTF-8");

    // The csv crate should quote the value containing a comma
    assert!(text.contains("\"Hello, World!\""));
}

#[test]
fn write_empty_resource() {
    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Csv,
            ..Default::default()
        },
        entries: IndexMap::new(),
    };

    let output = writer().write(&resource).expect("write should succeed");
    let text = String::from_utf8(output).expect("valid UTF-8");

    // Should have header only
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 1);
}

// ──────────────────────────────────────────────
// Round-trip tests
// ──────────────────────────────────────────────

#[test]
fn roundtrip_simple() {
    let content = fixture("simple.csv");
    let resource = parser().parse(&content).expect("parse should succeed");
    let output = writer().write(&resource).expect("write should succeed");
    let reparsed = parser().parse(&output).expect("reparse should succeed");

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, original) in &resource.entries {
        let reparsed_entry = reparsed
            .entries
            .get(key)
            .unwrap_or_else(|| panic!("Key '{}' missing after round-trip", key));
        assert_eq!(
            original.value, reparsed_entry.value,
            "Value mismatch for key '{}'",
            key
        );
    }
}

#[test]
fn roundtrip_quoting() {
    let content = fixture("quoting.csv");
    let resource = parser().parse(&content).expect("parse should succeed");
    let output = writer().write(&resource).expect("write should succeed");
    let reparsed = parser().parse(&output).expect("reparse should succeed");

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, original) in &resource.entries {
        let reparsed_entry = reparsed
            .entries
            .get(key)
            .unwrap_or_else(|| panic!("Key '{}' missing after round-trip", key));
        assert_eq!(
            original.value, reparsed_entry.value,
            "Value mismatch for key '{}'",
            key
        );
    }
}

#[test]
fn roundtrip_no_comments() {
    let content = fixture("no_comments.csv");
    let resource = parser().parse(&content).expect("parse should succeed");
    let output = writer().write(&resource).expect("write should succeed");
    let reparsed = parser().parse(&output).expect("reparse should succeed");

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, original) in &resource.entries {
        let reparsed_entry = reparsed
            .entries
            .get(key)
            .unwrap_or_else(|| panic!("Key '{}' missing after round-trip", key));
        assert_eq!(
            original.value, reparsed_entry.value,
            "Value mismatch for key '{}'",
            key
        );
    }
}

#[test]
fn roundtrip_comments_preserved() {
    let content = fixture("simple.csv");
    let resource = parser().parse(&content).expect("parse should succeed");
    let output = writer().write(&resource).expect("write should succeed");
    let reparsed = parser().parse(&output).expect("reparse should succeed");

    for (key, original) in &resource.entries {
        let reparsed_entry = &reparsed.entries[key];
        assert_eq!(
            original.comments.len(),
            reparsed_entry.comments.len(),
            "Comment count mismatch for key '{}'",
            key
        );
        for (i, comment) in original.comments.iter().enumerate() {
            assert_eq!(
                comment.text, reparsed_entry.comments[i].text,
                "Comment text mismatch for key '{}' comment {}",
                key, i
            );
        }
    }
}

// ──────────────────────────────────────────────
// Capabilities test
// ──────────────────────────────────────────────

#[test]
fn capabilities_are_correct() {
    let caps = parser().capabilities();
    assert!(caps.comments);
    assert!(caps.nested_keys);
    assert!(!caps.plurals);
    assert!(!caps.arrays);
    assert!(!caps.context);
    assert!(!caps.source_string);
}

// ──────────────────────────────────────────────
// Edge cases
// ──────────────────────────────────────────────

#[test]
fn entry_keys_match_entry_key_field() {
    let content = fixture("simple.csv");
    let resource = parser().parse(&content).expect("parse should succeed");

    for (map_key, entry) in &resource.entries {
        assert_eq!(
            map_key, &entry.key,
            "Map key should match entry.key for '{}'",
            map_key
        );
    }
}

#[test]
fn parse_with_id_column_name() {
    let content = b"id,en\ngreeting,Hello\nfarewell,Goodbye\n";
    let resource = parser().parse(content).expect("parse should succeed");
    assert_eq!(resource.entries.len(), 2);
    assert_eq!(
        resource.entries["greeting"].value,
        EntryValue::Simple("Hello".to_string())
    );
}

#[test]
fn parse_with_value_column_name() {
    let content = b"key,value\ngreeting,Hello\nfarewell,Goodbye\n";
    let resource = parser().parse(content).expect("parse should succeed");
    assert_eq!(resource.entries.len(), 2);
    assert_eq!(
        resource.entries["greeting"].value,
        EntryValue::Simple("Hello".to_string())
    );
}

#[test]
fn parse_csv_with_bom() {
    let mut content = vec![0xEF, 0xBB, 0xBF]; // UTF-8 BOM
    content.extend_from_slice(b"key,en\ngreeting,Hello\n");
    let resource = parser().parse(&content).expect("parse should succeed");
    assert_eq!(resource.entries.len(), 1);
    assert_eq!(
        resource.entries["greeting"].value,
        EntryValue::Simple("Hello".to_string())
    );

    // Check BOM flag
    match &resource.metadata.format_ext {
        Some(FormatExtension::Csv(ext)) => {
            assert_eq!(ext.has_bom, Some(true));
        }
        other => panic!("Expected CsvExt, got {:?}", other),
    }
}
