use i18n_convert::formats::i18next;
use i18n_convert::formats::Confidence;
use i18n_convert::formats::FormatParser;
use i18n_convert::formats::FormatWriter;
use i18n_convert::ir::*;

fn load_fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!("tests/fixtures/i18next/{name}")).unwrap()
}

// ─── Detection tests ──────────────────────────────────────────────

#[test]
fn detect_i18next_with_plural_suffixes() {
    let content = load_fixture("plurals.json");
    let confidence = i18next::Parser.detect(".json", &content);
    assert_eq!(confidence, Confidence::High);
}

#[test]
fn detect_non_json_extension() {
    let content = load_fixture("simple.json");
    let confidence = i18next::Parser.detect(".xml", &content);
    assert_eq!(confidence, Confidence::None);
}

#[test]
fn detect_simple_json_no_plurals() {
    let content = load_fixture("simple.json");
    let confidence = i18next::Parser.detect(".json", &content);
    // Simple JSON without _one/_other suffixes should not be detected
    assert_eq!(confidence, Confidence::None);
}

// ─── Simple flat key-value parsing ────────────────────────────────

#[test]
fn parse_simple_flat_keys() {
    let content = load_fixture("simple.json");
    let resource = i18next::Parser.parse(&content).unwrap();

    assert_eq!(resource.metadata.source_format, FormatId::I18nextJson);
    assert_eq!(resource.entries.len(), 3);
    assert_eq!(
        resource.entries["welcome"].value,
        EntryValue::Simple("Welcome to our app".to_string())
    );
    assert_eq!(
        resource.entries["goodbye"].value,
        EntryValue::Simple("See you later".to_string())
    );
    assert_eq!(
        resource.entries["app_title"].value,
        EntryValue::Simple("My Application".to_string())
    );
}

// ─── Plural parsing ──────────────────────────────────────────────

#[test]
fn parse_cardinal_plurals_all_forms() {
    let content = load_fixture("plurals.json");
    let resource = i18next::Parser.parse(&content).unwrap();

    // "item" should be grouped from item_zero..item_other
    match &resource.entries["item"].value {
        EntryValue::Plural(ps) => {
            assert!(!ps.ordinal);
            assert_eq!(ps.zero, Some("No items".to_string()));
            assert_eq!(ps.one, Some("{{count}} item".to_string()));
            assert_eq!(ps.two, Some("{{count}} items (dual)".to_string()));
            assert_eq!(ps.few, Some("{{count}} items (few)".to_string()));
            assert_eq!(ps.many, Some("{{count}} items (many)".to_string()));
            assert_eq!(ps.other, "{{count}} items");
        }
        other => panic!("Expected Plural for 'item', got: {other:?}"),
    }
}

#[test]
fn parse_cardinal_plurals_two_forms() {
    let content = load_fixture("plurals.json");
    let resource = i18next::Parser.parse(&content).unwrap();

    // "message" should be grouped from message_one/message_other
    match &resource.entries["message"].value {
        EntryValue::Plural(ps) => {
            assert!(!ps.ordinal);
            assert_eq!(ps.one, Some("You have {{count}} message".to_string()));
            assert_eq!(ps.other, "You have {{count}} messages");
            assert!(ps.zero.is_none());
            assert!(ps.two.is_none());
            assert!(ps.few.is_none());
            assert!(ps.many.is_none());
        }
        other => panic!("Expected Plural for 'message', got: {other:?}"),
    }
}

#[test]
fn parse_ordinal_plurals() {
    let content = load_fixture("plurals.json");
    let resource = i18next::Parser.parse(&content).unwrap();

    // "ordinal" should be grouped from ordinal_one..ordinal_other
    // But wait - these are cardinal suffixes (_one, _two, _few, _other), not ordinal.
    // In the fixture, ordinal_one etc. are cardinal plurals for the key "ordinal".
    match &resource.entries["ordinal"].value {
        EntryValue::Plural(ps) => {
            assert_eq!(ps.one, Some("{{count}}st".to_string()));
            assert_eq!(ps.two, Some("{{count}}nd".to_string()));
            assert_eq!(ps.few, Some("{{count}}rd".to_string()));
            assert_eq!(ps.other, "{{count}}th");
        }
        other => panic!("Expected Plural for 'ordinal', got: {other:?}"),
    }
}

// ─── Nested namespace parsing ────────────────────────────────────

#[test]
fn parse_nested_namespaces() {
    let content = load_fixture("nested.json");
    let resource = i18next::Parser.parse(&content).unwrap();

    // Flat keys from nested structure
    assert_eq!(
        resource.entries["common.save"].value,
        EntryValue::Simple("Save".to_string())
    );
    assert_eq!(
        resource.entries["common.cancel"].value,
        EntryValue::Simple("Cancel".to_string())
    );
    assert_eq!(
        resource.entries["common.delete"].value,
        EntryValue::Simple("Delete".to_string())
    );
    assert_eq!(
        resource.entries["dashboard.title"].value,
        EntryValue::Simple("Dashboard".to_string())
    );
    assert_eq!(
        resource.entries["settings.title"].value,
        EntryValue::Simple("Settings".to_string())
    );
    assert_eq!(
        resource.entries["settings.notifications.enabled"].value,
        EntryValue::Simple("Notifications are enabled".to_string())
    );
}

#[test]
fn parse_nested_plurals() {
    let content = load_fixture("nested.json");
    let resource = i18next::Parser.parse(&content).unwrap();

    // Plurals inside nested namespaces
    match &resource.entries["dashboard.stats.user"].value {
        EntryValue::Plural(ps) => {
            assert_eq!(ps.one, Some("{{count}} user".to_string()));
            assert_eq!(ps.other, "{{count}} users");
        }
        other => panic!("Expected Plural for 'dashboard.stats.user', got: {other:?}"),
    }

    match &resource.entries["dashboard.stats.session"].value {
        EntryValue::Plural(ps) => {
            assert_eq!(ps.one, Some("{{count}} active session".to_string()));
            assert_eq!(ps.other, "{{count}} active sessions");
        }
        other => panic!("Expected Plural for 'dashboard.stats.session', got: {other:?}"),
    }
}

// ─── Interpolation parsing ───────────────────────────────────────

#[test]
fn parse_interpolation_placeholders() {
    let content = load_fixture("interpolation.json");
    let resource = i18next::Parser.parse(&content).unwrap();

    assert_eq!(
        resource.entries["greeting"].value,
        EntryValue::Simple("Hello, {{name}}!".to_string())
    );
    // Check placeholder extraction
    assert_eq!(resource.entries["greeting"].placeholders.len(), 1);
    assert_eq!(resource.entries["greeting"].placeholders[0].name, "name");
    assert_eq!(
        resource.entries["greeting"].placeholders[0].original_syntax,
        "{{name}}"
    );
}

#[test]
fn parse_multiple_interpolations() {
    let content = load_fixture("interpolation.json");
    let resource = i18next::Parser.parse(&content).unwrap();

    let entry = &resource.entries["welcome_back"];
    assert_eq!(entry.placeholders.len(), 2);
    let names: Vec<&str> = entry.placeholders.iter().map(|p| p.name.as_str()).collect();
    assert!(names.contains(&"firstName"));
    assert!(names.contains(&"lastName"));
}

#[test]
fn parse_interpolation_with_format_hint() {
    let content = load_fixture("interpolation.json");
    let resource = i18next::Parser.parse(&content).unwrap();

    let entry = &resource.entries["price"];
    assert_eq!(entry.placeholders.len(), 1);
    assert_eq!(entry.placeholders[0].name, "price");
    assert_eq!(entry.placeholders[0].original_syntax, "{{price, currency}}");
}

#[test]
fn parse_nesting_preserved() {
    let content = load_fixture("interpolation.json");
    let resource = i18next::Parser.parse(&content).unwrap();

    // $t(greeting) nesting should be preserved as-is in the value
    assert_eq!(
        resource.entries["nesting_example"].value,
        EntryValue::Simple("This references $t(greeting)".to_string())
    );
}

#[test]
fn parse_plural_with_interpolation() {
    let content = load_fixture("interpolation.json");
    let resource = i18next::Parser.parse(&content).unwrap();

    match &resource.entries["email_count"].value {
        EntryValue::Plural(ps) => {
            assert_eq!(ps.one, Some("{{name}} has {{count}} new email".to_string()));
            assert_eq!(ps.other, "{{name}} has {{count}} new emails");
        }
        other => panic!("Expected Plural for 'email_count', got: {other:?}"),
    }

    // Should extract both placeholders from plural forms
    let entry = &resource.entries["email_count"];
    let names: Vec<&str> = entry.placeholders.iter().map(|p| p.name.as_str()).collect();
    assert!(names.contains(&"name"));
    assert!(names.contains(&"count"));
}

// ─── Writer tests ────────────────────────────────────────────────

#[test]
fn write_simple_entries() {
    let content = load_fixture("simple.json");
    let resource = i18next::Parser.parse(&content).unwrap();
    let written = i18next::Writer.write(&resource).unwrap();
    let written_str = std::str::from_utf8(&written).unwrap();

    let reparsed: serde_json::Value = serde_json::from_str(written_str).unwrap();
    assert_eq!(reparsed["welcome"], "Welcome to our app");
    assert_eq!(reparsed["goodbye"], "See you later");
    assert_eq!(reparsed["app_title"], "My Application");
}

#[test]
fn write_plurals_expand_suffixes() {
    let content = load_fixture("plurals.json");
    let resource = i18next::Parser.parse(&content).unwrap();
    let written = i18next::Writer.write(&resource).unwrap();
    let written_str = std::str::from_utf8(&written).unwrap();

    let reparsed: serde_json::Value = serde_json::from_str(written_str).unwrap();
    assert_eq!(reparsed["item_zero"], "No items");
    assert_eq!(reparsed["item_one"], "{{count}} item");
    assert_eq!(reparsed["item_other"], "{{count}} items");
    assert_eq!(reparsed["message_one"], "You have {{count}} message");
    assert_eq!(reparsed["message_other"], "You have {{count}} messages");
}

#[test]
fn write_nested_structure_reconstructed() {
    let content = load_fixture("nested.json");
    let resource = i18next::Parser.parse(&content).unwrap();
    let written = i18next::Writer.write(&resource).unwrap();
    let written_str = std::str::from_utf8(&written).unwrap();

    let reparsed: serde_json::Value = serde_json::from_str(written_str).unwrap();
    assert_eq!(reparsed["common"]["save"], "Save");
    assert_eq!(reparsed["common"]["cancel"], "Cancel");
    assert_eq!(reparsed["dashboard"]["title"], "Dashboard");
    assert_eq!(reparsed["dashboard"]["stats"]["user_one"], "{{count}} user");
    assert_eq!(
        reparsed["dashboard"]["stats"]["user_other"],
        "{{count}} users"
    );
}

// ─── Full roundtrip test ─────────────────────────────────────────

#[test]
fn roundtrip_full() {
    let content = load_fixture("full.json");
    let resource = i18next::Parser.parse(&content).unwrap();
    let written = i18next::Writer.write(&resource).unwrap();
    let reparsed = i18next::Parser.parse(&written).unwrap();

    assert_eq!(
        resource.entries.len(),
        reparsed.entries.len(),
        "Entry count mismatch: original has {}, reparsed has {}.\nOriginal keys: {:?}\nReparsed keys: {:?}",
        resource.entries.len(),
        reparsed.entries.len(),
        resource.entries.keys().collect::<Vec<_>>(),
        reparsed.entries.keys().collect::<Vec<_>>(),
    );

    for (key, entry) in &resource.entries {
        let reparsed_entry = reparsed.entries.get(key).unwrap_or_else(|| {
            panic!("Key '{key}' missing in reparsed output");
        });
        assert_eq!(
            entry.value, reparsed_entry.value,
            "Value mismatch for key: {key}"
        );
    }
}

#[test]
fn roundtrip_simple() {
    let content = load_fixture("simple.json");
    let resource = i18next::Parser.parse(&content).unwrap();
    let written = i18next::Writer.write(&resource).unwrap();
    let reparsed = i18next::Parser.parse(&written).unwrap();

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, entry) in &resource.entries {
        assert_eq!(
            entry.value, reparsed.entries[key].value,
            "Value mismatch for key: {key}"
        );
    }
}

#[test]
fn roundtrip_nested() {
    let content = load_fixture("nested.json");
    let resource = i18next::Parser.parse(&content).unwrap();
    let written = i18next::Writer.write(&resource).unwrap();
    let reparsed = i18next::Parser.parse(&written).unwrap();

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, entry) in &resource.entries {
        assert_eq!(
            entry.value, reparsed.entries[key].value,
            "Value mismatch for key: {key}"
        );
    }
}

#[test]
fn roundtrip_interpolation() {
    let content = load_fixture("interpolation.json");
    let resource = i18next::Parser.parse(&content).unwrap();
    let written = i18next::Writer.write(&resource).unwrap();
    let reparsed = i18next::Parser.parse(&written).unwrap();

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, entry) in &resource.entries {
        assert_eq!(
            entry.value, reparsed.entries[key].value,
            "Value mismatch for key: {key}"
        );
    }
}
