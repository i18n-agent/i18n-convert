use i18n_convert::formats::{FormatParser, FormatWriter, Confidence};
use i18n_convert::formats::markdown::{Parser, Writer};
use i18n_convert::ir::*;
use indexmap::IndexMap;

// ---------------------------------------------------------------------------
// Detection tests
// ---------------------------------------------------------------------------

#[test]
fn detect_by_extension_with_heading() {
    let parser = Parser;
    let content = b"# Hello\n\nWorld";
    assert_eq!(parser.detect(".md", content), Confidence::High);
}

#[test]
fn detect_by_extension_with_front_matter() {
    let parser = Parser;
    let content = b"---\nlocale: en\n---\n\n# Hello";
    assert_eq!(parser.detect(".md", content), Confidence::High);
}

#[test]
fn detect_by_extension_plain() {
    let parser = Parser;
    assert_eq!(parser.detect(".md", b"just text"), Confidence::Low);
}

#[test]
fn detect_no_match() {
    let parser = Parser;
    assert_eq!(parser.detect(".json", b"{}"), Confidence::None);
}

// ---------------------------------------------------------------------------
// Parse fixture: simple.md
// ---------------------------------------------------------------------------

#[test]
fn parse_simple_fixture() {
    let content = include_bytes!("fixtures/markdown/simple.md");
    let parser = Parser;
    let resource = parser.parse(content).expect("parse should succeed");

    assert_eq!(resource.metadata.source_format, FormatId::Markdown);
    assert_eq!(resource.metadata.locale, Some("en".to_string()));

    // Check front matter was stored
    match &resource.metadata.format_ext {
        Some(FormatExtension::Markdown(ext)) => {
            assert!(ext.front_matter.is_some());
            let fm = ext.front_matter.as_ref().expect("front_matter");
            assert!(fm.contains("locale: en"));
        }
        other => panic!("Expected MarkdownExt, got {:?}", other),
    }

    // Check entries
    assert!(resource.entries.len() >= 4, "Expected at least 4 entries, got {}", resource.entries.len());

    // "welcome"
    let welcome = resource.entries.get("welcome").expect("welcome should exist");
    match &welcome.value {
        EntryValue::Simple(s) => {
            assert!(s.contains("Hello and welcome"), "welcome value: {}", s);
        }
        other => panic!("Expected Simple, got {:?}", other),
    }

    // "welcome.getting-started" (h2 nested under h1 "Welcome")
    let gs = resource.entries.get("welcome.getting-started").expect("welcome.getting-started should exist");
    match &gs.value {
        EntryValue::Simple(s) => {
            assert!(s.contains("Follow these steps"), "welcome.getting-started value: {}", s);
        }
        other => panic!("Expected Simple, got {:?}", other),
    }

    // Nested heading: "welcome.faq.how-do-i-reset-my-password" (h3 under h2 "FAQ" under h1 "Welcome")
    let faq_reset = resource
        .entries
        .get("welcome.faq.how-do-i-reset-my-password")
        .expect("welcome.faq.how-do-i-reset-my-password should exist");
    match &faq_reset.value {
        EntryValue::Simple(s) => {
            assert!(
                s.contains("Settings > Security"),
                "welcome.faq.how-do-i-reset-my-password value: {}",
                s
            );
        }
        other => panic!("Expected Simple, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// Parse fixture: no_front_matter.md
// ---------------------------------------------------------------------------

#[test]
fn parse_no_front_matter() {
    let content = include_bytes!("fixtures/markdown/no_front_matter.md");
    let parser = Parser;
    let resource = parser.parse(content).expect("parse should succeed");

    assert_eq!(resource.metadata.locale, None);

    match &resource.metadata.format_ext {
        Some(FormatExtension::Markdown(ext)) => {
            assert!(ext.front_matter.is_none());
        }
        other => panic!("Expected MarkdownExt, got {:?}", other),
    }

    assert!(resource.entries.contains_key("about"));
    assert!(resource.entries.contains_key("about.team"));
    assert!(resource.entries.contains_key("about.contact"));
}

// ---------------------------------------------------------------------------
// Parse edge cases
// ---------------------------------------------------------------------------

#[test]
fn parse_empty_input() {
    let parser = Parser;
    let resource = parser.parse(b"").expect("empty input should parse");
    assert!(resource.entries.is_empty());
}

#[test]
fn parse_heading_only_no_content() {
    let parser = Parser;
    let resource = parser.parse(b"# Title\n").expect("should parse");
    // A heading with no content should not produce an entry
    assert!(resource.entries.is_empty());
}

#[test]
fn parse_heading_with_content() {
    let parser = Parser;
    let resource = parser
        .parse(b"# Title\n\nSome content here.")
        .expect("should parse");
    assert_eq!(resource.entries.len(), 1);
    let entry = resource.entries.get("title").expect("title should exist");
    assert_eq!(entry.value, EntryValue::Simple("Some content here.".to_string()));
}

#[test]
fn parse_preserves_paragraphs() {
    let parser = Parser;
    let input = b"# Section\n\nParagraph one.\n\nParagraph two.";
    let resource = parser.parse(input).expect("should parse");
    let entry = resource.entries.get("section").expect("section");
    match &entry.value {
        EntryValue::Simple(s) => {
            assert!(s.contains("Paragraph one.\n\nParagraph two."), "Got: {}", s);
        }
        other => panic!("Expected Simple, got {:?}", other),
    }
}

#[test]
fn parse_kebab_case_conversion() {
    let parser = Parser;
    let input = b"# My Cool Feature!\n\nDescription.";
    let resource = parser.parse(input).expect("should parse");
    assert!(resource.entries.contains_key("my-cool-feature"), "keys: {:?}", resource.entries.keys().collect::<Vec<_>>());
}

// ---------------------------------------------------------------------------
// Writer tests
// ---------------------------------------------------------------------------

#[test]
fn write_simple_entries() {
    let writer = Writer;

    let mut entries = IndexMap::new();
    entries.insert(
        "welcome".to_string(),
        I18nEntry {
            key: "welcome".to_string(),
            value: EntryValue::Simple("Hello and welcome.".to_string()),
            ..Default::default()
        },
    );
    entries.insert(
        "welcome.getting-started".to_string(),
        I18nEntry {
            key: "welcome.getting-started".to_string(),
            value: EntryValue::Simple("Follow these steps.".to_string()),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Markdown,
            format_ext: Some(FormatExtension::Markdown(MarkdownExt {
                front_matter: Some("locale: en".to_string()),
            })),
            ..Default::default()
        },
        entries,
    };

    let output = writer.write(&resource).expect("write should succeed");
    let text = String::from_utf8(output).expect("valid UTF-8");

    assert!(text.starts_with("---\n"), "Should start with front matter");
    assert!(text.contains("locale: en"), "Should contain locale");
    assert!(text.contains("# Welcome"), "Should contain h1 heading");
    assert!(text.contains("## Getting Started"), "Should contain h2 heading");
    assert!(text.contains("Hello and welcome."), "Should contain content");
    assert!(text.contains("Follow these steps."), "Should contain nested content");
}

#[test]
fn write_no_front_matter() {
    let writer = Writer;

    let mut entries = IndexMap::new();
    entries.insert(
        "about".to_string(),
        I18nEntry {
            key: "about".to_string(),
            value: EntryValue::Simple("About us.".to_string()),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Markdown,
            ..Default::default()
        },
        entries,
    };

    let output = writer.write(&resource).expect("write should succeed");
    let text = String::from_utf8(output).expect("valid UTF-8");
    assert!(!text.starts_with("---"), "Should not have front matter");
    assert!(text.contains("# About"), "Should contain heading");
}

// ---------------------------------------------------------------------------
// Round-trip tests
// ---------------------------------------------------------------------------

#[test]
fn roundtrip_simple() {
    let content = include_bytes!("fixtures/markdown/simple.md");
    let parser = Parser;
    let writer = Writer;

    let resource = parser.parse(content).expect("parse should succeed");
    let output = writer.write(&resource).expect("write should succeed");
    let reparsed = parser.parse(&output).expect("reparse should succeed");

    // Same number of entries
    assert_eq!(
        resource.entries.len(),
        reparsed.entries.len(),
        "Entry count mismatch: original={}, reparsed={}",
        resource.entries.len(),
        reparsed.entries.len()
    );

    // Same keys
    for key in resource.entries.keys() {
        assert!(
            reparsed.entries.contains_key(key),
            "Key '{}' missing after round-trip",
            key
        );
    }

    // Values should be semantically equivalent (whitespace may differ slightly)
    for (key, original) in &resource.entries {
        let reparsed_entry = reparsed.entries.get(key).expect("key should exist");
        match (&original.value, &reparsed_entry.value) {
            (EntryValue::Simple(a), EntryValue::Simple(b)) => {
                // Normalize whitespace for comparison
                let a_norm = a.trim();
                let b_norm = b.trim();
                assert_eq!(a_norm, b_norm, "Value mismatch for key '{}'", key);
            }
            _ => {
                assert_eq!(original.value, reparsed_entry.value, "Value mismatch for key '{}'", key);
            }
        }
    }
}

#[test]
fn roundtrip_no_front_matter() {
    let content = include_bytes!("fixtures/markdown/no_front_matter.md");
    let parser = Parser;
    let writer = Writer;

    let resource = parser.parse(content).expect("parse should succeed");
    let output = writer.write(&resource).expect("write should succeed");
    let reparsed = parser.parse(&output).expect("reparse should succeed");

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for key in resource.entries.keys() {
        assert!(
            reparsed.entries.contains_key(key),
            "Key '{}' missing after round-trip",
            key
        );
    }
}

#[test]
fn roundtrip_preserves_front_matter() {
    let content = include_bytes!("fixtures/markdown/simple.md");
    let parser = Parser;
    let writer = Writer;

    let resource = parser.parse(content).expect("parse should succeed");
    let output = writer.write(&resource).expect("write should succeed");
    let reparsed = parser.parse(&output).expect("reparse should succeed");

    // Front matter should be preserved
    match (&resource.metadata.format_ext, &reparsed.metadata.format_ext) {
        (
            Some(FormatExtension::Markdown(orig)),
            Some(FormatExtension::Markdown(re)),
        ) => {
            assert_eq!(orig.front_matter, re.front_matter, "Front matter mismatch");
        }
        _ => panic!("Expected MarkdownExt in both"),
    }

    // Locale should be preserved
    assert_eq!(resource.metadata.locale, reparsed.metadata.locale);
}

#[test]
fn roundtrip_programmatic() {
    let parser = Parser;
    let writer = Writer;

    let mut entries = IndexMap::new();
    entries.insert(
        "intro".to_string(),
        I18nEntry {
            key: "intro".to_string(),
            value: EntryValue::Simple("Welcome to the app.".to_string()),
            ..Default::default()
        },
    );
    entries.insert(
        "intro.features".to_string(),
        I18nEntry {
            key: "intro.features".to_string(),
            value: EntryValue::Simple("Here are the features:\n\n1. Fast\n2. Reliable".to_string()),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Markdown,
            format_ext: Some(FormatExtension::Markdown(MarkdownExt {
                front_matter: Some("locale: en".to_string()),
            })),
            ..Default::default()
        },
        entries,
    };

    let output = writer.write(&resource).expect("write should succeed");
    let reparsed = parser.parse(&output).expect("reparse should succeed");

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, original) in &resource.entries {
        let reparsed_entry = reparsed.entries.get(key).expect("key should exist");
        match (&original.value, &reparsed_entry.value) {
            (EntryValue::Simple(a), EntryValue::Simple(b)) => {
                assert_eq!(a.trim(), b.trim(), "Value mismatch for key '{}'", key);
            }
            _ => {
                assert_eq!(original.value, reparsed_entry.value);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Capabilities test
// ---------------------------------------------------------------------------

#[test]
fn capabilities_are_correct() {
    let parser = Parser;
    let caps = parser.capabilities();
    assert!(!caps.plurals);
    assert!(!caps.arrays);
    assert!(!caps.comments);
    assert!(!caps.context);
    assert!(caps.nested_keys);
    assert!(caps.inline_markup);
    assert!(caps.custom_properties);
}
