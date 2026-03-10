use i18n_convert::formats::{FormatParser, FormatWriter, Confidence};
use i18n_convert::formats::java_properties::{Parser, Writer};
use i18n_convert::ir::*;
use indexmap::IndexMap;

// ---------------------------------------------------------------------------
// Detection tests
// ---------------------------------------------------------------------------

#[test]
fn detect_by_extension() {
    let parser = Parser;
    assert_eq!(parser.detect(".properties", b""), Confidence::Definite);
}

#[test]
fn detect_by_content() {
    let parser = Parser;
    let content = b"greeting = Hello\nfarewell = Goodbye";
    assert_eq!(parser.detect(".txt", content), Confidence::Low);
}

#[test]
fn detect_no_match() {
    let parser = Parser;
    assert_eq!(parser.detect(".json", b"{}"), Confidence::None);
}

// ---------------------------------------------------------------------------
// Parse fixture: simple.properties
// ---------------------------------------------------------------------------

#[test]
fn parse_simple_fixture() {
    let content = include_bytes!("fixtures/java_properties/simple.properties");
    let parser = Parser;
    let resource = parser.parse(content).expect("parse should succeed");

    assert_eq!(resource.metadata.source_format, FormatId::JavaProperties);
    assert_eq!(resource.entries.len(), 4);

    // Check greeting with comment
    let greeting = resource.entries.get("greeting").expect("greeting should exist");
    assert_eq!(greeting.value, EntryValue::Simple("Hello, World!".to_string()));
    assert_eq!(greeting.comments.len(), 1);
    assert_eq!(greeting.comments[0].text, "Application messages");

    // Check app.title (no spaces around =)
    let title = resource.entries.get("app.title").expect("app.title should exist");
    assert_eq!(title.value, EntryValue::Simple("My Application".to_string()));

    // Check empty value
    let empty = resource.entries.get("empty.value").expect("empty.value should exist");
    assert_eq!(empty.value, EntryValue::Simple("".to_string()));
}

// ---------------------------------------------------------------------------
// Parse fixture: escapes.properties
// ---------------------------------------------------------------------------

#[test]
fn parse_escapes_fixture() {
    let content = include_bytes!("fixtures/java_properties/escapes.properties");
    let parser = Parser;
    let resource = parser.parse(content).expect("parse should succeed");

    assert_eq!(resource.entries.len(), 4);

    let newline = resource.entries.get("escaped.newline").expect("escaped.newline should exist");
    assert_eq!(newline.value, EntryValue::Simple("Line one\nLine two".to_string()));

    let tab = resource.entries.get("escaped.tab").expect("escaped.tab should exist");
    assert_eq!(tab.value, EntryValue::Simple("Column1\tColumn2".to_string()));

    let backslash = resource.entries.get("escaped.backslash").expect("escaped.backslash should exist");
    assert_eq!(backslash.value, EntryValue::Simple("C:\\Users\\test".to_string()));

    let unicode = resource.entries.get("escaped.unicode").expect("escaped.unicode should exist");
    assert_eq!(unicode.value, EntryValue::Simple("Hello World".to_string()));
}

// ---------------------------------------------------------------------------
// Parse fixture: separators.properties
// ---------------------------------------------------------------------------

#[test]
fn parse_separators_fixture() {
    let content = include_bytes!("fixtures/java_properties/separators.properties");
    let parser = Parser;
    let resource = parser.parse(content).expect("parse should succeed");

    assert_eq!(resource.entries.len(), 3);

    let eq = resource.entries.get("equals.sep").expect("equals.sep should exist");
    assert_eq!(eq.value, EntryValue::Simple("value with equals".to_string()));

    let colon = resource.entries.get("colon.sep").expect("colon.sep should exist");
    assert_eq!(colon.value, EntryValue::Simple("value with colon".to_string()));

    let compact = resource.entries.get("no.spaces").expect("no.spaces should exist");
    assert_eq!(compact.value, EntryValue::Simple("compact style".to_string()));
}

// ---------------------------------------------------------------------------
// Parse fixture: multiline.properties
// ---------------------------------------------------------------------------

#[test]
fn parse_multiline_fixture() {
    let content = include_bytes!("fixtures/java_properties/multiline.properties");
    let parser = Parser;
    let resource = parser.parse(content).expect("parse should succeed");

    assert_eq!(resource.entries.len(), 2);

    let long = resource.entries.get("long.message").expect("long.message should exist");
    assert_eq!(
        long.value,
        EntryValue::Simple("This is a long value that spans multiple lines".to_string())
    );

    let short = resource.entries.get("short").expect("short should exist");
    assert_eq!(short.value, EntryValue::Simple("simple value".to_string()));
}

// ---------------------------------------------------------------------------
// Parse fixture: comments.properties
// ---------------------------------------------------------------------------

#[test]
fn parse_comments_fixture() {
    let content = include_bytes!("fixtures/java_properties/comments.properties");
    let parser = Parser;
    let resource = parser.parse(content).expect("parse should succeed");

    assert_eq!(resource.entries.len(), 4);

    let greeting = resource.entries.get("greeting").expect("greeting should exist");
    assert_eq!(greeting.comments.len(), 1);
    assert_eq!(greeting.comments[0].text, "Hash comment");

    let farewell = resource.entries.get("farewell").expect("farewell should exist");
    assert_eq!(farewell.comments.len(), 1);
    assert_eq!(farewell.comments[0].text, "Exclamation comment");

    let no_comment = resource.entries.get("no.comment").expect("no.comment should exist");
    assert!(no_comment.comments.is_empty());
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
            source_format: FormatId::JavaProperties,
            ..Default::default()
        },
        entries,
    };

    let output = writer.write(&resource).expect("write should succeed");
    let text = String::from_utf8(output).expect("should be valid UTF-8");

    assert!(text.contains("# A greeting"));
    assert!(text.contains("greeting = Hello"));
    assert!(text.contains("farewell = Goodbye"));
}

#[test]
fn write_escapes_keys_and_values() {
    let writer = Writer;

    let mut entries = IndexMap::new();
    entries.insert(
        "key with spaces".to_string(),
        I18nEntry {
            key: "key with spaces".to_string(),
            value: EntryValue::Simple("hello\nworld".to_string()),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::JavaProperties,
            ..Default::default()
        },
        entries,
    };

    let output = writer.write(&resource).expect("write should succeed");
    let text = String::from_utf8(output).expect("should be valid UTF-8");

    assert!(text.contains("key\\ with\\ spaces"));
    assert!(text.contains("hello\\nworld"));
}

// ---------------------------------------------------------------------------
// Round-trip tests
// ---------------------------------------------------------------------------

#[test]
fn roundtrip_simple() {
    let content = include_bytes!("fixtures/java_properties/simple.properties");
    let parser = Parser;
    let writer = Writer;

    let resource = parser.parse(content).expect("parse should succeed");
    let output = writer.write(&resource).expect("write should succeed");
    let reparsed = parser.parse(&output).expect("reparse should succeed");

    assert_eq!(resource.entries.len(), reparsed.entries.len());

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
    }
}

#[test]
fn roundtrip_escapes() {
    let content = include_bytes!("fixtures/java_properties/escapes.properties");
    let parser = Parser;
    let writer = Writer;

    let resource = parser.parse(content).expect("parse should succeed");
    let output = writer.write(&resource).expect("write should succeed");
    let reparsed = parser.parse(&output).expect("reparse should succeed");

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, original) in &resource.entries {
        let reparsed_entry = reparsed.entries.get(key).expect("key should exist");
        assert_eq!(original.value, reparsed_entry.value, "Value mismatch for key '{}'", key);
    }
}

#[test]
fn roundtrip_comments() {
    let content = include_bytes!("fixtures/java_properties/comments.properties");
    let parser = Parser;
    let writer = Writer;

    let resource = parser.parse(content).expect("parse should succeed");
    let output = writer.write(&resource).expect("write should succeed");
    let reparsed = parser.parse(&output).expect("reparse should succeed");

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, original) in &resource.entries {
        let reparsed_entry = reparsed.entries.get(key).expect("key should exist");
        assert_eq!(original.value, reparsed_entry.value, "Value mismatch for key '{}'", key);
        assert_eq!(
            original.comments.len(),
            reparsed_entry.comments.len(),
            "Comment count mismatch for key '{}'",
            key
        );
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
        .parse(b"# Just a comment\n! Another comment\n")
        .expect("comment-only input should parse");
    assert!(resource.entries.is_empty());
}

#[test]
fn parse_blank_lines() {
    let parser = Parser;
    let input = b"\n\n  \n# comment\n\ngreeting = Hello\n\n";
    let resource = parser.parse(input).expect("should parse with blank lines");
    assert_eq!(resource.entries.len(), 1);
}

#[test]
fn capabilities_are_correct() {
    let parser = Parser;
    let caps = parser.capabilities();
    assert!(caps.comments);
    assert!(caps.nested_keys);
    assert!(!caps.plurals);
    assert!(!caps.arrays);
    assert!(!caps.context);
}
