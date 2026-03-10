use i18n_convert::formats::ispring_xliff;
use i18n_convert::formats::Confidence;
use i18n_convert::formats::FormatParser;
use i18n_convert::formats::FormatWriter;
use i18n_convert::ir::*;

fn load_fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!("tests/fixtures/ispring_xliff/{name}")).unwrap()
}

// ---------------------------------------------------------------------------
// Detection tests
// ---------------------------------------------------------------------------

#[test]
fn detect_ispring_xliff_extension_with_markers() {
    let parser = ispring_xliff::Parser;
    let content = br#"<?xml version="1.0"?><xliff version="1.2"><file original="ispring_course"></file></xliff>"#;
    assert_eq!(parser.detect(".xliff", content), Confidence::Definite);
    assert_eq!(parser.detect(".xlf", content), Confidence::Definite);
}

#[test]
fn detect_ispring_tool_name_marker() {
    let parser = ispring_xliff::Parser;
    let content = br#"<?xml version="1.0"?><xliff version="1.2"><file original="course"><header><tool tool-id="ispring" tool-name="iSpring Suite"/></header></file></xliff>"#;
    assert_eq!(parser.detect(".xliff", content), Confidence::Definite);
}

#[test]
fn detect_no_markers_returns_none() {
    let parser = ispring_xliff::Parser;
    let content = br#"<?xml version="1.0"?><xliff version="1.2"><file original="my_project" source-language="en"><body></body></file></xliff>"#;
    assert_eq!(parser.detect(".xliff", content), Confidence::None);
    assert_eq!(parser.detect(".xlf", content), Confidence::None);
}

#[test]
fn detect_wrong_extension_returns_none() {
    let parser = ispring_xliff::Parser;
    let content = br#"<xliff><file original="ispring_course"></file></xliff>"#;
    assert_eq!(parser.detect(".json", content), Confidence::None);
    assert_eq!(parser.detect(".xml", content), Confidence::None);
    assert_eq!(parser.detect(".txt", content), Confidence::None);
}

#[test]
fn detect_case_insensitive_ispring() {
    let parser = ispring_xliff::Parser;
    let content = br#"<?xml version="1.0"?><xliff version="1.2"><file original="ISPRING_Course"></file></xliff>"#;
    assert_eq!(parser.detect(".xliff", content), Confidence::Definite);
}

// ---------------------------------------------------------------------------
// Parse simple fixture
// ---------------------------------------------------------------------------

#[test]
fn parse_simple_entries() {
    let content = load_fixture("simple.xliff");
    let resource = ispring_xliff::Parser.parse(&content).unwrap();

    assert_eq!(resource.entries.len(), 5);
    assert_eq!(resource.metadata.source_locale, Some("en".to_string()));
    assert_eq!(resource.metadata.locale, Some("fr".to_string()));

    let slide1_title = &resource.entries["slide1_title"];
    assert_eq!(slide1_title.source, Some("Introduction to the Course".to_string()));
    assert_eq!(
        slide1_title.value,
        EntryValue::Simple("Introduction au cours".to_string())
    );

    let nav_next = &resource.entries["nav_next"];
    assert_eq!(nav_next.source, Some("Next".to_string()));
    assert_eq!(nav_next.value, EntryValue::Simple("Suivant".to_string()));
}

#[test]
fn parse_simple_metadata() {
    let content = load_fixture("simple.xliff");
    let resource = ispring_xliff::Parser.parse(&content).unwrap();

    assert_eq!(resource.metadata.source_format, FormatId::IspringXliff);
    assert_eq!(resource.metadata.tool_name, Some("iSpring Suite".to_string()));

    match &resource.metadata.format_ext {
        Some(FormatExtension::IspringXliff(ext)) => {
            assert_eq!(ext.xliff_version, Some("1.2".to_string()));
            assert_eq!(ext.source_language, Some("en".to_string()));
        }
        other => panic!("Expected IspringXliffExt, got {:?}", other),
    }
}

#[test]
fn parse_simple_comments() {
    let content = load_fixture("simple.xliff");
    let resource = ispring_xliff::Parser.parse(&content).unwrap();

    let slide1_text = &resource.entries["slide1_text"];
    assert_eq!(slide1_text.comments.len(), 1);
    assert_eq!(slide1_text.comments[0].text, "Main slide text");
    assert_eq!(slide1_text.comments[0].role, CommentRole::Developer);
}

// ---------------------------------------------------------------------------
// Parse quiz fixture
// ---------------------------------------------------------------------------

#[test]
fn parse_quiz_entries() {
    let content = load_fixture("quiz.xliff");
    let resource = ispring_xliff::Parser.parse(&content).unwrap();

    assert_eq!(resource.entries.len(), 10);
    assert_eq!(resource.metadata.source_locale, Some("en".to_string()));
    assert_eq!(resource.metadata.locale, Some("de".to_string()));

    let q1 = &resource.entries["quiz1_q1"];
    assert_eq!(
        q1.source,
        Some("What is the capital of France?".to_string())
    );
    assert_eq!(
        q1.value,
        EntryValue::Simple("Was ist die Hauptstadt von Frankreich?".to_string())
    );

    let submit = &resource.entries["quiz_btn_submit"];
    assert_eq!(submit.source, Some("Submit".to_string()));
    assert_eq!(submit.value, EntryValue::Simple("Absenden".to_string()));
}

#[test]
fn parse_quiz_multiple_notes() {
    let content = load_fixture("quiz.xliff");
    let resource = ispring_xliff::Parser.parse(&content).unwrap();

    let q2 = &resource.entries["quiz1_q2"];
    assert_eq!(q2.comments.len(), 2);
    assert_eq!(q2.comments[0].role, CommentRole::Developer);
    assert_eq!(q2.comments[0].text, "Multiple response question");
    assert_eq!(q2.comments[1].role, CommentRole::Translator);
    assert_eq!(q2.comments[1].text, "Keep technical terms as-is");
}

#[test]
fn parse_quiz_tool_version() {
    let content = load_fixture("quiz.xliff");
    let resource = ispring_xliff::Parser.parse(&content).unwrap();

    assert_eq!(
        resource.metadata.tool_version,
        Some("11.3".to_string())
    );
}

// ---------------------------------------------------------------------------
// Parse source-only fixture
// ---------------------------------------------------------------------------

#[test]
fn parse_source_only_uses_source_as_value() {
    let content = load_fixture("source_only.xliff");
    let resource = ispring_xliff::Parser.parse(&content).unwrap();

    assert_eq!(resource.entries.len(), 7);
    assert_eq!(resource.metadata.locale, Some("ja".to_string()));

    // Without a target, value should be the source text
    let slide1_title = &resource.entries["slide1_title"];
    assert_eq!(slide1_title.source, Some("Getting Started".to_string()));
    assert_eq!(
        slide1_title.value,
        EntryValue::Simple("Getting Started".to_string())
    );

    let nav_finish = &resource.entries["nav_finish"];
    assert_eq!(nav_finish.source, Some("Finish".to_string()));
    assert_eq!(
        nav_finish.value,
        EntryValue::Simple("Finish".to_string())
    );
}

#[test]
fn parse_source_only_with_note() {
    let content = load_fixture("source_only.xliff");
    let resource = ispring_xliff::Parser.parse(&content).unwrap();

    let slide2_text = &resource.entries["slide2_text"];
    assert_eq!(slide2_text.comments.len(), 1);
    assert_eq!(slide2_text.comments[0].text, "Main content slide");
}

// ---------------------------------------------------------------------------
// Format extension preservation
// ---------------------------------------------------------------------------

#[test]
fn format_extension_roundtrip() {
    let content = load_fixture("simple.xliff");
    let resource = ispring_xliff::Parser.parse(&content).unwrap();
    let written = ispring_xliff::Writer.write(&resource).unwrap();
    let reparsed = ispring_xliff::Parser.parse(&written).unwrap();

    match (&resource.metadata.format_ext, &reparsed.metadata.format_ext) {
        (
            Some(FormatExtension::IspringXliff(orig)),
            Some(FormatExtension::IspringXliff(rt)),
        ) => {
            assert_eq!(orig.xliff_version, rt.xliff_version);
            assert_eq!(orig.source_language, rt.source_language);
        }
        _ => panic!("Expected IspringXliff extensions in both"),
    }
}

// ---------------------------------------------------------------------------
// Writer output validation
// ---------------------------------------------------------------------------

#[test]
fn writer_produces_valid_xml_with_ispring_markers() {
    let content = load_fixture("simple.xliff");
    let resource = ispring_xliff::Parser.parse(&content).unwrap();
    let written = ispring_xliff::Writer.write(&resource).unwrap();
    let xml_str = String::from_utf8(written).expect("Writer should produce valid UTF-8");

    assert!(xml_str.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
    assert!(xml_str.contains("<xliff version=\"1.2\""));
    assert!(xml_str.contains("xmlns=\"urn:oasis:names:tc:xliff:document:1.2\""));
    assert!(xml_str.contains("original=\"ispring_course\""));
    assert!(xml_str.contains("tool-id=\"ispring\""));
    assert!(xml_str.contains("tool-name=\"iSpring Suite\""));
    assert!(xml_str.contains("<trans-unit id=\"slide1_title\""));
    assert!(xml_str.contains("<source>Introduction to the Course</source>"));
    assert!(xml_str.contains("<target>Introduction au cours</target>"));
}

#[test]
fn writer_includes_header_and_tool() {
    let content = load_fixture("quiz.xliff");
    let resource = ispring_xliff::Parser.parse(&content).unwrap();
    let written = ispring_xliff::Writer.write(&resource).unwrap();
    let xml_str = String::from_utf8(written).unwrap();

    assert!(xml_str.contains("<header>"));
    assert!(xml_str.contains("</header>"));
    assert!(xml_str.contains("tool-id=\"ispring\""));
    assert!(xml_str.contains("tool-version=\"11.3\""));
}

#[test]
fn writer_includes_notes() {
    let content = load_fixture("simple.xliff");
    let resource = ispring_xliff::Parser.parse(&content).unwrap();
    let written = ispring_xliff::Writer.write(&resource).unwrap();
    let xml_str = String::from_utf8(written).unwrap();

    assert!(xml_str.contains("<note from=\"developer\">Main slide text</note>"));
}

#[test]
fn writer_source_only_omits_target() {
    // When source == value (source-only mode), the writer should still write the
    // target because the value is the translated content.
    // But if there's no state and target equals source, we still write the target.
    let content = load_fixture("source_only.xliff");
    let resource = ispring_xliff::Parser.parse(&content).unwrap();
    let written = ispring_xliff::Writer.write(&resource).unwrap();
    let xml_str = String::from_utf8(written).unwrap();

    // Source-only entries: value == source, so target is written with source text
    assert!(xml_str.contains("<source>Getting Started</source>"));
    assert!(xml_str.contains("<target>Getting Started</target>"));
}

// ---------------------------------------------------------------------------
// Roundtrip tests
// ---------------------------------------------------------------------------

#[test]
fn roundtrip_simple() {
    let content = load_fixture("simple.xliff");
    let resource = ispring_xliff::Parser.parse(&content).unwrap();
    let written = ispring_xliff::Writer.write(&resource).unwrap();
    let reparsed = ispring_xliff::Parser.parse(&written).unwrap();

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
fn roundtrip_quiz() {
    let content = load_fixture("quiz.xliff");
    let resource = ispring_xliff::Parser.parse(&content).unwrap();
    let written = ispring_xliff::Writer.write(&resource).unwrap();
    let reparsed = ispring_xliff::Parser.parse(&written).unwrap();

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
fn roundtrip_source_only() {
    let content = load_fixture("source_only.xliff");
    let resource = ispring_xliff::Parser.parse(&content).unwrap();
    let written = ispring_xliff::Writer.write(&resource).unwrap();
    let reparsed = ispring_xliff::Parser.parse(&written).unwrap();

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, entry) in &resource.entries {
        let reparsed_entry = &reparsed.entries[key];
        assert_eq!(
            entry.source, reparsed_entry.source,
            "Source mismatch for key: {key}"
        );
        // After roundtrip, value should remain the same (source text)
        assert_eq!(
            entry.value, reparsed_entry.value,
            "Value mismatch for key: {key}"
        );
    }
}

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

#[test]
fn parse_empty_body() {
    let content = br#"<?xml version="1.0" encoding="UTF-8"?>
<xliff version="1.2" xmlns="urn:oasis:names:tc:xliff:document:1.2">
  <file original="ispring_course" source-language="en" datatype="plaintext">
    <header>
      <tool tool-id="ispring" tool-name="iSpring Suite"/>
    </header>
    <body>
    </body>
  </file>
</xliff>"#;

    let resource = ispring_xliff::Parser.parse(content).unwrap();
    assert_eq!(resource.entries.len(), 0);
    assert_eq!(resource.metadata.source_locale, Some("en".to_string()));
    assert_eq!(resource.metadata.locale, None);
}

#[test]
fn parse_missing_target_language() {
    let content = br#"<?xml version="1.0" encoding="UTF-8"?>
<xliff version="1.2" xmlns="urn:oasis:names:tc:xliff:document:1.2">
  <file original="ispring_course" source-language="en" datatype="plaintext">
    <header>
      <tool tool-id="ispring" tool-name="iSpring Suite"/>
    </header>
    <body>
      <trans-unit id="test1">
        <source>Hello</source>
      </trans-unit>
    </body>
  </file>
</xliff>"#;

    let resource = ispring_xliff::Parser.parse(content).unwrap();
    assert_eq!(resource.metadata.locale, None);
    assert_eq!(resource.entries.len(), 1);
    // Without target, value uses source
    assert_eq!(
        resource.entries["test1"].value,
        EntryValue::Simple("Hello".to_string())
    );
}

#[test]
fn parse_empty_target_element() {
    let content = br#"<?xml version="1.0" encoding="UTF-8"?>
<xliff version="1.2" xmlns="urn:oasis:names:tc:xliff:document:1.2">
  <file original="ispring_course" source-language="en" target-language="fr" datatype="plaintext">
    <header>
      <tool tool-id="ispring" tool-name="iSpring Suite"/>
    </header>
    <body>
      <trans-unit id="empty_target">
        <source>Some text</source>
        <target></target>
      </trans-unit>
    </body>
  </file>
</xliff>"#;

    let resource = ispring_xliff::Parser.parse(content).unwrap();
    // Empty target element should result in empty string value (not fallback to source)
    assert_eq!(
        resource.entries["empty_target"].value,
        EntryValue::Simple(String::new())
    );
    assert_eq!(
        resource.entries["empty_target"].source,
        Some("Some text".to_string())
    );
}

#[test]
fn parse_target_with_state() {
    let content = br#"<?xml version="1.0" encoding="UTF-8"?>
<xliff version="1.2" xmlns="urn:oasis:names:tc:xliff:document:1.2">
  <file original="ispring_course" source-language="en" target-language="fr" datatype="plaintext">
    <header>
      <tool tool-id="ispring" tool-name="iSpring Suite"/>
    </header>
    <body>
      <trans-unit id="translated_entry">
        <source>Hello</source>
        <target state="translated">Bonjour</target>
      </trans-unit>
      <trans-unit id="new_entry">
        <source>World</source>
        <target state="new">Monde</target>
      </trans-unit>
    </body>
  </file>
</xliff>"#;

    let resource = ispring_xliff::Parser.parse(content).unwrap();

    let translated = &resource.entries["translated_entry"];
    assert_eq!(translated.state, Some(TranslationState::Translated));
    assert_eq!(
        translated.value,
        EntryValue::Simple("Bonjour".to_string())
    );

    let new_entry = &resource.entries["new_entry"];
    assert_eq!(new_entry.state, Some(TranslationState::New));
    assert_eq!(new_entry.value, EntryValue::Simple("Monde".to_string()));
}

#[test]
fn roundtrip_preserves_translation_state() {
    let content = br#"<?xml version="1.0" encoding="UTF-8"?>
<xliff version="1.2" xmlns="urn:oasis:names:tc:xliff:document:1.2">
  <file original="ispring_course" source-language="en" target-language="fr" datatype="plaintext">
    <header>
      <tool tool-id="ispring" tool-name="iSpring Suite"/>
    </header>
    <body>
      <trans-unit id="entry1">
        <source>Hello</source>
        <target state="translated">Bonjour</target>
      </trans-unit>
    </body>
  </file>
</xliff>"#;

    let resource = ispring_xliff::Parser.parse(content).unwrap();
    let written = ispring_xliff::Writer.write(&resource).unwrap();
    let reparsed = ispring_xliff::Parser.parse(&written).unwrap();

    assert_eq!(
        reparsed.entries["entry1"].state,
        Some(TranslationState::Translated)
    );
    assert_eq!(
        reparsed.entries["entry1"].value,
        EntryValue::Simple("Bonjour".to_string())
    );
}

#[test]
fn capabilities_match_spec() {
    let parser = ispring_xliff::Parser;
    let caps = parser.capabilities();

    assert!(caps.source_string);
    assert!(caps.translation_state);
    assert!(caps.comments);
    assert!(caps.context);
    assert!(!caps.plurals);
    assert!(!caps.arrays);
    assert!(!caps.nested_keys);
}

#[test]
fn parse_special_characters_in_text() {
    let content = br#"<?xml version="1.0" encoding="UTF-8"?>
<xliff version="1.2" xmlns="urn:oasis:names:tc:xliff:document:1.2">
  <file original="ispring_course" source-language="en" target-language="fr" datatype="plaintext">
    <header>
      <tool tool-id="ispring" tool-name="iSpring Suite"/>
    </header>
    <body>
      <trans-unit id="special">
        <source>Price: &lt;$100 &amp; free shipping</source>
        <target>Prix: &lt;100$ &amp; livraison gratuite</target>
      </trans-unit>
    </body>
  </file>
</xliff>"#;

    let resource = ispring_xliff::Parser.parse(content).unwrap();
    let entry = &resource.entries["special"];
    assert_eq!(
        entry.source,
        Some("Price: <$100 & free shipping".to_string())
    );
    assert_eq!(
        entry.value,
        EntryValue::Simple("Prix: <100$ & livraison gratuite".to_string())
    );

    // Roundtrip should preserve special characters
    let written = ispring_xliff::Writer.write(&resource).unwrap();
    let reparsed = ispring_xliff::Parser.parse(&written).unwrap();
    assert_eq!(reparsed.entries["special"].source, entry.source);
    assert_eq!(reparsed.entries["special"].value, entry.value);
}
