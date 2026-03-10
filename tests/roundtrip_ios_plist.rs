use i18n_convert::formats::ios_plist::{Parser, Writer};
use i18n_convert::formats::{Confidence, FormatParser, FormatWriter};
use i18n_convert::ir::*;
use indexmap::IndexMap;

// ---------------------------------------------------------------------------
// Detection tests
// ---------------------------------------------------------------------------

#[test]
fn detect_by_extension() {
    let parser = Parser;
    assert_eq!(
        parser.detect(".plist", b"<plist><dict></dict></plist>"),
        Confidence::Definite
    );
}

#[test]
fn detect_by_content_plist_tag() {
    let parser = Parser;
    let content = b"<?xml version=\"1.0\"?><plist version=\"1.0\"><dict></dict></plist>";
    assert_eq!(parser.detect(".xml", content), Confidence::High);
}

#[test]
fn detect_by_content_doctype() {
    let parser = Parser;
    let content = b"<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\">";
    assert_eq!(parser.detect(".xml", content), Confidence::High);
}

#[test]
fn detect_no_match() {
    let parser = Parser;
    assert_eq!(parser.detect(".json", b"{}"), Confidence::None);
}

#[test]
fn detect_excludes_stringsdict() {
    let parser = Parser;
    let content = b"<plist><dict><key>NSStringLocalizedFormatKey</key></dict></plist>";
    assert_eq!(parser.detect(".plist", content), Confidence::None);
}

// ---------------------------------------------------------------------------
// Parse fixture: simple.plist
// ---------------------------------------------------------------------------

#[test]
fn parse_simple_fixture() {
    let content = include_bytes!("fixtures/ios_plist/simple.plist");
    let parser = Parser;
    let resource = parser.parse(content).expect("parse should succeed");

    assert_eq!(resource.metadata.source_format, FormatId::IosPlist);
    assert_eq!(resource.entries.len(), 4);

    let greeting = resource
        .entries
        .get("greeting")
        .expect("greeting should exist");
    assert_eq!(greeting.value, EntryValue::Simple("Hello".to_string()));

    let farewell = resource
        .entries
        .get("farewell")
        .expect("farewell should exist");
    assert_eq!(farewell.value, EntryValue::Simple("Goodbye".to_string()));

    let app = resource
        .entries
        .get("app_name")
        .expect("app_name should exist");
    assert_eq!(app.value, EntryValue::Simple("My App".to_string()));

    let welcome = resource
        .entries
        .get("welcome_message")
        .expect("welcome_message should exist");
    assert_eq!(
        welcome.value,
        EntryValue::Simple("Welcome to our application!".to_string())
    );
}

// ---------------------------------------------------------------------------
// Parse fixture: nested.plist
// ---------------------------------------------------------------------------

#[test]
fn parse_nested_fixture() {
    let content = include_bytes!("fixtures/ios_plist/nested.plist");
    let parser = Parser;
    let resource = parser.parse(content).expect("parse should succeed");

    assert_eq!(resource.entries.len(), 6);

    let welcome = resource
        .entries
        .get("messages.welcome")
        .expect("messages.welcome should exist");
    assert_eq!(welcome.value, EntryValue::Simple("Welcome".to_string()));

    let not_found = resource
        .entries
        .get("messages.error.not_found")
        .expect("messages.error.not_found should exist");
    assert_eq!(
        not_found.value,
        EntryValue::Simple("Page not found".to_string())
    );

    let server = resource
        .entries
        .get("messages.error.server")
        .expect("messages.error.server should exist");
    assert_eq!(
        server.value,
        EntryValue::Simple("Internal server error".to_string())
    );

    let language = resource
        .entries
        .get("settings.language")
        .expect("settings.language should exist");
    assert_eq!(language.value, EntryValue::Simple("Language".to_string()));

    let theme = resource
        .entries
        .get("settings.theme")
        .expect("settings.theme should exist");
    assert_eq!(theme.value, EntryValue::Simple("Theme".to_string()));

    let title = resource
        .entries
        .get("app_title")
        .expect("app_title should exist");
    assert_eq!(
        title.value,
        EntryValue::Simple("My Application".to_string())
    );
}

// ---------------------------------------------------------------------------
// Parse fixture: arrays.plist
// ---------------------------------------------------------------------------

#[test]
fn parse_arrays_fixture() {
    let content = include_bytes!("fixtures/ios_plist/arrays.plist");
    let parser = Parser;
    let resource = parser.parse(content).expect("parse should succeed");

    let colors = resource.entries.get("colors").expect("colors should exist");
    match &colors.value {
        EntryValue::Array(arr) => {
            assert_eq!(
                arr,
                &vec!["Red".to_string(), "Blue".to_string(), "Green".to_string()]
            );
        }
        other => panic!("Expected array, got {other:?}"),
    }

    let greeting = resource
        .entries
        .get("greeting")
        .expect("greeting should exist");
    assert_eq!(greeting.value, EntryValue::Simple("Hello".to_string()));

    let sizes = resource.entries.get("sizes").expect("sizes should exist");
    match &sizes.value {
        EntryValue::Array(arr) => {
            assert_eq!(arr.len(), 4);
            assert_eq!(arr[0], "Small");
            assert_eq!(arr[3], "Extra Large");
        }
        other => panic!("Expected array, got {other:?}"),
    }

    // Nested array
    let nav_items = resource
        .entries
        .get("nav.items")
        .expect("nav.items should exist");
    match &nav_items.value {
        EntryValue::Array(arr) => {
            assert_eq!(
                arr,
                &vec![
                    "Home".to_string(),
                    "About".to_string(),
                    "Contact".to_string()
                ]
            );
        }
        other => panic!("Expected array, got {other:?}"),
    }

    let nav_title = resource
        .entries
        .get("nav.title")
        .expect("nav.title should exist");
    assert_eq!(
        nav_title.value,
        EntryValue::Simple("Navigation".to_string())
    );
}

// ---------------------------------------------------------------------------
// Format extension tests
// ---------------------------------------------------------------------------

#[test]
fn parse_sets_format_extension() {
    let content = include_bytes!("fixtures/ios_plist/simple.plist");
    let parser = Parser;
    let resource = parser.parse(content).expect("parse should succeed");

    match &resource.metadata.format_ext {
        Some(FormatExtension::IosPlist(ext)) => {
            assert_eq!(ext.plist_format, Some("xml1".to_string()));
        }
        other => panic!("Expected IosPlist extension, got {other:?}"),
    }
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
            source_format: FormatId::IosPlist,
            ..Default::default()
        },
        entries,
    };

    let output = writer.write(&resource).expect("write should succeed");
    let text = String::from_utf8(output).expect("should be valid UTF-8");

    assert!(text.contains("<plist"));
    assert!(text.contains("<key>greeting</key>"));
    assert!(text.contains("<string>Hello</string>"));
    assert!(text.contains("<key>farewell</key>"));
    assert!(text.contains("<string>Goodbye</string>"));
}

#[test]
fn write_nested_entries() {
    let writer = Writer;

    let mut entries = IndexMap::new();
    entries.insert(
        "a.b".to_string(),
        I18nEntry {
            key: "a.b".to_string(),
            value: EntryValue::Simple("nested".to_string()),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::IosPlist,
            ..Default::default()
        },
        entries,
    };

    let output = writer.write(&resource).expect("write should succeed");
    let text = String::from_utf8(output).expect("should be valid UTF-8");

    // Should have nested dict structure
    assert!(text.contains("<key>a</key>"));
    assert!(text.contains("<key>b</key>"));
    assert!(text.contains("<string>nested</string>"));
}

#[test]
fn write_array_entries() {
    let writer = Writer;

    let mut entries = IndexMap::new();
    entries.insert(
        "colors".to_string(),
        I18nEntry {
            key: "colors".to_string(),
            value: EntryValue::Array(vec!["Red".to_string(), "Blue".to_string()]),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::IosPlist,
            ..Default::default()
        },
        entries,
    };

    let output = writer.write(&resource).expect("write should succeed");
    let text = String::from_utf8(output).expect("should be valid UTF-8");

    assert!(text.contains("<array>"));
    assert!(text.contains("<string>Red</string>"));
    assert!(text.contains("<string>Blue</string>"));
}

// ---------------------------------------------------------------------------
// Round-trip tests
// ---------------------------------------------------------------------------

#[test]
fn roundtrip_simple() {
    let content = include_bytes!("fixtures/ios_plist/simple.plist");
    let parser = Parser;
    let writer = Writer;

    let resource = parser.parse(content).expect("parse should succeed");
    let output = writer.write(&resource).expect("write should succeed");
    let reparsed = parser.parse(&output).expect("reparse should succeed");

    assert_eq!(resource.entries.len(), reparsed.entries.len());

    for (key, original) in &resource.entries {
        let reparsed_entry = reparsed.entries.get(key).unwrap_or_else(|| {
            panic!("Key '{key}' missing after round-trip");
        });
        assert_eq!(
            original.value, reparsed_entry.value,
            "Value mismatch for key '{key}'"
        );
    }
}

#[test]
fn roundtrip_nested() {
    let content = include_bytes!("fixtures/ios_plist/nested.plist");
    let parser = Parser;
    let writer = Writer;

    let resource = parser.parse(content).expect("parse should succeed");
    let output = writer.write(&resource).expect("write should succeed");
    let reparsed = parser.parse(&output).expect("reparse should succeed");

    assert_eq!(resource.entries.len(), reparsed.entries.len());

    for (key, original) in &resource.entries {
        let reparsed_entry = reparsed.entries.get(key).unwrap_or_else(|| {
            panic!("Key '{key}' missing after round-trip");
        });
        assert_eq!(
            original.value, reparsed_entry.value,
            "Value mismatch for key '{key}'"
        );
    }
}

#[test]
fn roundtrip_arrays() {
    let content = include_bytes!("fixtures/ios_plist/arrays.plist");
    let parser = Parser;
    let writer = Writer;

    let resource = parser.parse(content).expect("parse should succeed");
    let output = writer.write(&resource).expect("write should succeed");
    let reparsed = parser.parse(&output).expect("reparse should succeed");

    assert_eq!(resource.entries.len(), reparsed.entries.len());

    for (key, original) in &resource.entries {
        let reparsed_entry = reparsed.entries.get(key).unwrap_or_else(|| {
            panic!("Key '{key}' missing after round-trip");
        });
        assert_eq!(
            original.value, reparsed_entry.value,
            "Value mismatch for key '{key}'"
        );
    }
}

// ---------------------------------------------------------------------------
// Edge case tests
// ---------------------------------------------------------------------------

#[test]
fn parse_empty_dict() {
    let content = br#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
</dict>
</plist>"#;

    let parser = Parser;
    let resource = parser.parse(content).expect("empty dict should parse");
    assert!(resource.entries.is_empty());
}

#[test]
fn parse_special_characters() {
    let content = br#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>ampersand</key>
    <string>Tom &amp; Jerry</string>
    <key>quotes</key>
    <string>She said &quot;hello&quot;</string>
    <key>angle</key>
    <string>a &lt; b</string>
</dict>
</plist>"#;

    let parser = Parser;
    let resource = parser.parse(content).expect("parse should succeed");

    let amp = resource.entries.get("ampersand").expect("should exist");
    assert_eq!(amp.value, EntryValue::Simple("Tom & Jerry".to_string()));

    let quotes = resource.entries.get("quotes").expect("should exist");
    assert_eq!(
        quotes.value,
        EntryValue::Simple("She said \"hello\"".to_string())
    );

    let angle = resource.entries.get("angle").expect("should exist");
    assert_eq!(angle.value, EntryValue::Simple("a < b".to_string()));
}

#[test]
fn roundtrip_special_characters() {
    let parser = Parser;
    let writer = Writer;

    let mut entries = IndexMap::new();
    entries.insert(
        "special".to_string(),
        I18nEntry {
            key: "special".to_string(),
            value: EntryValue::Simple("Tom & Jerry <3 \"friends\"".to_string()),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::IosPlist,
            ..Default::default()
        },
        entries,
    };

    let output = writer.write(&resource).expect("write should succeed");
    let reparsed = parser.parse(&output).expect("reparse should succeed");

    let entry = reparsed.entries.get("special").expect("should exist");
    assert_eq!(
        entry.value,
        EntryValue::Simple("Tom & Jerry <3 \"friends\"".to_string())
    );
}

#[test]
fn capabilities_are_correct() {
    let parser = Parser;
    let caps = parser.capabilities();
    assert!(!caps.comments);
    assert!(!caps.plurals);
    assert!(caps.nested_keys);
    assert!(caps.arrays);
    assert!(!caps.context);
}

#[test]
fn write_produces_valid_xml_plist() {
    let writer = Writer;

    let mut entries = IndexMap::new();
    entries.insert(
        "key".to_string(),
        I18nEntry {
            key: "key".to_string(),
            value: EntryValue::Simple("value".to_string()),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::IosPlist,
            ..Default::default()
        },
        entries,
    };

    let output = writer.write(&resource).expect("write should succeed");
    let text = String::from_utf8(output).expect("should be valid UTF-8");

    assert!(text.contains("<?xml version=\"1.0\""));
    assert!(text.contains("<!DOCTYPE plist"));
    assert!(text.contains("<plist version=\"1.0\">"));
}

#[test]
fn parse_skips_non_string_values() {
    let content = br#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>title</key>
    <string>Hello</string>
    <key>version</key>
    <integer>42</integer>
    <key>enabled</key>
    <true/>
</dict>
</plist>"#;

    let parser = Parser;
    let resource = parser.parse(content).expect("parse should succeed");

    // Only the string value should be parsed
    assert_eq!(resource.entries.len(), 1);
    assert!(resource.entries.contains_key("title"));
}

#[test]
fn roundtrip_deeply_nested() {
    let parser = Parser;
    let writer = Writer;

    let mut entries = IndexMap::new();
    entries.insert(
        "a.b.c.d".to_string(),
        I18nEntry {
            key: "a.b.c.d".to_string(),
            value: EntryValue::Simple("deep".to_string()),
            ..Default::default()
        },
    );
    entries.insert(
        "a.b.e".to_string(),
        I18nEntry {
            key: "a.b.e".to_string(),
            value: EntryValue::Simple("sibling".to_string()),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::IosPlist,
            ..Default::default()
        },
        entries,
    };

    let output = writer.write(&resource).expect("write should succeed");
    let reparsed = parser.parse(&output).expect("reparse should succeed");

    assert_eq!(reparsed.entries.len(), 2);

    let deep = reparsed.entries.get("a.b.c.d").expect("should exist");
    assert_eq!(deep.value, EntryValue::Simple("deep".to_string()));

    let sibling = reparsed.entries.get("a.b.e").expect("should exist");
    assert_eq!(sibling.value, EntryValue::Simple("sibling".to_string()));
}
