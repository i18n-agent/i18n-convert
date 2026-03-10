use i18n_convert::formats::{Confidence, FormatParser, FormatWriter};
use i18n_convert::formats::js_format::{Parser, Writer};
use i18n_convert::ir::*;
use indexmap::IndexMap;

fn parser() -> Parser {
    Parser
}

fn writer() -> Writer {
    Writer
}

fn fixture(name: &str) -> Vec<u8> {
    let path = format!(
        "{}/tests/fixtures/js_format/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read(&path).unwrap_or_else(|e| panic!("Failed to read fixture {path}: {e}"))
}

// ──────────────────────────────────────────────
// Detection tests
// ──────────────────────────────────────────────

#[test]
fn detect_js_by_extension_with_module_exports() {
    let content = b"module.exports = { greeting: 'Hello' };";
    assert_eq!(parser().detect(".js", content), Confidence::High);
}

#[test]
fn detect_js_by_extension_with_export_default() {
    let content = b"export default { greeting: 'Hello' };";
    assert_eq!(parser().detect(".js", content), Confidence::High);
}

#[test]
fn detect_js_by_extension_with_exports_dot() {
    let content = b"exports.messages = { greeting: 'Hello' };";
    assert_eq!(parser().detect(".js", content), Confidence::High);
}

#[test]
fn detect_js_by_extension_no_export() {
    let content = b"var x = { greeting: 'Hello' };";
    assert_eq!(parser().detect(".js", content), Confidence::Low);
}

#[test]
fn detect_js_wrong_extension() {
    let content = b"module.exports = { greeting: 'Hello' };";
    assert_eq!(parser().detect(".ts", content), Confidence::None);
}

#[test]
fn detect_js_wrong_extension_json() {
    let content = b"{ \"greeting\": \"Hello\" }";
    assert_eq!(parser().detect(".json", content), Confidence::None);
}

// ──────────────────────────────────────────────
// Parse fixture tests
// ──────────────────────────────────────────────

#[test]
fn parse_simple_fixture() {
    let content = fixture("simple.js");
    let resource = parser().parse(&content).unwrap();

    assert_eq!(resource.metadata.source_format, FormatId::JavaScript);
    assert_eq!(resource.entries.len(), 3);

    assert_eq!(
        resource.entries["greeting"].value,
        EntryValue::Simple("Hello".to_string())
    );
    assert_eq!(
        resource.entries["farewell"].value,
        EntryValue::Simple("Goodbye".to_string())
    );
    assert_eq!(
        resource.entries["welcome"].value,
        EntryValue::Simple("Welcome to our app".to_string())
    );
}

#[test]
fn parse_nested_fixture() {
    let content = fixture("nested.js");
    let resource = parser().parse(&content).unwrap();

    assert_eq!(resource.entries.len(), 5);

    assert_eq!(
        resource.entries["common.greeting"].value,
        EntryValue::Simple("Hello".to_string())
    );
    assert_eq!(
        resource.entries["common.farewell"].value,
        EntryValue::Simple("Goodbye".to_string())
    );
    assert_eq!(
        resource.entries["pages.home.title"].value,
        EntryValue::Simple("Home Page".to_string())
    );
    assert_eq!(
        resource.entries["pages.home.description"].value,
        EntryValue::Simple("Welcome to our website".to_string())
    );
    assert_eq!(
        resource.entries["pages.about.title"].value,
        EntryValue::Simple("About Us".to_string())
    );
}

#[test]
fn parse_plurals_fixture() {
    let content = fixture("plurals.js");
    let resource = parser().parse(&content).unwrap();

    // Should have 2 plural groups: items and files
    assert_eq!(resource.entries.len(), 2);

    let items = &resource.entries["items"];
    match &items.value {
        EntryValue::Plural(ps) => {
            assert_eq!(ps.one, Some("1 item".to_string()));
            assert_eq!(ps.other, "{count} items".to_string());
            assert!(ps.zero.is_none());
        }
        other => panic!("Expected Plural, got {:?}", other),
    }

    let files = &resource.entries["files"];
    match &files.value {
        EntryValue::Plural(ps) => {
            assert_eq!(ps.zero, Some("No files".to_string()));
            assert_eq!(ps.one, Some("1 file".to_string()));
            assert_eq!(ps.other, "{count} files".to_string());
        }
        other => panic!("Expected Plural, got {:?}", other),
    }
}

#[test]
fn parse_esmodule_fixture() {
    let content = fixture("esmodule.js");
    let resource = parser().parse(&content).unwrap();

    assert_eq!(resource.metadata.source_format, FormatId::JavaScript);
    assert_eq!(resource.entries.len(), 3);

    assert_eq!(
        resource.entries["greeting"].value,
        EntryValue::Simple("Hello".to_string())
    );
    assert_eq!(
        resource.entries["nested.welcome"].value,
        EntryValue::Simple("Welcome".to_string())
    );

    // Verify export style was detected
    match &resource.metadata.format_ext {
        Some(FormatExtension::JavaScript(ext)) => {
            assert_eq!(ext.export_style, Some("export default".to_string()));
        }
        other => panic!("Expected JavaScript extension, got {:?}", other),
    }
}

// ──────────────────────────────────────────────
// Format extension preservation tests
// ──────────────────────────────────────────────

#[test]
fn preserve_module_exports_style() {
    let content = b"module.exports = {\n  greeting: \"Hello\"\n};\n";
    let resource = parser().parse(content).unwrap();
    match &resource.metadata.format_ext {
        Some(FormatExtension::JavaScript(ext)) => {
            assert_eq!(ext.export_style, Some("module.exports".to_string()));
        }
        other => panic!("Expected JavaScript extension, got {:?}", other),
    }
}

#[test]
fn preserve_export_default_style() {
    let content = b"export default {\n  greeting: \"Hello\"\n};\n";
    let resource = parser().parse(content).unwrap();
    match &resource.metadata.format_ext {
        Some(FormatExtension::JavaScript(ext)) => {
            assert_eq!(ext.export_style, Some("export default".to_string()));
        }
        other => panic!("Expected JavaScript extension, got {:?}", other),
    }
}

#[test]
fn preserve_double_quote_style() {
    let content = b"module.exports = {\n  greeting: \"Hello\"\n};\n";
    let resource = parser().parse(content).unwrap();
    match &resource.metadata.format_ext {
        Some(FormatExtension::JavaScript(ext)) => {
            assert_eq!(ext.quote_style, Some('"'));
        }
        other => panic!("Expected JavaScript extension, got {:?}", other),
    }
}

#[test]
fn preserve_single_quote_style() {
    let content = b"module.exports = {\n  greeting: 'Hello'\n};\n";
    let resource = parser().parse(content).unwrap();
    match &resource.metadata.format_ext {
        Some(FormatExtension::JavaScript(ext)) => {
            assert_eq!(ext.quote_style, Some('\''));
        }
        other => panic!("Expected JavaScript extension, got {:?}", other),
    }
}

// ──────────────────────────────────────────────
// Comment preservation tests
// ──────────────────────────────────────────────

#[test]
fn parse_comments_before_keys() {
    let content = b"module.exports = {\n  // General greeting\n  greeting: \"Hello\",\n  // Farewell message\n  farewell: \"Goodbye\"\n};\n";
    let resource = parser().parse(content).unwrap();

    assert_eq!(resource.entries.len(), 2);

    let greeting = &resource.entries["greeting"];
    assert_eq!(greeting.comments.len(), 1);
    assert_eq!(greeting.comments[0].text, "General greeting");
    assert_eq!(greeting.comments[0].role, CommentRole::Translator);

    let farewell = &resource.entries["farewell"];
    assert_eq!(farewell.comments.len(), 1);
    assert_eq!(farewell.comments[0].text, "Farewell message");
}

#[test]
fn parse_multiple_comments_before_key() {
    let content = b"module.exports = {\n  // Line 1\n  // Line 2\n  greeting: \"Hello\"\n};\n";
    let resource = parser().parse(content).unwrap();

    let greeting = &resource.entries["greeting"];
    assert_eq!(greeting.comments.len(), 2);
    assert_eq!(greeting.comments[0].text, "Line 1");
    assert_eq!(greeting.comments[1].text, "Line 2");
}

// ──────────────────────────────────────────────
// Writer tests
// ──────────────────────────────────────────────

#[test]
fn write_simple_entries() {
    let mut entries = IndexMap::new();
    entries.insert(
        "greeting".to_string(),
        I18nEntry {
            key: "greeting".to_string(),
            value: EntryValue::Simple("Hello".to_string()),
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
            source_format: FormatId::JavaScript,
            format_ext: Some(FormatExtension::JavaScript(JavaScriptExt {
                export_style: Some("module.exports".to_string()),
                quote_style: Some('"'),
            })),
            ..Default::default()
        },
        entries,
    };

    let output = writer().write(&resource).unwrap();
    let text = std::str::from_utf8(&output).unwrap();

    assert!(text.starts_with("module.exports = {"));
    assert!(text.contains("greeting: \"Hello\""));
    assert!(text.contains("farewell: \"Goodbye\""));
    assert!(text.ends_with(";\n"));
}

#[test]
fn write_nested_entries() {
    let mut entries = IndexMap::new();
    entries.insert(
        "common.greeting".to_string(),
        I18nEntry {
            key: "common.greeting".to_string(),
            value: EntryValue::Simple("Hello".to_string()),
            ..Default::default()
        },
    );
    entries.insert(
        "common.farewell".to_string(),
        I18nEntry {
            key: "common.farewell".to_string(),
            value: EntryValue::Simple("Goodbye".to_string()),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::JavaScript,
            format_ext: Some(FormatExtension::JavaScript(JavaScriptExt {
                export_style: Some("module.exports".to_string()),
                quote_style: Some('"'),
            })),
            ..Default::default()
        },
        entries,
    };

    let output = writer().write(&resource).unwrap();
    let text = std::str::from_utf8(&output).unwrap();

    assert!(text.contains("common: {"));
    assert!(text.contains("greeting: \"Hello\""));
    assert!(text.contains("farewell: \"Goodbye\""));
}

#[test]
fn write_plural_entries() {
    let mut entries = IndexMap::new();
    entries.insert(
        "items".to_string(),
        I18nEntry {
            key: "items".to_string(),
            value: EntryValue::Plural(PluralSet {
                one: Some("1 item".to_string()),
                other: "{count} items".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::JavaScript,
            format_ext: Some(FormatExtension::JavaScript(JavaScriptExt {
                export_style: Some("module.exports".to_string()),
                quote_style: Some('"'),
            })),
            ..Default::default()
        },
        entries,
    };

    let output = writer().write(&resource).unwrap();
    let text = std::str::from_utf8(&output).unwrap();

    assert!(text.contains("items_one: \"1 item\""));
    assert!(text.contains("items_other: \"{count} items\""));
}

#[test]
fn write_with_export_default() {
    let mut entries = IndexMap::new();
    entries.insert(
        "greeting".to_string(),
        I18nEntry {
            key: "greeting".to_string(),
            value: EntryValue::Simple("Hello".to_string()),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::JavaScript,
            format_ext: Some(FormatExtension::JavaScript(JavaScriptExt {
                export_style: Some("export default".to_string()),
                quote_style: Some('"'),
            })),
            ..Default::default()
        },
        entries,
    };

    let output = writer().write(&resource).unwrap();
    let text = std::str::from_utf8(&output).unwrap();

    assert!(text.starts_with("export default {"));
}

#[test]
fn write_empty_resource() {
    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::JavaScript,
            format_ext: Some(FormatExtension::JavaScript(JavaScriptExt::default())),
            ..Default::default()
        },
        entries: IndexMap::new(),
    };

    let output = writer().write(&resource).unwrap();
    let text = std::str::from_utf8(&output).unwrap();

    assert!(text.contains("{}"));
    assert!(text.ends_with(";\n"));
}

// ──────────────────────────────────────────────
// Roundtrip tests
// ──────────────────────────────────────────────

#[test]
fn roundtrip_simple() {
    let content = fixture("simple.js");
    let resource = parser().parse(&content).unwrap();
    let output = writer().write(&resource).unwrap();
    let reparsed = parser().parse(&output).unwrap();

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, entry) in &resource.entries {
        let reparsed_entry = &reparsed.entries[key];
        assert_eq!(entry.value, reparsed_entry.value, "Mismatch for key: {key}");
    }
}

#[test]
fn roundtrip_nested() {
    let content = fixture("nested.js");
    let resource = parser().parse(&content).unwrap();
    let output = writer().write(&resource).unwrap();
    let reparsed = parser().parse(&output).unwrap();

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, entry) in &resource.entries {
        let reparsed_entry = &reparsed.entries[key];
        assert_eq!(entry.value, reparsed_entry.value, "Mismatch for key: {key}");
    }
}

#[test]
fn roundtrip_plurals() {
    let content = fixture("plurals.js");
    let resource = parser().parse(&content).unwrap();
    let output = writer().write(&resource).unwrap();
    let reparsed = parser().parse(&output).unwrap();

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, entry) in &resource.entries {
        let reparsed_entry = &reparsed.entries[key];
        assert_eq!(entry.value, reparsed_entry.value, "Mismatch for key: {key}");
    }
}

#[test]
fn roundtrip_esmodule() {
    let content = fixture("esmodule.js");
    let resource = parser().parse(&content).unwrap();
    let output = writer().write(&resource).unwrap();
    let reparsed = parser().parse(&output).unwrap();

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, entry) in &resource.entries {
        let reparsed_entry = &reparsed.entries[key];
        assert_eq!(entry.value, reparsed_entry.value, "Mismatch for key: {key}");
    }
}

// ──────────────────────────────────────────────
// Edge case tests
// ──────────────────────────────────────────────

#[test]
fn parse_empty_input() {
    let resource = parser().parse(b"").unwrap();
    assert!(resource.entries.is_empty());
    assert_eq!(resource.metadata.source_format, FormatId::JavaScript);
}

#[test]
fn parse_empty_object() {
    let content = b"module.exports = {};";
    let resource = parser().parse(content).unwrap();
    assert!(resource.entries.is_empty());
}

#[test]
fn parse_single_quotes() {
    let content = b"module.exports = {\n  greeting: 'Hello',\n  farewell: 'Goodbye'\n};\n";
    let resource = parser().parse(content).unwrap();

    assert_eq!(resource.entries.len(), 2);
    assert_eq!(
        resource.entries["greeting"].value,
        EntryValue::Simple("Hello".to_string())
    );
}

#[test]
fn parse_trailing_commas() {
    let content = b"module.exports = {\n  greeting: \"Hello\",\n  farewell: \"Goodbye\",\n};\n";
    let resource = parser().parse(content).unwrap();
    assert_eq!(resource.entries.len(), 2);
}

#[test]
fn parse_with_inline_comments() {
    // inline comments after values should not break parsing
    let content = b"module.exports = {\n  greeting: \"Hello\", // inline comment\n  farewell: \"Goodbye\"\n};\n";
    let resource = parser().parse(content).unwrap();
    assert_eq!(resource.entries.len(), 2);
    assert_eq!(
        resource.entries["greeting"].value,
        EntryValue::Simple("Hello".to_string())
    );
}

#[test]
fn entry_keys_match_entry_key_field() {
    let content = fixture("nested.js");
    let resource = parser().parse(&content).unwrap();

    for (map_key, entry) in &resource.entries {
        assert_eq!(
            map_key, &entry.key,
            "Map key should match entry.key for '{map_key}'"
        );
    }
}

#[test]
fn capabilities_are_correct() {
    let caps = parser().capabilities();
    assert!(caps.plurals);
    assert!(caps.comments);
    assert!(caps.nested_keys);
    assert!(!caps.arrays);
    assert!(!caps.context);
    assert!(!caps.source_string);
}

#[test]
fn writer_capabilities_match_parser() {
    let parser_caps = parser().capabilities();
    let writer_caps = writer().capabilities();
    assert_eq!(parser_caps, writer_caps);
}
