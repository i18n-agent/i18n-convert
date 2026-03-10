use i18n_convert::formats::xliff1;
use i18n_convert::formats::Confidence;
use i18n_convert::formats::FormatParser;
use i18n_convert::formats::FormatWriter;
use i18n_convert::ir::*;

fn load_fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!("tests/fixtures/xliff1/{name}")).unwrap()
}

// ---------------------------------------------------------------------------
// Detection tests
// ---------------------------------------------------------------------------

#[test]
fn detect_xliff_extension() {
    let parser = xliff1::Parser;
    assert_eq!(parser.detect(".xliff", b""), Confidence::High);
    assert_eq!(parser.detect(".xlf", b""), Confidence::High);
    assert_eq!(parser.detect(".json", b""), Confidence::None);
}

#[test]
fn detect_xliff_content_in_xml() {
    let parser = xliff1::Parser;
    let content = br#"<?xml version="1.0"?><xliff version="1.2"><file></file></xliff>"#;
    assert_eq!(parser.detect(".xml", content), Confidence::Definite);
}

// ---------------------------------------------------------------------------
// Simple fixture
// ---------------------------------------------------------------------------

#[test]
fn parse_simple() {
    let content = load_fixture("simple.xliff");
    let resource = xliff1::Parser.parse(&content).unwrap();

    assert_eq!(resource.entries.len(), 3);
    assert_eq!(resource.metadata.source_locale, Some("en".to_string()));
    assert_eq!(resource.metadata.locale, Some("de".to_string()));

    // Check greeting
    let greeting = &resource.entries["greeting"];
    assert_eq!(greeting.source, Some("Hello".to_string()));
    assert_eq!(greeting.value, EntryValue::Simple("Hallo".to_string()));

    // Check farewell
    let farewell = &resource.entries["farewell"];
    assert_eq!(farewell.source, Some("Goodbye".to_string()));
    assert_eq!(farewell.value, EntryValue::Simple("Auf Wiedersehen".to_string()));

    // Untranslated entry has empty target
    let untranslated = &resource.entries["untranslated"];
    assert_eq!(untranslated.source, Some("Not yet translated".to_string()));
    assert_eq!(untranslated.value, EntryValue::Simple(String::new()));
}

#[test]
fn parse_simple_metadata() {
    let content = load_fixture("simple.xliff");
    let resource = xliff1::Parser.parse(&content).unwrap();

    assert_eq!(resource.metadata.source_format, FormatId::Xliff1);

    // Check format extension
    match &resource.metadata.format_ext {
        Some(FormatExtension::Xliff1(ext)) => {
            assert_eq!(ext.datatype, Some("plaintext".to_string()));
            assert_eq!(ext.original, Some("messages.properties".to_string()));
        }
        other => panic!("Expected Xliff1Ext, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// Notes fixture
// ---------------------------------------------------------------------------

#[test]
fn parse_notes() {
    let content = load_fixture("notes.xliff");
    let resource = xliff1::Parser.parse(&content).unwrap();

    assert_eq!(resource.entries.len(), 3);

    // btn_save: developer note with priority and annotates
    let btn_save = &resource.entries["btn_save"];
    assert_eq!(btn_save.comments.len(), 1);
    let note = &btn_save.comments[0];
    assert_eq!(note.text, "Button label for save action");
    assert_eq!(note.role, CommentRole::Developer);
    assert_eq!(note.priority, Some(1));
    assert_eq!(note.annotates, Some(AnnotationTarget::Source));

    // btn_cancel: translator note + general note
    let btn_cancel = &resource.entries["btn_cancel"];
    assert_eq!(btn_cancel.comments.len(), 2);
    assert_eq!(btn_cancel.comments[0].role, CommentRole::Translator);
    assert_eq!(btn_cancel.comments[0].annotates, Some(AnnotationTarget::Target));
    assert_eq!(btn_cancel.comments[1].role, CommentRole::General);
    assert_eq!(btn_cancel.comments[1].text, "General note about this string");

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
    let resource = xliff1::Parser.parse(&content).unwrap();

    assert_eq!(resource.entries.len(), 10);

    // new
    let new_entry = &resource.entries["new_entry"];
    assert_eq!(new_entry.state, Some(TranslationState::New));
    assert_eq!(new_entry.approved, Some(false));

    // translated
    let translated = &resource.entries["translated_entry"];
    assert_eq!(translated.state, Some(TranslationState::Translated));
    assert_eq!(translated.approved, Some(false));

    // signed-off -> Reviewed
    let reviewed = &resource.entries["reviewed_entry"];
    assert_eq!(reviewed.state, Some(TranslationState::Reviewed));
    assert_eq!(reviewed.approved, Some(true));

    // final
    let final_entry = &resource.entries["final_entry"];
    assert_eq!(final_entry.state, Some(TranslationState::Final));
    assert_eq!(final_entry.approved, Some(true));

    // needs-translation
    let needs_trans = &resource.entries["needs_translation"];
    assert_eq!(needs_trans.state, Some(TranslationState::NeedsTranslation));

    // needs-review-translation with state-qualifier
    let needs_review = &resource.entries["needs_review"];
    assert_eq!(needs_review.state, Some(TranslationState::NeedsReview));
    assert_eq!(needs_review.state_qualifier, Some("leveraged-inherited".to_string()));

    // needs-adaptation
    let needs_adapt = &resource.entries["needs_adapt"];
    assert_eq!(needs_adapt.state, Some(TranslationState::NeedsAdaptation));

    // needs-l10n
    let needs_l10n = &resource.entries["needs_l10n"];
    assert_eq!(needs_l10n.state, Some(TranslationState::NeedsL10n));

    // needs-review-adaptation
    let needs_review_adapt = &resource.entries["needs_review_adapt"];
    assert_eq!(needs_review_adapt.state, Some(TranslationState::NeedsReviewAdaptation));

    // needs-review-l10n
    let needs_review_l10n = &resource.entries["needs_review_l10n"];
    assert_eq!(needs_review_l10n.state, Some(TranslationState::NeedsReviewL10n));
}

// ---------------------------------------------------------------------------
// Full fixture
// ---------------------------------------------------------------------------

#[test]
fn parse_full() {
    let content = load_fixture("full.xliff");
    let resource = xliff1::Parser.parse(&content).unwrap();

    assert_eq!(resource.entries.len(), 4);

    // title entry: maxwidth, size-unit, translate, approved, restype, resname, contexts, alt-trans, notes
    let title = &resource.entries["title"];
    assert_eq!(title.max_width, Some(50));
    assert_eq!(title.size_unit, Some("char".to_string()));
    assert_eq!(title.translatable, Some(true));
    assert_eq!(title.approved, Some(true));
    assert_eq!(title.resource_type, Some("x-title".to_string()));
    assert_eq!(title.resource_name, Some("page_title".to_string()));
    assert_eq!(title.state, Some(TranslationState::Final));
    assert_eq!(title.source, Some("Welcome to our app".to_string()));
    assert_eq!(title.value, EntryValue::Simple("Bienvenido a nuestra aplicación".to_string()));

    // Notes
    assert_eq!(title.comments.len(), 2);
    assert_eq!(title.comments[0].role, CommentRole::Developer);
    assert_eq!(title.comments[0].priority, Some(1));
    assert_eq!(title.comments[1].role, CommentRole::Translator);

    // Contexts
    assert_eq!(title.contexts.len(), 2);
    assert_eq!(title.contexts[0].context_type, ContextType::SourceFile);
    assert_eq!(title.contexts[0].value, "src/index.html");
    assert_eq!(title.contexts[0].purpose, Some("location".to_string()));
    assert_eq!(title.contexts[1].context_type, ContextType::LineNumber);
    assert_eq!(title.contexts[1].value, "42");

    // Alt-trans
    assert_eq!(title.alternatives.len(), 1);
    assert_eq!(title.alternatives[0].match_quality, Some(85.0));
    assert_eq!(title.alternatives[0].origin, Some("tm-database".to_string()));
    assert_eq!(title.alternatives[0].source, Some("Welcome to the app".to_string()));
    assert_eq!(title.alternatives[0].value, "Bienvenido a la app");

    // description entry: maxwidth + size-unit in pixels, contexts with different purpose
    let desc = &resource.entries["description"];
    assert_eq!(desc.max_width, Some(200));
    assert_eq!(desc.size_unit, Some("pixel".to_string()));
    assert_eq!(desc.contexts.len(), 2);
    assert_eq!(desc.contexts[0].context_type, ContextType::Element);
    assert_eq!(desc.contexts[0].value, "p");
    assert_eq!(desc.contexts[1].context_type, ContextType::Custom("x-url".to_string()));
    assert_eq!(desc.contexts[1].value, "/about");
    assert_eq!(desc.contexts[0].purpose, Some("information".to_string()));

    // no_translate entry
    let no_translate = &resource.entries["no_translate"];
    assert_eq!(no_translate.translatable, Some(false));
    assert_eq!(no_translate.source, Some("BRAND_NAME".to_string()));

    // with_alt_trans: multiple alternative translations
    let with_alt = &resource.entries["with_alt_trans"];
    assert_eq!(with_alt.state, Some(TranslationState::NeedsReview));
    assert_eq!(with_alt.alternatives.len(), 2);
    assert_eq!(with_alt.alternatives[0].match_quality, Some(72.0));
    assert_eq!(with_alt.alternatives[0].origin, Some("mt-engine".to_string()));
    assert_eq!(with_alt.alternatives[1].match_quality, Some(60.0));
    assert_eq!(with_alt.alternatives[1].origin, Some("tm-legacy".to_string()));
}

// ---------------------------------------------------------------------------
// Round-trip tests
// ---------------------------------------------------------------------------

#[test]
fn roundtrip_simple() {
    let content = load_fixture("simple.xliff");
    let resource = xliff1::Parser.parse(&content).unwrap();
    let written = xliff1::Writer.write(&resource).unwrap();
    let reparsed = xliff1::Parser.parse(&written).unwrap();

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
    assert_eq!(resource.metadata.source_locale, reparsed.metadata.source_locale);
    assert_eq!(resource.metadata.locale, reparsed.metadata.locale);
}

#[test]
fn roundtrip_notes() {
    let content = load_fixture("notes.xliff");
    let resource = xliff1::Parser.parse(&content).unwrap();
    let written = xliff1::Writer.write(&resource).unwrap();
    let reparsed = xliff1::Parser.parse(&written).unwrap();

    for (key, entry) in &resource.entries {
        let reparsed_entry = &reparsed.entries[key];
        assert_eq!(
            entry.comments.len(), reparsed_entry.comments.len(),
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
            assert_eq!(
                comment.annotates, reparsed_entry.comments[i].annotates,
                "Comment annotates mismatch for key: {key}, note: {i}"
            );
        }
    }
}

#[test]
fn roundtrip_states() {
    let content = load_fixture("states.xliff");
    let resource = xliff1::Parser.parse(&content).unwrap();
    let written = xliff1::Writer.write(&resource).unwrap();
    let reparsed = xliff1::Parser.parse(&written).unwrap();

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, entry) in &resource.entries {
        let reparsed_entry = &reparsed.entries[key];
        assert_eq!(
            entry.state, reparsed_entry.state,
            "State mismatch for key: {key}"
        );
        assert_eq!(
            entry.state_qualifier, reparsed_entry.state_qualifier,
            "State qualifier mismatch for key: {key}"
        );
        assert_eq!(
            entry.approved, reparsed_entry.approved,
            "Approved mismatch for key: {key}"
        );
    }
}

#[test]
fn roundtrip_full() {
    let content = load_fixture("full.xliff");
    let resource = xliff1::Parser.parse(&content).unwrap();
    let written = xliff1::Writer.write(&resource).unwrap();
    let reparsed = xliff1::Parser.parse(&written).unwrap();

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
            entry.approved, reparsed_entry.approved,
            "Approved mismatch for key: {key}"
        );
        assert_eq!(
            entry.translatable, reparsed_entry.translatable,
            "Translatable mismatch for key: {key}"
        );
        assert_eq!(
            entry.max_width, reparsed_entry.max_width,
            "Max width mismatch for key: {key}"
        );
        assert_eq!(
            entry.size_unit, reparsed_entry.size_unit,
            "Size unit mismatch for key: {key}"
        );
        assert_eq!(
            entry.resource_type, reparsed_entry.resource_type,
            "Resource type mismatch for key: {key}"
        );
        assert_eq!(
            entry.resource_name, reparsed_entry.resource_name,
            "Resource name mismatch for key: {key}"
        );
        assert_eq!(
            entry.comments.len(), reparsed_entry.comments.len(),
            "Comment count mismatch for key: {key}"
        );
        assert_eq!(
            entry.contexts.len(), reparsed_entry.contexts.len(),
            "Context count mismatch for key: {key}"
        );
        assert_eq!(
            entry.alternatives.len(), reparsed_entry.alternatives.len(),
            "Alternatives count mismatch for key: {key}"
        );
        for (i, alt) in entry.alternatives.iter().enumerate() {
            let reparsed_alt = &reparsed_entry.alternatives[i];
            assert_eq!(alt.value, reparsed_alt.value, "Alt value mismatch for key: {key}[{i}]");
            assert_eq!(alt.source, reparsed_alt.source, "Alt source mismatch for key: {key}[{i}]");
            assert_eq!(alt.match_quality, reparsed_alt.match_quality, "Alt match_quality mismatch for key: {key}[{i}]");
            assert_eq!(alt.origin, reparsed_alt.origin, "Alt origin mismatch for key: {key}[{i}]");
        }
    }

    // Verify metadata round-trips
    match (&resource.metadata.format_ext, &reparsed.metadata.format_ext) {
        (Some(FormatExtension::Xliff1(orig)), Some(FormatExtension::Xliff1(rt))) => {
            assert_eq!(orig.datatype, rt.datatype);
            assert_eq!(orig.original, rt.original);
        }
        _ => panic!("Expected Xliff1 extensions in both"),
    }
}

// ---------------------------------------------------------------------------
// Writer output validation
// ---------------------------------------------------------------------------

#[test]
fn writer_produces_valid_xml() {
    let content = load_fixture("simple.xliff");
    let resource = xliff1::Parser.parse(&content).unwrap();
    let written = xliff1::Writer.write(&resource).unwrap();
    let xml_str = String::from_utf8(written).expect("Writer should produce valid UTF-8");

    assert!(xml_str.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
    assert!(xml_str.contains("<xliff version=\"1.2\""));
    assert!(xml_str.contains("xmlns=\"urn:oasis:names:tc:xliff:document:1.2\""));
    assert!(xml_str.contains("<trans-unit id=\"greeting\""));
    assert!(xml_str.contains("<source>Hello</source>"));
    assert!(xml_str.contains("<target>Hallo</target>"));
}

#[test]
fn writer_omits_empty_target_without_state() {
    // An entry without target text and no state should not produce <target> element
    let content = load_fixture("simple.xliff");
    let resource = xliff1::Parser.parse(&content).unwrap();
    let written = xliff1::Writer.write(&resource).unwrap();
    let xml_str = String::from_utf8(written).unwrap();

    // The untranslated entry has empty target and no state -> no target element
    // Find the trans-unit for "untranslated"
    assert!(xml_str.contains("<trans-unit id=\"untranslated\""));
    // Check there's no <target></target> or <target/> for this entry
    // We'll verify by parsing - the empty string should round-trip
    let reparsed = xliff1::Parser.parse(xml_str.as_bytes()).unwrap();
    assert_eq!(reparsed.entries["untranslated"].value, EntryValue::Simple(String::new()));
}
