use i18n_convert::formats::{FormatParser, FormatWriter, Confidence};
use i18n_convert::formats::ios_strings::{Parser, Writer};
use i18n_convert::ir::*;
use indexmap::IndexMap;

// ---------------------------------------------------------------------------
// Detection tests
// ---------------------------------------------------------------------------

#[test]
fn detect_by_extension() {
    let parser = Parser;
    assert_eq!(parser.detect(".strings", b""), Confidence::Definite);
}

#[test]
fn detect_by_content() {
    let parser = Parser;
    let content = b"\"key\" = \"value\";";
    assert_eq!(parser.detect(".txt", content), Confidence::High);
}

#[test]
fn detect_no_match() {
    let parser = Parser;
    assert_eq!(parser.detect(".json", b"{}"), Confidence::None);
}

// ---------------------------------------------------------------------------
// Parse fixture: simple.strings
// ---------------------------------------------------------------------------

#[test]
fn parse_simple_fixture() {
    let content = include_bytes!("fixtures/ios_strings/simple.strings");
    let parser = Parser;
    let resource = parser.parse(content).expect("parse should succeed");

    assert_eq!(resource.metadata.source_format, FormatId::IosStrings);
    assert_eq!(resource.entries.len(), 5);

    // Check first entry
    let app_title = resource.entries.get("app_title").expect("app_title should exist");
    assert_eq!(app_title.value, EntryValue::Simple("My Application".to_string()));
    assert_eq!(app_title.comments.len(), 1);
    assert_eq!(app_title.comments[0].text, "App title");
    assert_eq!(app_title.comments[0].role, CommentRole::General);

    // Check entry with no comment
    let no_comment = resource.entries.get("no_comment").expect("no_comment should exist");
    assert_eq!(
        no_comment.value,
        EntryValue::Simple("This has no comment".to_string())
    );
    assert!(no_comment.comments.is_empty());

    // Check empty value
    let empty = resource.entries.get("empty_value").expect("empty_value should exist");
    assert_eq!(empty.value, EntryValue::Simple("".to_string()));

    // Check dotted key
    let settings = resource
        .entries
        .get("settings.general.title")
        .expect("settings key should exist");
    assert_eq!(
        settings.value,
        EntryValue::Simple("General Settings".to_string())
    );
}

// ---------------------------------------------------------------------------
// Parse fixture: escapes.strings
// ---------------------------------------------------------------------------

#[test]
fn parse_escapes_fixture() {
    let content = include_bytes!("fixtures/ios_strings/escapes.strings");
    let parser = Parser;
    let resource = parser.parse(content).expect("parse should succeed");

    assert_eq!(resource.entries.len(), 5);

    let escaped_quote = resource.entries.get("escaped_quote").unwrap();
    assert_eq!(
        escaped_quote.value,
        EntryValue::Simple("She said \"hello\"".to_string())
    );

    let escaped_backslash = resource.entries.get("escaped_backslash").unwrap();
    assert_eq!(
        escaped_backslash.value,
        EntryValue::Simple("C:\\Users\\test".to_string())
    );

    let newline = resource.entries.get("newline").unwrap();
    assert_eq!(
        newline.value,
        EntryValue::Simple("Line one\nLine two".to_string())
    );

    let tab = resource.entries.get("tab").unwrap();
    assert_eq!(
        tab.value,
        EntryValue::Simple("Column1\tColumn2".to_string())
    );

    let mixed = resource.entries.get("mixed").unwrap();
    assert_eq!(
        mixed.value,
        EntryValue::Simple("Say \"hi\"\nThen press\\tab\there".to_string())
    );
}

// ---------------------------------------------------------------------------
// Parse fixture: unicode.strings
// ---------------------------------------------------------------------------

#[test]
fn parse_unicode_fixture() {
    let content = include_bytes!("fixtures/ios_strings/unicode.strings");
    let parser = Parser;
    let resource = parser.parse(content).expect("parse should succeed");

    assert_eq!(resource.entries.len(), 5);

    let accent = resource.entries.get("accent").unwrap();
    assert_eq!(
        accent.value,
        EntryValue::Simple("caf\u{00E9}".to_string()) // cafe with accent
    );

    let japanese = resource.entries.get("japanese").unwrap();
    assert_eq!(
        japanese.value,
        EntryValue::Simple("\u{3053}\u{3093}\u{306B}\u{3061}\u{306F}".to_string()) // konnichiwa
    );

    let direct = resource.entries.get("direct_unicode").unwrap();
    assert_eq!(
        direct.value,
        EntryValue::Simple("Bonjour le monde".to_string())
    );

    let emoji = resource.entries.get("emoji").unwrap();
    assert_eq!(
        emoji.value,
        EntryValue::Simple("Hello World \u{1F600}".to_string())
    );

    let mixed = resource.entries.get("mixed_unicode").unwrap();
    assert_eq!(
        mixed.value,
        EntryValue::Simple("\u{00FC}ber cool".to_string())
    );
}

// ---------------------------------------------------------------------------
// Writer tests
// ---------------------------------------------------------------------------

#[test]
fn write_simple_entries() {
    let writer = Writer;

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
            source_format: FormatId::IosStrings,
            ..Default::default()
        },
        entries,
    };

    let output = writer.write(&resource).expect("write should succeed");
    let text = String::from_utf8(output).expect("should be valid UTF-8");

    assert!(text.contains("/* A greeting */"));
    assert!(text.contains("\"greeting\" = \"Hello\";"));
    assert!(text.contains("\"farewell\" = \"Goodbye\";"));
}

#[test]
fn write_escapes_values() {
    let writer = Writer;

    let mut entries = IndexMap::new();
    entries.insert(
        "escaped".to_string(),
        I18nEntry {
            key: "escaped".to_string(),
            value: EntryValue::Simple("She said \"hello\"\nNew line\\back".to_string()),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::IosStrings,
            ..Default::default()
        },
        entries,
    };

    let output = writer.write(&resource).expect("write should succeed");
    let text = String::from_utf8(output).expect("should be valid UTF-8");

    assert!(text.contains(r#""escaped" = "She said \"hello\"\nNew line\\back";"#));
}

// ---------------------------------------------------------------------------
// Round-trip tests
// ---------------------------------------------------------------------------

#[test]
fn roundtrip_simple() {
    let content = include_bytes!("fixtures/ios_strings/simple.strings");
    let parser = Parser;
    let writer = Writer;

    let resource = parser.parse(content).expect("parse should succeed");
    let output = writer.write(&resource).expect("write should succeed");
    let reparsed = parser.parse(&output).expect("reparse should succeed");

    // Same number of entries
    assert_eq!(resource.entries.len(), reparsed.entries.len());

    // Same keys, values, and comments
    for (key, original) in &resource.entries {
        let reparsed_entry = reparsed.entries.get(key).unwrap_or_else(|| {
            panic!("Key '{}' missing after round-trip", key);
        });
        assert_eq!(original.value, reparsed_entry.value, "Value mismatch for key '{}'", key);
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

#[test]
fn roundtrip_escapes() {
    let content = include_bytes!("fixtures/ios_strings/escapes.strings");
    let parser = Parser;
    let writer = Writer;

    let resource = parser.parse(content).expect("parse should succeed");
    let output = writer.write(&resource).expect("write should succeed");
    let reparsed = parser.parse(&output).expect("reparse should succeed");

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, original) in &resource.entries {
        let reparsed_entry = reparsed.entries.get(key).unwrap();
        assert_eq!(original.value, reparsed_entry.value, "Value mismatch for key '{}'", key);
    }
}

#[test]
fn roundtrip_unicode() {
    let content = include_bytes!("fixtures/ios_strings/unicode.strings");
    let parser = Parser;
    let writer = Writer;

    let resource = parser.parse(content).expect("parse should succeed");
    let output = writer.write(&resource).expect("write should succeed");
    let reparsed = parser.parse(&output).expect("reparse should succeed");

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, original) in &resource.entries {
        let reparsed_entry = reparsed.entries.get(key).unwrap();
        assert_eq!(original.value, reparsed_entry.value, "Value mismatch for key '{}'", key);
    }
}

// ---------------------------------------------------------------------------
// Edge case tests
// ---------------------------------------------------------------------------

#[test]
fn parse_empty_input() {
    let parser = Parser;
    let resource = parser.parse(b"").expect("empty input should parse");
    assert!(resource.entries.is_empty());
}

#[test]
fn parse_only_comments() {
    let parser = Parser;
    let resource = parser
        .parse(b"/* Just a comment */")
        .expect("comment-only input should parse");
    assert!(resource.entries.is_empty());
}

#[test]
fn parse_single_line_comment() {
    let parser = Parser;
    let input = b"// Single line comment\n\"key\" = \"value\";";
    let resource = parser.parse(input).expect("should parse");
    let entry = resource.entries.get("key").unwrap();
    assert_eq!(entry.value, EntryValue::Simple("value".to_string()));
    assert_eq!(entry.comments.len(), 1);
    assert_eq!(entry.comments[0].text, "Single line comment");
}

#[test]
fn parse_error_unterminated_string() {
    let parser = Parser;
    let input = b"\"key\" = \"unterminated;";
    assert!(parser.parse(input).is_err());
}

#[test]
fn parse_error_missing_semicolon() {
    let parser = Parser;
    let input = b"\"key\" = \"value\"";
    assert!(parser.parse(input).is_err());
}

#[test]
fn capabilities_are_correct() {
    let parser = Parser;
    let caps = parser.capabilities();
    assert!(caps.comments);
    assert!(!caps.plurals);
    assert!(!caps.arrays);
    assert!(!caps.context);
    assert!(!caps.nested_keys);
}
