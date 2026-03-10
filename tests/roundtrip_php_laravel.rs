use i18n_convert::formats::php_laravel::{Parser, Writer};
use i18n_convert::formats::{Confidence, FormatParser, FormatWriter};
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
        "{}/tests/fixtures/php_laravel/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read(&path).unwrap_or_else(|e| panic!("Failed to read fixture {path}: {e}"))
}

// ──────────────────────────────────────────────
// Detection tests
// ──────────────────────────────────────────────

#[test]
fn detect_php_extension_with_return_array() {
    let content = b"<?php\n\nreturn [\n    'key' => 'value',\n];\n";
    assert_eq!(parser().detect(".php", content), Confidence::Definite);
}

#[test]
fn detect_php_extension_with_array_syntax() {
    let content = b"<?php\n\nreturn array(\n    'key' => 'value',\n);\n";
    assert_eq!(parser().detect(".php", content), Confidence::Definite);
}

#[test]
fn detect_php_extension_only() {
    let content = b"<?php\n\necho 'hello';\n";
    assert_eq!(parser().detect(".php", content), Confidence::Low);
}

#[test]
fn detect_non_php_with_content() {
    let content = b"<?php\n\nreturn [\n    'key' => 'value',\n];\n";
    assert_eq!(parser().detect(".txt", content), Confidence::High);
}

#[test]
fn detect_no_match() {
    let content = b"just some text";
    assert_eq!(parser().detect(".json", content), Confidence::None);
}

// ──────────────────────────────────────────────
// Parsing tests
// ──────────────────────────────────────────────

#[test]
fn parse_simple_fixture() {
    let content = fixture("simple.php");
    let resource = parser().parse(&content).expect("parse should succeed");

    assert_eq!(resource.metadata.source_format, FormatId::PhpLaravel);
    assert_eq!(resource.entries.len(), 4);

    assert_eq!(
        resource.entries["greeting"].value,
        EntryValue::Simple("Hello, World!".to_string())
    );
    assert_eq!(
        resource.entries["farewell"].value,
        EntryValue::Simple("Goodbye!".to_string())
    );
    assert_eq!(
        resource.entries["app_name"].value,
        EntryValue::Simple("My Application".to_string())
    );
    assert_eq!(
        resource.entries["empty_value"].value,
        EntryValue::Simple("".to_string())
    );
}

#[test]
fn parse_nested_fixture() {
    let content = fixture("nested.php");
    let resource = parser().parse(&content).expect("parse should succeed");

    assert_eq!(resource.entries.len(), 4);

    assert_eq!(
        resource.entries["greeting"].value,
        EntryValue::Simple("Hello, World!".to_string())
    );
    assert_eq!(
        resource.entries["nested.welcome"].value,
        EntryValue::Simple("Welcome to :name".to_string())
    );
    assert_eq!(
        resource.entries["nested.items"].value,
        EntryValue::Simple("{0} No items|{1} One item|[2,*] :count items".to_string())
    );
    assert_eq!(
        resource.entries["farewell"].value,
        EntryValue::Simple("Goodbye!".to_string())
    );
}

#[test]
fn parse_comments_fixture() {
    let content = fixture("comments.php");
    let resource = parser().parse(&content).expect("parse should succeed");

    assert_eq!(resource.entries.len(), 4);

    // Entry with single-line comment
    let greeting = &resource.entries["greeting"];
    assert_eq!(greeting.comments.len(), 1);
    assert_eq!(greeting.comments[0].text, "A greeting message");
    assert_eq!(greeting.comments[0].role, CommentRole::General);

    // Entry without comment
    let farewell = &resource.entries["farewell"];
    assert!(farewell.comments.is_empty());

    // Entry with single-line comment
    let error = &resource.entries["error"];
    assert_eq!(error.comments.len(), 1);
    assert_eq!(error.comments[0].text, "A comment about errors");

    // Entry with block comment
    let success = &resource.entries["success"];
    assert_eq!(success.comments.len(), 1);
    assert_eq!(success.comments[0].text, "Block comment about success");
}

#[test]
fn parse_escapes_fixture() {
    let content = fixture("escapes.php");
    let resource = parser().parse(&content).expect("parse should succeed");

    assert_eq!(resource.entries.len(), 3);

    assert_eq!(
        resource.entries["escaped_quote"].value,
        EntryValue::Simple("It's a test".to_string())
    );
    assert_eq!(
        resource.entries["escaped_backslash"].value,
        EntryValue::Simple("C:\\Users\\test".to_string())
    );
    assert_eq!(
        resource.entries["mixed"].value,
        EntryValue::Simple("Say 'hello' and \\go".to_string())
    );
}

#[test]
fn parse_full_fixture() {
    let content = fixture("full.php");
    let resource = parser().parse(&content).expect("parse should succeed");

    assert_eq!(resource.entries.len(), 6);

    // Nested key
    assert_eq!(
        resource.entries["nested.welcome"].value,
        EntryValue::Simple("Welcome to :name".to_string())
    );
    // Deeply nested
    assert_eq!(
        resource.entries["nested.deep.value"].value,
        EntryValue::Simple("Deep nested value".to_string())
    );
    // Comment
    assert_eq!(resource.entries["greeting"].comments.len(), 1);
    assert_eq!(
        resource.entries["greeting"].comments[0].text,
        "Main greeting"
    );
    // Escapes
    assert_eq!(
        resource.entries["escaped"].value,
        EntryValue::Simple("It's a \\test".to_string())
    );
}

#[test]
fn parse_invalid_php_returns_error() {
    let content = b"not a php file";
    assert!(parser().parse(content).is_err());
}

#[test]
fn parse_format_extension_is_set() {
    let content = fixture("simple.php");
    let resource = parser().parse(&content).expect("parse should succeed");

    match &resource.metadata.format_ext {
        Some(FormatExtension::PhpLaravel(ext)) => {
            assert!(ext.quote_style.is_some());
        }
        other => panic!("Expected PhpLaravelExt, got {other:?}"),
    }
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
            source_format: FormatId::PhpLaravel,
            ..Default::default()
        },
        entries,
    };

    let output = writer().write(&resource).expect("write should succeed");
    let text = String::from_utf8(output).expect("valid UTF-8");

    assert!(text.starts_with("<?php\n\nreturn [\n"));
    assert!(text.ends_with("];\n"));
    assert!(text.contains("'greeting' => 'Hello',"));
    assert!(text.contains("'farewell' => 'Goodbye',"));
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
            source_format: FormatId::PhpLaravel,
            ..Default::default()
        },
        entries,
    };

    let output = writer().write(&resource).expect("write should succeed");
    let text = String::from_utf8(output).expect("valid UTF-8");

    assert!(text.contains("'common' => ["));
    assert!(text.contains("'greeting' => 'Hello',"));
    assert!(text.contains("'farewell' => 'Goodbye',"));
}

#[test]
fn write_with_comments() {
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

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::PhpLaravel,
            ..Default::default()
        },
        entries,
    };

    let output = writer().write(&resource).expect("write should succeed");
    let text = String::from_utf8(output).expect("valid UTF-8");

    assert!(text.contains("// A greeting"));
    assert!(text.contains("'greeting' => 'Hello',"));
}

#[test]
fn write_escapes_values() {
    let mut entries = IndexMap::new();
    entries.insert(
        "escaped".to_string(),
        I18nEntry {
            key: "escaped".to_string(),
            value: EntryValue::Simple("It's a \\test".to_string()),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::PhpLaravel,
            ..Default::default()
        },
        entries,
    };

    let output = writer().write(&resource).expect("write should succeed");
    let text = String::from_utf8(output).expect("valid UTF-8");

    assert!(text.contains("'escaped' => 'It\\'s a \\\\test',"));
}

#[test]
fn write_empty_resource() {
    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::PhpLaravel,
            ..Default::default()
        },
        entries: IndexMap::new(),
    };

    let output = writer().write(&resource).expect("write should succeed");
    let text = String::from_utf8(output).expect("valid UTF-8");

    assert_eq!(text, "<?php\n\nreturn [\n];\n");
}

// ──────────────────────────────────────────────
// Round-trip tests
// ──────────────────────────────────────────────

#[test]
fn roundtrip_simple() {
    let content = fixture("simple.php");
    let resource = parser().parse(&content).expect("parse should succeed");
    let output = writer().write(&resource).expect("write should succeed");
    let reparsed = parser().parse(&output).expect("reparse should succeed");

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, original) in &resource.entries {
        let reparsed_entry = reparsed
            .entries
            .get(key)
            .unwrap_or_else(|| panic!("Key '{key}' missing after round-trip"));
        assert_eq!(
            original.value, reparsed_entry.value,
            "Value mismatch for key '{key}'"
        );
    }
}

#[test]
fn roundtrip_nested() {
    let content = fixture("nested.php");
    let resource = parser().parse(&content).expect("parse should succeed");
    let output = writer().write(&resource).expect("write should succeed");
    let reparsed = parser().parse(&output).expect("reparse should succeed");

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, original) in &resource.entries {
        let reparsed_entry = reparsed
            .entries
            .get(key)
            .unwrap_or_else(|| panic!("Key '{key}' missing after round-trip"));
        assert_eq!(
            original.value, reparsed_entry.value,
            "Value mismatch for key '{key}'"
        );
    }
}

#[test]
fn roundtrip_comments() {
    let content = fixture("comments.php");
    let resource = parser().parse(&content).expect("parse should succeed");
    let output = writer().write(&resource).expect("write should succeed");
    let reparsed = parser().parse(&output).expect("reparse should succeed");

    assert_eq!(resource.entries.len(), reparsed.entries.len());
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
fn roundtrip_escapes() {
    let content = fixture("escapes.php");
    let resource = parser().parse(&content).expect("parse should succeed");
    let output = writer().write(&resource).expect("write should succeed");
    let reparsed = parser().parse(&output).expect("reparse should succeed");

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, original) in &resource.entries {
        let reparsed_entry = reparsed
            .entries
            .get(key)
            .unwrap_or_else(|| panic!("Key '{key}' missing after round-trip"));
        assert_eq!(
            original.value, reparsed_entry.value,
            "Value mismatch for key '{key}'"
        );
    }
}

#[test]
fn roundtrip_full() {
    let content = fixture("full.php");
    let resource = parser().parse(&content).expect("parse should succeed");
    let output = writer().write(&resource).expect("write should succeed");
    let reparsed = parser().parse(&output).expect("reparse should succeed");

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, original) in &resource.entries {
        let reparsed_entry = reparsed
            .entries
            .get(key)
            .unwrap_or_else(|| panic!("Key '{key}' missing after round-trip"));
        assert_eq!(
            original.value, reparsed_entry.value,
            "Value mismatch for key '{key}'"
        );
    }
}

// ──────────────────────────────────────────────
// Capabilities test
// ──────────────────────────────────────────────

#[test]
fn capabilities_are_correct() {
    let caps = parser().capabilities();
    assert!(caps.comments);
    assert!(caps.nested_keys);
    assert!(!caps.plurals);
    assert!(!caps.arrays);
    assert!(!caps.context);
    assert!(!caps.source_string);
}

// ──────────────────────────────────────────────
// Edge cases
// ──────────────────────────────────────────────

#[test]
fn entry_keys_match_entry_key_field() {
    let content = fixture("nested.php");
    let resource = parser().parse(&content).expect("parse should succeed");

    for (map_key, entry) in &resource.entries {
        assert_eq!(
            map_key, &entry.key,
            "Map key should match entry.key for '{map_key}'"
        );
    }
}

#[test]
fn parse_double_quoted_strings() {
    let content = br#"<?php

return [
    "key" => "value with \"quotes\"",
];
"#;
    let resource = parser().parse(content).expect("parse should succeed");
    assert_eq!(resource.entries.len(), 1);
    assert_eq!(
        resource.entries["key"].value,
        EntryValue::Simple("value with \"quotes\"".to_string())
    );
}

#[test]
fn parse_return_array_syntax() {
    let content = br#"<?php

return array(
    'key' => 'value',
    'other' => 'data',
);
"#;
    let resource = parser().parse(content).expect("parse should succeed");
    assert_eq!(resource.entries.len(), 2);
    assert_eq!(
        resource.entries["key"].value,
        EntryValue::Simple("value".to_string())
    );
}
