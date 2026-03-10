use i18n_convert::formats::*;
use i18n_convert::ir::*;
use indexmap::IndexMap;

/// Build an I18nResource with EVERY IR field populated.
/// This exercises the maximum surface area for roundtrip testing.
fn kitchen_sink_ir() -> I18nResource {
    let mut entries = IndexMap::new();

    // 1. Simple string with all metadata fields populated
    let mut simple_props = IndexMap::new();
    simple_props.insert("custom_key".to_string(), "custom_value".to_string());
    entries.insert(
        "simple".to_string(),
        I18nEntry {
            key: "simple".to_string(),
            value: EntryValue::Simple("Hello, World!".to_string()),
            comments: vec![
                Comment {
                    text: "General comment".to_string(),
                    role: CommentRole::General,
                    priority: None,
                    annotates: None,
                },
                Comment {
                    text: "Developer note".to_string(),
                    role: CommentRole::Developer,
                    priority: Some(1),
                    annotates: Some(AnnotationTarget::Source),
                },
                Comment {
                    text: "Translator note".to_string(),
                    role: CommentRole::Translator,
                    priority: None,
                    annotates: Some(AnnotationTarget::Target),
                },
            ],
            contexts: vec![ContextEntry {
                context_type: ContextType::Description,
                value: "Greeting context".to_string(),
                purpose: Some("disambiguation".to_string()),
            }],
            source: Some("Hello, World!".to_string()),
            previous_source: Some("Hi, World!".to_string()),
            previous_comment: Some("Old comment".to_string()),
            placeholders: vec![],
            translatable: Some(true),
            state: Some(TranslationState::Translated),
            state_qualifier: Some("exact-match".to_string()),
            approved: Some(true),
            obsolete: false,
            max_width: Some(100),
            min_width: Some(10),
            max_height: Some(50),
            min_height: Some(5),
            size_unit: Some("pixel".to_string()),
            max_bytes: Some(200),
            min_bytes: Some(1),
            source_references: vec![
                SourceRef {
                    file: "main.c".to_string(),
                    line: Some(42),
                },
                SourceRef {
                    file: "utils.c".to_string(),
                    line: None,
                },
            ],
            flags: vec!["fuzzy".to_string(), "c-format".to_string()],
            device_variants: Some({
                let mut dv = IndexMap::new();
                dv.insert(
                    DeviceType::Phone,
                    EntryValue::Simple("Hello on phone!".to_string()),
                );
                dv.insert(
                    DeviceType::Tablet,
                    EntryValue::Simple("Hello on tablet!".to_string()),
                );
                dv
            }),
            alternatives: vec![AlternativeTranslation {
                value: "Hi, World!".to_string(),
                source: Some("Hello, World!".to_string()),
                match_quality: Some(0.95),
                origin: Some("TM".to_string()),
                alt_type: Some("proposal".to_string()),
            }],
            properties: simple_props,
            resource_type: Some("string".to_string()),
            resource_name: Some("greeting".to_string()),
            format_ext: None,
        },
    );

    // 2. Plural entry
    entries.insert(
        "items".to_string(),
        I18nEntry {
            key: "items".to_string(),
            value: EntryValue::Plural(PluralSet {
                zero: Some("No items".to_string()),
                one: Some("1 item".to_string()),
                two: Some("2 items".to_string()),
                few: Some("A few items".to_string()),
                many: Some("Many items".to_string()),
                other: "%d items".to_string(),
                exact_matches: {
                    let mut em = IndexMap::new();
                    em.insert(0, "Exactly zero items".to_string());
                    em.insert(42, "The answer items".to_string());
                    em
                },
                range_matches: vec![PluralRange {
                    from: Some(1),
                    to: Some(5),
                    inclusive: true,
                    value: "A handful".to_string(),
                }],
                ordinal: false,
            }),
            comments: vec![Comment {
                text: "Item count".to_string(),
                role: CommentRole::Extracted,
                priority: None,
                annotates: None,
            }],
            ..Default::default()
        },
    );

    // 3. Array entry
    entries.insert(
        "colors".to_string(),
        I18nEntry {
            key: "colors".to_string(),
            value: EntryValue::Array(vec![
                "Red".to_string(),
                "Green".to_string(),
                "Blue".to_string(),
            ]),
            ..Default::default()
        },
    );

    // 4. Select/gender entry
    entries.insert(
        "pronoun".to_string(),
        I18nEntry {
            key: "pronoun".to_string(),
            value: EntryValue::Select(SelectSet {
                variable: "gender".to_string(),
                cases: {
                    let mut cases = IndexMap::new();
                    cases.insert("male".to_string(), "He".to_string());
                    cases.insert("female".to_string(), "She".to_string());
                    cases.insert("other".to_string(), "They".to_string());
                    cases
                },
            }),
            ..Default::default()
        },
    );

    // 5. Multi-variable plural
    entries.insert(
        "files_in_folders".to_string(),
        I18nEntry {
            key: "files_in_folders".to_string(),
            value: EntryValue::MultiVariablePlural(MultiVariablePlural {
                pattern: "%#@files@ in %#@folders@".to_string(),
                variables: {
                    let mut vars = IndexMap::new();
                    vars.insert(
                        "files".to_string(),
                        PluralVariable {
                            name: "files".to_string(),
                            format_specifier: Some("d".to_string()),
                            arg_num: Some(1),
                            plural_set: PluralSet {
                                one: Some("%d file".to_string()),
                                other: "%d files".to_string(),
                                ..Default::default()
                            },
                        },
                    );
                    vars.insert(
                        "folders".to_string(),
                        PluralVariable {
                            name: "folders".to_string(),
                            format_specifier: Some("d".to_string()),
                            arg_num: Some(2),
                            plural_set: PluralSet {
                                one: Some("%d folder".to_string()),
                                other: "%d folders".to_string(),
                                ..Default::default()
                            },
                        },
                    );
                    vars
                },
            }),
            ..Default::default()
        },
    );

    // 6. Entry with placeholders
    entries.insert(
        "welcome".to_string(),
        I18nEntry {
            key: "welcome".to_string(),
            value: EntryValue::Simple("Welcome, {name}!".to_string()),
            placeholders: vec![Placeholder {
                name: "name".to_string(),
                original_syntax: "{name}".to_string(),
                placeholder_type: Some(PlaceholderType::String),
                position: Some(0),
                example: Some("John".to_string()),
                description: Some("User's name".to_string()),
                format: None,
                optional_parameters: Some({
                    let mut m = IndexMap::new();
                    m.insert("decimalDigits".to_string(), "0".to_string());
                    m
                }),
            }],
            ..Default::default()
        },
    );

    // 7. Untranslatable entry
    entries.insert(
        "api_key".to_string(),
        I18nEntry {
            key: "api_key".to_string(),
            value: EntryValue::Simple("DEBUG_KEY".to_string()),
            translatable: Some(false),
            ..Default::default()
        },
    );

    I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::default(),
            locale: Some("en".to_string()),
            source_locale: Some("en".to_string()),
            headers: {
                let mut h = IndexMap::new();
                h.insert(
                    "Content-Type".to_string(),
                    "text/plain; charset=UTF-8".to_string(),
                );
                h
            },
            properties: IndexMap::new(),
            encoding: Some("UTF-8".to_string()),
            direction: Some(TextDirection::Ltr),
            created_at: Some("2024-01-01".to_string()),
            modified_at: Some("2024-06-01".to_string()),
            created_by: Some("test".to_string()),
            modified_by: Some("test".to_string()),
            tool_name: Some("i18n-convert".to_string()),
            tool_version: Some("0.1.0".to_string()),
            format_ext: None,
        },
        entries,
    }
}

// ── Android XML roundtrip ──────────────────────────────────────────────────
// Supports: plurals, arrays, comments, translatable_flag, device_variants, inline_markup

#[test]
fn kitchen_sink_roundtrip_android_xml() {
    let ir = kitchen_sink_ir();
    let written = android_xml::Writer.write(&ir).unwrap();
    let parsed = android_xml::Parser.parse(&written).unwrap();

    // Simple string value
    assert_eq!(
        ir.entries["simple"].value, parsed.entries["simple"].value,
        "Simple string value should survive Android XML roundtrip"
    );

    // Translatable flag
    assert_eq!(
        ir.entries["api_key"].translatable, parsed.entries["api_key"].translatable,
        "Translatable flag should survive"
    );

    // Plurals survive (at least one/other, the core categories)
    assert!(
        matches!(parsed.entries["items"].value, EntryValue::Plural(_)),
        "Plural should survive Android XML roundtrip"
    );
    if let (EntryValue::Plural(orig), EntryValue::Plural(parsed_ps)) =
        (&ir.entries["items"].value, &parsed.entries["items"].value)
    {
        assert_eq!(orig.other, parsed_ps.other, "Plural 'other' should match");
        assert_eq!(orig.one, parsed_ps.one, "Plural 'one' should match");
        assert_eq!(orig.zero, parsed_ps.zero, "Plural 'zero' should match");
    }

    // Arrays survive
    assert_eq!(
        ir.entries["colors"].value, parsed.entries["colors"].value,
        "Array should survive Android XML roundtrip"
    );

    // Comments survive -- Android XML writer only writes CommentRole::General comments
    assert!(
        !parsed.entries["simple"].comments.is_empty(),
        "General role comments should survive Android XML roundtrip"
    );
}

// ── Xcstrings roundtrip ────────────────────────────────────────────────────
// Supports: plurals, comments, translatable_flag, translation_state, device_variants

#[test]
fn kitchen_sink_roundtrip_xcstrings() {
    let ir = kitchen_sink_ir();
    let written = xcstrings::Writer.write(&ir).unwrap();
    let parsed = xcstrings::Parser.parse(&written).unwrap();

    // Simple string value
    assert_eq!(
        ir.entries["simple"].value, parsed.entries["simple"].value,
        "Simple string should survive xcstrings roundtrip"
    );

    // Plurals survive
    assert!(
        matches!(parsed.entries["items"].value, EntryValue::Plural(_)),
        "Plural should survive xcstrings roundtrip"
    );

    // Multi-variable plurals survive
    assert!(
        matches!(
            parsed.entries["files_in_folders"].value,
            EntryValue::MultiVariablePlural(_) | EntryValue::Plural(_)
        ),
        "Multi-variable plural should survive xcstrings roundtrip"
    );

    // Translatable flag
    assert_eq!(
        ir.entries["api_key"].translatable, parsed.entries["api_key"].translatable,
        "Translatable flag should survive xcstrings roundtrip"
    );

    // Comments survive (xcstrings stores one comment per key)
    if !ir.entries["simple"].comments.is_empty() {
        assert!(
            !parsed.entries["simple"].comments.is_empty(),
            "At least first comment should survive xcstrings roundtrip"
        );
    }
}

// ── iOS Strings roundtrip ──────────────────────────────────────────────────
// Supports: comments only

#[test]
fn kitchen_sink_roundtrip_ios_strings() {
    let ir = kitchen_sink_ir();
    let written = ios_strings::Writer.write(&ir).unwrap();
    let parsed = ios_strings::Parser.parse(&written).unwrap();

    // Simple string values should survive (the writer falls back to string for non-simple)
    assert_eq!(
        ir.entries["simple"].value, parsed.entries["simple"].value,
        "Simple string should survive iOS strings roundtrip"
    );

    // Comments survive
    if !ir.entries["simple"].comments.is_empty() {
        assert!(
            !parsed.entries["simple"].comments.is_empty(),
            "Comments should survive iOS strings roundtrip"
        );
    }
}

// ── Stringsdict roundtrip ──────────────────────────────────────────────────
// Supports: plurals only

#[test]
fn kitchen_sink_roundtrip_stringsdict() {
    // Stringsdict only handles plurals, so build an IR with just plural entries
    let mut entries = IndexMap::new();
    entries.insert(
        "items".to_string(),
        kitchen_sink_ir().entries["items"].clone(),
    );
    entries.insert(
        "files_in_folders".to_string(),
        kitchen_sink_ir().entries["files_in_folders"].clone(),
    );

    let ir = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Stringsdict,
            ..Default::default()
        },
        entries,
    };

    let written = stringsdict::Writer.write(&ir).unwrap();
    let parsed = stringsdict::Parser.parse(&written).unwrap();

    // Plural should survive
    assert!(
        matches!(parsed.entries["items"].value, EntryValue::Plural(_)),
        "Plural should survive stringsdict roundtrip"
    );

    // Multi-variable plural should survive
    assert!(
        parsed.entries.contains_key("files_in_folders"),
        "Multi-variable plural key should survive stringsdict roundtrip"
    );
}

// ── ARB roundtrip ──────────────────────────────────────────────────────────
// Supports: plurals, comments, context, select_gender, custom_properties

#[test]
fn kitchen_sink_roundtrip_arb() {
    let ir = kitchen_sink_ir();
    let written = arb::Writer.write(&ir).unwrap();
    let parsed = arb::Parser.parse(&written).unwrap();

    // Simple string value
    assert_eq!(
        ir.entries["simple"].value, parsed.entries["simple"].value,
        "Simple string should survive ARB roundtrip"
    );

    // Plural entry survives as ICU string
    assert!(
        parsed.entries.contains_key("items"),
        "Plural key should survive in ARB (as ICU string)"
    );

    // Select/gender survives as ICU string
    assert!(
        parsed.entries.contains_key("pronoun"),
        "Select key should survive in ARB (as ICU string)"
    );

    // Locale metadata survives
    assert_eq!(
        ir.metadata.locale, parsed.metadata.locale,
        "Locale should survive ARB roundtrip"
    );
}

// ── JSON Structured roundtrip ──────────────────────────────────────────────
// Supports: nested_keys only

#[test]
fn kitchen_sink_roundtrip_json_structured() {
    let ir = kitchen_sink_ir();
    let written = json_structured::Writer.write(&ir).unwrap();
    let parsed = json_structured::Parser.parse(&written).unwrap();

    // Simple string value
    assert_eq!(
        ir.entries["simple"].value, parsed.entries["simple"].value,
        "Simple string should survive JSON roundtrip"
    );

    // Untranslatable entry value
    assert_eq!(
        ir.entries["api_key"].value, parsed.entries["api_key"].value,
        "API key value should survive JSON roundtrip"
    );

    // Welcome message with placeholder
    assert_eq!(
        ir.entries["welcome"].value, parsed.entries["welcome"].value,
        "Placeholder string should survive JSON roundtrip"
    );
}

// ── i18next JSON roundtrip ─────────────────────────────────────────────────
// Supports: plurals, context, nested_keys

#[test]
fn kitchen_sink_roundtrip_i18next() {
    let ir = kitchen_sink_ir();
    let written = i18next::Writer.write(&ir).unwrap();
    let parsed = i18next::Parser.parse(&written).unwrap();

    // Simple string value
    assert_eq!(
        ir.entries["simple"].value, parsed.entries["simple"].value,
        "Simple string should survive i18next roundtrip"
    );

    // Plurals survive via suffixed keys
    assert!(
        parsed.entries.contains_key("items"),
        "Plural key should survive i18next roundtrip"
    );
    if let EntryValue::Plural(ps) = &parsed.entries["items"].value {
        // At minimum, other should survive
        assert!(
            !ps.other.is_empty(),
            "Plural 'other' should survive i18next roundtrip"
        );
    }
}

// ── XLIFF 1.2 roundtrip ───────────────────────────────────────────────────
// Supports: comments, context, source_string, translatable_flag, translation_state,
//           max_width, inline_markup, alternatives, source_references, custom_properties

#[test]
fn kitchen_sink_roundtrip_xliff() {
    let ir = kitchen_sink_ir();
    let written = xliff1::Writer.write(&ir).unwrap();
    let parsed = xliff1::Parser.parse(&written).unwrap();

    // Simple string value
    assert_eq!(
        ir.entries["simple"].value, parsed.entries["simple"].value,
        "Simple string should survive XLIFF roundtrip"
    );

    // Source string
    assert_eq!(
        ir.entries["simple"].source, parsed.entries["simple"].source,
        "Source string should survive XLIFF roundtrip"
    );

    // Translation state
    assert!(
        parsed.entries["simple"].state.is_some(),
        "Translation state should survive XLIFF roundtrip"
    );

    // Comments
    assert!(
        !parsed.entries["simple"].comments.is_empty(),
        "Comments should survive XLIFF roundtrip"
    );

    // Max width
    assert_eq!(
        ir.entries["simple"].max_width, parsed.entries["simple"].max_width,
        "Max width should survive XLIFF roundtrip"
    );

    // Contexts survive (source_references are not directly serialized by the XLIFF writer,
    // but context-group elements are)
    // The XLIFF writer writes context entries as context-group elements.
    // source_references capability is declared but the actual data goes through contexts.

    // Custom properties: XLIFF supports them via prop-group, but the current writer
    // may not serialize all properties. Check what survives.

    // Resource name
    assert_eq!(
        ir.entries["simple"].resource_name, parsed.entries["simple"].resource_name,
        "Resource name should survive XLIFF roundtrip"
    );

    // Resource type
    assert_eq!(
        ir.entries["simple"].resource_type, parsed.entries["simple"].resource_type,
        "Resource type should survive XLIFF roundtrip"
    );
}

// ── PO roundtrip ───────────────────────────────────────────────────────────
// Supports: plurals, comments, context, source_string, translation_state, source_references

#[test]
fn kitchen_sink_roundtrip_po() {
    let ir = kitchen_sink_ir();
    let written = po::Writer.write(&ir).unwrap();
    let parsed = po::Parser.parse(&written).unwrap();

    // Simple string value (in PO, the key=msgid=source, value=msgstr)
    // PO stores source as msgid and translation as msgstr
    // Our parser uses msgid as key and msgstr as value
    // When writing from IR, the source field becomes msgid if present
    assert!(
        parsed.entries.contains_key("simple")
            || parsed.entries.values().any(|e| {
                if let EntryValue::Simple(v) = &e.value {
                    v == "Hello, World!"
                } else {
                    false
                }
            }),
        "Simple entry should survive PO roundtrip in some form"
    );

    // Source references survive in PO
    // The entry that had source references should still have them
    let entry_with_refs = parsed
        .entries
        .values()
        .find(|e| !e.source_references.is_empty());
    assert!(
        entry_with_refs.is_some(),
        "Source references should survive PO roundtrip"
    );

    // Comments survive
    let entry_with_comments = parsed.entries.values().find(|e| !e.comments.is_empty());
    assert!(
        entry_with_comments.is_some(),
        "Comments should survive PO roundtrip"
    );
}

// ── YAML Rails roundtrip ───────────────────────────────────────────────────
// Supports: plurals, arrays, nested_keys

#[test]
fn kitchen_sink_roundtrip_yaml_rails() {
    let ir = kitchen_sink_ir();
    let written = yaml_rails::Writer.write(&ir).unwrap();
    let parsed = yaml_rails::Parser.parse(&written).unwrap();

    // Simple string value
    assert_eq!(
        ir.entries["simple"].value, parsed.entries["simple"].value,
        "Simple string should survive YAML Rails roundtrip"
    );

    // Plurals survive
    assert!(
        matches!(parsed.entries["items"].value, EntryValue::Plural(_)),
        "Plural should survive YAML Rails roundtrip"
    );
    if let (EntryValue::Plural(orig), EntryValue::Plural(parsed_ps)) =
        (&ir.entries["items"].value, &parsed.entries["items"].value)
    {
        assert_eq!(
            orig.other, parsed_ps.other,
            "Plural 'other' should match in YAML Rails roundtrip"
        );
    }

    // Arrays survive
    assert_eq!(
        ir.entries["colors"].value, parsed.entries["colors"].value,
        "Array should survive YAML Rails roundtrip"
    );
}
