use i18n_convert::formats::tmx;
use i18n_convert::formats::Confidence;
use i18n_convert::formats::FormatParser;
use i18n_convert::formats::FormatWriter;
use i18n_convert::ir::*;

fn load_fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!("tests/fixtures/tmx/{name}")).expect("fixture file should exist")
}

// ---------------------------------------------------------------------------
// Detection tests
// ---------------------------------------------------------------------------

#[test]
fn detect_tmx_extension() {
    let parser = tmx::Parser;
    assert_eq!(parser.detect(".tmx", b""), Confidence::Definite);
}

#[test]
fn detect_tmx_content() {
    let parser = tmx::Parser;
    let content = b"<tmx version=\"1.4\"><header/><body></body></tmx>";
    assert_eq!(parser.detect(".xml", content), Confidence::High);
}

#[test]
fn detect_tmx_no_match() {
    let parser = tmx::Parser;
    assert_eq!(parser.detect(".json", b""), Confidence::None);
    assert_eq!(parser.detect(".xliff", b""), Confidence::None);
    assert_eq!(parser.detect(".xml", b"<root/>"), Confidence::None);
}

// ---------------------------------------------------------------------------
// Capabilities test
// ---------------------------------------------------------------------------

#[test]
fn capabilities() {
    let caps = tmx::Parser.capabilities();
    assert!(caps.comments);
    assert!(caps.source_string);
    assert!(caps.custom_properties);
    assert!(!caps.plurals);
    assert!(!caps.arrays);
    assert!(!caps.translation_state);
    assert!(!caps.context);
    assert!(!caps.inline_markup);
    assert!(!caps.alternatives);
    assert!(!caps.nested_keys);
}

// ---------------------------------------------------------------------------
// Simple fixture
// ---------------------------------------------------------------------------

#[test]
fn parse_simple() {
    let content = load_fixture("simple.tmx");
    let resource = tmx::Parser
        .parse(&content)
        .expect("should parse simple.tmx");

    assert_eq!(resource.entries.len(), 3);
    assert_eq!(resource.metadata.source_locale, Some("en".to_string()));
    assert_eq!(resource.metadata.locale, Some("de".to_string()));
    assert_eq!(resource.metadata.source_format, FormatId::Tmx);

    // Check greeting
    let greeting = &resource.entries["greeting"];
    assert_eq!(greeting.source, Some("Hello".to_string()));
    assert_eq!(greeting.value, EntryValue::Simple("Hallo".to_string()));

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
}

#[test]
fn parse_simple_metadata() {
    let content = load_fixture("simple.tmx");
    let resource = tmx::Parser
        .parse(&content)
        .expect("should parse simple.tmx");

    assert_eq!(resource.metadata.source_format, FormatId::Tmx);
    assert_eq!(
        resource.metadata.tool_name,
        Some("i18n-convert".to_string())
    );
    assert_eq!(resource.metadata.tool_version, Some("0.1.0".to_string()));

    // Check format extension
    match &resource.metadata.format_ext {
        Some(FormatExtension::Tmx(ext)) => {
            assert_eq!(ext.seg_type, Some("sentence".to_string()));
            assert_eq!(ext.o_tmf, Some("undefined".to_string()));
        }
        other => panic!("Expected TmxExt, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Notes fixture
// ---------------------------------------------------------------------------

#[test]
fn parse_notes() {
    let content = load_fixture("notes.tmx");
    let resource = tmx::Parser.parse(&content).expect("should parse notes.tmx");

    assert_eq!(resource.entries.len(), 3);

    // btn_save: one note
    let btn_save = &resource.entries["btn_save"];
    assert_eq!(btn_save.comments.len(), 1);
    assert_eq!(btn_save.comments[0].text, "Button label for save action");
    assert_eq!(btn_save.comments[0].role, CommentRole::General);

    // btn_cancel: one note
    let btn_cancel = &resource.entries["btn_cancel"];
    assert_eq!(btn_cancel.comments.len(), 1);
    assert_eq!(btn_cancel.comments[0].text, "Keep it short");

    // menu_file: two notes
    let menu_file = &resource.entries["menu_file"];
    assert_eq!(menu_file.comments.len(), 2);
    assert_eq!(menu_file.comments[0].text, "Main menu item");
    assert_eq!(menu_file.comments[1].text, "Should match OS convention");
}

// ---------------------------------------------------------------------------
// Properties fixture
// ---------------------------------------------------------------------------

#[test]
fn parse_properties() {
    let content = load_fixture("properties.tmx");
    let resource = tmx::Parser
        .parse(&content)
        .expect("should parse properties.tmx");

    assert_eq!(resource.entries.len(), 2);
    assert_eq!(resource.metadata.tool_name, Some("MyTool".to_string()));
    assert_eq!(resource.metadata.tool_version, Some("2.0".to_string()));

    // greeting: has props and change metadata
    let greeting = &resource.entries["greeting"];
    assert_eq!(
        greeting.properties.get("domain"),
        Some(&"general".to_string())
    );
    assert_eq!(
        greeting.properties.get("project"),
        Some(&"webapp".to_string())
    );
    assert_eq!(
        greeting.properties.get("changedate"),
        Some(&"20240101T120000Z".to_string())
    );
    assert_eq!(
        greeting.properties.get("changeid"),
        Some(&"translator1".to_string())
    );
    assert_eq!(greeting.comments.len(), 1);
    assert_eq!(greeting.comments[0].text, "A greeting message");

    // farewell: has prop and change metadata, no notes
    let farewell = &resource.entries["farewell"];
    assert_eq!(
        farewell.properties.get("domain"),
        Some(&"general".to_string())
    );
    assert_eq!(
        farewell.properties.get("changedate"),
        Some(&"20240215T083000Z".to_string())
    );
    assert_eq!(farewell.comments.len(), 0);

    // Check format extension: paragraph segtype, custom o-tmf
    match &resource.metadata.format_ext {
        Some(FormatExtension::Tmx(ext)) => {
            assert_eq!(ext.seg_type, Some("paragraph".to_string()));
            assert_eq!(ext.o_tmf, Some("ABCTransMem".to_string()));
        }
        other => panic!("Expected TmxExt, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Full fixture
// ---------------------------------------------------------------------------

#[test]
fn parse_full() {
    let content = load_fixture("full.tmx");
    let resource = tmx::Parser.parse(&content).expect("should parse full.tmx");

    assert_eq!(resource.entries.len(), 4);
    assert_eq!(resource.metadata.source_locale, Some("en".to_string()));
    assert_eq!(resource.metadata.locale, Some("es".to_string()));

    // welcome
    let welcome = &resource.entries["welcome"];
    assert_eq!(
        welcome.source,
        Some("Welcome to our application".to_string())
    );
    assert_eq!(
        welcome.value,
        EntryValue::Simple("Bienvenido a nuestra aplicación".to_string())
    );
    assert_eq!(welcome.comments.len(), 1);
    assert_eq!(welcome.properties.get("domain"), Some(&"web".to_string()));
    assert_eq!(welcome.properties.get("client"), Some(&"acme".to_string()));
    assert_eq!(
        welcome.properties.get("changedate"),
        Some(&"20240301T100000Z".to_string())
    );

    // items_count
    let items = &resource.entries["items_count"];
    assert_eq!(items.source, Some("%d items".to_string()));
    assert_eq!(items.value, EntryValue::Simple("%d elementos".to_string()));

    // empty_target
    let empty = &resource.entries["empty_target"];
    assert_eq!(empty.source, Some("Untranslated text".to_string()));
    assert_eq!(empty.value, EntryValue::Simple(String::new()));

    // multi_lang: TMX can have multiple target langs, but our IR uses the first non-source
    let multi = &resource.entries["multi_lang"];
    assert_eq!(multi.source, Some("Hello World".to_string()));
    assert_eq!(multi.value, EntryValue::Simple("Hola Mundo".to_string()));
}

// ---------------------------------------------------------------------------
// Round-trip tests
// ---------------------------------------------------------------------------

#[test]
fn roundtrip_simple() {
    let content = load_fixture("simple.tmx");
    let resource = tmx::Parser.parse(&content).expect("should parse");
    let written = tmx::Writer.write(&resource).expect("should write");
    let reparsed = tmx::Parser.parse(&written).expect("should reparse");

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
    assert_eq!(
        resource.metadata.source_locale,
        reparsed.metadata.source_locale
    );
    assert_eq!(resource.metadata.locale, reparsed.metadata.locale);
}

#[test]
fn roundtrip_notes() {
    let content = load_fixture("notes.tmx");
    let resource = tmx::Parser.parse(&content).expect("should parse");
    let written = tmx::Writer.write(&resource).expect("should write");
    let reparsed = tmx::Parser.parse(&written).expect("should reparse");

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
        }
    }
}

#[test]
fn roundtrip_properties() {
    let content = load_fixture("properties.tmx");
    let resource = tmx::Parser.parse(&content).expect("should parse");
    let written = tmx::Writer.write(&resource).expect("should write");
    let reparsed = tmx::Parser.parse(&written).expect("should reparse");

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
        // Check custom properties round-trip (domain, project)
        for (prop_key, prop_val) in &entry.properties {
            assert_eq!(
                reparsed_entry.properties.get(prop_key),
                Some(prop_val),
                "Property '{prop_key}' mismatch for key: {key}"
            );
        }
    }

    // Verify metadata round-trips
    assert_eq!(resource.metadata.tool_name, reparsed.metadata.tool_name);
    assert_eq!(
        resource.metadata.tool_version,
        reparsed.metadata.tool_version
    );
    match (&resource.metadata.format_ext, &reparsed.metadata.format_ext) {
        (Some(FormatExtension::Tmx(orig)), Some(FormatExtension::Tmx(rt))) => {
            assert_eq!(orig.seg_type, rt.seg_type);
            assert_eq!(orig.o_tmf, rt.o_tmf);
        }
        _ => panic!("Expected Tmx extensions in both"),
    }
}

#[test]
fn roundtrip_full() {
    let content = load_fixture("full.tmx");
    let resource = tmx::Parser.parse(&content).expect("should parse");
    let written = tmx::Writer.write(&resource).expect("should write");
    let reparsed = tmx::Parser.parse(&written).expect("should reparse");

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
            entry.comments.len(),
            reparsed_entry.comments.len(),
            "Comment count mismatch for key: {key}"
        );
    }
}

// ---------------------------------------------------------------------------
// Writer output validation
// ---------------------------------------------------------------------------

#[test]
fn writer_produces_valid_xml() {
    let content = load_fixture("simple.tmx");
    let resource = tmx::Parser.parse(&content).expect("should parse");
    let written = tmx::Writer.write(&resource).expect("should write");
    let xml_str = String::from_utf8(written).expect("Writer should produce valid UTF-8");

    assert!(xml_str.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
    assert!(xml_str.contains("<tmx version=\"1.4\""));
    assert!(xml_str.contains("creationtool=\"i18n-convert\""));
    assert!(xml_str.contains("srclang=\"en\""));
    assert!(xml_str.contains("<tu tuid=\"greeting\""));
    assert!(xml_str.contains("<tuv xml:lang=\"en\""));
    assert!(xml_str.contains("<seg>Hello</seg>"));
    assert!(xml_str.contains("<seg>Hallo</seg>"));
}

#[test]
fn writer_omits_empty_target() {
    let content = load_fixture("simple.tmx");
    let resource = tmx::Parser.parse(&content).expect("should parse");
    let written = tmx::Writer.write(&resource).expect("should write");
    let reparsed = tmx::Parser.parse(&written).expect("should reparse");
    assert_eq!(
        reparsed.entries["untranslated"].value,
        EntryValue::Simple(String::new())
    );
}
