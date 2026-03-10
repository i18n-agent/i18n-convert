use i18n_convert::formats::yaml_rails;
use i18n_convert::formats::{FormatParser, FormatWriter};
use i18n_convert::ir::*;

fn load_fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!("tests/fixtures/yaml_rails/{name}")).unwrap()
}

// ─── Parse tests ────────────────────────────────────────────────────────────

#[test]
fn parse_simple() {
    let content = load_fixture("simple.yml");
    let resource = yaml_rails::Parser.parse(&content).unwrap();

    assert_eq!(resource.metadata.locale, Some("en".to_string()));
    assert_eq!(resource.metadata.source_format, FormatId::YamlRails);
    assert_eq!(resource.entries.len(), 4);
    assert_eq!(
        resource.entries["greeting"].value,
        EntryValue::Simple("Hello".to_string())
    );
    assert_eq!(
        resource.entries["farewell"].value,
        EntryValue::Simple("Goodbye".to_string())
    );
    assert_eq!(
        resource.entries["app_name"].value,
        EntryValue::Simple("My Application".to_string())
    );
    assert_eq!(
        resource.entries["welcome_message"].value,
        EntryValue::Simple("Welcome to our app".to_string())
    );
}

#[test]
fn parse_nested() {
    let content = load_fixture("nested.yml");
    let resource = yaml_rails::Parser.parse(&content).unwrap();

    assert_eq!(resource.metadata.locale, Some("en".to_string()));
    assert_eq!(resource.entries.len(), 7);
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
        EntryValue::Simple("Welcome to the home page".to_string())
    );
    assert_eq!(
        resource.entries["pages.about.title"].value,
        EntryValue::Simple("About Us".to_string())
    );
    assert_eq!(
        resource.entries["pages.about.team.lead"].value,
        EntryValue::Simple("Team Lead".to_string())
    );
    assert_eq!(
        resource.entries["pages.about.team.members"].value,
        EntryValue::Simple("Team Members".to_string())
    );
}

#[test]
fn parse_plurals() {
    let content = load_fixture("plurals.yml");
    let resource = yaml_rails::Parser.parse(&content).unwrap();

    // "items" should be a Plural with all 6 forms
    match &resource.entries["items"].value {
        EntryValue::Plural(ps) => {
            assert_eq!(ps.zero, Some("no items".to_string()));
            assert_eq!(ps.one, Some("one item".to_string()));
            assert_eq!(ps.two, Some("two items".to_string()));
            assert_eq!(ps.few, Some("a few items".to_string()));
            assert_eq!(ps.many, Some("many items".to_string()));
            assert_eq!(ps.other, "%{count} items");
        }
        other => panic!("Expected Plural for 'items', got {other:?}"),
    }

    // "messages" should be a Plural with one/other only
    match &resource.entries["messages"].value {
        EntryValue::Plural(ps) => {
            assert_eq!(ps.one, Some("one message".to_string()));
            assert_eq!(ps.other, "%{count} messages");
            assert_eq!(ps.zero, None);
            assert_eq!(ps.two, None);
            assert_eq!(ps.few, None);
            assert_eq!(ps.many, None);
        }
        other => panic!("Expected Plural for 'messages', got {other:?}"),
    }

    // "simple_key" should be a Simple value
    assert_eq!(
        resource.entries["simple_key"].value,
        EntryValue::Simple("not a plural".to_string())
    );
}

#[test]
fn parse_plurals_extract_placeholders() {
    let content = load_fixture("plurals.yml");
    let resource = yaml_rails::Parser.parse(&content).unwrap();

    // Plural entries should have extracted %{count} placeholders
    let items_entry = &resource.entries["items"];
    assert!(
        items_entry.placeholders.iter().any(|p| p.name == "count"),
        "Expected 'count' placeholder in items entry"
    );
}

#[test]
fn parse_interpolation() {
    let content = load_fixture("interpolation.yml");
    let resource = yaml_rails::Parser.parse(&content).unwrap();

    assert_eq!(resource.entries.len(), 4);

    // Check greeting has %{name} placeholder
    let greeting = &resource.entries["greeting"];
    assert_eq!(
        greeting.value,
        EntryValue::Simple("Hello, %{name}!".to_string())
    );
    assert_eq!(greeting.placeholders.len(), 1);
    assert_eq!(greeting.placeholders[0].name, "name");
    assert_eq!(greeting.placeholders[0].original_syntax, "%{name}");

    // Check welcome has two placeholders
    let welcome = &resource.entries["welcome"];
    assert_eq!(welcome.placeholders.len(), 2);
    assert_eq!(welcome.placeholders[0].name, "app_name");
    assert_eq!(welcome.placeholders[1].name, "user");

    // Check items_count has two placeholders
    let items_count = &resource.entries["items_count"];
    assert_eq!(items_count.placeholders.len(), 2);
    assert_eq!(items_count.placeholders[0].name, "count");
    assert_eq!(items_count.placeholders[1].name, "container");

    // No interpolation entry should have no placeholders
    let no_interp = &resource.entries["no_interpolation"];
    assert!(no_interp.placeholders.is_empty());
}

// ─── Roundtrip tests ────────────────────────────────────────────────────────

#[test]
fn roundtrip_simple() {
    let content = load_fixture("simple.yml");
    let resource = yaml_rails::Parser.parse(&content).unwrap();
    let written = yaml_rails::Writer.write(&resource).unwrap();
    let reparsed = yaml_rails::Parser.parse(&written).unwrap();

    assert_eq!(resource.metadata.locale, reparsed.metadata.locale);
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
fn roundtrip_nested() {
    let content = load_fixture("nested.yml");
    let resource = yaml_rails::Parser.parse(&content).unwrap();
    let written = yaml_rails::Writer.write(&resource).unwrap();
    let reparsed = yaml_rails::Parser.parse(&written).unwrap();

    assert_eq!(resource.metadata.locale, reparsed.metadata.locale);
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
fn roundtrip_plurals() {
    let content = load_fixture("plurals.yml");
    let resource = yaml_rails::Parser.parse(&content).unwrap();
    let written = yaml_rails::Writer.write(&resource).unwrap();
    let reparsed = yaml_rails::Parser.parse(&written).unwrap();

    assert_eq!(resource.metadata.locale, reparsed.metadata.locale);
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
fn roundtrip_interpolation() {
    let content = load_fixture("interpolation.yml");
    let resource = yaml_rails::Parser.parse(&content).unwrap();
    let written = yaml_rails::Writer.write(&resource).unwrap();
    let reparsed = yaml_rails::Parser.parse(&written).unwrap();

    assert_eq!(resource.metadata.locale, reparsed.metadata.locale);
    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, entry) in &resource.entries {
        let reparsed_entry = &reparsed.entries[key];
        assert_eq!(
            entry.value, reparsed_entry.value,
            "Value mismatch for key: {key}"
        );
        // Placeholders should also survive round-trip
        assert_eq!(
            entry.placeholders.len(),
            reparsed_entry.placeholders.len(),
            "Placeholder count mismatch for key: {key}"
        );
    }
}

// ─── Writer tests ───────────────────────────────────────────────────────────

#[test]
fn writer_produces_valid_yaml() {
    let content = load_fixture("nested.yml");
    let resource = yaml_rails::Parser.parse(&content).unwrap();
    let written = yaml_rails::Writer.write(&resource).unwrap();
    let text = std::str::from_utf8(&written).unwrap();

    // Must start with locale root key
    assert!(
        text.starts_with("en:") || text.starts_with("en:\n"),
        "Output should start with locale root key, got: {}",
        &text[..text.len().min(50)]
    );
}

#[test]
fn writer_reconstructs_nested_structure() {
    let content = load_fixture("nested.yml");
    let resource = yaml_rails::Parser.parse(&content).unwrap();
    let written = yaml_rails::Writer.write(&resource).unwrap();
    let text = std::str::from_utf8(&written).unwrap();

    // Verify nested structure is present (not flat dot-separated keys)
    assert!(text.contains("common:"), "Expected nested 'common' key");
    assert!(text.contains("pages:"), "Expected nested 'pages' key");
    assert!(text.contains("home:"), "Expected nested 'home' key");
    assert!(text.contains("about:"), "Expected nested 'about' key");
}

#[test]
fn writer_uses_locale_from_metadata() {
    let mut entries = indexmap::IndexMap::new();
    entries.insert(
        "greeting".to_string(),
        I18nEntry {
            key: "greeting".to_string(),
            value: EntryValue::Simple("Hallo".to_string()),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::YamlRails,
            locale: Some("de".to_string()),
            ..Default::default()
        },
        entries,
    };

    let written = yaml_rails::Writer.write(&resource).unwrap();
    let text = std::str::from_utf8(&written).unwrap();
    assert!(
        text.starts_with("de:"),
        "Expected 'de:' locale root, got: {}",
        &text[..text.len().min(50)]
    );
}

#[test]
fn writer_defaults_to_en_locale() {
    let mut entries = indexmap::IndexMap::new();
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
            source_format: FormatId::YamlRails,
            locale: None,
            ..Default::default()
        },
        entries,
    };

    let written = yaml_rails::Writer.write(&resource).unwrap();
    let text = std::str::from_utf8(&written).unwrap();
    assert!(
        text.starts_with("en:"),
        "Expected default 'en:' locale root, got: {}",
        &text[..text.len().min(50)]
    );
}

#[test]
fn writer_outputs_plural_sub_keys() {
    let content = load_fixture("plurals.yml");
    let resource = yaml_rails::Parser.parse(&content).unwrap();
    let written = yaml_rails::Writer.write(&resource).unwrap();
    let text = std::str::from_utf8(&written).unwrap();

    // Verify plural sub-keys are expanded back out
    assert!(text.contains("one:"), "Expected 'one' plural sub-key");
    assert!(text.contains("other:"), "Expected 'other' plural sub-key");
    assert!(text.contains("zero:"), "Expected 'zero' plural sub-key");
}

// ─── Detection tests ────────────────────────────────────────────────────────

#[test]
fn detect_high_confidence_with_locale_key() {
    let parser = yaml_rails::Parser;
    let content = b"en:\n  greeting: Hello\n";
    assert_eq!(
        parser.detect(".yml", content),
        i18n_convert::formats::Confidence::High
    );
    assert_eq!(
        parser.detect(".yaml", content),
        i18n_convert::formats::Confidence::High
    );
}

#[test]
fn detect_low_confidence_yml_without_locale() {
    let parser = yaml_rails::Parser;
    let content = b"config:\n  debug: true\n";
    assert_eq!(
        parser.detect(".yml", content),
        i18n_convert::formats::Confidence::Low
    );
}

#[test]
fn detect_none_for_non_yaml() {
    let parser = yaml_rails::Parser;
    let content = b"en:\n  greeting: Hello\n";
    assert_eq!(
        parser.detect(".json", content),
        i18n_convert::formats::Confidence::None
    );
}
