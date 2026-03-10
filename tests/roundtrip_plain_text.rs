use i18n_convert::formats::plain_text;
use i18n_convert::formats::{Confidence, FormatParser, FormatWriter};
use i18n_convert::ir::*;
use indexmap::IndexMap;

fn parser() -> plain_text::Parser {
    plain_text::Parser
}

fn writer() -> plain_text::Writer {
    plain_text::Writer
}

fn fixture(name: &str) -> Vec<u8> {
    let path = format!(
        "{}/tests/fixtures/plain_text/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read(&path).unwrap_or_else(|e| panic!("Failed to read fixture {path}: {e}"))
}

// ──────────────────────────────────────────────
// Detection tests
// ──────────────────────────────────────────────

#[test]
fn detect_txt_extension() {
    assert_eq!(parser().detect(".txt", b""), Confidence::Definite);
}

#[test]
fn detect_txt_extension_with_content() {
    assert_eq!(
        parser().detect(".txt", b"Hello, world!"),
        Confidence::Definite
    );
}

#[test]
fn detect_non_txt_extensions() {
    assert_eq!(parser().detect(".json", b""), Confidence::None);
    assert_eq!(parser().detect(".yml", b""), Confidence::None);
    assert_eq!(parser().detect(".xml", b""), Confidence::None);
    assert_eq!(parser().detect(".md", b""), Confidence::None);
    assert_eq!(parser().detect(".ini", b""), Confidence::None);
}

// ──────────────────────────────────────────────
// Parse fixture tests
// ──────────────────────────────────────────────

#[test]
fn parse_simple_fixture() {
    let content = fixture("simple.txt");
    let resource = parser().parse(&content).expect("parse should succeed");

    assert_eq!(resource.metadata.source_format, FormatId::PlainText);
    assert_eq!(resource.entries.len(), 1);
    assert_eq!(
        resource.entries["content"].value,
        EntryValue::Simple("Hello, welcome to our application.".to_string())
    );
}

#[test]
fn parse_multiline_fixture() {
    let content = fixture("multiline.txt");
    let resource = parser().parse(&content).expect("parse should succeed");

    assert_eq!(resource.entries.len(), 1);
    assert_eq!(
        resource.entries["content"].value,
        EntryValue::Simple(
            "Hello, welcome to our application.\nThis is the second line.\nAnd this is the third line.".to_string()
        )
    );
}

#[test]
fn parse_sections_fixture() {
    let content = fixture("sections.txt");
    let resource = parser().parse(&content).expect("parse should succeed");

    assert_eq!(resource.entries.len(), 3);
    assert_eq!(
        resource.entries["content.0"].value,
        EntryValue::Simple("First section content.\nThis belongs to section one.".to_string())
    );
    assert_eq!(
        resource.entries["content.1"].value,
        EntryValue::Simple("Second section content.\nThis belongs to section two.".to_string())
    );
    assert_eq!(
        resource.entries["content.2"].value,
        EntryValue::Simple("Third section content.".to_string())
    );
}

#[test]
fn parse_empty_fixture() {
    let content = fixture("empty.txt");
    let resource = parser().parse(&content).expect("parse should succeed");
    assert_eq!(resource.entries.len(), 0);
    assert_eq!(resource.metadata.source_format, FormatId::PlainText);
}

#[test]
fn parse_single_line_no_newline() {
    let content = b"Hello, world!";
    let resource = parser().parse(content).expect("parse should succeed");

    assert_eq!(resource.entries.len(), 1);
    assert_eq!(
        resource.entries["content"].value,
        EntryValue::Simple("Hello, world!".to_string())
    );
}

#[test]
fn parse_single_line_with_newline() {
    let content = b"Hello, world!\n";
    let resource = parser().parse(content).expect("parse should succeed");

    assert_eq!(resource.entries.len(), 1);
    assert_eq!(
        resource.entries["content"].value,
        EntryValue::Simple("Hello, world!".to_string())
    );
}

// ──────────────────────────────────────────────
// Writer tests
// ──────────────────────────────────────────────

#[test]
fn write_simple_content() {
    let mut entries = IndexMap::new();
    entries.insert(
        "content".to_string(),
        I18nEntry {
            key: "content".to_string(),
            value: EntryValue::Simple("Hello, world!".to_string()),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::PlainText,
            format_ext: Some(FormatExtension::PlainText(PlainTextExt {
                line_ending: Some("\n".to_string()),
            })),
            ..Default::default()
        },
        entries,
    };

    let output = writer().write(&resource).expect("write should succeed");
    let text = std::str::from_utf8(&output).expect("valid UTF-8");
    assert_eq!(text, "Hello, world!\n");
}

#[test]
fn write_multiline_content() {
    let mut entries = IndexMap::new();
    entries.insert(
        "content".to_string(),
        I18nEntry {
            key: "content".to_string(),
            value: EntryValue::Simple("Line one.\nLine two.".to_string()),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::PlainText,
            format_ext: Some(FormatExtension::PlainText(PlainTextExt {
                line_ending: Some("\n".to_string()),
            })),
            ..Default::default()
        },
        entries,
    };

    let output = writer().write(&resource).expect("write should succeed");
    let text = std::str::from_utf8(&output).expect("valid UTF-8");
    assert_eq!(text, "Line one.\nLine two.\n");
}

#[test]
fn write_sections_with_delimiters() {
    let mut entries = IndexMap::new();
    entries.insert(
        "content.0".to_string(),
        I18nEntry {
            key: "content.0".to_string(),
            value: EntryValue::Simple("Section one.".to_string()),
            ..Default::default()
        },
    );
    entries.insert(
        "content.1".to_string(),
        I18nEntry {
            key: "content.1".to_string(),
            value: EntryValue::Simple("Section two.".to_string()),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::PlainText,
            format_ext: Some(FormatExtension::PlainText(PlainTextExt {
                line_ending: Some("\n".to_string()),
            })),
            ..Default::default()
        },
        entries,
    };

    let output = writer().write(&resource).expect("write should succeed");
    let text = std::str::from_utf8(&output).expect("valid UTF-8");

    assert!(text.contains("Section one."));
    assert!(text.contains("---"));
    assert!(text.contains("Section two."));
}

#[test]
fn write_empty_resource() {
    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::PlainText,
            format_ext: Some(FormatExtension::PlainText(PlainTextExt {
                line_ending: Some("\n".to_string()),
            })),
            ..Default::default()
        },
        entries: IndexMap::new(),
    };

    let output = writer().write(&resource).expect("write should succeed");
    assert!(output.is_empty());
}

#[test]
fn write_output_ends_with_newline() {
    let mut entries = IndexMap::new();
    entries.insert(
        "content".to_string(),
        I18nEntry {
            key: "content".to_string(),
            value: EntryValue::Simple("Hello".to_string()),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::PlainText,
            format_ext: Some(FormatExtension::PlainText(PlainTextExt {
                line_ending: Some("\n".to_string()),
            })),
            ..Default::default()
        },
        entries,
    };

    let output = writer().write(&resource).expect("write should succeed");
    assert!(output.ends_with(b"\n"));
}

// ──────────────────────────────────────────────
// Roundtrip tests
// ──────────────────────────────────────────────

#[test]
fn roundtrip_simple() {
    let content = fixture("simple.txt");
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
fn roundtrip_multiline() {
    let content = fixture("multiline.txt");
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
fn roundtrip_sections() {
    let content = fixture("sections.txt");
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

// ──────────────────────────────────────────────
// Format extension tests
// ──────────────────────────────────────────────

#[test]
fn metadata_has_plain_text_format_id() {
    let content = fixture("simple.txt");
    let resource = parser().parse(&content).expect("parse should succeed");
    assert_eq!(resource.metadata.source_format, FormatId::PlainText);
}

#[test]
fn metadata_has_plain_text_extension() {
    let content = fixture("simple.txt");
    let resource = parser().parse(&content).expect("parse should succeed");
    assert!(matches!(
        resource.metadata.format_ext,
        Some(FormatExtension::PlainText(PlainTextExt { .. }))
    ));
}

#[test]
fn line_ending_detected_lf() {
    let content = b"Hello\nWorld\n";
    let resource = parser().parse(content).expect("parse should succeed");

    match &resource.metadata.format_ext {
        Some(FormatExtension::PlainText(ext)) => {
            assert_eq!(ext.line_ending, Some("\n".to_string()));
        }
        other => panic!("Expected PlainText extension, got {:?}", other),
    }
}

#[test]
fn line_ending_detected_crlf() {
    let content = b"Hello\r\nWorld\r\n";
    let resource = parser().parse(content).expect("parse should succeed");

    match &resource.metadata.format_ext {
        Some(FormatExtension::PlainText(ext)) => {
            assert_eq!(ext.line_ending, Some("\r\n".to_string()));
        }
        other => panic!("Expected PlainText extension, got {:?}", other),
    }
}

#[test]
fn crlf_roundtrip_preserves_content() {
    let content = b"Hello\r\nWorld\r\n";
    let resource = parser().parse(content).expect("parse should succeed");
    let output = writer().write(&resource).expect("write should succeed");
    let reparsed = parser().parse(&output).expect("reparse should succeed");

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, entry) in &resource.entries {
        assert_eq!(entry.value, reparsed.entries[key].value, "Mismatch for key: {key}");
    }
}

// ──────────────────────────────────────────────
// Edge cases
// ──────────────────────────────────────────────

#[test]
fn parse_unicode_content() {
    let content = "\u{3053}\u{3093}\u{306b}\u{3061}\u{306f}\u{4e16}\u{754c}\n";
    let resource = parser()
        .parse(content.as_bytes())
        .expect("parse should succeed");

    assert_eq!(resource.entries.len(), 1);
    assert_eq!(
        resource.entries["content"].value,
        EntryValue::Simple("\u{3053}\u{3093}\u{306b}\u{3061}\u{306f}\u{4e16}\u{754c}".to_string())
    );
}

#[test]
fn parse_whitespace_only() {
    let content = b"   \n  \n   \n";
    let resource = parser().parse(content).expect("parse should succeed");

    // Whitespace-only content should still produce an entry
    assert_eq!(resource.entries.len(), 1);
}

#[test]
fn entry_keys_match_entry_key_field() {
    let content = fixture("sections.txt");
    let resource = parser().parse(&content).expect("parse should succeed");

    for (map_key, entry) in &resource.entries {
        assert_eq!(
            map_key, &entry.key,
            "Map key should match entry.key for '{map_key}'"
        );
    }
}

#[test]
fn capabilities_all_false() {
    let caps = parser().capabilities();
    assert!(!caps.plurals);
    assert!(!caps.arrays);
    assert!(!caps.comments);
    assert!(!caps.context);
    assert!(!caps.source_string);
    assert!(!caps.translatable_flag);
    assert!(!caps.translation_state);
    assert!(!caps.max_width);
    assert!(!caps.device_variants);
    assert!(!caps.select_gender);
    assert!(!caps.nested_keys);
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
fn no_locale_in_metadata() {
    let content = fixture("simple.txt");
    let resource = parser().parse(&content).expect("parse should succeed");
    assert_eq!(resource.metadata.locale, None);
}

#[test]
fn parse_content_with_delimiter_like_text() {
    // Content that has "---" as part of regular text should be treated as section delimiter
    let content = b"Before\n---\nAfter";
    let resource = parser().parse(content).expect("parse should succeed");

    assert_eq!(resource.entries.len(), 2);
    assert_eq!(
        resource.entries["content.0"].value,
        EntryValue::Simple("Before".to_string())
    );
    assert_eq!(
        resource.entries["content.1"].value,
        EntryValue::Simple("After".to_string())
    );
}
