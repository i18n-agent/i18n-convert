use i18n_convert::ir::*;
use i18n_convert::formats::{Confidence, FormatParser, FormatWriter};
use i18n_convert::formats::stringsdict::{Parser, Writer};
use indexmap::IndexMap;

// ─── Fixture loading helpers ──────────────────────────────────────────────────

fn fixture(name: &str) -> Vec<u8> {
    let path = format!(
        "{}/tests/fixtures/stringsdict/{}",
        env!("CARGO_MANIFEST_DIR"),
        name
    );
    std::fs::read(&path).unwrap_or_else(|e| panic!("Failed to read fixture {}: {}", path, e))
}

// ─── Parse tests ──────────────────────────────────────────────────────────────

#[test]
fn parse_single_var() {
    let content = fixture("single_var.stringsdict");
    let parser = Parser;
    let resource = parser.parse(&content).expect("should parse single_var");

    assert_eq!(resource.metadata.source_format, FormatId::Stringsdict);
    assert_eq!(resource.entries.len(), 1);

    let entry = resource.entries.get("item_count").expect("should have item_count");
    assert_eq!(entry.key, "item_count");

    match &entry.value {
        EntryValue::Plural(ps) => {
            assert_eq!(ps.one, Some("%d item".to_string()));
            assert_eq!(ps.other, "%d items");
            assert!(ps.zero.is_none());
            assert!(ps.two.is_none());
            assert!(ps.few.is_none());
            assert!(ps.many.is_none());
        }
        other => panic!("Expected Plural, got {:?}", other),
    }
}

#[test]
fn parse_multi_var() {
    let content = fixture("multi_var.stringsdict");
    let parser = Parser;
    let resource = parser.parse(&content).expect("should parse multi_var");

    assert_eq!(resource.entries.len(), 1);

    let entry = resource
        .entries
        .get("files_in_folders")
        .expect("should have files_in_folders");

    match &entry.value {
        EntryValue::MultiVariablePlural(mvp) => {
            assert_eq!(mvp.pattern, "%#@files@ in %#@folders@");
            assert_eq!(mvp.variables.len(), 2);

            let files_var = mvp.variables.get("files").expect("should have files var");
            assert_eq!(files_var.format_specifier, Some("d".to_string()));
            assert_eq!(files_var.plural_set.one, Some("%d file".to_string()));
            assert_eq!(files_var.plural_set.other, "%d files");

            let folders_var = mvp.variables.get("folders").expect("should have folders var");
            assert_eq!(folders_var.format_specifier, Some("d".to_string()));
            assert_eq!(folders_var.plural_set.one, Some("%d folder".to_string()));
            assert_eq!(folders_var.plural_set.other, "%d folders");
        }
        other => panic!("Expected MultiVariablePlural, got {:?}", other),
    }
}

#[test]
fn parse_all_categories() {
    let content = fixture("all_categories.stringsdict");
    let parser = Parser;
    let resource = parser.parse(&content).expect("should parse all_categories");

    assert_eq!(resource.entries.len(), 1);

    let entry = resource
        .entries
        .get("messages_count")
        .expect("should have messages_count");

    match &entry.value {
        EntryValue::Plural(ps) => {
            assert_eq!(ps.zero, Some("No messages".to_string()));
            assert_eq!(ps.one, Some("%d message".to_string()));
            assert_eq!(ps.two, Some("%d messages (dual)".to_string()));
            assert_eq!(ps.few, Some("%d messages (few)".to_string()));
            assert_eq!(ps.many, Some("%d messages (many)".to_string()));
            assert_eq!(ps.other, "%d messages");
        }
        other => panic!("Expected Plural, got {:?}", other),
    }
}

// ─── Write tests ──────────────────────────────────────────────────────────────

#[test]
fn write_single_var_produces_valid_plist() {
    let writer = Writer;
    let parser = Parser;

    let mut entries = IndexMap::new();
    entries.insert(
        "item_count".to_string(),
        I18nEntry {
            key: "item_count".to_string(),
            value: EntryValue::Plural(PluralSet {
                one: Some("%d item".to_string()),
                other: "%d items".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Stringsdict,
            ..Default::default()
        },
        entries,
    };

    let output = writer.write(&resource).expect("should write");
    let output_str = String::from_utf8(output.clone()).expect("should be valid UTF-8");

    // Verify it's valid XML plist
    assert!(output_str.contains("<?xml version=\"1.0\""));
    assert!(output_str.contains("NSStringLocalizedFormatKey"));
    assert!(output_str.contains("NSStringPluralRuleType"));
    assert!(output_str.contains("%d item"));
    assert!(output_str.contains("%d items"));

    // Verify it can be re-parsed
    let reparsed = parser.parse(&output).expect("should reparse written output");
    assert_eq!(reparsed.entries.len(), 1);
}

#[test]
fn write_multi_var_produces_valid_plist() {
    let writer = Writer;
    let parser = Parser;

    let mut variables = IndexMap::new();
    variables.insert(
        "files".to_string(),
        PluralVariable {
            name: "files".to_string(),
            format_specifier: Some("d".to_string()),
            arg_num: None,
            plural_set: PluralSet {
                one: Some("%d file".to_string()),
                other: "%d files".to_string(),
                ..Default::default()
            },
        },
    );
    variables.insert(
        "folders".to_string(),
        PluralVariable {
            name: "folders".to_string(),
            format_specifier: Some("d".to_string()),
            arg_num: None,
            plural_set: PluralSet {
                one: Some("%d folder".to_string()),
                other: "%d folders".to_string(),
                ..Default::default()
            },
        },
    );

    let mut entries = IndexMap::new();
    entries.insert(
        "files_in_folders".to_string(),
        I18nEntry {
            key: "files_in_folders".to_string(),
            value: EntryValue::MultiVariablePlural(MultiVariablePlural {
                pattern: "%#@files@ in %#@folders@".to_string(),
                variables,
            }),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Stringsdict,
            ..Default::default()
        },
        entries,
    };

    let output = writer.write(&resource).expect("should write");

    // Verify it can be re-parsed
    let reparsed = parser.parse(&output).expect("should reparse written output");
    assert_eq!(reparsed.entries.len(), 1);

    let entry = reparsed
        .entries
        .get("files_in_folders")
        .expect("should have files_in_folders");

    match &entry.value {
        EntryValue::MultiVariablePlural(mvp) => {
            assert_eq!(mvp.pattern, "%#@files@ in %#@folders@");
            assert_eq!(mvp.variables.len(), 2);
        }
        other => panic!("Expected MultiVariablePlural, got {:?}", other),
    }
}

// ─── Round-trip tests ─────────────────────────────────────────────────────────

#[test]
fn roundtrip_single_var() {
    let parser = Parser;
    let writer = Writer;

    let content = fixture("single_var.stringsdict");
    let resource1 = parser.parse(&content).expect("first parse");
    let written = writer.write(&resource1).expect("write");
    let resource2 = parser.parse(&written).expect("second parse");

    // Compare entry-by-entry
    assert_eq!(resource1.entries.len(), resource2.entries.len());

    for (key, entry1) in &resource1.entries {
        let entry2 = resource2.entries.get(key).unwrap_or_else(|| {
            panic!("Key '{}' missing after round-trip", key);
        });
        assert_eq!(
            entry1.value, entry2.value,
            "Value mismatch for key '{}'",
            key
        );
    }
}

#[test]
fn roundtrip_multi_var() {
    let parser = Parser;
    let writer = Writer;

    let content = fixture("multi_var.stringsdict");
    let resource1 = parser.parse(&content).expect("first parse");
    let written = writer.write(&resource1).expect("write");
    let resource2 = parser.parse(&written).expect("second parse");

    assert_eq!(resource1.entries.len(), resource2.entries.len());

    for (key, entry1) in &resource1.entries {
        let entry2 = resource2.entries.get(key).unwrap_or_else(|| {
            panic!("Key '{}' missing after round-trip", key);
        });
        assert_eq!(
            entry1.value, entry2.value,
            "Value mismatch for key '{}'",
            key
        );
    }
}

#[test]
fn roundtrip_all_categories() {
    let parser = Parser;
    let writer = Writer;

    let content = fixture("all_categories.stringsdict");
    let resource1 = parser.parse(&content).expect("first parse");
    let written = writer.write(&resource1).expect("write");
    let resource2 = parser.parse(&written).expect("second parse");

    assert_eq!(resource1.entries.len(), resource2.entries.len());

    for (key, entry1) in &resource1.entries {
        let entry2 = resource2.entries.get(key).unwrap_or_else(|| {
            panic!("Key '{}' missing after round-trip", key);
        });
        assert_eq!(
            entry1.value, entry2.value,
            "Value mismatch for key '{}'",
            key
        );
    }
}

// ─── Edge case tests ──────────────────────────────────────────────────────────

#[test]
fn roundtrip_all_six_categories_preserved() {
    let parser = Parser;
    let writer = Writer;

    // Build an entry with all 6 CLDR categories
    let mut entries = IndexMap::new();
    entries.insert(
        "full_plural".to_string(),
        I18nEntry {
            key: "full_plural".to_string(),
            value: EntryValue::Plural(PluralSet {
                zero: Some("zero val".to_string()),
                one: Some("one val".to_string()),
                two: Some("two val".to_string()),
                few: Some("few val".to_string()),
                many: Some("many val".to_string()),
                other: "other val".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Stringsdict,
            ..Default::default()
        },
        entries,
    };

    let written = writer.write(&resource).expect("write");
    let reparsed = parser.parse(&written).expect("reparse");

    let entry = reparsed.entries.get("full_plural").expect("should exist");
    match &entry.value {
        EntryValue::Plural(ps) => {
            assert_eq!(ps.zero, Some("zero val".to_string()));
            assert_eq!(ps.one, Some("one val".to_string()));
            assert_eq!(ps.two, Some("two val".to_string()));
            assert_eq!(ps.few, Some("few val".to_string()));
            assert_eq!(ps.many, Some("many val".to_string()));
            assert_eq!(ps.other, "other val");
        }
        other => panic!("Expected Plural, got {:?}", other),
    }
}

#[test]
fn roundtrip_multiple_entries() {
    let parser = Parser;
    let writer = Writer;

    let mut entries = IndexMap::new();

    entries.insert(
        "greeting".to_string(),
        I18nEntry {
            key: "greeting".to_string(),
            value: EntryValue::Plural(PluralSet {
                one: Some("%d greeting".to_string()),
                other: "%d greetings".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        },
    );

    let mut variables = IndexMap::new();
    variables.insert(
        "photos".to_string(),
        PluralVariable {
            name: "photos".to_string(),
            format_specifier: Some("d".to_string()),
            arg_num: None,
            plural_set: PluralSet {
                one: Some("%d photo".to_string()),
                other: "%d photos".to_string(),
                ..Default::default()
            },
        },
    );
    variables.insert(
        "albums".to_string(),
        PluralVariable {
            name: "albums".to_string(),
            format_specifier: Some("d".to_string()),
            arg_num: None,
            plural_set: PluralSet {
                one: Some("%d album".to_string()),
                other: "%d albums".to_string(),
                ..Default::default()
            },
        },
    );

    entries.insert(
        "photo_album".to_string(),
        I18nEntry {
            key: "photo_album".to_string(),
            value: EntryValue::MultiVariablePlural(MultiVariablePlural {
                pattern: "%#@photos@ in %#@albums@".to_string(),
                variables,
            }),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Stringsdict,
            ..Default::default()
        },
        entries,
    };

    let written = writer.write(&resource).expect("write");
    let reparsed = parser.parse(&written).expect("reparse");

    assert_eq!(reparsed.entries.len(), 2);

    // Check single-var entry
    let greeting = reparsed.entries.get("greeting").expect("should have greeting");
    match &greeting.value {
        EntryValue::Plural(ps) => {
            assert_eq!(ps.one, Some("%d greeting".to_string()));
            assert_eq!(ps.other, "%d greetings");
        }
        other => panic!("Expected Plural for greeting, got {:?}", other),
    }

    // Check multi-var entry
    let photo_album = reparsed
        .entries
        .get("photo_album")
        .expect("should have photo_album");
    match &photo_album.value {
        EntryValue::MultiVariablePlural(mvp) => {
            assert_eq!(mvp.pattern, "%#@photos@ in %#@albums@");
            assert_eq!(mvp.variables.len(), 2);
            assert!(mvp.variables.contains_key("photos"));
            assert!(mvp.variables.contains_key("albums"));
        }
        other => panic!("Expected MultiVariablePlural for photo_album, got {:?}", other),
    }
}

#[test]
fn write_preserves_format_specifier_lld() {
    let writer = Writer;
    let parser = Parser;

    let mut variables = IndexMap::new();
    variables.insert(
        "bytes".to_string(),
        PluralVariable {
            name: "bytes".to_string(),
            format_specifier: Some("lld".to_string()),
            arg_num: None,
            plural_set: PluralSet {
                one: Some("%lld byte".to_string()),
                other: "%lld bytes".to_string(),
                ..Default::default()
            },
        },
    );

    let mut entries = IndexMap::new();
    entries.insert(
        "byte_count".to_string(),
        I18nEntry {
            key: "byte_count".to_string(),
            value: EntryValue::MultiVariablePlural(MultiVariablePlural {
                pattern: "%#@bytes@".to_string(),
                variables,
            }),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Stringsdict,
            ..Default::default()
        },
        entries,
    };

    let output = writer.write(&resource).expect("write");
    let output_str = String::from_utf8(output.clone()).expect("valid utf-8");
    assert!(
        output_str.contains("<string>lld</string>"),
        "Should preserve lld format specifier in output"
    );

    // Round-trip check
    let reparsed = parser.parse(&output).expect("reparse");
    let entry = reparsed.entries.get("byte_count").expect("should exist");
    // Note: single-var pattern "%#@bytes@" with one variable gets detected as single-var
    // and flattened to Plural. But the format specifier would be in the text.
    // With the multi-variable pattern, it stays as MultiVariablePlural since the
    // pattern is exactly "%#@bytes@" with 1 var.
    // Actually let's check: is_single_var check is variables.len() == 1 && var_names.len() == 1 && format_key matches
    // The format_key is "%#@bytes@" which equals "%#@bytes@", so it IS single var.
    match &entry.value {
        EntryValue::Plural(ps) => {
            assert_eq!(ps.one, Some("%lld byte".to_string()));
            assert_eq!(ps.other, "%lld bytes");
        }
        other => panic!("Expected Plural, got {:?}", other),
    }
}

#[test]
fn parse_empty_dict_fails() {
    let parser = Parser;
    let content = br#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>bad_entry</key>
    <dict>
    </dict>
</dict>
</plist>"#;

    let result = parser.parse(content);
    assert!(result.is_err(), "Should fail on entry missing NSStringLocalizedFormatKey");
}

#[test]
fn detection_confidence() {
    let parser = Parser;

    // .stringsdict extension is always Definite
    assert_eq!(parser.detect(".stringsdict", b"anything"), Confidence::Definite);

    // .plist with NSStringLocalizedFormatKey content
    assert_eq!(
        parser.detect(".plist", b"NSStringLocalizedFormatKey"),
        Confidence::Definite
    );

    // .plist without the key
    assert_eq!(parser.detect(".plist", b"some other content"), Confidence::None);

    // Unrelated extension
    assert_eq!(parser.detect(".json", b"NSStringLocalizedFormatKey"), Confidence::None);
    assert_eq!(parser.detect(".xml", b"NSStringLocalizedFormatKey"), Confidence::Definite);
}

#[test]
fn write_simple_string_entry() {
    let writer = Writer;

    let mut entries = IndexMap::new();
    entries.insert(
        "hello".to_string(),
        I18nEntry {
            key: "hello".to_string(),
            value: EntryValue::Simple("Hello, World!".to_string()),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Stringsdict,
            ..Default::default()
        },
        entries,
    };

    let output = writer.write(&resource).expect("write simple string");
    let output_str = String::from_utf8(output).expect("valid UTF-8");
    assert!(output_str.contains("Hello, World!"));
    assert!(output_str.contains("NSStringLocalizedFormatKey"));
}
