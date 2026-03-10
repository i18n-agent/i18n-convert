use i18n_convert::formats::excel::{Parser, Writer};
use i18n_convert::formats::{Confidence, FormatParser, FormatWriter};
use i18n_convert::ir::*;
use indexmap::IndexMap;

// ---------------------------------------------------------------------------
// Helper: create a simple .xlsx in memory for testing
// ---------------------------------------------------------------------------

fn make_test_xlsx(rows: &[(&str, &str, Option<&str>)]) -> Vec<u8> {
    let mut workbook = rust_xlsxwriter::Workbook::new();
    let worksheet = workbook.add_worksheet();

    // Header
    worksheet.write_string(0, 0, "key").expect("write header");
    worksheet.write_string(0, 1, "value").expect("write header");
    worksheet
        .write_string(0, 2, "comment")
        .expect("write header");

    for (i, (key, value, comment)) in rows.iter().enumerate() {
        let row = (i + 1) as u32;
        worksheet.write_string(row, 0, *key).expect("write key");
        worksheet.write_string(row, 1, *value).expect("write value");
        if let Some(c) = comment {
            worksheet.write_string(row, 2, *c).expect("write comment");
        }
    }

    workbook.save_to_buffer().expect("save workbook")
}

fn make_test_xlsx_custom_headers(headers: &[&str], rows: &[Vec<&str>]) -> Vec<u8> {
    let mut workbook = rust_xlsxwriter::Workbook::new();
    let worksheet = workbook.add_worksheet();

    for (i, header) in headers.iter().enumerate() {
        worksheet
            .write_string(0, i as u16, *header)
            .expect("write header");
    }

    for (row_idx, row) in rows.iter().enumerate() {
        let row_num = (row_idx + 1) as u32;
        for (col_idx, cell) in row.iter().enumerate() {
            worksheet
                .write_string(row_num, col_idx as u16, *cell)
                .expect("write cell");
        }
    }

    workbook.save_to_buffer().expect("save workbook")
}

// ---------------------------------------------------------------------------
// Detection tests
// ---------------------------------------------------------------------------

#[test]
fn detect_by_xlsx_extension() {
    let parser = Parser;
    assert_eq!(parser.detect(".xlsx", b""), Confidence::Definite);
}

#[test]
fn detect_by_xls_extension() {
    let parser = Parser;
    assert_eq!(parser.detect(".xls", b""), Confidence::Definite);
}

#[test]
fn detect_no_match() {
    let parser = Parser;
    assert_eq!(parser.detect(".json", b"{}"), Confidence::None);
}

#[test]
fn detect_by_content() {
    let parser = Parser;
    let xlsx_bytes = make_test_xlsx(&[("key1", "value1", None)]);
    assert_eq!(parser.detect(".dat", &xlsx_bytes), Confidence::High);
}

// ---------------------------------------------------------------------------
// Parse tests
// ---------------------------------------------------------------------------

#[test]
fn parse_simple() {
    let parser = Parser;
    let xlsx_bytes = make_test_xlsx(&[
        ("greeting", "Hello, World!", Some("Main greeting")),
        ("farewell", "Goodbye!", None),
        ("app.title", "My Application", Some("Shown in header")),
    ]);

    let resource = parser.parse(&xlsx_bytes).expect("parse should succeed");

    assert_eq!(resource.metadata.source_format, FormatId::Excel);
    assert_eq!(resource.entries.len(), 3);

    let greeting = resource
        .entries
        .get("greeting")
        .expect("greeting should exist");
    assert_eq!(
        greeting.value,
        EntryValue::Simple("Hello, World!".to_string())
    );
    assert_eq!(greeting.comments.len(), 1);
    assert_eq!(greeting.comments[0].text, "Main greeting");

    let farewell = resource
        .entries
        .get("farewell")
        .expect("farewell should exist");
    assert_eq!(farewell.value, EntryValue::Simple("Goodbye!".to_string()));
    assert!(farewell.comments.is_empty());

    let app_title = resource
        .entries
        .get("app.title")
        .expect("app.title should exist");
    assert_eq!(
        app_title.value,
        EntryValue::Simple("My Application".to_string())
    );
    assert_eq!(app_title.comments[0].text, "Shown in header");
}

#[test]
fn parse_stores_extension_data() {
    let parser = Parser;
    let xlsx_bytes = make_test_xlsx(&[("k", "v", None)]);
    let resource = parser.parse(&xlsx_bytes).expect("parse should succeed");

    match &resource.metadata.format_ext {
        Some(FormatExtension::Excel(ext)) => {
            assert!(ext.sheet_name.is_some());
            assert_eq!(ext.key_column, Some(0));
            assert_eq!(ext.value_column, Some(1));
        }
        other => panic!("Expected ExcelExt, got {other:?}"),
    }
}

#[test]
fn parse_case_insensitive_headers() {
    let parser = Parser;
    let xlsx_bytes = make_test_xlsx_custom_headers(
        &["Key", "Translation", "Description"],
        &[vec!["hello", "Hello", "A greeting"]],
    );

    let resource = parser.parse(&xlsx_bytes).expect("parse should succeed");
    assert_eq!(resource.entries.len(), 1);

    let entry = resource.entries.get("hello").expect("hello should exist");
    assert_eq!(entry.value, EntryValue::Simple("Hello".to_string()));
    assert_eq!(entry.comments[0].text, "A greeting");
}

#[test]
fn parse_locale_code_header() {
    let parser = Parser;
    let xlsx_bytes = make_test_xlsx_custom_headers(&["ID", "en"], &[vec!["greeting", "Hello"]]);

    let resource = parser.parse(&xlsx_bytes).expect("parse should succeed");
    assert_eq!(resource.entries.len(), 1);
    let entry = resource
        .entries
        .get("greeting")
        .expect("greeting should exist");
    assert_eq!(entry.value, EntryValue::Simple("Hello".to_string()));
}

#[test]
fn parse_skips_empty_key_rows() {
    let parser = Parser;
    let xlsx_bytes = make_test_xlsx(&[
        ("key1", "value1", None),
        ("", "orphan_value", None),
        ("key2", "value2", None),
    ]);

    let resource = parser.parse(&xlsx_bytes).expect("parse should succeed");
    assert_eq!(resource.entries.len(), 2);
    assert!(resource.entries.contains_key("key1"));
    assert!(resource.entries.contains_key("key2"));
}

#[test]
fn parse_error_no_key_column() {
    let parser = Parser;
    let xlsx_bytes = make_test_xlsx_custom_headers(&["foo", "bar"], &[vec!["a", "b"]]);
    assert!(parser.parse(&xlsx_bytes).is_err());
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
            comments: vec![Comment {
                text: "A greeting".to_string(),
                role: CommentRole::General,
                priority: None,
                annotates: None,
            }],
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
            source_format: FormatId::Excel,
            ..Default::default()
        },
        entries,
    };

    let output = writer.write(&resource).expect("write should succeed");

    // Re-parse to verify
    let parser = Parser;
    let reparsed = parser.parse(&output).expect("reparse should succeed");
    assert_eq!(reparsed.entries.len(), 2);

    let greeting = reparsed.entries.get("greeting").expect("greeting");
    assert_eq!(greeting.value, EntryValue::Simple("Hello".to_string()));
    assert_eq!(greeting.comments[0].text, "A greeting");
}

// ---------------------------------------------------------------------------
// Round-trip tests
// ---------------------------------------------------------------------------

#[test]
fn roundtrip_simple() {
    let parser = Parser;
    let writer = Writer;

    let original_xlsx = make_test_xlsx(&[
        ("greeting", "Hello, World!", Some("Main greeting")),
        ("farewell", "Goodbye!", None),
        ("app.title", "My Application", Some("Shown in header")),
    ]);

    let resource = parser.parse(&original_xlsx).expect("parse should succeed");
    let output = writer.write(&resource).expect("write should succeed");
    let reparsed = parser.parse(&output).expect("reparse should succeed");

    // Same number of entries
    assert_eq!(resource.entries.len(), reparsed.entries.len());

    // Same keys and values
    for (key, original) in &resource.entries {
        let reparsed_entry = reparsed
            .entries
            .get(key)
            .unwrap_or_else(|| panic!("Key '{key}' missing after round-trip"));
        assert_eq!(
            original.value, reparsed_entry.value,
            "Value mismatch for key '{key}'"
        );
        assert_eq!(
            original.comments.len(),
            reparsed_entry.comments.len(),
            "Comment count mismatch for key '{key}'"
        );
        for (i, comment) in original.comments.iter().enumerate() {
            assert_eq!(
                comment.text, reparsed_entry.comments[i].text,
                "Comment text mismatch for key '{key}' comment {i}"
            );
        }
    }
}

#[test]
fn roundtrip_no_comments() {
    let parser = Parser;
    let writer = Writer;

    let original_xlsx = make_test_xlsx(&[("key1", "value1", None), ("key2", "value2", None)]);

    let resource = parser.parse(&original_xlsx).expect("parse should succeed");
    let output = writer.write(&resource).expect("write should succeed");
    let reparsed = parser.parse(&output).expect("reparse should succeed");

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, original) in &resource.entries {
        let reparsed_entry = reparsed.entries.get(key).expect("key should exist");
        assert_eq!(original.value, reparsed_entry.value);
    }
}

#[test]
fn roundtrip_preserves_key_order() {
    let parser = Parser;
    let writer = Writer;

    let original_xlsx = make_test_xlsx(&[
        ("z_last", "Z", None),
        ("a_first", "A", None),
        ("m_middle", "M", None),
    ]);

    let resource = parser.parse(&original_xlsx).expect("parse should succeed");
    let output = writer.write(&resource).expect("write should succeed");
    let reparsed = parser.parse(&output).expect("reparse should succeed");

    let original_keys: Vec<_> = resource.entries.keys().collect();
    let reparsed_keys: Vec<_> = reparsed.entries.keys().collect();
    assert_eq!(
        original_keys, reparsed_keys,
        "Key order should be preserved"
    );
}

// ---------------------------------------------------------------------------
// Capabilities test
// ---------------------------------------------------------------------------

#[test]
fn capabilities_are_correct() {
    let parser = Parser;
    let caps = parser.capabilities();
    assert!(caps.comments);
    assert!(caps.nested_keys);
    assert!(!caps.plurals);
    assert!(!caps.arrays);
    assert!(!caps.context);
    assert!(!caps.inline_markup);
}
