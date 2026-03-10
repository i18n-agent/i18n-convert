use i18n_convert::formats::captivate_xml;
use i18n_convert::formats::Confidence;
use i18n_convert::formats::FormatParser;
use i18n_convert::formats::FormatWriter;
use i18n_convert::ir::*;

fn load_fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!("tests/fixtures/captivate_xml/{name}")).unwrap()
}

// ===========================================================================
// Detection tests
// ===========================================================================

#[test]
fn detect_xml_with_captivate_tool() {
    let parser = captivate_xml::Parser;
    let content = br#"<?xml version="1.0"?><xliff version="1.2"><file original="captivate_project"><header><tool tool-id="captivate"/></header><body></body></file></xliff>"#;
    assert_eq!(parser.detect(".xml", content), Confidence::Definite);
}

#[test]
fn detect_xml_with_captivate_in_original() {
    let parser = captivate_xml::Parser;
    let content = br#"<?xml version="1.0"?><xliff version="1.2"><file original="my_captivate_course"><body></body></file></xliff>"#;
    assert_eq!(parser.detect(".xml", content), Confidence::Definite);
}

#[test]
fn detect_xml_with_slide_item_ids() {
    let parser = captivate_xml::Parser;
    let content = br#"<?xml version="1.0"?><xliff version="1.2"><file original="course"><body><trans-unit id="slide_1_item_1"><source>Hello</source></trans-unit></body></file></xliff>"#;
    assert_eq!(parser.detect(".xml", content), Confidence::Definite);
}

#[test]
fn detect_non_xml_extension_returns_none() {
    let parser = captivate_xml::Parser;
    let content = br#"<?xml version="1.0"?><xliff version="1.2"><file original="captivate_project"><body></body></file></xliff>"#;
    assert_eq!(parser.detect(".json", content), Confidence::None);
    assert_eq!(parser.detect(".xliff", content), Confidence::None);
    assert_eq!(parser.detect(".xlf", content), Confidence::None);
}

#[test]
fn detect_xml_without_captivate_markers_returns_none() {
    let parser = captivate_xml::Parser;
    let content = br#"<?xml version="1.0"?><xliff version="1.2"><file original="generic_project"><body><trans-unit id="msg_1"><source>Hello</source></trans-unit></body></file></xliff>"#;
    assert_eq!(parser.detect(".xml", content), Confidence::None);
}

#[test]
fn detect_captivate_case_insensitive() {
    let parser = captivate_xml::Parser;
    let content = br#"<?xml version="1.0"?><xliff version="1.2"><file original="CAPTIVATE_PROJECT"><body></body></file></xliff>"#;
    assert_eq!(parser.detect(".xml", content), Confidence::Definite);
}

// ===========================================================================
// Parse simple fixture
// ===========================================================================

#[test]
fn parse_simple_entry_count() {
    let content = load_fixture("simple.xml");
    let resource = captivate_xml::Parser.parse(&content).unwrap();
    assert_eq!(resource.entries.len(), 3);
}

#[test]
fn parse_simple_metadata() {
    let content = load_fixture("simple.xml");
    let resource = captivate_xml::Parser.parse(&content).unwrap();

    assert_eq!(resource.metadata.source_format, FormatId::CaptivateXml);
    assert_eq!(resource.metadata.source_locale, Some("en".to_string()));
    assert_eq!(resource.metadata.tool_name, Some("Adobe Captivate".to_string()));

    // Check format extension
    match &resource.metadata.format_ext {
        Some(FormatExtension::CaptivateXml(_)) => {}
        other => panic!("Expected CaptivateXmlExt, got {:?}", other),
    }
}

#[test]
fn parse_simple_source_values() {
    let content = load_fixture("simple.xml");
    let resource = captivate_xml::Parser.parse(&content).unwrap();

    let e1 = &resource.entries["slide_1_item_1"];
    assert_eq!(e1.source, Some("Welcome to this course".to_string()));
    // Mono-lingual: value should equal source
    assert_eq!(e1.value, EntryValue::Simple("Welcome to this course".to_string()));

    let e2 = &resource.entries["slide_1_item_2"];
    assert_eq!(e2.source, Some("This module covers the basics".to_string()));

    let e3 = &resource.entries["slide_2_item_1"];
    assert_eq!(e3.source, Some("Click the button below to continue".to_string()));
}

#[test]
fn parse_simple_slide_item_ext() {
    let content = load_fixture("simple.xml");
    let resource = captivate_xml::Parser.parse(&content).unwrap();

    let e1 = &resource.entries["slide_1_item_1"];
    match &e1.format_ext {
        Some(FormatExtension::CaptivateXml(ext)) => {
            assert_eq!(ext.slide_id, Some("1".to_string()));
            assert_eq!(ext.item_id, Some("1".to_string()));
            assert_eq!(ext.css_style, None);
        }
        other => panic!("Expected CaptivateXmlExt, got {:?}", other),
    }

    let e3 = &resource.entries["slide_2_item_1"];
    match &e3.format_ext {
        Some(FormatExtension::CaptivateXml(ext)) => {
            assert_eq!(ext.slide_id, Some("2".to_string()));
            assert_eq!(ext.item_id, Some("1".to_string()));
        }
        other => panic!("Expected CaptivateXmlExt, got {:?}", other),
    }
}

// ===========================================================================
// Parse formatted fixture (inline <g> markup)
// ===========================================================================

#[test]
fn parse_formatted_entry_count() {
    let content = load_fixture("formatted.xml");
    let resource = captivate_xml::Parser.parse(&content).unwrap();
    assert_eq!(resource.entries.len(), 4);
}

#[test]
fn parse_formatted_inline_bold() {
    let content = load_fixture("formatted.xml");
    let resource = captivate_xml::Parser.parse(&content).unwrap();

    let e1 = &resource.entries["slide_1_item_1"];
    let source = e1.source.as_deref().unwrap();
    assert!(source.contains("<g"), "Source should contain <g> element: {source}");
    assert!(source.contains("ctype=\"bold\""), "Should preserve ctype: {source}");
    assert!(source.contains("Important:"), "Should contain text content: {source}");
    assert!(source.contains("Pay attention to this section"), "Should contain trailing text: {source}");
}

#[test]
fn parse_formatted_inline_italic() {
    let content = load_fixture("formatted.xml");
    let resource = captivate_xml::Parser.parse(&content).unwrap();

    let e2 = &resource.entries["slide_1_item_2"];
    let source = e2.source.as_deref().unwrap();
    assert!(source.contains("<g"), "Source should contain <g> element: {source}");
    assert!(source.contains("ctype=\"italic\""), "Should preserve italic ctype: {source}");
    assert!(source.contains("emphasized"), "Should contain emphasized text: {source}");
}

#[test]
fn parse_formatted_multiple_g_elements() {
    let content = load_fixture("formatted.xml");
    let resource = captivate_xml::Parser.parse(&content).unwrap();

    let e3 = &resource.entries["slide_2_item_1"];
    let source = e3.source.as_deref().unwrap();
    // Should have two <g> elements
    let g_count = source.matches("<g ").count();
    assert_eq!(g_count, 2, "Should have 2 <g> elements, got {g_count} in: {source}");
    assert!(source.contains("Step 1:"), "Should contain Step 1: {source}");
    assert!(source.contains("Settings"), "Should contain Settings: {source}");
}

#[test]
fn parse_formatted_plain_text_entry() {
    let content = load_fixture("formatted.xml");
    let resource = captivate_xml::Parser.parse(&content).unwrap();

    let e4 = &resource.entries["slide_2_item_2"];
    let source = e4.source.as_deref().unwrap();
    assert!(!source.contains("<g"), "Plain text should not have <g> elements");
    assert_eq!(source, "Plain text without formatting");
}

// ===========================================================================
// Parse quiz fixture (notes/comments)
// ===========================================================================

#[test]
fn parse_quiz_entry_count() {
    let content = load_fixture("quiz.xml");
    let resource = captivate_xml::Parser.parse(&content).unwrap();
    assert_eq!(resource.entries.len(), 7);
}

#[test]
fn parse_quiz_notes_general() {
    let content = load_fixture("quiz.xml");
    let resource = captivate_xml::Parser.parse(&content).unwrap();

    let q1 = &resource.entries["slide_3_item_1"];
    assert_eq!(q1.source, Some("Quiz Question: What is 2+2?".to_string()));
    assert_eq!(q1.comments.len(), 1);
    assert_eq!(q1.comments[0].text, "Multiple choice question");
    assert_eq!(q1.comments[0].role, CommentRole::General);

    let a2 = &resource.entries["slide_3_item_3"];
    assert_eq!(a2.comments.len(), 1);
    assert_eq!(a2.comments[0].text, "Correct answer");
}

#[test]
fn parse_quiz_notes_developer() {
    let content = load_fixture("quiz.xml");
    let resource = captivate_xml::Parser.parse(&content).unwrap();

    let tf = &resource.entries["slide_4_item_1"];
    assert_eq!(tf.comments.len(), 1);
    assert_eq!(tf.comments[0].text, "Boolean question type");
    assert_eq!(tf.comments[0].role, CommentRole::Developer);
}

#[test]
fn parse_quiz_entries_without_notes() {
    let content = load_fixture("quiz.xml");
    let resource = captivate_xml::Parser.parse(&content).unwrap();

    let t = &resource.entries["slide_4_item_2"];
    assert_eq!(t.source, Some("True".to_string()));
    assert!(t.comments.is_empty());

    let f = &resource.entries["slide_4_item_3"];
    assert_eq!(f.source, Some("False".to_string()));
    assert!(f.comments.is_empty());
}

// ===========================================================================
// Parse styled fixture (css-style)
// ===========================================================================

#[test]
fn parse_styled_entry_count() {
    let content = load_fixture("styled.xml");
    let resource = captivate_xml::Parser.parse(&content).unwrap();
    assert_eq!(resource.entries.len(), 5);
}

#[test]
fn parse_styled_css_attributes() {
    let content = load_fixture("styled.xml");
    let resource = captivate_xml::Parser.parse(&content).unwrap();

    let title = &resource.entries["slide_1_item_1"];
    match &title.format_ext {
        Some(FormatExtension::CaptivateXml(ext)) => {
            assert_eq!(ext.css_style, Some("font-family:Arial;font-size:24px;".to_string()));
        }
        other => panic!("Expected CaptivateXmlExt, got {:?}", other),
    }

    let subtitle = &resource.entries["slide_1_item_2"];
    match &subtitle.format_ext {
        Some(FormatExtension::CaptivateXml(ext)) => {
            assert_eq!(ext.css_style, Some("font-family:Verdana;color:#333333;".to_string()));
        }
        other => panic!("Expected CaptivateXmlExt, got {:?}", other),
    }
}

#[test]
fn parse_styled_no_css_style() {
    let content = load_fixture("styled.xml");
    let resource = captivate_xml::Parser.parse(&content).unwrap();

    let body = &resource.entries["slide_2_item_2"];
    match &body.format_ext {
        Some(FormatExtension::CaptivateXml(ext)) => {
            assert_eq!(ext.css_style, None);
        }
        other => panic!("Expected CaptivateXmlExt with no css_style, got {:?}", other),
    }
}

#[test]
fn parse_styled_with_inline_markup_and_css() {
    let content = load_fixture("styled.xml");
    let resource = captivate_xml::Parser.parse(&content).unwrap();

    let exam = &resource.entries["slide_3_item_1"];
    let source = exam.source.as_deref().unwrap();
    assert!(source.contains("<g"), "Should contain inline markup: {source}");
    assert!(source.contains("Final Exam"), "Should contain text: {source}");

    match &exam.format_ext {
        Some(FormatExtension::CaptivateXml(ext)) => {
            assert_eq!(ext.css_style, Some("text-align:center;font-size:18px;".to_string()));
        }
        other => panic!("Expected CaptivateXmlExt, got {:?}", other),
    }
}

// ===========================================================================
// Source-only (mono-lingual) handling
// ===========================================================================

#[test]
fn mono_lingual_value_equals_source() {
    let content = load_fixture("simple.xml");
    let resource = captivate_xml::Parser.parse(&content).unwrap();

    for (_key, entry) in &resource.entries {
        let source = entry.source.as_deref().unwrap();
        assert_eq!(
            entry.value,
            EntryValue::Simple(source.to_string()),
            "In mono-lingual mode, value should equal source for key: {}",
            entry.key
        );
    }
}

// ===========================================================================
// Writer output validation
// ===========================================================================

#[test]
fn writer_produces_valid_xliff() {
    let content = load_fixture("simple.xml");
    let resource = captivate_xml::Parser.parse(&content).unwrap();
    let written = captivate_xml::Writer.write(&resource).unwrap();
    let xml_str = String::from_utf8(written).expect("Writer should produce valid UTF-8");

    assert!(xml_str.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"),
        "Should have XML declaration");
    assert!(xml_str.contains("<xliff version=\"1.2\""),
        "Should have xliff root element");
    assert!(xml_str.contains("xmlns=\"urn:oasis:names:tc:xliff:document:1.2\""),
        "Should have XLIFF namespace");
    assert!(xml_str.contains("original=\"captivate_project\""),
        "Should have captivate original attribute");
    assert!(xml_str.contains("tool-id=\"captivate\""),
        "Should have captivate tool-id");
    assert!(xml_str.contains("tool-name=\"Adobe Captivate\""),
        "Should have Adobe Captivate tool-name");
}

#[test]
fn writer_includes_trans_units() {
    let content = load_fixture("simple.xml");
    let resource = captivate_xml::Parser.parse(&content).unwrap();
    let written = captivate_xml::Writer.write(&resource).unwrap();
    let xml_str = String::from_utf8(written).unwrap();

    assert!(xml_str.contains("id=\"slide_1_item_1\""),
        "Should contain slide_1_item_1 trans-unit");
    assert!(xml_str.contains("id=\"slide_2_item_1\""),
        "Should contain slide_2_item_1 trans-unit");
    assert!(xml_str.contains("<source>Welcome to this course</source>"),
        "Should contain source text");
}

#[test]
fn writer_omits_target_when_same_as_source() {
    let content = load_fixture("simple.xml");
    let resource = captivate_xml::Parser.parse(&content).unwrap();
    let written = captivate_xml::Writer.write(&resource).unwrap();
    let xml_str = String::from_utf8(written).unwrap();

    // In mono-lingual mode, value == source, so no <target> should be written
    assert!(!xml_str.contains("<target>"),
        "Should not write <target> when value equals source");
}

#[test]
fn writer_includes_css_style() {
    let content = load_fixture("styled.xml");
    let resource = captivate_xml::Parser.parse(&content).unwrap();
    let written = captivate_xml::Writer.write(&resource).unwrap();
    let xml_str = String::from_utf8(written).unwrap();

    assert!(xml_str.contains("css-style=\"font-family:Arial;font-size:24px;\""),
        "Should preserve css-style attribute");
}

#[test]
fn writer_includes_notes() {
    let content = load_fixture("quiz.xml");
    let resource = captivate_xml::Parser.parse(&content).unwrap();
    let written = captivate_xml::Writer.write(&resource).unwrap();
    let xml_str = String::from_utf8(written).unwrap();

    assert!(xml_str.contains("<note>Multiple choice question</note>"),
        "Should include general notes");
    assert!(xml_str.contains("from=\"developer\""),
        "Should include developer role on notes");
}

#[test]
fn writer_preserves_inline_markup() {
    let content = load_fixture("formatted.xml");
    let resource = captivate_xml::Parser.parse(&content).unwrap();
    let written = captivate_xml::Writer.write(&resource).unwrap();
    let xml_str = String::from_utf8(written).unwrap();

    assert!(xml_str.contains("<g "), "Should preserve <g> elements in output");
    assert!(xml_str.contains("ctype=\"bold\""), "Should preserve ctype attribute");
}

// ===========================================================================
// Roundtrip tests
// ===========================================================================

#[test]
fn roundtrip_simple() {
    let content = load_fixture("simple.xml");
    let resource = captivate_xml::Parser.parse(&content).unwrap();
    let written = captivate_xml::Writer.write(&resource).unwrap();
    let reparsed = captivate_xml::Parser.parse(&written).unwrap();

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
}

#[test]
fn roundtrip_formatted() {
    let content = load_fixture("formatted.xml");
    let resource = captivate_xml::Parser.parse(&content).unwrap();
    let written = captivate_xml::Writer.write(&resource).unwrap();
    let reparsed = captivate_xml::Parser.parse(&written).unwrap();

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, entry) in &resource.entries {
        let reparsed_entry = &reparsed.entries[key];
        assert_eq!(
            entry.source, reparsed_entry.source,
            "Source mismatch for key: {key}"
        );
    }
}

#[test]
fn roundtrip_quiz_notes() {
    let content = load_fixture("quiz.xml");
    let resource = captivate_xml::Parser.parse(&content).unwrap();
    let written = captivate_xml::Writer.write(&resource).unwrap();
    let reparsed = captivate_xml::Parser.parse(&written).unwrap();

    assert_eq!(resource.entries.len(), reparsed.entries.len());
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
        }
    }
}

#[test]
fn roundtrip_styled_css() {
    let content = load_fixture("styled.xml");
    let resource = captivate_xml::Parser.parse(&content).unwrap();
    let written = captivate_xml::Writer.write(&resource).unwrap();
    let reparsed = captivate_xml::Parser.parse(&written).unwrap();

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, entry) in &resource.entries {
        let reparsed_entry = &reparsed.entries[key];

        // Compare format extensions (css_style, slide_id, item_id)
        match (&entry.format_ext, &reparsed_entry.format_ext) {
            (Some(FormatExtension::CaptivateXml(orig)), Some(FormatExtension::CaptivateXml(rt))) => {
                assert_eq!(orig.css_style, rt.css_style, "css_style mismatch for key: {key}");
                assert_eq!(orig.slide_id, rt.slide_id, "slide_id mismatch for key: {key}");
                assert_eq!(orig.item_id, rt.item_id, "item_id mismatch for key: {key}");
            }
            (None, None) => {}
            (orig, rt) => panic!("Format ext mismatch for key {key}: {:?} vs {:?}", orig, rt),
        }
    }
}

// ===========================================================================
// Edge cases
// ===========================================================================

#[test]
fn parse_empty_body() {
    let content = br#"<?xml version="1.0" encoding="UTF-8"?>
<xliff version="1.2" xmlns="urn:oasis:names:tc:xliff:document:1.2">
  <file original="captivate_project" source-language="en" datatype="plaintext">
    <header>
      <tool tool-id="captivate" tool-name="Adobe Captivate"/>
    </header>
    <body>
    </body>
  </file>
</xliff>"#;

    let resource = captivate_xml::Parser.parse(content).unwrap();
    assert_eq!(resource.entries.len(), 0);
    assert_eq!(resource.metadata.source_format, FormatId::CaptivateXml);
    assert_eq!(resource.metadata.source_locale, Some("en".to_string()));
}

#[test]
fn parse_malformed_id_no_slide_pattern() {
    let content = br#"<?xml version="1.0" encoding="UTF-8"?>
<xliff version="1.2" xmlns="urn:oasis:names:tc:xliff:document:1.2">
  <file original="captivate_project" source-language="en" datatype="plaintext">
    <header>
      <tool tool-id="captivate" tool-name="Adobe Captivate"/>
    </header>
    <body>
      <trans-unit id="custom_id_123">
        <source>Some text</source>
      </trans-unit>
      <trans-unit id="another_id">
        <source>Other text</source>
      </trans-unit>
    </body>
  </file>
</xliff>"#;

    let resource = captivate_xml::Parser.parse(content).unwrap();
    assert_eq!(resource.entries.len(), 2);

    // Non-slide_N_item_M IDs should still parse, but slide_id/item_id will be None
    let e1 = &resource.entries["custom_id_123"];
    match &e1.format_ext {
        // format_ext might be None since no slide_id, item_id, or css_style
        None => {} // acceptable
        Some(FormatExtension::CaptivateXml(ext)) => {
            assert_eq!(ext.slide_id, None);
            assert_eq!(ext.item_id, None);
        }
        other => panic!("Unexpected format_ext: {:?}", other),
    }
}

#[test]
fn parse_with_target_element() {
    let content = br#"<?xml version="1.0" encoding="UTF-8"?>
<xliff version="1.2" xmlns="urn:oasis:names:tc:xliff:document:1.2">
  <file original="captivate_project" source-language="en" target-language="de" datatype="plaintext">
    <header>
      <tool tool-id="captivate" tool-name="Adobe Captivate"/>
    </header>
    <body>
      <trans-unit id="slide_1_item_1">
        <source>Welcome</source>
        <target>Willkommen</target>
      </trans-unit>
      <trans-unit id="slide_1_item_2">
        <source>Goodbye</source>
      </trans-unit>
    </body>
  </file>
</xliff>"#;

    let resource = captivate_xml::Parser.parse(content).unwrap();
    assert_eq!(resource.metadata.locale, Some("de".to_string()));

    // Entry with target: value should be the target text
    let e1 = &resource.entries["slide_1_item_1"];
    assert_eq!(e1.source, Some("Welcome".to_string()));
    assert_eq!(e1.value, EntryValue::Simple("Willkommen".to_string()));

    // Entry without target: value should be the source text
    let e2 = &resource.entries["slide_1_item_2"];
    assert_eq!(e2.source, Some("Goodbye".to_string()));
    assert_eq!(e2.value, EntryValue::Simple("Goodbye".to_string()));
}

#[test]
fn roundtrip_with_target_writes_target() {
    let content = br#"<?xml version="1.0" encoding="UTF-8"?>
<xliff version="1.2" xmlns="urn:oasis:names:tc:xliff:document:1.2">
  <file original="captivate_project" source-language="en" target-language="de" datatype="plaintext">
    <header>
      <tool tool-id="captivate" tool-name="Adobe Captivate"/>
    </header>
    <body>
      <trans-unit id="slide_1_item_1">
        <source>Welcome</source>
        <target>Willkommen</target>
      </trans-unit>
    </body>
  </file>
</xliff>"#;

    let resource = captivate_xml::Parser.parse(content).unwrap();
    let written = captivate_xml::Writer.write(&resource).unwrap();
    let xml_str = String::from_utf8(written).unwrap();

    // Since value ("Willkommen") differs from source ("Welcome"), target should be written
    assert!(xml_str.contains("<target>Willkommen</target>"),
        "Should write <target> when value differs from source. Output:\n{xml_str}");
    assert!(xml_str.contains("<source>Welcome</source>"),
        "Should write <source>");

    // Roundtrip
    let reparsed = captivate_xml::Parser.parse(xml_str.as_bytes()).unwrap();
    let e1 = &reparsed.entries["slide_1_item_1"];
    assert_eq!(e1.source, Some("Welcome".to_string()));
    assert_eq!(e1.value, EntryValue::Simple("Willkommen".to_string()));
}

#[test]
fn capabilities_match() {
    let parser_caps = captivate_xml::Parser.capabilities();
    let writer_caps = captivate_xml::Writer.capabilities();

    assert!(parser_caps.source_string);
    assert!(parser_caps.inline_markup);
    assert!(parser_caps.comments);
    assert!(parser_caps.context);
    assert!(!parser_caps.plurals);
    assert!(!parser_caps.arrays);

    assert_eq!(parser_caps, writer_caps);
}

#[test]
fn roundtrip_empty_body_produces_empty() {
    let content = br#"<?xml version="1.0" encoding="UTF-8"?>
<xliff version="1.2" xmlns="urn:oasis:names:tc:xliff:document:1.2">
  <file original="captivate_project" source-language="en" datatype="plaintext">
    <header>
      <tool tool-id="captivate" tool-name="Adobe Captivate"/>
    </header>
    <body>
    </body>
  </file>
</xliff>"#;

    let resource = captivate_xml::Parser.parse(content).unwrap();
    let written = captivate_xml::Writer.write(&resource).unwrap();
    let reparsed = captivate_xml::Parser.parse(&written).unwrap();
    assert_eq!(reparsed.entries.len(), 0);
}

#[test]
fn writer_preserves_original_attribute() {
    let content = load_fixture("simple.xml");
    let resource = captivate_xml::Parser.parse(&content).unwrap();
    let written = captivate_xml::Writer.write(&resource).unwrap();
    let xml_str = String::from_utf8(written).unwrap();

    assert!(xml_str.contains("original=\"captivate_project\""),
        "Should preserve the original attribute in output");
}
