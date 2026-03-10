use i18n_convert::formats::srt;
use i18n_convert::formats::{Confidence, FormatParser, FormatWriter};
use i18n_convert::ir::*;
use indexmap::IndexMap;

fn parser() -> srt::Parser {
    srt::Parser
}

fn writer() -> srt::Writer {
    srt::Writer
}

fn fixture(name: &str) -> Vec<u8> {
    let path = format!("{}/tests/fixtures/srt/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read(&path).unwrap_or_else(|e| panic!("Failed to read fixture {path}: {e}"))
}

// ──────────────────────────────────────────────
// Detection tests
// ──────────────────────────────────────────────

#[test]
fn detect_srt_by_extension() {
    let content = b"1\n00:00:01,000 --> 00:00:04,000\nHello\n";
    assert_eq!(parser().detect(".srt", content), Confidence::Definite);
}

#[test]
fn detect_ignores_non_srt_extension() {
    let content = b"1\n00:00:01,000 --> 00:00:04,000\nHello\n";
    assert_eq!(parser().detect(".txt", content), Confidence::High);
}

#[test]
fn detect_no_match_for_random_content() {
    let content = b"This is just some random text.";
    assert_eq!(parser().detect(".txt", content), Confidence::None);
}

// ──────────────────────────────────────────────
// Parsing tests
// ──────────────────────────────────────────────

#[test]
fn parse_basic_srt() {
    let content = fixture("basic.srt");
    let resource = parser().parse(&content).unwrap();

    assert_eq!(resource.metadata.source_format, FormatId::Srt);
    assert_eq!(resource.entries.len(), 3);

    assert_eq!(
        resource.entries["1"].value,
        EntryValue::Simple("Hello and welcome to our show.".to_string())
    );
    assert_eq!(
        resource.entries["2"].value,
        EntryValue::Simple("Today we'll be talking about\nlocalization.".to_string())
    );
    assert_eq!(
        resource.entries["3"].value,
        EntryValue::Simple("Thank you for watching!".to_string())
    );
}

#[test]
fn parse_srt_timecodes() {
    let content = fixture("basic.srt");
    let resource = parser().parse(&content).unwrap();

    let entry1 = &resource.entries["1"];
    assert_eq!(
        entry1.properties.get("srt.start_time").map(|s| s.as_str()),
        Some("00:00:01,000")
    );
    assert_eq!(
        entry1.properties.get("srt.end_time").map(|s| s.as_str()),
        Some("00:00:04,000")
    );
    assert_eq!(
        entry1.properties.get("srt.sequence").map(|s| s.as_str()),
        Some("1")
    );

    let entry2 = &resource.entries["2"];
    assert_eq!(
        entry2.properties.get("srt.start_time").map(|s| s.as_str()),
        Some("00:00:05,500")
    );
    assert_eq!(
        entry2.properties.get("srt.end_time").map(|s| s.as_str()),
        Some("00:00:08,000")
    );
}

#[test]
fn parse_single_entry_srt() {
    let content = fixture("single.srt");
    let resource = parser().parse(&content).unwrap();

    assert_eq!(resource.entries.len(), 1);
    assert_eq!(
        resource.entries["1"].value,
        EntryValue::Simple("A single subtitle entry.".to_string())
    );
}

#[test]
fn parse_multiline_text_srt() {
    let content = fixture("multiline.srt");
    let resource = parser().parse(&content).unwrap();

    assert_eq!(resource.entries.len(), 2);
    assert_eq!(
        resource.entries["1"].value,
        EntryValue::Simple(
            "This is the first line\nof a multi-line subtitle\nwith three lines.".to_string()
        )
    );
    assert_eq!(
        resource.entries["2"].value,
        EntryValue::Simple("Second entry with just one line.".to_string())
    );
}

#[test]
fn parse_srt_format_extension() {
    let content = fixture("basic.srt");
    let resource = parser().parse(&content).unwrap();

    let entry = &resource.entries["1"];
    match &entry.format_ext {
        Some(FormatExtension::Srt(ext)) => {
            assert_eq!(ext.sequence_number, Some(1));
            assert_eq!(ext.start_time, Some("00:00:01,000".to_string()));
            assert_eq!(ext.end_time, Some("00:00:04,000".to_string()));
        }
        other => panic!("Expected SrtExt, got {other:?}"),
    }
}

#[test]
fn parse_empty_srt() {
    let content = b"";
    let resource = parser().parse(content).unwrap();
    assert_eq!(resource.entries.len(), 0);
}

#[test]
fn parse_invalid_sequence_returns_error() {
    let content = b"abc\n00:00:01,000 --> 00:00:04,000\nHello\n";
    let result = parser().parse(content);
    assert!(result.is_err());
}

#[test]
fn parse_invalid_timecode_returns_error() {
    let content = b"1\ninvalid timecode line\nHello\n";
    let result = parser().parse(content);
    assert!(result.is_err());
}

// ──────────────────────────────────────────────
// Writing tests
// ──────────────────────────────────────────────

#[test]
fn write_basic_srt() {
    let mut entries = IndexMap::new();

    let mut entry1 = I18nEntry {
        key: "1".to_string(),
        value: EntryValue::Simple("Hello, world!".to_string()),
        ..Default::default()
    };
    entry1
        .properties
        .insert("srt.sequence".to_string(), "1".to_string());
    entry1
        .properties
        .insert("srt.start_time".to_string(), "00:00:01,000".to_string());
    entry1
        .properties
        .insert("srt.end_time".to_string(), "00:00:04,000".to_string());
    entries.insert("1".to_string(), entry1);

    let mut entry2 = I18nEntry {
        key: "2".to_string(),
        value: EntryValue::Simple("Goodbye!".to_string()),
        ..Default::default()
    };
    entry2
        .properties
        .insert("srt.sequence".to_string(), "2".to_string());
    entry2
        .properties
        .insert("srt.start_time".to_string(), "00:00:05,000".to_string());
    entry2
        .properties
        .insert("srt.end_time".to_string(), "00:00:08,000".to_string());
    entries.insert("2".to_string(), entry2);

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Srt,
            ..Default::default()
        },
        entries,
    };

    let output = writer().write(&resource).unwrap();
    let output_str = std::str::from_utf8(&output).unwrap();

    assert!(output_str.contains("1\n00:00:01,000 --> 00:00:04,000\nHello, world!"));
    assert!(output_str.contains("2\n00:00:05,000 --> 00:00:08,000\nGoodbye!"));
}

#[test]
fn write_srt_with_synthetic_timecodes() {
    let mut entries = IndexMap::new();

    entries.insert(
        "1".to_string(),
        I18nEntry {
            key: "1".to_string(),
            value: EntryValue::Simple("First entry".to_string()),
            ..Default::default()
        },
    );
    entries.insert(
        "2".to_string(),
        I18nEntry {
            key: "2".to_string(),
            value: EntryValue::Simple("Second entry".to_string()),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Srt,
            ..Default::default()
        },
        entries,
    };

    let output = writer().write(&resource).unwrap();
    let output_str = std::str::from_utf8(&output).unwrap();

    // Should contain sequence numbers and some form of timecodes
    assert!(output_str.contains("1\n"));
    assert!(output_str.contains("2\n"));
    assert!(output_str.contains("-->"));
    assert!(output_str.contains("First entry"));
    assert!(output_str.contains("Second entry"));
}

#[test]
fn write_empty_srt() {
    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Srt,
            ..Default::default()
        },
        entries: IndexMap::new(),
    };

    let output = writer().write(&resource).unwrap();
    let output_str = std::str::from_utf8(&output).unwrap();
    assert!(output_str.is_empty());
}

// ──────────────────────────────────────────────
// Round-trip tests
// ──────────────────────────────────────────────

#[test]
fn roundtrip_basic_srt() {
    let content = fixture("basic.srt");
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
fn roundtrip_single_srt() {
    let content = fixture("single.srt");
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
fn roundtrip_multiline_srt() {
    let content = fixture("multiline.srt");
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
fn roundtrip_preserves_timecodes() {
    let content = fixture("basic.srt");
    let resource = parser().parse(&content).unwrap();
    let output = writer().write(&resource).unwrap();
    let reparsed = parser().parse(&output).unwrap();

    for (key, entry) in &resource.entries {
        let reparsed_entry = &reparsed.entries[key];
        assert_eq!(
            entry.properties.get("srt.start_time"),
            reparsed_entry.properties.get("srt.start_time"),
            "Start time mismatch for key: {key}"
        );
        assert_eq!(
            entry.properties.get("srt.end_time"),
            reparsed_entry.properties.get("srt.end_time"),
            "End time mismatch for key: {key}"
        );
        assert_eq!(
            entry.properties.get("srt.sequence"),
            reparsed_entry.properties.get("srt.sequence"),
            "Sequence mismatch for key: {key}"
        );
    }
}

// ──────────────────────────────────────────────
// Capabilities test
// ──────────────────────────────────────────────

#[test]
fn capabilities_reports_custom_properties() {
    let caps = parser().capabilities();
    assert!(caps.custom_properties);
    assert!(!caps.plurals);
    assert!(!caps.arrays);
    assert!(!caps.comments);
    assert!(!caps.nested_keys);
}

// ──────────────────────────────────────────────
// Edge cases
// ──────────────────────────────────────────────

#[test]
fn entry_keys_match_entry_key_field() {
    let content = fixture("basic.srt");
    let resource = parser().parse(&content).unwrap();

    for (map_key, entry) in &resource.entries {
        assert_eq!(
            map_key, &entry.key,
            "Map key should match entry.key for '{map_key}'"
        );
    }
}

#[test]
fn parse_srt_with_bom() {
    let mut content = vec![0xEF, 0xBB, 0xBF]; // UTF-8 BOM
    content.extend_from_slice(b"1\n00:00:01,000 --> 00:00:04,000\nHello\n");
    let resource = parser().parse(&content).unwrap();
    assert_eq!(resource.entries.len(), 1);
    assert_eq!(
        resource.entries["1"].value,
        EntryValue::Simple("Hello".to_string())
    );
}

#[test]
fn write_multiline_text() {
    let mut entries = IndexMap::new();
    let mut entry = I18nEntry {
        key: "1".to_string(),
        value: EntryValue::Simple("Line one\nLine two".to_string()),
        ..Default::default()
    };
    entry
        .properties
        .insert("srt.sequence".to_string(), "1".to_string());
    entry
        .properties
        .insert("srt.start_time".to_string(), "00:00:01,000".to_string());
    entry
        .properties
        .insert("srt.end_time".to_string(), "00:00:04,000".to_string());
    entries.insert("1".to_string(), entry);

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Srt,
            ..Default::default()
        },
        entries,
    };

    let output = writer().write(&resource).unwrap();
    let output_str = std::str::from_utf8(&output).unwrap();

    assert!(output_str.contains("Line one\nLine two"));
}
