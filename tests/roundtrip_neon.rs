use i18n_convert::formats::{FormatParser, FormatWriter, Confidence};
use i18n_convert::formats::neon::{Parser, Writer};
use i18n_convert::ir::*;
use indexmap::IndexMap;

// ---------------------------------------------------------------------------
// Detection tests
// ---------------------------------------------------------------------------

#[test]
fn detect_by_extension() {
    let parser = Parser;
    assert_eq!(parser.detect(".neon", b""), Confidence::Definite);
}

#[test]
fn detect_no_match() {
    let parser = Parser;
    assert_eq!(parser.detect(".json", b"{}"), Confidence::None);
}

#[test]
fn detect_no_match_yaml() {
    let parser = Parser;
    assert_eq!(parser.detect(".yml", b"key: value"), Confidence::None);
}

// ---------------------------------------------------------------------------
// Parse fixture: simple.neon
// ---------------------------------------------------------------------------

#[test]
fn parse_simple_fixture() {
    let content = include_bytes!("fixtures/neon/simple.neon");
    let parser = Parser;
    let resource = parser.parse(content).expect("parse should succeed");

    assert_eq!(resource.metadata.source_format, FormatId::Neon);
    assert_eq!(resource.entries.len(), 4);

    let greeting = resource.entries.get("greeting").expect("greeting should exist");
    assert_eq!(greeting.value, EntryValue::Simple("Hello".to_string()));

    let farewell = resource.entries.get("farewell").expect("farewell should exist");
    assert_eq!(farewell.value, EntryValue::Simple("Goodbye".to_string()));

    let app = resource.entries.get("app_name").expect("app_name should exist");
    assert_eq!(app.value, EntryValue::Simple("My App".to_string()));

    let welcome = resource
        .entries
        .get("welcome_message")
        .expect("welcome_message should exist");
    assert_eq!(
        welcome.value,
        EntryValue::Simple("Welcome to our application!".to_string())
    );
}

// ---------------------------------------------------------------------------
// Parse fixture: nested.neon
// ---------------------------------------------------------------------------

#[test]
fn parse_nested_fixture() {
    let content = include_bytes!("fixtures/neon/nested.neon");
    let parser = Parser;
    let resource = parser.parse(content).expect("parse should succeed");

    assert_eq!(resource.entries.len(), 6);

    let welcome = resource
        .entries
        .get("messages.welcome")
        .expect("messages.welcome should exist");
    assert_eq!(welcome.value, EntryValue::Simple("Welcome".to_string()));

    let not_found = resource
        .entries
        .get("messages.error.not_found")
        .expect("messages.error.not_found should exist");
    assert_eq!(
        not_found.value,
        EntryValue::Simple("Page not found".to_string())
    );

    let server = resource
        .entries
        .get("messages.error.server")
        .expect("messages.error.server should exist");
    assert_eq!(
        server.value,
        EntryValue::Simple("Internal server error".to_string())
    );

    let language = resource
        .entries
        .get("settings.language")
        .expect("settings.language should exist");
    assert_eq!(language.value, EntryValue::Simple("Language".to_string()));

    let theme = resource
        .entries
        .get("settings.theme")
        .expect("settings.theme should exist");
    assert_eq!(theme.value, EntryValue::Simple("Theme".to_string()));

    let title = resource
        .entries
        .get("app_title")
        .expect("app_title should exist");
    assert_eq!(
        title.value,
        EntryValue::Simple("My Application".to_string())
    );
}

// ---------------------------------------------------------------------------
// Parse fixture: plurals.neon
// ---------------------------------------------------------------------------

#[test]
fn parse_plurals_fixture() {
    let content = include_bytes!("fixtures/neon/plurals.neon");
    let parser = Parser;
    let resource = parser.parse(content).expect("parse should succeed");

    // Should have 2 plural groups (items, files) + 1 simple (greeting)
    assert_eq!(resource.entries.len(), 3);

    let items = resource.entries.get("items").expect("items should exist");
    match &items.value {
        EntryValue::Plural(ps) => {
            assert_eq!(ps.one, Some("%count% item".to_string()));
            assert_eq!(ps.other, "%count% items".to_string());
            assert!(ps.zero.is_none());
        }
        other => panic!("Expected plural value, got {:?}", other),
    }

    let files = resource.entries.get("files").expect("files should exist");
    match &files.value {
        EntryValue::Plural(ps) => {
            assert_eq!(ps.zero, Some("No files".to_string()));
            assert_eq!(ps.one, Some("%count% file".to_string()));
            assert_eq!(ps.other, "%count% files".to_string());
        }
        other => panic!("Expected plural value, got {:?}", other),
    }

    let greeting = resource.entries.get("greeting").expect("greeting should exist");
    assert_eq!(greeting.value, EntryValue::Simple("Hello".to_string()));
}

// ---------------------------------------------------------------------------
// Parse fixture: comments.neon
// ---------------------------------------------------------------------------

#[test]
fn parse_comments_fixture() {
    let content = include_bytes!("fixtures/neon/comments.neon");
    let parser = Parser;
    let resource = parser.parse(content).expect("parse should succeed");

    let greeting = resource.entries.get("greeting").expect("greeting should exist");
    assert_eq!(greeting.comments.len(), 1);
    assert_eq!(greeting.comments[0].text, "Application strings");

    let farewell = resource.entries.get("farewell").expect("farewell should exist");
    assert_eq!(farewell.comments.len(), 1);
    assert_eq!(farewell.comments[0].text, "Farewell message shown on exit");

    let app = resource.entries.get("app_name").expect("app_name should exist");
    assert!(app.comments.is_empty());

    let home = resource
        .entries
        .get("nav.home")
        .expect("nav.home should exist");
    assert_eq!(home.comments.len(), 1);
    assert_eq!(home.comments[0].text, "Home page link");
}

// ---------------------------------------------------------------------------
// Format extension tests
// ---------------------------------------------------------------------------

#[test]
fn parse_sets_format_extension() {
    let content = include_bytes!("fixtures/neon/simple.neon");
    let parser = Parser;
    let resource = parser.parse(content).expect("parse should succeed");

    match &resource.metadata.format_ext {
        Some(FormatExtension::Neon(ext)) => {
            assert_eq!(*ext, NeonExt {});
        }
        other => panic!("Expected Neon extension, got {:?}", other),
    }
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
            source_format: FormatId::Neon,
            ..Default::default()
        },
        entries,
    };

    let output = writer.write(&resource).expect("write should succeed");
    let text = String::from_utf8(output).expect("should be valid UTF-8");

    assert!(text.contains("greeting: Hello"));
    assert!(text.contains("farewell: Goodbye"));
}

#[test]
fn write_nested_entries() {
    let writer = Writer;

    let mut entries = IndexMap::new();
    entries.insert(
        "messages.welcome".to_string(),
        I18nEntry {
            key: "messages.welcome".to_string(),
            value: EntryValue::Simple("Welcome".to_string()),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Neon,
            ..Default::default()
        },
        entries,
    };

    let output = writer.write(&resource).expect("write should succeed");
    let text = String::from_utf8(output).expect("should be valid UTF-8");

    assert!(text.contains("messages:"));
    assert!(text.contains("\twelcome: Welcome"));
}

#[test]
fn write_plural_entries() {
    let writer = Writer;

    let mut entries = IndexMap::new();
    entries.insert(
        "items".to_string(),
        I18nEntry {
            key: "items".to_string(),
            value: EntryValue::Plural(PluralSet {
                one: Some("%count% item".to_string()),
                other: "%count% items".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Neon,
            ..Default::default()
        },
        entries,
    };

    let output = writer.write(&resource).expect("write should succeed");
    let text = String::from_utf8(output).expect("should be valid UTF-8");

    assert!(text.contains("items_one:"));
    assert!(text.contains("items_other:"));
}

#[test]
fn write_comments() {
    let writer = Writer;

    let mut entries = IndexMap::new();
    entries.insert(
        "greeting".to_string(),
        I18nEntry {
            key: "greeting".to_string(),
            value: EntryValue::Simple("Hello".to_string()),
            comments: vec![Comment {
                text: "A greeting message".to_string(),
                role: CommentRole::General,
                priority: None,
                annotates: None,
            }],
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Neon,
            ..Default::default()
        },
        entries,
    };

    let output = writer.write(&resource).expect("write should succeed");
    let text = String::from_utf8(output).expect("should be valid UTF-8");

    assert!(text.contains("# A greeting message"));
    assert!(text.contains("greeting: Hello"));
}

// ---------------------------------------------------------------------------
// Round-trip tests
// ---------------------------------------------------------------------------

#[test]
fn roundtrip_simple() {
    let content = include_bytes!("fixtures/neon/simple.neon");
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
    }
}

#[test]
fn roundtrip_nested() {
    let content = include_bytes!("fixtures/neon/nested.neon");
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
    }
}

#[test]
fn roundtrip_plurals() {
    let content = include_bytes!("fixtures/neon/plurals.neon");
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
        .parse(b"# Just a comment\n# Another comment\n")
        .expect("comment-only input should parse");
    assert!(resource.entries.is_empty());
}

#[test]
fn parse_quoted_values() {
    let parser = Parser;
    let content = b"single: 'Hello World'\ndouble: \"Goodbye World\"";
    let resource = parser.parse(content).expect("parse should succeed");

    let single = resource.entries.get("single").expect("should exist");
    assert_eq!(single.value, EntryValue::Simple("Hello World".to_string()));

    let double = resource.entries.get("double").expect("should exist");
    assert_eq!(double.value, EntryValue::Simple("Goodbye World".to_string()));
}

#[test]
fn parse_value_with_colon() {
    let parser = Parser;
    let content = b"url: https://example.com";
    let resource = parser.parse(content).expect("parse should succeed");

    let url = resource.entries.get("url").expect("should exist");
    assert_eq!(
        url.value,
        EntryValue::Simple("https://example.com".to_string())
    );
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

#[test]
fn parse_plural_without_other_is_not_grouped() {
    let parser = Parser;
    let content = b"items_one: 1 item\nstuff: hello";
    let resource = parser.parse(content).expect("parse should succeed");

    // items_one without items_other should be treated as a simple entry
    assert_eq!(resource.entries.len(), 2);
    assert!(resource.entries.contains_key("items_one"));
    assert!(resource.entries.contains_key("stuff"));
}

#[test]
fn roundtrip_preserves_entry_count() {
    let parser = Parser;
    let writer = Writer;

    let mut entries = IndexMap::new();
    entries.insert(
        "a".to_string(),
        I18nEntry {
            key: "a".to_string(),
            value: EntryValue::Simple("A".to_string()),
            ..Default::default()
        },
    );
    entries.insert(
        "b.c".to_string(),
        I18nEntry {
            key: "b.c".to_string(),
            value: EntryValue::Simple("BC".to_string()),
            ..Default::default()
        },
    );
    entries.insert(
        "items".to_string(),
        I18nEntry {
            key: "items".to_string(),
            value: EntryValue::Plural(PluralSet {
                one: Some("1 item".to_string()),
                other: "many items".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Neon,
            ..Default::default()
        },
        entries,
    };

    let output = writer.write(&resource).expect("write should succeed");
    let reparsed = parser.parse(&output).expect("reparse should succeed");

    assert_eq!(reparsed.entries.len(), 3);

    let a = reparsed.entries.get("a").expect("a should exist");
    assert_eq!(a.value, EntryValue::Simple("A".to_string()));

    let bc = reparsed.entries.get("b.c").expect("b.c should exist");
    assert_eq!(bc.value, EntryValue::Simple("BC".to_string()));

    let items = reparsed.entries.get("items").expect("items should exist");
    match &items.value {
        EntryValue::Plural(ps) => {
            assert_eq!(ps.one, Some("1 item".to_string()));
            assert_eq!(ps.other, "many items");
        }
        other => panic!("Expected plural, got {:?}", other),
    }
}

#[test]
fn write_uses_tab_indentation() {
    let writer = Writer;

    let mut entries = IndexMap::new();
    entries.insert(
        "a.b".to_string(),
        I18nEntry {
            key: "a.b".to_string(),
            value: EntryValue::Simple("val".to_string()),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Neon,
            ..Default::default()
        },
        entries,
    };

    let output = writer.write(&resource).expect("write should succeed");
    let text = String::from_utf8(output).expect("should be valid UTF-8");

    // Verify tab indentation is used
    assert!(text.contains("\tb: val"));
}
