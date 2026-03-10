use i18n_convert::formats::ini::{Parser, Writer};
use i18n_convert::formats::{Confidence, FormatParser, FormatWriter};
use i18n_convert::ir::*;
use indexmap::IndexMap;

// ---------------------------------------------------------------------------
// Detection tests
// ---------------------------------------------------------------------------

#[test]
fn detect_by_extension() {
    let parser = Parser;
    assert_eq!(parser.detect(".ini", b""), Confidence::Definite);
}

#[test]
fn detect_by_content() {
    let parser = Parser;
    let content = b"[section]\nkey = value";
    assert_eq!(parser.detect(".txt", content), Confidence::High);
}

#[test]
fn detect_no_match() {
    let parser = Parser;
    assert_eq!(parser.detect(".json", b"{}"), Confidence::None);
}

// ---------------------------------------------------------------------------
// Parse fixture: simple.ini
// ---------------------------------------------------------------------------

#[test]
fn parse_simple_fixture() {
    let content = include_bytes!("fixtures/ini/simple.ini");
    let parser = Parser;
    let resource = parser.parse(content).expect("parse should succeed");

    assert_eq!(resource.metadata.source_format, FormatId::Ini);
    assert_eq!(resource.entries.len(), 4);

    let greeting = resource
        .entries
        .get("general.greeting")
        .expect("general.greeting should exist");
    assert_eq!(
        greeting.value,
        EntryValue::Simple("Hello, World!".to_string())
    );

    let farewell = resource
        .entries
        .get("general.farewell")
        .expect("general.farewell should exist");
    assert_eq!(farewell.value, EntryValue::Simple("Goodbye!".to_string()));

    let welcome = resource
        .entries
        .get("messages.welcome")
        .expect("messages.welcome should exist");
    assert_eq!(
        welcome.value,
        EntryValue::Simple("Welcome to our app".to_string())
    );

    let not_found = resource
        .entries
        .get("messages.error.not_found")
        .expect("messages.error.not_found should exist");
    assert_eq!(
        not_found.value,
        EntryValue::Simple("Page not found".to_string())
    );
}

// ---------------------------------------------------------------------------
// Parse fixture: plurals.ini
// ---------------------------------------------------------------------------

#[test]
fn parse_plurals_fixture() {
    let content = include_bytes!("fixtures/ini/plurals.ini");
    let parser = Parser;
    let resource = parser.parse(content).expect("parse should succeed");

    // Should have 2 plural groups: messages.items and messages.files
    assert_eq!(resource.entries.len(), 2);

    let items = resource
        .entries
        .get("messages.items")
        .expect("messages.items should exist");
    match &items.value {
        EntryValue::Plural(ps) => {
            assert_eq!(ps.one, Some("%d item".to_string()));
            assert_eq!(ps.other, "%d items".to_string());
            assert!(ps.zero.is_none());
        }
        other => panic!("Expected plural value, got {other:?}"),
    }

    let files = resource
        .entries
        .get("messages.files")
        .expect("messages.files should exist");
    match &files.value {
        EntryValue::Plural(ps) => {
            assert_eq!(ps.zero, Some("No files".to_string()));
            assert_eq!(ps.one, Some("%d file".to_string()));
            assert_eq!(ps.other, "%d files".to_string());
        }
        other => panic!("Expected plural value, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Parse fixture: comments.ini
// ---------------------------------------------------------------------------

#[test]
fn parse_comments_fixture() {
    let content = include_bytes!("fixtures/ini/comments.ini");
    let parser = Parser;
    let resource = parser.parse(content).expect("parse should succeed");

    assert_eq!(resource.entries.len(), 3);

    let greeting = resource
        .entries
        .get("general.greeting")
        .expect("greeting should exist");
    assert_eq!(greeting.comments.len(), 1);
    assert_eq!(greeting.comments[0].text, "General greeting");

    let farewell = resource
        .entries
        .get("general.farewell")
        .expect("farewell should exist");
    assert_eq!(farewell.comments.len(), 1);
    assert_eq!(farewell.comments[0].text, "Farewell message");

    let no_comment = resource
        .entries
        .get("general.no_comment")
        .expect("no_comment should exist");
    assert!(no_comment.comments.is_empty());
}

// ---------------------------------------------------------------------------
// Parse fixture: no_sections.ini
// ---------------------------------------------------------------------------

#[test]
fn parse_no_sections_fixture() {
    let content = include_bytes!("fixtures/ini/no_sections.ini");
    let parser = Parser;
    let resource = parser.parse(content).expect("parse should succeed");

    assert_eq!(resource.entries.len(), 3);

    let greeting = resource
        .entries
        .get("greeting")
        .expect("greeting should exist");
    assert_eq!(greeting.value, EntryValue::Simple("Hello".to_string()));
    assert_eq!(greeting.comments.len(), 1);
    assert_eq!(greeting.comments[0].text, "Root level keys");
}

// ---------------------------------------------------------------------------
// Writer tests
// ---------------------------------------------------------------------------

#[test]
fn write_simple_entries() {
    let writer = Writer;

    let mut entries = IndexMap::new();
    entries.insert(
        "general.greeting".to_string(),
        I18nEntry {
            key: "general.greeting".to_string(),
            value: EntryValue::Simple("Hello".to_string()),
            format_ext: Some(FormatExtension::Ini(IniExt {
                section: Some("general".to_string()),
                delimiter: Some('='),
                comment_char: None,
            })),
            ..Default::default()
        },
    );
    entries.insert(
        "general.farewell".to_string(),
        I18nEntry {
            key: "general.farewell".to_string(),
            value: EntryValue::Simple("Goodbye".to_string()),
            format_ext: Some(FormatExtension::Ini(IniExt {
                section: Some("general".to_string()),
                delimiter: Some('='),
                comment_char: None,
            })),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Ini,
            ..Default::default()
        },
        entries,
    };

    let output = writer.write(&resource).expect("write should succeed");
    let text = String::from_utf8(output).expect("should be valid UTF-8");

    assert!(text.contains("[general]"));
    assert!(text.contains("greeting = Hello"));
    assert!(text.contains("farewell = Goodbye"));
}

#[test]
fn write_plural_entries() {
    let writer = Writer;

    let mut entries = IndexMap::new();
    entries.insert(
        "messages.items".to_string(),
        I18nEntry {
            key: "messages.items".to_string(),
            value: EntryValue::Plural(PluralSet {
                one: Some("%d item".to_string()),
                other: "%d items".to_string(),
                ..Default::default()
            }),
            format_ext: Some(FormatExtension::Ini(IniExt {
                section: Some("messages".to_string()),
                delimiter: Some('='),
                comment_char: None,
            })),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Ini,
            ..Default::default()
        },
        entries,
    };

    let output = writer.write(&resource).expect("write should succeed");
    let text = String::from_utf8(output).expect("should be valid UTF-8");

    assert!(text.contains("[messages]"));
    assert!(text.contains("items_one = %d item"));
    assert!(text.contains("items_other = %d items"));
}

// ---------------------------------------------------------------------------
// Round-trip tests
// ---------------------------------------------------------------------------

#[test]
fn roundtrip_simple() {
    let content = include_bytes!("fixtures/ini/simple.ini");
    let parser = Parser;
    let writer = Writer;

    let resource = parser.parse(content).expect("parse should succeed");
    let output = writer.write(&resource).expect("write should succeed");
    let reparsed = parser.parse(&output).expect("reparse should succeed");

    assert_eq!(resource.entries.len(), reparsed.entries.len());

    for (key, original) in &resource.entries {
        let reparsed_entry = reparsed.entries.get(key).unwrap_or_else(|| {
            panic!("Key '{key}' missing after round-trip");
        });
        assert_eq!(
            original.value, reparsed_entry.value,
            "Value mismatch for key '{key}'"
        );
    }
}

#[test]
fn roundtrip_plurals() {
    let content = include_bytes!("fixtures/ini/plurals.ini");
    let parser = Parser;
    let writer = Writer;

    let resource = parser.parse(content).expect("parse should succeed");
    let output = writer.write(&resource).expect("write should succeed");
    let reparsed = parser.parse(&output).expect("reparse should succeed");

    assert_eq!(resource.entries.len(), reparsed.entries.len());

    for (key, original) in &resource.entries {
        let reparsed_entry = reparsed.entries.get(key).unwrap_or_else(|| {
            panic!("Key '{key}' missing after round-trip");
        });
        assert_eq!(
            original.value, reparsed_entry.value,
            "Value mismatch for key '{key}'"
        );
    }
}

#[test]
fn roundtrip_no_sections() {
    let content = include_bytes!("fixtures/ini/no_sections.ini");
    let parser = Parser;
    let writer = Writer;

    let resource = parser.parse(content).expect("parse should succeed");
    let output = writer.write(&resource).expect("write should succeed");
    let reparsed = parser.parse(&output).expect("reparse should succeed");

    assert_eq!(resource.entries.len(), reparsed.entries.len());

    for (key, original) in &resource.entries {
        let reparsed_entry = reparsed.entries.get(key).unwrap_or_else(|| {
            panic!("Key '{key}' missing after round-trip");
        });
        assert_eq!(
            original.value, reparsed_entry.value,
            "Value mismatch for key '{key}'"
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
        .parse(b"; Just a comment\n# Another comment\n")
        .expect("comment-only input should parse");
    assert!(resource.entries.is_empty());
}

#[test]
fn parse_only_section_header() {
    let parser = Parser;
    let resource = parser
        .parse(b"[empty_section]\n")
        .expect("section-only input should parse");
    assert!(resource.entries.is_empty());
}

#[test]
fn capabilities_are_correct() {
    let parser = Parser;
    let caps = parser.capabilities();
    assert!(caps.comments);
    assert!(caps.plurals);
    assert!(caps.nested_keys);
    assert!(!caps.arrays);
    assert!(!caps.context);
}
