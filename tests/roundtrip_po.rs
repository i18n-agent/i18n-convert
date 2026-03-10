use i18n_convert::formats::po;
use i18n_convert::formats::{Confidence, FormatParser, FormatWriter};
use i18n_convert::ir::*;

fn load_fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!("tests/fixtures/po/{name}")).unwrap()
}

// ---------------------------------------------------------------------------
// Detection
// ---------------------------------------------------------------------------

#[test]
fn detect_po_by_extension() {
    assert_eq!(po::Parser.detect(".po", b""), Confidence::Definite);
    assert_eq!(po::Parser.detect(".pot", b""), Confidence::Definite);
    assert_eq!(po::Parser.detect(".txt", b""), Confidence::None);
}

#[test]
fn detect_po_by_content() {
    let content = b"msgid \"hello\"\nmsgstr \"hallo\"";
    assert_eq!(po::Parser.detect(".txt", content), Confidence::Definite);
}

// ---------------------------------------------------------------------------
// Simple
// ---------------------------------------------------------------------------

#[test]
fn parse_simple() {
    let content = load_fixture("simple.po");
    let resource = po::Parser.parse(&content).unwrap();

    // Check metadata
    assert_eq!(resource.metadata.source_format, FormatId::Po);
    assert_eq!(resource.metadata.locale, Some("de".to_string()));
    assert_eq!(
        resource.metadata.headers.get("Content-Type"),
        Some(&"text/plain; charset=UTF-8".to_string())
    );

    // Check entries (3 regular entries, header is not an entry)
    assert_eq!(resource.entries.len(), 3);

    assert_eq!(
        resource.entries["Hello"].value,
        EntryValue::Simple("Hallo".to_string())
    );
    assert_eq!(resource.entries["Hello"].source, Some("Hello".to_string()));
    assert_eq!(
        resource.entries["Goodbye"].value,
        EntryValue::Simple("Auf Wiedersehen".to_string())
    );
    assert_eq!(
        resource.entries["Welcome to %s"].value,
        EntryValue::Simple("Willkommen bei %s".to_string())
    );
}

// ---------------------------------------------------------------------------
// Comments
// ---------------------------------------------------------------------------

#[test]
fn parse_comments() {
    let content = load_fixture("comments.po");
    let resource = po::Parser.parse(&content).unwrap();

    assert_eq!(resource.entries.len(), 2);

    let entry = &resource.entries["File %s has %d lines"];

    // Translator comment
    let translator_comments: Vec<_> = entry
        .comments
        .iter()
        .filter(|c| c.role == CommentRole::Translator)
        .collect();
    assert_eq!(translator_comments.len(), 1);
    assert_eq!(translator_comments[0].text, "This is a translator comment");

    // Extracted comment
    let extracted_comments: Vec<_> = entry
        .comments
        .iter()
        .filter(|c| c.role == CommentRole::Extracted)
        .collect();
    assert_eq!(extracted_comments.len(), 1);
    assert_eq!(extracted_comments[0].text, "This is an extracted comment");

    // Source references
    assert_eq!(entry.source_references.len(), 2);
    assert_eq!(entry.source_references[0].file, "src/main.c");
    assert_eq!(entry.source_references[0].line, Some(42));
    assert_eq!(entry.source_references[1].file, "src/utils.c");
    assert_eq!(entry.source_references[1].line, Some(10));

    // Flags
    assert!(entry.flags.contains(&"c-format".to_string()));

    // Second entry
    let settings = &resource.entries["Settings"];
    let extracted: Vec<_> = settings
        .comments
        .iter()
        .filter(|c| c.role == CommentRole::Extracted)
        .collect();
    assert_eq!(extracted.len(), 1);
    assert_eq!(extracted[0].text, "Developer note: shown on settings page");
}

// ---------------------------------------------------------------------------
// Plurals
// ---------------------------------------------------------------------------

#[test]
fn parse_plurals() {
    let content = load_fixture("plurals.po");
    let resource = po::Parser.parse(&content).unwrap();

    // Check Plural-Forms header was parsed
    assert_eq!(
        resource.metadata.headers.get("Plural-Forms"),
        Some(&"nplurals=2; plural=(n != 1);".to_string())
    );

    // Check PoExt
    if let Some(FormatExtension::Po(ref ext)) = resource.metadata.format_ext {
        assert_eq!(
            ext.plural_forms_header,
            Some("nplurals=2; plural=(n != 1);".to_string())
        );
    } else {
        panic!("Expected PoExt");
    }

    assert_eq!(resource.entries.len(), 2);

    match &resource.entries["%d file"].value {
        EntryValue::Plural(ps) => {
            assert_eq!(ps.one, Some("%d Datei".to_string()));
            assert_eq!(ps.other, "%d Dateien");
        }
        _ => panic!("Expected Plural for '%d file'"),
    }

    match &resource.entries["%d message"].value {
        EntryValue::Plural(ps) => {
            assert_eq!(ps.one, Some("%d Nachricht".to_string()));
            assert_eq!(ps.other, "%d Nachrichten");
        }
        _ => panic!("Expected Plural for '%d message'"),
    }
}

// ---------------------------------------------------------------------------
// Context
// ---------------------------------------------------------------------------

#[test]
fn parse_context() {
    let content = load_fixture("context.po");
    let resource = po::Parser.parse(&content).unwrap();

    assert_eq!(resource.entries.len(), 3);

    // Entries with context use "ctx\x04msgid" as key
    let menu_open = &resource.entries["menu\x04Open"];
    assert_eq!(menu_open.value, EntryValue::Simple("Oeffnen".to_string()));
    assert_eq!(menu_open.contexts.len(), 1);
    assert_eq!(
        menu_open.contexts[0].context_type,
        ContextType::Disambiguation
    );
    assert_eq!(menu_open.contexts[0].value, "menu");

    let button_open = &resource.entries["button\x04Open"];
    assert_eq!(button_open.value, EntryValue::Simple("Oeffnen".to_string()));
    assert_eq!(button_open.contexts[0].value, "button");

    let save_as = &resource.entries["window_title\x04Save As"];
    assert_eq!(
        save_as.value,
        EntryValue::Simple("Speichern unter".to_string())
    );
}

// ---------------------------------------------------------------------------
// Full (previous msgid, obsolete, fuzzy, multiline)
// ---------------------------------------------------------------------------

#[test]
fn parse_full() {
    let content = load_fixture("full.po");
    let resource = po::Parser.parse(&content).unwrap();

    // Check headers
    assert_eq!(
        resource.metadata.headers.get("Project-Id-Version"),
        Some(&"MyApp 1.0".to_string())
    );
    assert_eq!(resource.metadata.locale, Some("de".to_string()));

    // Greeting entry with all comment types
    let greeting = &resource.entries["Welcome, %s!"];
    assert_eq!(
        greeting.value,
        EntryValue::Simple("Willkommen, %s!".to_string())
    );
    assert_eq!(greeting.source, Some("Welcome, %s!".to_string()));
    // Translator comment
    assert!(greeting
        .comments
        .iter()
        .any(|c| c.role == CommentRole::Translator && c.text == "Translator comment for greeting"));
    // Extracted comment
    assert!(
        greeting
            .comments
            .iter()
            .any(|c| c.role == CommentRole::Extracted
                && c.text == "Extracted: shown on the home page")
    );
    // Source references
    assert_eq!(greeting.source_references.len(), 2);
    // Flags
    assert!(greeting.flags.contains(&"c-format".to_string()));

    // Fuzzy entry
    let sure = &resource.entries["Are you sure?"];
    assert_eq!(sure.state, Some(TranslationState::NeedsReview));
    assert!(sure.flags.contains(&"fuzzy".to_string()));

    // Previous msgid entry (plural)
    let item = &resource.entries["%d item"];
    assert_eq!(item.previous_source, Some("old item count".to_string()));
    match &item.value {
        EntryValue::Plural(ps) => {
            assert_eq!(ps.one, Some("%d Gegenstand".to_string()));
            assert_eq!(ps.other, "Gegenstaende");
        }
        _ => panic!("Expected Plural"),
    }

    // Context entry
    let file_entry = &resource.entries["menu\x04File"];
    assert_eq!(file_entry.value, EntryValue::Simple("Datei".to_string()));
    assert_eq!(file_entry.contexts[0].value, "menu");

    // Multiline entry
    let multiline = &resource.entries["This is a long multiline string that spans several lines."];
    assert_eq!(
        multiline.value,
        EntryValue::Simple(
            "Dies ist ein langer mehrzeiliger String, der mehrere Zeilen umfasst.".to_string()
        )
    );

    // Obsolete entry
    let obsolete = &resource.entries["Removed feature"];
    assert!(obsolete.obsolete);
    assert_eq!(
        obsolete.value,
        EntryValue::Simple("Entferntes Feature".to_string())
    );
}

// ---------------------------------------------------------------------------
// Round-trip tests
// ---------------------------------------------------------------------------

#[test]
fn roundtrip_simple() {
    let content = load_fixture("simple.po");
    let resource = po::Parser.parse(&content).unwrap();
    let written = po::Writer.write(&resource).unwrap();
    let reparsed = po::Parser.parse(&written).unwrap();

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, entry) in &resource.entries {
        let reparsed_entry = &reparsed.entries[key];
        assert_eq!(
            entry.value, reparsed_entry.value,
            "Value mismatch for key: {key}"
        );
        assert_eq!(
            entry.source, reparsed_entry.source,
            "Source mismatch for key: {key}"
        );
    }
    // Check locale preserved
    assert_eq!(resource.metadata.locale, reparsed.metadata.locale);
}

#[test]
fn roundtrip_comments() {
    let content = load_fixture("comments.po");
    let resource = po::Parser.parse(&content).unwrap();
    let written = po::Writer.write(&resource).unwrap();
    let reparsed = po::Parser.parse(&written).unwrap();

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, entry) in &resource.entries {
        let reparsed_entry = &reparsed.entries[key];
        assert_eq!(
            entry.value, reparsed_entry.value,
            "Value mismatch for key: {key}"
        );
        // Comment count should match
        assert_eq!(
            entry.comments.len(),
            reparsed_entry.comments.len(),
            "Comment count mismatch for key: {key}"
        );
        // Source references should match
        assert_eq!(
            entry.source_references.len(),
            reparsed_entry.source_references.len(),
            "SourceRef count mismatch for key: {key}"
        );
    }
}

#[test]
fn roundtrip_plurals() {
    let content = load_fixture("plurals.po");
    let resource = po::Parser.parse(&content).unwrap();
    let written = po::Writer.write(&resource).unwrap();
    let reparsed = po::Parser.parse(&written).unwrap();

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, entry) in &resource.entries {
        let reparsed_entry = &reparsed.entries[key];
        assert_eq!(
            entry.value, reparsed_entry.value,
            "Value mismatch for key: {key}"
        );
    }
}

#[test]
fn roundtrip_context() {
    let content = load_fixture("context.po");
    let resource = po::Parser.parse(&content).unwrap();
    let written = po::Writer.write(&resource).unwrap();
    let reparsed = po::Parser.parse(&written).unwrap();

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, entry) in &resource.entries {
        let reparsed_entry = &reparsed.entries[key];
        assert_eq!(
            entry.value, reparsed_entry.value,
            "Value mismatch for key: {key}"
        );
        assert_eq!(
            entry.contexts.len(),
            reparsed_entry.contexts.len(),
            "Context count mismatch for key: {key}"
        );
        for (ctx, rctx) in entry.contexts.iter().zip(reparsed_entry.contexts.iter()) {
            assert_eq!(
                ctx.value, rctx.value,
                "Context value mismatch for key: {key}"
            );
            assert_eq!(
                ctx.context_type, rctx.context_type,
                "Context type mismatch for key: {key}"
            );
        }
    }
}

#[test]
fn roundtrip_full() {
    let content = load_fixture("full.po");
    let resource = po::Parser.parse(&content).unwrap();
    let written = po::Writer.write(&resource).unwrap();
    let reparsed = po::Parser.parse(&written).unwrap();

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, entry) in &resource.entries {
        let reparsed_entry = reparsed
            .entries
            .get(key)
            .unwrap_or_else(|| panic!("Missing key in reparsed: {key}"));
        assert_eq!(
            entry.value, reparsed_entry.value,
            "Value mismatch for key: {key}"
        );
        assert_eq!(
            entry.obsolete, reparsed_entry.obsolete,
            "Obsolete mismatch for key: {key}"
        );
        assert_eq!(
            entry.state, reparsed_entry.state,
            "State mismatch for key: {key}"
        );
    }
}

// ---------------------------------------------------------------------------
// Writer output format
// ---------------------------------------------------------------------------

#[test]
fn writer_produces_valid_po() {
    let content = load_fixture("simple.po");
    let resource = po::Parser.parse(&content).unwrap();
    let written = po::Writer.write(&resource).unwrap();
    let output = std::str::from_utf8(&written).unwrap();

    // Should contain the header
    assert!(output.contains("msgid \"\""));
    assert!(output.contains("msgstr \"\""));
    assert!(output.contains("Language: de"));

    // Should contain entries
    assert!(output.contains("msgid \"Hello\""));
    assert!(output.contains("msgstr \"Hallo\""));
}

#[test]
fn writer_capabilities_match_parser() {
    assert_eq!(po::Parser.capabilities(), po::Writer.capabilities());
}
