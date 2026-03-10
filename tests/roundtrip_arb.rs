use i18n_convert::formats::arb;
use i18n_convert::formats::{Confidence, FormatParser, FormatWriter};
use i18n_convert::ir::*;
use indexmap::IndexMap;

fn load_fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!("tests/fixtures/arb/{name}")).unwrap()
}

// ── Detection tests ──────────────────────────────────────────────────

#[test]
fn detect_arb_extension() {
    assert_eq!(arb::Parser.detect(".arb", b"{}"), Confidence::Definite);
}

#[test]
fn detect_json_with_locale() {
    let content = br#"{"@@locale": "en", "hello": "Hello"}"#;
    assert_eq!(arb::Parser.detect(".json", content), Confidence::Definite);
}

#[test]
fn detect_json_without_locale() {
    let content = br#"{"hello": "Hello"}"#;
    assert_eq!(arb::Parser.detect(".json", content), Confidence::None);
}

#[test]
fn detect_unrelated_extension() {
    assert_eq!(
        arb::Parser.detect(".xml", b"<resources/>"),
        Confidence::None
    );
}

// ── Simple parse tests ───────────────────────────────────────────────

#[test]
fn parse_simple() {
    let content = load_fixture("simple.arb");
    let resource = arb::Parser.parse(&content).unwrap();

    assert_eq!(resource.metadata.source_format, FormatId::Arb);
    assert_eq!(resource.metadata.locale, Some("en".to_string()));
    assert_eq!(resource.entries.len(), 3);
    assert_eq!(
        resource.entries["appTitle"].value,
        EntryValue::Simple("My App".to_string())
    );
    assert_eq!(
        resource.entries["welcomeMessage"].value,
        EntryValue::Simple("Welcome to our app!".to_string())
    );
    assert_eq!(
        resource.entries["goodbye"].value,
        EntryValue::Simple("Goodbye".to_string())
    );
}

// ── Metadata parse tests ────────────────────────────────────────────

#[test]
fn parse_metadata_file_level() {
    let content = load_fixture("metadata.arb");
    let resource = arb::Parser.parse(&content).unwrap();

    assert_eq!(resource.metadata.locale, Some("en".to_string()));
    assert_eq!(
        resource.metadata.modified_at,
        Some("2024-01-15T10:30:00Z".to_string())
    );
    assert_eq!(
        resource.metadata.created_by,
        Some("Flutter Dev".to_string())
    );
    assert_eq!(
        resource.metadata.headers.get("@@context"),
        Some(&"Main application strings".to_string())
    );
}

#[test]
fn parse_metadata_custom_fields() {
    let content = load_fixture("metadata.arb");
    let resource = arb::Parser.parse(&content).unwrap();

    match &resource.metadata.format_ext {
        Some(FormatExtension::Arb(arb_ext)) => {
            assert_eq!(
                arb_ext.custom_fields.get("@@x-generator"),
                Some(&serde_json::Value::String("i18n-agent".to_string()))
            );
            assert_eq!(
                arb_ext.custom_fields.get("@@x-project-id"),
                Some(&serde_json::Value::String("flutter-demo".to_string()))
            );
        }
        _ => panic!("Expected ArbExt on metadata"),
    }
}

#[test]
fn parse_metadata_entry_description() {
    let content = load_fixture("metadata.arb");
    let resource = arb::Parser.parse(&content).unwrap();

    let entry = &resource.entries["appTitle"];
    assert!(!entry.comments.is_empty());
    assert_eq!(entry.comments[0].text, "The title of the application");
    assert_eq!(entry.comments[0].role, CommentRole::Extracted);
}

#[test]
fn parse_metadata_entry_context() {
    let content = load_fixture("metadata.arb");
    let resource = arb::Parser.parse(&content).unwrap();

    let entry = &resource.entries["appTitle"];
    assert!(!entry.contexts.is_empty());
    assert_eq!(entry.contexts[0].value, "App bar title");
    assert_eq!(entry.contexts[0].context_type, ContextType::Description);
}

#[test]
fn parse_metadata_entry_type() {
    let content = load_fixture("metadata.arb");
    let resource = arb::Parser.parse(&content).unwrap();

    let entry = &resource.entries["appTitle"];
    match &entry.format_ext {
        Some(FormatExtension::Arb(arb_ext)) => {
            assert_eq!(arb_ext.message_type, Some("text".to_string()));
        }
        _ => panic!("Expected ArbExt on entry"),
    }
}

#[test]
fn parse_metadata_entry_placeholder() {
    let content = load_fixture("metadata.arb");
    let resource = arb::Parser.parse(&content).unwrap();

    let entry = &resource.entries["welcomeMessage"];
    assert_eq!(entry.placeholders.len(), 1);
    assert_eq!(entry.placeholders[0].name, "userName");
    assert_eq!(
        entry.placeholders[0].placeholder_type,
        Some(PlaceholderType::String)
    );
    assert_eq!(entry.placeholders[0].example, Some("John".to_string()));
}

// ── Plurals / ICU parse tests ────────────────────────────────────────

#[test]
fn parse_plurals_icu_preserved_as_simple() {
    let content = load_fixture("plurals.arb");
    let resource = arb::Parser.parse(&content).unwrap();

    // ICU plural syntax is preserved as Simple value (per plan: "preserve ICU syntax as-is")
    match &resource.entries["itemCount"].value {
        EntryValue::Simple(s) => {
            assert!(s.contains("{count, plural,"));
            assert!(s.contains("one{1 item}"));
            assert!(s.contains("other{{count} items}"));
        }
        _ => panic!("Expected Simple value for ICU plural string"),
    }
}

#[test]
fn parse_plurals_has_placeholders() {
    let content = load_fixture("plurals.arb");
    let resource = arb::Parser.parse(&content).unwrap();

    let entry = &resource.entries["itemCount"];
    assert_eq!(entry.placeholders.len(), 1);
    assert_eq!(entry.placeholders[0].name, "count");
    assert_eq!(
        entry.placeholders[0].placeholder_type,
        Some(PlaceholderType::Integer)
    );
}

#[test]
fn parse_select_icu_preserved_as_simple() {
    let content = load_fixture("plurals.arb");
    let resource = arb::Parser.parse(&content).unwrap();

    match &resource.entries["genderGreeting"].value {
        EntryValue::Simple(s) => {
            assert!(s.contains("{gender, select,"));
            assert!(s.contains("male{Mr. {name}}"));
            assert!(s.contains("other{Dear {name}}"));
        }
        _ => panic!("Expected Simple value for ICU select string"),
    }
}

// ── Placeholder parse tests ─────────────────────────────────────────

#[test]
fn parse_placeholders_basic() {
    let content = load_fixture("placeholders.arb");
    let resource = arb::Parser.parse(&content).unwrap();

    let entry = &resource.entries["greeting"];
    assert_eq!(entry.placeholders.len(), 2);
    assert_eq!(entry.placeholders[0].name, "firstName");
    assert_eq!(entry.placeholders[0].example, Some("Jane".to_string()));
    assert_eq!(entry.placeholders[1].name, "lastName");
    assert_eq!(entry.placeholders[1].example, Some("Doe".to_string()));
}

#[test]
fn parse_placeholders_with_format() {
    let content = load_fixture("placeholders.arb");
    let resource = arb::Parser.parse(&content).unwrap();

    let entry = &resource.entries["totalPrice"];
    assert_eq!(entry.placeholders.len(), 1);
    let ph = &entry.placeholders[0];
    assert_eq!(ph.name, "price");
    assert_eq!(ph.placeholder_type, Some(PlaceholderType::Double));
    assert_eq!(ph.format, Some("currency".to_string()));
    assert_eq!(ph.example, Some("$12.99".to_string()));
}

#[test]
fn parse_placeholders_optional_parameters() {
    let content = load_fixture("placeholders.arb");
    let resource = arb::Parser.parse(&content).unwrap();

    let entry = &resource.entries["totalPrice"];
    let ph = &entry.placeholders[0];
    let opt = ph
        .optional_parameters
        .as_ref()
        .expect("Expected optional parameters");
    assert_eq!(opt.get("decimalDigits"), Some(&"2".to_string()));
    assert_eq!(opt.get("name"), Some(&"USD".to_string()));
    assert_eq!(opt.get("symbol"), Some(&"$".to_string()));
    assert_eq!(
        opt.get("customPattern"),
        Some(&"\u{a4}#,##0.00".to_string())
    );
}

#[test]
fn parse_placeholders_datetime() {
    let content = load_fixture("placeholders.arb");
    let resource = arb::Parser.parse(&content).unwrap();

    let entry = &resource.entries["eventDate"];
    assert_eq!(entry.placeholders.len(), 1);
    let ph = &entry.placeholders[0];
    assert_eq!(ph.name, "date");
    assert_eq!(ph.placeholder_type, Some(PlaceholderType::DateTime));
    assert_eq!(ph.format, Some("yMMMd".to_string()));
}

// ── Writer tests ─────────────────────────────────────────────────────

#[test]
fn write_simple() {
    let content = load_fixture("simple.arb");
    let resource = arb::Parser.parse(&content).unwrap();
    let output = arb::Writer.write(&resource).unwrap();
    let output_str = std::str::from_utf8(&output).unwrap();

    // Verify it's valid JSON
    let parsed: serde_json::Value = serde_json::from_str(output_str).unwrap();
    assert_eq!(parsed["@@locale"], "en");
    assert_eq!(parsed["appTitle"], "My App");
    assert_eq!(parsed["welcomeMessage"], "Welcome to our app!");
    assert_eq!(parsed["goodbye"], "Goodbye");
}

#[test]
fn write_metadata() {
    let content = load_fixture("metadata.arb");
    let resource = arb::Parser.parse(&content).unwrap();
    let output = arb::Writer.write(&resource).unwrap();
    let output_str = std::str::from_utf8(&output).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(output_str).unwrap();
    assert_eq!(parsed["@@locale"], "en");
    assert_eq!(parsed["@@last_modified"], "2024-01-15T10:30:00Z");
    assert_eq!(parsed["@@author"], "Flutter Dev");
    assert_eq!(parsed["@@context"], "Main application strings");
    assert_eq!(parsed["@@x-generator"], "i18n-agent");
    assert_eq!(parsed["@@x-project-id"], "flutter-demo");

    // Check @key metadata
    assert_eq!(
        parsed["@appTitle"]["description"],
        "The title of the application"
    );
    assert_eq!(parsed["@appTitle"]["type"], "text");
    assert_eq!(parsed["@appTitle"]["context"], "App bar title");

    // Check placeholder in @welcomeMessage
    assert_eq!(
        parsed["@welcomeMessage"]["placeholders"]["userName"]["type"],
        "String"
    );
    assert_eq!(
        parsed["@welcomeMessage"]["placeholders"]["userName"]["example"],
        "John"
    );
}

#[test]
fn write_placeholders_with_optional_params() {
    let content = load_fixture("placeholders.arb");
    let resource = arb::Parser.parse(&content).unwrap();
    let output = arb::Writer.write(&resource).unwrap();
    let output_str = std::str::from_utf8(&output).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(output_str).unwrap();

    let ph = &parsed["@totalPrice"]["placeholders"]["price"];
    assert_eq!(ph["type"], "double");
    assert_eq!(ph["format"], "currency");
    assert_eq!(ph["example"], "$12.99");
    assert_eq!(ph["optionalParameters"]["decimalDigits"], "2");
    assert_eq!(ph["optionalParameters"]["name"], "USD");
    assert_eq!(ph["optionalParameters"]["symbol"], "$");
}

// ── Round-trip tests ─────────────────────────────────────────────────

#[test]
fn roundtrip_simple() {
    let content = load_fixture("simple.arb");
    let resource = arb::Parser.parse(&content).unwrap();
    let written = arb::Writer.write(&resource).unwrap();
    let reparsed = arb::Parser.parse(&written).unwrap();

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
fn roundtrip_metadata() {
    let content = load_fixture("metadata.arb");
    let resource = arb::Parser.parse(&content).unwrap();
    let written = arb::Writer.write(&resource).unwrap();
    let reparsed = arb::Parser.parse(&written).unwrap();

    // File-level metadata
    assert_eq!(resource.metadata.locale, reparsed.metadata.locale);
    assert_eq!(resource.metadata.modified_at, reparsed.metadata.modified_at);
    assert_eq!(resource.metadata.created_by, reparsed.metadata.created_by);
    assert_eq!(
        resource.metadata.headers.get("@@context"),
        reparsed.metadata.headers.get("@@context")
    );

    // Custom fields
    assert_eq!(resource.metadata.format_ext, reparsed.metadata.format_ext);

    // Entries
    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, entry) in &resource.entries {
        let reparsed_entry = &reparsed.entries[key];
        assert_eq!(
            entry.value, reparsed_entry.value,
            "Value mismatch for key: {key}"
        );
        assert_eq!(
            entry.comments, reparsed_entry.comments,
            "Comments mismatch for key: {key}"
        );
        assert_eq!(
            entry.contexts, reparsed_entry.contexts,
            "Contexts mismatch for key: {key}"
        );
        assert_eq!(
            entry.placeholders.len(),
            reparsed_entry.placeholders.len(),
            "Placeholder count mismatch for key: {key}"
        );
        for (i, ph) in entry.placeholders.iter().enumerate() {
            let rph = &reparsed_entry.placeholders[i];
            assert_eq!(
                ph.name, rph.name,
                "Placeholder name mismatch for key: {key}[{i}]"
            );
            assert_eq!(
                ph.placeholder_type, rph.placeholder_type,
                "Placeholder type mismatch for key: {key}[{i}]"
            );
            assert_eq!(
                ph.example, rph.example,
                "Placeholder example mismatch for key: {key}[{i}]"
            );
            assert_eq!(
                ph.format, rph.format,
                "Placeholder format mismatch for key: {key}[{i}]"
            );
            assert_eq!(
                ph.optional_parameters, rph.optional_parameters,
                "Placeholder optional_parameters mismatch for key: {key}[{i}]"
            );
        }
        assert_eq!(
            entry.format_ext, reparsed_entry.format_ext,
            "Format ext mismatch for key: {key}"
        );
    }
}

#[test]
fn roundtrip_plurals() {
    let content = load_fixture("plurals.arb");
    let resource = arb::Parser.parse(&content).unwrap();
    let written = arb::Writer.write(&resource).unwrap();
    let reparsed = arb::Parser.parse(&written).unwrap();

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, entry) in &resource.entries {
        let reparsed_entry = &reparsed.entries[key];
        assert_eq!(
            entry.value, reparsed_entry.value,
            "Value mismatch for key: {key}"
        );
        assert_eq!(
            entry.placeholders.len(),
            reparsed_entry.placeholders.len(),
            "Placeholder count mismatch for key: {key}"
        );
    }
}

#[test]
fn roundtrip_placeholders() {
    let content = load_fixture("placeholders.arb");
    let resource = arb::Parser.parse(&content).unwrap();
    let written = arb::Writer.write(&resource).unwrap();
    let reparsed = arb::Parser.parse(&written).unwrap();

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, entry) in &resource.entries {
        let reparsed_entry = &reparsed.entries[key];
        assert_eq!(
            entry.value, reparsed_entry.value,
            "Value mismatch for key: {key}"
        );
        assert_eq!(
            entry.placeholders.len(),
            reparsed_entry.placeholders.len(),
            "Placeholder count mismatch for key: {key}"
        );
        for (i, ph) in entry.placeholders.iter().enumerate() {
            let rph = &reparsed_entry.placeholders[i];
            assert_eq!(ph.name, rph.name);
            assert_eq!(ph.placeholder_type, rph.placeholder_type);
            assert_eq!(ph.format, rph.format);
            assert_eq!(ph.example, rph.example);
            assert_eq!(ph.optional_parameters, rph.optional_parameters);
        }
    }
}

// ── Programmatic construction + write test ───────────────────────────

#[test]
fn write_programmatic_resource() {
    let mut entries = IndexMap::new();

    entries.insert(
        "hello".to_string(),
        I18nEntry {
            key: "hello".to_string(),
            value: EntryValue::Simple("Hello, {name}!".to_string()),
            comments: vec![Comment {
                text: "A greeting message".to_string(),
                role: CommentRole::Extracted,
                priority: None,
                annotates: None,
            }],
            placeholders: vec![Placeholder {
                name: "name".to_string(),
                original_syntax: "{name}".to_string(),
                placeholder_type: Some(PlaceholderType::String),
                position: None,
                example: Some("World".to_string()),
                description: None,
                format: None,
                optional_parameters: None,
            }],
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Arb,
            locale: Some("en".to_string()),
            ..Default::default()
        },
        entries,
    };

    let output = arb::Writer.write(&resource).unwrap();
    let output_str = std::str::from_utf8(&output).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(output_str).unwrap();

    assert_eq!(parsed["@@locale"], "en");
    assert_eq!(parsed["hello"], "Hello, {name}!");
    assert_eq!(parsed["@hello"]["description"], "A greeting message");
    assert_eq!(parsed["@hello"]["placeholders"]["name"]["type"], "String");
    assert_eq!(parsed["@hello"]["placeholders"]["name"]["example"], "World");
}

#[test]
fn write_plural_set_to_icu() {
    let mut entries = IndexMap::new();

    entries.insert(
        "items".to_string(),
        I18nEntry {
            key: "items".to_string(),
            value: EntryValue::Plural(PluralSet {
                zero: Some("No items".to_string()),
                one: Some("1 item".to_string()),
                other: "{count} items".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Arb,
            locale: Some("en".to_string()),
            ..Default::default()
        },
        entries,
    };

    let output = arb::Writer.write(&resource).unwrap();
    let output_str = std::str::from_utf8(&output).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(output_str).unwrap();

    let val = parsed["items"].as_str().unwrap();
    assert!(val.contains("{count, plural,"));
    assert!(val.contains("zero{No items}"));
    assert!(val.contains("one{1 item}"));
    assert!(val.contains("other{{count} items}"));
}

#[test]
fn write_select_set_to_icu() {
    let mut cases = IndexMap::new();
    cases.insert("male".to_string(), "He".to_string());
    cases.insert("female".to_string(), "She".to_string());
    cases.insert("other".to_string(), "They".to_string());

    let mut entries = IndexMap::new();
    entries.insert(
        "pronoun".to_string(),
        I18nEntry {
            key: "pronoun".to_string(),
            value: EntryValue::Select(SelectSet {
                variable: "gender".to_string(),
                cases,
            }),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Arb,
            locale: Some("en".to_string()),
            ..Default::default()
        },
        entries,
    };

    let output = arb::Writer.write(&resource).unwrap();
    let output_str = std::str::from_utf8(&output).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(output_str).unwrap();

    let val = parsed["pronoun"].as_str().unwrap();
    assert!(val.contains("{gender, select,"));
    assert!(val.contains("male{He}"));
    assert!(val.contains("female{She}"));
    assert!(val.contains("other{They}"));
}

// ── Edge cases ───────────────────────────────────────────────────────

#[test]
fn parse_empty_arb() {
    let content = b"{}";
    let resource = arb::Parser.parse(content).unwrap();
    assert_eq!(resource.entries.len(), 0);
    assert_eq!(resource.metadata.locale, None);
}

#[test]
fn parse_locale_only() {
    let content = br#"{"@@locale": "ja"}"#;
    let resource = arb::Parser.parse(content).unwrap();
    assert_eq!(resource.entries.len(), 0);
    assert_eq!(resource.metadata.locale, Some("ja".to_string()));
}

#[test]
fn parse_invalid_json() {
    let result = arb::Parser.parse(b"not json at all");
    assert!(result.is_err());
}

#[test]
fn write_empty_resource() {
    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Arb,
            ..Default::default()
        },
        entries: IndexMap::new(),
    };
    let output = arb::Writer.write(&resource).unwrap();
    let output_str = std::str::from_utf8(&output).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(output_str).unwrap();
    assert!(parsed.as_object().unwrap().is_empty());
}

#[test]
fn capabilities_reflect_arb() {
    let caps = arb::Parser.capabilities();
    assert!(caps.plurals);
    assert!(caps.comments);
    assert!(caps.context);
    assert!(caps.select_gender);
    assert!(caps.custom_properties);
    assert!(!caps.arrays);
    assert!(!caps.nested_keys);
    assert!(!caps.translation_state);
}
