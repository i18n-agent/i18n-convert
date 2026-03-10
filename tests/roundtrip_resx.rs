use i18n_convert::formats::resx;
use i18n_convert::formats::Confidence;
use i18n_convert::formats::FormatParser;
use i18n_convert::formats::FormatWriter;
use i18n_convert::ir::*;

fn load_fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!("tests/fixtures/resx/{name}")).expect("fixture file should exist")
}

// ---------------------------------------------------------------------------
// Detection tests
// ---------------------------------------------------------------------------

#[test]
fn detect_resx_extension() {
    let parser = resx::Parser;
    assert_eq!(parser.detect(".resx", b""), Confidence::Definite);
    assert_eq!(parser.detect(".json", b""), Confidence::None);
    assert_eq!(parser.detect(".xml", b""), Confidence::None);
}

#[test]
fn detect_resx_content_in_xml() {
    let parser = resx::Parser;
    let content = br#"<?xml version="1.0"?><root><resheader name="resmimetype"><value>text/microsoft-resx</value></resheader></root>"#;
    assert_eq!(parser.detect(".xml", content), Confidence::High);
}

// ---------------------------------------------------------------------------
// Simple fixture
// ---------------------------------------------------------------------------

#[test]
fn parse_simple() {
    let content = load_fixture("simple.resx");
    let resource = resx::Parser.parse(&content).expect("parse should succeed");

    assert_eq!(resource.entries.len(), 3);
    assert_eq!(resource.metadata.source_format, FormatId::Resx);

    // Check greeting
    let greeting = &resource.entries["greeting"];
    assert_eq!(
        greeting.value,
        EntryValue::Simple("Hello, World!".to_string())
    );

    // Check farewell
    let farewell = &resource.entries["farewell"];
    assert_eq!(farewell.value, EntryValue::Simple("Goodbye!".to_string()));

    // Check empty value
    let empty = &resource.entries["empty_value"];
    assert_eq!(empty.value, EntryValue::Simple(String::new()));
}

#[test]
fn parse_simple_headers() {
    let content = load_fixture("simple.resx");
    let resource = resx::Parser.parse(&content).expect("parse should succeed");

    assert_eq!(
        resource.metadata.headers.get("resmimetype"),
        Some(&"text/microsoft-resx".to_string())
    );
    assert_eq!(
        resource.metadata.headers.get("version"),
        Some(&"2.0".to_string())
    );
}

// ---------------------------------------------------------------------------
// Comments fixture
// ---------------------------------------------------------------------------

#[test]
fn parse_comments() {
    let content = load_fixture("comments.resx");
    let resource = resx::Parser.parse(&content).expect("parse should succeed");

    assert_eq!(resource.entries.len(), 3);

    // greeting has a comment
    let greeting = &resource.entries["greeting"];
    assert_eq!(greeting.comments.len(), 1);
    assert_eq!(greeting.comments[0].text, "Main greeting message");
    assert_eq!(greeting.comments[0].role, CommentRole::Developer);

    // farewell has a comment
    let farewell = &resource.entries["farewell"];
    assert_eq!(farewell.comments.len(), 1);
    assert_eq!(farewell.comments[0].text, "Farewell message shown on exit");

    // no_comment has no comment
    let no_comment = &resource.entries["no_comment"];
    assert_eq!(no_comment.comments.len(), 0);
}

// ---------------------------------------------------------------------------
// Full fixture
// ---------------------------------------------------------------------------

#[test]
fn parse_full() {
    let content = load_fixture("full.resx");
    let resource = resx::Parser.parse(&content).expect("parse should succeed");

    assert_eq!(resource.entries.len(), 4);

    // Check headers (including reader/writer)
    assert_eq!(
        resource.metadata.headers.get("reader"),
        Some(&"System.Resources.ResXResourceReader".to_string())
    );
    assert_eq!(
        resource.metadata.headers.get("writer"),
        Some(&"System.Resources.ResXResourceWriter".to_string())
    );

    // Check app_title with type and mimetype
    let app_title = &resource.entries["app_title"];
    assert_eq!(
        app_title.value,
        EntryValue::Simple("My Application".to_string())
    );
    assert_eq!(app_title.comments.len(), 1);
    assert_eq!(app_title.comments[0].text, "Application title");

    match &app_title.format_ext {
        Some(FormatExtension::Resx(ext)) => {
            assert_eq!(ext.type_name, Some("System.String".to_string()));
            assert_eq!(ext.mimetype, Some("text/plain".to_string()));
        }
        other => panic!("Expected ResxExt, got {:?}", other),
    }

    // Check special chars are decoded
    let special = &resource.entries["special_chars"];
    assert_eq!(
        special.value,
        EntryValue::Simple("Less < Greater > Amp & Quote \"".to_string())
    );
}

// ---------------------------------------------------------------------------
// Round-trip tests
// ---------------------------------------------------------------------------

#[test]
fn roundtrip_simple() {
    let content = load_fixture("simple.resx");
    let resource = resx::Parser.parse(&content).expect("parse should succeed");
    let written = resx::Writer.write(&resource).expect("write should succeed");
    let reparsed = resx::Parser
        .parse(&written)
        .expect("reparse should succeed");

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
fn roundtrip_comments() {
    let content = load_fixture("comments.resx");
    let resource = resx::Parser.parse(&content).expect("parse should succeed");
    let written = resx::Writer.write(&resource).expect("write should succeed");
    let reparsed = resx::Parser
        .parse(&written)
        .expect("reparse should succeed");

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, entry) in &resource.entries {
        let reparsed_entry = &reparsed.entries[key];
        assert_eq!(
            entry.value, reparsed_entry.value,
            "Value mismatch for key: {key}"
        );
        assert_eq!(
            entry.comments.len(),
            reparsed_entry.comments.len(),
            "Comment count mismatch for key: {key}"
        );
        for (i, comment) in entry.comments.iter().enumerate() {
            assert_eq!(
                comment.text, reparsed_entry.comments[i].text,
                "Comment text mismatch for key: {key}, comment: {i}"
            );
        }
    }
}

#[test]
fn roundtrip_full() {
    let content = load_fixture("full.resx");
    let resource = resx::Parser.parse(&content).expect("parse should succeed");
    let written = resx::Writer.write(&resource).expect("write should succeed");
    let reparsed = resx::Parser
        .parse(&written)
        .expect("reparse should succeed");

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, entry) in &resource.entries {
        let reparsed_entry = &reparsed.entries[key];
        assert_eq!(
            entry.value, reparsed_entry.value,
            "Value mismatch for key: {key}"
        );
        assert_eq!(
            entry.comments.len(),
            reparsed_entry.comments.len(),
            "Comment count mismatch for key: {key}"
        );
    }

    // Verify headers round-trip
    assert_eq!(
        resource.metadata.headers.get("resmimetype"),
        reparsed.metadata.headers.get("resmimetype")
    );
    assert_eq!(
        resource.metadata.headers.get("version"),
        reparsed.metadata.headers.get("version")
    );

    // Verify extension data round-trips for entries that have it
    let app_title = &resource.entries["app_title"];
    let rt_app_title = &reparsed.entries["app_title"];
    match (&app_title.format_ext, &rt_app_title.format_ext) {
        (Some(FormatExtension::Resx(orig)), Some(FormatExtension::Resx(rt))) => {
            assert_eq!(orig.type_name, rt.type_name);
            assert_eq!(orig.mimetype, rt.mimetype);
        }
        _ => panic!("Expected Resx extensions in both"),
    }
}

// ---------------------------------------------------------------------------
// Writer output validation
// ---------------------------------------------------------------------------

#[test]
fn writer_produces_valid_xml() {
    let content = load_fixture("simple.resx");
    let resource = resx::Parser.parse(&content).expect("parse should succeed");
    let written = resx::Writer.write(&resource).expect("write should succeed");
    let xml_str = String::from_utf8(written).expect("Writer should produce valid UTF-8");

    assert!(xml_str.contains("<?xml version=\"1.0\" encoding=\"utf-8\"?>"));
    assert!(xml_str.contains("<root>"));
    assert!(xml_str.contains("<resheader name=\"resmimetype\""));
    assert!(xml_str.contains("<data name=\"greeting\""));
    assert!(xml_str.contains("xml:space=\"preserve\""));
    assert!(xml_str.contains("<value>Hello, World!</value>"));
}
