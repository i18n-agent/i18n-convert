use i18n_convert::formats::qt_linguist;
use i18n_convert::formats::Confidence;
use i18n_convert::formats::FormatParser;
use i18n_convert::formats::FormatWriter;
use i18n_convert::ir::*;

fn load_fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!("tests/fixtures/qt_linguist/{name}")).expect("fixture file should exist")
}

/// The context separator used internally for keys
const CTX_SEP: &str = "\x04";

// ---------------------------------------------------------------------------
// Detection tests
// ---------------------------------------------------------------------------

#[test]
fn detect_ts_with_doctype() {
    let parser = qt_linguist::Parser;
    let content = b"<?xml version=\"1.0\"?><!DOCTYPE TS><TS version=\"2.1\"></TS>";
    assert_eq!(parser.detect(".ts", content), Confidence::Definite);
}

#[test]
fn detect_ts_without_doctype() {
    let parser = qt_linguist::Parser;
    let content = b"<?xml version=\"1.0\"?><TS version=\"2.1\"></TS>";
    assert_eq!(parser.detect(".ts", content), Confidence::Definite);
}

#[test]
fn detect_ts_extension_without_content() {
    let parser = qt_linguist::Parser;
    // .ts extension alone without Qt content should not match (could be TypeScript)
    assert_eq!(parser.detect(".ts", b""), Confidence::None);
    assert_eq!(parser.detect(".ts", b"export default {}"), Confidence::None);
}

#[test]
fn detect_non_ts_extension() {
    let parser = qt_linguist::Parser;
    assert_eq!(parser.detect(".json", b""), Confidence::None);
    assert_eq!(parser.detect(".xml", b""), Confidence::None);
}

#[test]
fn detect_qt_content_in_xml_extension() {
    let parser = qt_linguist::Parser;
    let content = b"<?xml version=\"1.0\"?><!DOCTYPE TS><TS version=\"2.1\"></TS>";
    assert_eq!(parser.detect(".xml", content), Confidence::High);
}

// ---------------------------------------------------------------------------
// Simple fixture
// ---------------------------------------------------------------------------

#[test]
fn parse_simple() {
    let content = load_fixture("simple.ts");
    let resource = qt_linguist::Parser
        .parse(&content)
        .expect("parse should succeed");

    assert_eq!(resource.entries.len(), 2);
    assert_eq!(resource.metadata.source_format, FormatId::QtLinguist);
    assert_eq!(resource.metadata.locale, Some("de_DE".to_string()));
    assert_eq!(resource.metadata.source_locale, Some("en_US".to_string()));

    // Check Hello entry
    let hello_key = format!("MainWindow{}Hello", CTX_SEP);
    let hello = &resource.entries[&hello_key];
    assert_eq!(hello.source, Some("Hello".to_string()));
    assert_eq!(hello.value, EntryValue::Simple("Hallo".to_string()));
    assert_eq!(hello.state, Some(TranslationState::Translated));

    // Check context
    assert_eq!(hello.contexts.len(), 1);
    assert_eq!(hello.contexts[0].value, "MainWindow");
    assert_eq!(
        hello.contexts[0].context_type,
        ContextType::Disambiguation
    );

    // Check Goodbye entry
    let goodbye_key = format!("MainWindow{}Goodbye", CTX_SEP);
    let goodbye = &resource.entries[&goodbye_key];
    assert_eq!(goodbye.source, Some("Goodbye".to_string()));
    assert_eq!(
        goodbye.value,
        EntryValue::Simple("Auf Wiedersehen".to_string())
    );
}

// ---------------------------------------------------------------------------
// States fixture
// ---------------------------------------------------------------------------

#[test]
fn parse_states() {
    let content = load_fixture("states.ts");
    let resource = qt_linguist::Parser
        .parse(&content)
        .expect("parse should succeed");

    assert_eq!(resource.entries.len(), 4);

    // Translated (no type attribute)
    let translated_key = format!("Dialog{}Translated text", CTX_SEP);
    let translated = &resource.entries[&translated_key];
    assert_eq!(translated.state, Some(TranslationState::Translated));
    assert!(!translated.obsolete);

    // Unfinished -> NeedsReview
    let unfinished_key = format!("Dialog{}Unfinished text", CTX_SEP);
    let unfinished = &resource.entries[&unfinished_key];
    assert_eq!(unfinished.state, Some(TranslationState::NeedsReview));
    assert!(!unfinished.obsolete);

    // Obsolete
    let obsolete_key = format!("Dialog{}Obsolete text", CTX_SEP);
    let obsolete = &resource.entries[&obsolete_key];
    assert_eq!(obsolete.state, Some(TranslationState::Obsolete));
    assert!(obsolete.obsolete);

    // Vanished
    let vanished_key = format!("Dialog{}Vanished text", CTX_SEP);
    let vanished = &resource.entries[&vanished_key];
    assert_eq!(vanished.state, Some(TranslationState::Vanished));
    assert!(vanished.obsolete);
}

// ---------------------------------------------------------------------------
// Plurals fixture
// ---------------------------------------------------------------------------

#[test]
fn parse_plurals() {
    let content = load_fixture("plurals.ts");
    let resource = qt_linguist::Parser
        .parse(&content)
        .expect("parse should succeed");

    assert_eq!(resource.entries.len(), 2);

    // Plural entry
    let plural_key = format!("FileDialog{}%n file(s)", CTX_SEP);
    let plural_entry = &resource.entries[&plural_key];
    assert_eq!(plural_entry.source, Some("%n file(s)".to_string()));

    match &plural_entry.value {
        EntryValue::Plural(plural_set) => {
            assert_eq!(plural_set.one, Some("%n Datei".to_string()));
            assert_eq!(plural_set.other, "%n Dateien".to_string());
        }
        other => panic!("Expected Plural value, got {:?}", other),
    }

    // Check numerus flag in extension
    match &plural_entry.format_ext {
        Some(FormatExtension::QtLinguist(ext)) => {
            assert_eq!(ext.numerus, Some(true));
        }
        other => panic!("Expected QtLinguistExt, got {:?}", other),
    }

    // Non-plural entry in same context
    let open_key = format!("FileDialog{}Open", CTX_SEP);
    let open_entry = &resource.entries[&open_key];
    assert_eq!(
        open_entry.value,
        EntryValue::Simple("Oeffnen".to_string())
    );
}

// ---------------------------------------------------------------------------
// Full fixture
// ---------------------------------------------------------------------------

#[test]
fn parse_full() {
    let content = load_fixture("full.ts");
    let resource = qt_linguist::Parser
        .parse(&content)
        .expect("parse should succeed");

    assert_eq!(resource.entries.len(), 5);

    // Hello entry: location, comments, oldsource, oldcomment, extra elements
    let hello_key = format!("MainWindow{}Hello", CTX_SEP);
    let hello = &resource.entries[&hello_key];

    // Source references (locations)
    assert_eq!(hello.source_references.len(), 1);
    assert_eq!(hello.source_references[0].file, "mainwindow.cpp");
    assert_eq!(hello.source_references[0].line, Some(42));

    // Old source
    assert_eq!(hello.previous_source, Some("Hi".to_string()));

    // Old comment
    assert_eq!(
        hello.previous_comment,
        Some("Old greeting comment".to_string())
    );

    // Comments: developer + translator
    assert_eq!(hello.comments.len(), 2);
    assert_eq!(hello.comments[0].role, CommentRole::Developer);
    assert_eq!(hello.comments[0].text, "Greeting shown on main screen");
    assert_eq!(hello.comments[1].role, CommentRole::Translator);
    assert_eq!(hello.comments[1].text, "Informal greeting");

    // Extra elements
    match &hello.format_ext {
        Some(FormatExtension::QtLinguist(ext)) => {
            assert_eq!(
                ext.extra_elements.get("extra-po-msgid_plural"),
                Some(&"Hellos".to_string())
            );
        }
        other => panic!("Expected QtLinguistExt, got {:?}", other),
    }

    // Quit entry: unfinished
    let quit_key = format!("MainWindow{}Quit", CTX_SEP);
    let quit = &resource.entries[&quit_key];
    assert_eq!(quit.state, Some(TranslationState::NeedsReview));
    assert_eq!(quit.comments.len(), 1);
    assert_eq!(quit.comments[0].text, "Menu action to quit the application");

    // About entry: obsolete
    let about_key = format!("MainWindow{}About", CTX_SEP);
    let about = &resource.entries[&about_key];
    assert_eq!(about.state, Some(TranslationState::Obsolete));
    assert!(about.obsolete);

    // Settings entry in different context
    let settings_key = format!("SettingsDialog{}Settings", CTX_SEP);
    let settings = &resource.entries[&settings_key];
    assert_eq!(
        settings.value,
        EntryValue::Simple("Einstellungen".to_string())
    );
    assert_eq!(settings.contexts[0].value, "SettingsDialog");
    assert_eq!(settings.source_references.len(), 1);
    assert_eq!(settings.source_references[0].file, "settings.cpp");
    assert_eq!(settings.source_references[0].line, Some(10));
}

// ---------------------------------------------------------------------------
// Metadata
// ---------------------------------------------------------------------------

#[test]
fn parse_metadata() {
    let content = load_fixture("simple.ts");
    let resource = qt_linguist::Parser
        .parse(&content)
        .expect("parse should succeed");

    assert_eq!(resource.metadata.source_format, FormatId::QtLinguist);
    assert_eq!(resource.metadata.locale, Some("de_DE".to_string()));
    assert_eq!(resource.metadata.source_locale, Some("en_US".to_string()));
    assert_eq!(
        resource.metadata.properties.get("ts_version"),
        Some(&"2.1".to_string())
    );
}

// ---------------------------------------------------------------------------
// Round-trip tests
// ---------------------------------------------------------------------------

#[test]
fn roundtrip_simple() {
    let content = load_fixture("simple.ts");
    let resource = qt_linguist::Parser
        .parse(&content)
        .expect("parse should succeed");
    let written = qt_linguist::Writer
        .write(&resource)
        .expect("write should succeed");
    let reparsed = qt_linguist::Parser
        .parse(&written)
        .expect("reparse should succeed");

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, entry) in &resource.entries {
        let reparsed_entry = &reparsed.entries[key];
        assert_eq!(
            entry.value, reparsed_entry.value,
            "Value mismatch for key: {key:?}"
        );
        assert_eq!(
            entry.source, reparsed_entry.source,
            "Source mismatch for key: {key:?}"
        );
        assert_eq!(
            entry.state, reparsed_entry.state,
            "State mismatch for key: {key:?}"
        );
    }
    assert_eq!(
        resource.metadata.locale, reparsed.metadata.locale,
        "Locale mismatch"
    );
    assert_eq!(
        resource.metadata.source_locale, reparsed.metadata.source_locale,
        "Source locale mismatch"
    );
}

#[test]
fn roundtrip_states() {
    let content = load_fixture("states.ts");
    let resource = qt_linguist::Parser
        .parse(&content)
        .expect("parse should succeed");
    let written = qt_linguist::Writer
        .write(&resource)
        .expect("write should succeed");
    let reparsed = qt_linguist::Parser
        .parse(&written)
        .expect("reparse should succeed");

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, entry) in &resource.entries {
        let reparsed_entry = &reparsed.entries[key];
        assert_eq!(
            entry.state, reparsed_entry.state,
            "State mismatch for key: {key:?}"
        );
        assert_eq!(
            entry.obsolete, reparsed_entry.obsolete,
            "Obsolete mismatch for key: {key:?}"
        );
    }
}

#[test]
fn roundtrip_plurals() {
    let content = load_fixture("plurals.ts");
    let resource = qt_linguist::Parser
        .parse(&content)
        .expect("parse should succeed");
    let written = qt_linguist::Writer
        .write(&resource)
        .expect("write should succeed");
    let reparsed = qt_linguist::Parser
        .parse(&written)
        .expect("reparse should succeed");

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, entry) in &resource.entries {
        let reparsed_entry = &reparsed.entries[key];
        assert_eq!(
            entry.value, reparsed_entry.value,
            "Value mismatch for key: {key:?}"
        );
        assert_eq!(
            entry.source, reparsed_entry.source,
            "Source mismatch for key: {key:?}"
        );
    }
}

#[test]
fn roundtrip_full() {
    let content = load_fixture("full.ts");
    let resource = qt_linguist::Parser
        .parse(&content)
        .expect("parse should succeed");
    let written = qt_linguist::Writer
        .write(&resource)
        .expect("write should succeed");
    let reparsed = qt_linguist::Parser
        .parse(&written)
        .expect("reparse should succeed");

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, entry) in &resource.entries {
        let reparsed_entry = &reparsed.entries[key];
        assert_eq!(
            entry.value, reparsed_entry.value,
            "Value mismatch for key: {key:?}"
        );
        assert_eq!(
            entry.source, reparsed_entry.source,
            "Source mismatch for key: {key:?}"
        );
        assert_eq!(
            entry.state, reparsed_entry.state,
            "State mismatch for key: {key:?}"
        );
        assert_eq!(
            entry.obsolete, reparsed_entry.obsolete,
            "Obsolete mismatch for key: {key:?}"
        );
        assert_eq!(
            entry.previous_source, reparsed_entry.previous_source,
            "Previous source mismatch for key: {key:?}"
        );
        assert_eq!(
            entry.previous_comment, reparsed_entry.previous_comment,
            "Previous comment mismatch for key: {key:?}"
        );
        assert_eq!(
            entry.comments.len(),
            reparsed_entry.comments.len(),
            "Comment count mismatch for key: {key:?}"
        );
        for (i, comment) in entry.comments.iter().enumerate() {
            assert_eq!(
                comment.text, reparsed_entry.comments[i].text,
                "Comment text mismatch for key: {key:?}, comment: {i}"
            );
            assert_eq!(
                comment.role, reparsed_entry.comments[i].role,
                "Comment role mismatch for key: {key:?}, comment: {i}"
            );
        }
        assert_eq!(
            entry.source_references.len(),
            reparsed_entry.source_references.len(),
            "Source references count mismatch for key: {key:?}"
        );
        for (i, src_ref) in entry.source_references.iter().enumerate() {
            assert_eq!(
                src_ref.file, reparsed_entry.source_references[i].file,
                "Source ref file mismatch for key: {key:?}, ref: {i}"
            );
            assert_eq!(
                src_ref.line, reparsed_entry.source_references[i].line,
                "Source ref line mismatch for key: {key:?}, ref: {i}"
            );
        }
        assert_eq!(
            entry.contexts.len(),
            reparsed_entry.contexts.len(),
            "Context count mismatch for key: {key:?}"
        );
    }

    // Verify extra elements round-trip
    let hello_key = format!("MainWindow{}Hello", CTX_SEP);
    let hello = &resource.entries[&hello_key];
    let rt_hello = &reparsed.entries[&hello_key];
    match (&hello.format_ext, &rt_hello.format_ext) {
        (Some(FormatExtension::QtLinguist(orig)), Some(FormatExtension::QtLinguist(rt))) => {
            assert_eq!(orig.extra_elements, rt.extra_elements);
        }
        _ => panic!("Expected QtLinguist extensions in both"),
    }
}

// ---------------------------------------------------------------------------
// Writer output validation
// ---------------------------------------------------------------------------

#[test]
fn writer_produces_valid_xml() {
    let content = load_fixture("simple.ts");
    let resource = qt_linguist::Parser
        .parse(&content)
        .expect("parse should succeed");
    let written = qt_linguist::Writer
        .write(&resource)
        .expect("write should succeed");
    let xml_str = String::from_utf8(written).expect("Writer should produce valid UTF-8");

    assert!(xml_str.contains("<?xml version=\"1.0\" encoding=\"utf-8\"?>"));
    assert!(xml_str.contains("<!DOCTYPE TS>"));
    assert!(xml_str.contains("<TS version=\"2.1\""));
    assert!(xml_str.contains("language=\"de_DE\""));
    assert!(xml_str.contains("sourcelanguage=\"en_US\""));
    assert!(xml_str.contains("<context>"));
    assert!(xml_str.contains("<name>MainWindow</name>"));
    assert!(xml_str.contains("<source>Hello</source>"));
    assert!(xml_str.contains("<translation>Hallo</translation>"));
}
