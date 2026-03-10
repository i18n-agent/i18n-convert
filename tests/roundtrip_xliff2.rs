use i18n_convert::formats::xliff2;
use i18n_convert::formats::Confidence;
use i18n_convert::formats::FormatParser;
use i18n_convert::formats::FormatWriter;
use i18n_convert::ir::*;

fn load_fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!("tests/fixtures/xliff2/{name}")).expect("fixture file should exist")
}

// ---------------------------------------------------------------------------
// Detection tests
// ---------------------------------------------------------------------------

#[test]
fn detect_xliff2_by_content() {
    let parser = xliff2::Parser;
    let content = br#"<?xml version="1.0"?><xliff xmlns="urn:oasis:names:tc:xliff:document:2.0" version="2.0" srcLang="en"><file id="f1"></file></xliff>"#;
    assert_eq!(parser.detect(".xliff", content), Confidence::Definite);
    assert_eq!(parser.detect(".xlf", content), Confidence::Definite);
    assert_eq!(parser.detect(".xml", content), Confidence::Definite);
}

#[test]
fn detect_xliff2_extension_only() {
    let parser = xliff2::Parser;
    // Without content indicating version 2.0, extension alone yields Low
    assert_eq!(parser.detect(".xliff", b""), Confidence::Low);
    assert_eq!(parser.detect(".xlf", b""), Confidence::Low);
}

#[test]
fn detect_xliff2_no_match() {
    let parser = xliff2::Parser;
    assert_eq!(parser.detect(".json", b""), Confidence::None);
    assert_eq!(parser.detect(".xml", b"<root/>"), Confidence::None);
}

#[test]
fn detect_xliff2_namespace() {
    let parser = xliff2::Parser;
    let content = b"<xliff xmlns=\"urn:oasis:names:tc:xliff:document:2.0\">";
    assert_eq!(parser.detect(".xml", content), Confidence::Definite);
}

// ---------------------------------------------------------------------------
// Capabilities test
// ---------------------------------------------------------------------------

#[test]
fn capabilities() {
    let caps = xliff2::Parser.capabilities();
    assert!(caps.comments);
    assert!(caps.source_string);
    assert!(caps.translation_state);
    assert!(caps.custom_properties);
    assert!(caps.inline_markup);
    assert!(caps.alternatives);
    assert!(!caps.plurals);
    assert!(!caps.arrays);
    assert!(!caps.translatable_flag);
    assert!(!caps.nested_keys);
}

// ---------------------------------------------------------------------------
// Simple fixture
// ---------------------------------------------------------------------------

#[test]
fn parse_simple() {
    let content = load_fixture("simple.xliff");
    let resource = xliff2::Parser
        .parse(&content)
        .expect("should parse simple.xliff");

    assert_eq!(resource.entries.len(), 3);
    assert_eq!(resource.metadata.source_locale, Some("en".to_string()));
    assert_eq!(resource.metadata.locale, Some("de".to_string()));
    assert_eq!(resource.metadata.source_format, FormatId::Xliff2);

    // Check greeting
    let greeting = &resource.entries["greeting"];
    assert_eq!(greeting.source, Some("Hello".to_string()));
    assert_eq!(greeting.value, EntryValue::Simple("Hallo".to_string()));
    assert_eq!(greeting.state, Some(TranslationState::Translated));

    // Check farewell
    let farewell = &resource.entries["farewell"];
    assert_eq!(farewell.source, Some("Goodbye".to_string()));
    assert_eq!(
        farewell.value,
        EntryValue::Simple("Auf Wiedersehen".to_string())
    );

    // Untranslated entry has empty target
    let untranslated = &resource.entries["untranslated"];
    assert_eq!(untranslated.source, Some("Not yet translated".to_string()));
    assert_eq!(untranslated.value, EntryValue::Simple(String::new()));
    assert_eq!(untranslated.state, Some(TranslationState::New));
}

#[test]
fn parse_simple_metadata() {
    let content = load_fixture("simple.xliff");
    let resource = xliff2::Parser
        .parse(&content)
        .expect("should parse simple.xliff");

    assert_eq!(resource.metadata.source_format, FormatId::Xliff2);

    // Check format extension
    match &resource.metadata.format_ext {
        Some(FormatExtension::Xliff2(ext)) => {
            assert_eq!(
                ext.original_data.get("original"),
                Some(&"messages.json".to_string())
            );
        }
        other => panic!("Expected Xliff2Ext, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Notes fixture
// ---------------------------------------------------------------------------

#[test]
fn parse_notes() {
    let content = load_fixture("notes.xliff");
    let resource = xliff2::Parser
        .parse(&content)
        .expect("should parse notes.xliff");

    assert_eq!(resource.entries.len(), 3);

    // btn_save: developer (description) note with priority
    let btn_save = &resource.entries["btn_save"];
    assert_eq!(btn_save.comments.len(), 1);
    let note = &btn_save.comments[0];
    assert_eq!(note.text, "Button label for save action");
    assert_eq!(note.role, CommentRole::Developer);
    assert_eq!(note.priority, Some(1));

    // btn_cancel: translator note + general note
    let btn_cancel = &resource.entries["btn_cancel"];
    assert_eq!(btn_cancel.comments.len(), 2);
    assert_eq!(btn_cancel.comments[0].role, CommentRole::Translator);
    assert_eq!(btn_cancel.comments[0].text, "Keep it short");
    assert_eq!(btn_cancel.comments[1].role, CommentRole::General);
    assert_eq!(
        btn_cancel.comments[1].text,
        "General note about this string"
    );

    // menu_file: two notes
    let menu_file = &resource.entries["menu_file"];
    assert_eq!(menu_file.comments.len(), 2);
    assert_eq!(menu_file.comments[0].role, CommentRole::Developer);
    assert_eq!(menu_file.comments[0].priority, Some(2));
    assert_eq!(menu_file.comments[1].role, CommentRole::Translator);
}

// ---------------------------------------------------------------------------
// States fixture
// ---------------------------------------------------------------------------

#[test]
fn parse_states() {
    let content = load_fixture("states.xliff");
    let resource = xliff2::Parser
        .parse(&content)
        .expect("should parse states.xliff");

    assert_eq!(resource.entries.len(), 4);

    let initial = &resource.entries["initial_entry"];
    assert_eq!(initial.state, Some(TranslationState::New));

    let translated = &resource.entries["translated_entry"];
    assert_eq!(translated.state, Some(TranslationState::Translated));

    let reviewed = &resource.entries["reviewed_entry"];
    assert_eq!(reviewed.state, Some(TranslationState::Reviewed));

    let final_entry = &resource.entries["final_entry"];
    assert_eq!(final_entry.state, Some(TranslationState::Final));
}

// ---------------------------------------------------------------------------
// Full fixture (groups, notes, states, original)
// ---------------------------------------------------------------------------

#[test]
fn parse_full() {
    let content = load_fixture("full.xliff");
    let resource = xliff2::Parser
        .parse(&content)
        .expect("should parse full.xliff");

    assert_eq!(resource.entries.len(), 5);
    assert_eq!(resource.metadata.source_locale, Some("en".to_string()));
    assert_eq!(resource.metadata.locale, Some("es".to_string()));

    // Units inside group
    let nav_home = &resource.entries["nav.home"];
    assert_eq!(nav_home.source, Some("Home".to_string()));
    assert_eq!(nav_home.value, EntryValue::Simple("Inicio".to_string()));
    assert_eq!(nav_home.state, Some(TranslationState::Translated));
    assert_eq!(nav_home.comments.len(), 1);
    assert_eq!(nav_home.comments[0].text, "Navigation home link");

    let nav_about = &resource.entries["nav.about"];
    assert_eq!(nav_about.source, Some("About".to_string()));
    assert_eq!(nav_about.value, EntryValue::Simple("Acerca de".to_string()));

    // title with notes
    let title = &resource.entries["title"];
    assert_eq!(title.state, Some(TranslationState::Final));
    assert_eq!(title.comments.len(), 2);
    assert_eq!(title.comments[0].role, CommentRole::Developer);
    assert_eq!(title.comments[0].priority, Some(1));
    assert_eq!(title.comments[1].role, CommentRole::Translator);

    // items_count with needs-review state
    let items = &resource.entries["items_count"];
    assert_eq!(items.state, Some(TranslationState::NeedsReview));

    // empty_target
    let empty = &resource.entries["empty_target"];
    assert_eq!(empty.value, EntryValue::Simple(String::new()));
    assert_eq!(empty.state, Some(TranslationState::New));

    // Check original in extension
    match &resource.metadata.format_ext {
        Some(FormatExtension::Xliff2(ext)) => {
            assert_eq!(
                ext.original_data.get("original"),
                Some(&"app.json".to_string())
            );
        }
        other => panic!("Expected Xliff2Ext, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Round-trip tests
// ---------------------------------------------------------------------------

#[test]
fn roundtrip_simple() {
    let content = load_fixture("simple.xliff");
    let resource = xliff2::Parser.parse(&content).expect("should parse");
    let written = xliff2::Writer.write(&resource).expect("should write");
    let reparsed = xliff2::Parser.parse(&written).expect("should reparse");

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
        assert_eq!(
            entry.state, reparsed_entry.state,
            "State mismatch for key: {key}"
        );
    }
    assert_eq!(
        resource.metadata.source_locale,
        reparsed.metadata.source_locale
    );
    assert_eq!(resource.metadata.locale, reparsed.metadata.locale);
}

#[test]
fn roundtrip_notes() {
    let content = load_fixture("notes.xliff");
    let resource = xliff2::Parser.parse(&content).expect("should parse");
    let written = xliff2::Writer.write(&resource).expect("should write");
    let reparsed = xliff2::Parser.parse(&written).expect("should reparse");

    for (key, entry) in &resource.entries {
        let reparsed_entry = &reparsed.entries[key];
        assert_eq!(
            entry.comments.len(),
            reparsed_entry.comments.len(),
            "Comment count mismatch for key: {key}"
        );
        for (i, comment) in entry.comments.iter().enumerate() {
            assert_eq!(
                comment.text, reparsed_entry.comments[i].text,
                "Comment text mismatch for key: {key}, note: {i}"
            );
            assert_eq!(
                comment.role, reparsed_entry.comments[i].role,
                "Comment role mismatch for key: {key}, note: {i}"
            );
            assert_eq!(
                comment.priority, reparsed_entry.comments[i].priority,
                "Comment priority mismatch for key: {key}, note: {i}"
            );
        }
    }
}

#[test]
fn roundtrip_states() {
    let content = load_fixture("states.xliff");
    let resource = xliff2::Parser.parse(&content).expect("should parse");
    let written = xliff2::Writer.write(&resource).expect("should write");
    let reparsed = xliff2::Parser.parse(&written).expect("should reparse");

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, entry) in &resource.entries {
        let reparsed_entry = &reparsed.entries[key];
        assert_eq!(
            entry.state, reparsed_entry.state,
            "State mismatch for key: {key}"
        );
    }
}

#[test]
fn roundtrip_full() {
    let content = load_fixture("full.xliff");
    let resource = xliff2::Parser.parse(&content).expect("should parse");
    let written = xliff2::Writer.write(&resource).expect("should write");
    let reparsed = xliff2::Parser.parse(&written).expect("should reparse");

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
        assert_eq!(
            entry.state, reparsed_entry.state,
            "State mismatch for key: {key}"
        );
        assert_eq!(
            entry.comments.len(),
            reparsed_entry.comments.len(),
            "Comment count mismatch for key: {key}"
        );
    }

    // Verify metadata round-trips
    assert_eq!(
        resource.metadata.source_locale,
        reparsed.metadata.source_locale
    );
    assert_eq!(resource.metadata.locale, reparsed.metadata.locale);
    match (&resource.metadata.format_ext, &reparsed.metadata.format_ext) {
        (Some(FormatExtension::Xliff2(orig)), Some(FormatExtension::Xliff2(rt))) => {
            assert_eq!(
                orig.original_data.get("original"),
                rt.original_data.get("original")
            );
        }
        _ => panic!("Expected Xliff2 extensions in both"),
    }
}

// ---------------------------------------------------------------------------
// Writer output validation
// ---------------------------------------------------------------------------

#[test]
fn writer_produces_valid_xml() {
    let content = load_fixture("simple.xliff");
    let resource = xliff2::Parser.parse(&content).expect("should parse");
    let written = xliff2::Writer.write(&resource).expect("should write");
    let xml_str = String::from_utf8(written).expect("Writer should produce valid UTF-8");

    assert!(xml_str.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
    assert!(xml_str.contains("<xliff"));
    assert!(xml_str.contains("version=\"2.0\""));
    assert!(xml_str.contains("xmlns=\"urn:oasis:names:tc:xliff:document:2.0\""));
    assert!(xml_str.contains("<unit id=\"greeting\""));
    assert!(xml_str.contains("<segment"));
    assert!(xml_str.contains("<source>Hello</source>"));
    assert!(xml_str.contains("<target>Hallo</target>"));
}

#[test]
fn writer_omits_empty_target() {
    let content = load_fixture("simple.xliff");
    let resource = xliff2::Parser.parse(&content).expect("should parse");
    let written = xliff2::Writer.write(&resource).expect("should write");
    let reparsed = xliff2::Parser.parse(&written).expect("should reparse");
    assert_eq!(
        reparsed.entries["untranslated"].value,
        EntryValue::Simple(String::new())
    );
}
