use i18n_convert::ir::*;
use indexmap::IndexMap;

#[test]
fn create_simple_resource() {
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
            source_format: FormatId::AndroidXml,
            locale: Some("en".to_string()),
            ..Default::default()
        },
        entries,
    };

    assert_eq!(resource.entries.len(), 1);
    assert_eq!(resource.metadata.locale, Some("en".to_string()));
}

#[test]
fn create_plural_entry() {
    let entry = I18nEntry {
        key: "items".to_string(),
        value: EntryValue::Plural(PluralSet {
            one: Some("{count} item".to_string()),
            other: "{count} items".to_string(),
            ..Default::default()
        }),
        ..Default::default()
    };

    match &entry.value {
        EntryValue::Plural(ps) => {
            assert_eq!(ps.one, Some("{count} item".to_string()));
            assert_eq!(ps.other, "{count} items");
            assert!(ps.zero.is_none());
            assert!(ps.few.is_none());
        }
        _ => panic!("Expected Plural"),
    }
}

#[test]
fn create_entry_with_all_fields() {
    let entry = I18nEntry {
        key: "test".to_string(),
        value: EntryValue::Simple("Test".to_string()),
        comments: vec![Comment {
            text: "Developer note".to_string(),
            role: CommentRole::Developer,
            priority: Some(1),
            annotates: Some(AnnotationTarget::General),
        }],
        contexts: vec![ContextEntry {
            context_type: ContextType::Disambiguation,
            value: "menu item".to_string(),
            purpose: None,
        }],
        source: Some("Test".to_string()),
        previous_source: Some("OldTest".to_string()),
        previous_comment: None,
        placeholders: vec![Placeholder {
            name: "count".to_string(),
            original_syntax: "%1$d".to_string(),
            placeholder_type: Some(PlaceholderType::Integer),
            position: Some(1),
            example: Some("42".to_string()),
            description: None,
            format: None,
            optional_parameters: None,
        }],
        translatable: Some(true),
        state: Some(TranslationState::Translated),
        state_qualifier: None,
        approved: Some(true),
        obsolete: false,
        max_width: Some(100),
        min_width: None,
        max_height: None,
        min_height: None,
        size_unit: Some("pixel".to_string()),
        max_bytes: None,
        min_bytes: None,
        source_references: vec![SourceRef {
            file: "main.rs".to_string(),
            line: Some(42),
        }],
        flags: vec!["c-format".to_string()],
        device_variants: None,
        alternatives: vec![AlternativeTranslation {
            value: "Testing".to_string(),
            source: None,
            match_quality: Some(85.0),
            origin: Some("TM".to_string()),
            alt_type: None,
        }],
        properties: IndexMap::new(),
        resource_type: None,
        resource_name: None,
        format_ext: None,
    };

    assert_eq!(entry.comments.len(), 1);
    assert_eq!(entry.placeholders[0].name, "count");
    assert_eq!(entry.state, Some(TranslationState::Translated));
    assert_eq!(entry.alternatives[0].match_quality, Some(85.0));
}

#[test]
fn create_multi_variable_plural() {
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

    let entry = I18nEntry {
        key: "file_count".to_string(),
        value: EntryValue::MultiVariablePlural(MultiVariablePlural {
            pattern: "%#@files@ in %#@folders@".to_string(),
            variables,
        }),
        ..Default::default()
    };

    match &entry.value {
        EntryValue::MultiVariablePlural(mvp) => {
            assert_eq!(mvp.variables.len(), 2);
            assert!(mvp.variables.contains_key("files"));
            assert!(mvp.variables.contains_key("folders"));
        }
        _ => panic!("Expected MultiVariablePlural"),
    }
}

#[test]
fn create_select_entry() {
    let mut cases = IndexMap::new();
    cases.insert("male".to_string(), "He liked this".to_string());
    cases.insert("female".to_string(), "She liked this".to_string());
    cases.insert("other".to_string(), "They liked this".to_string());

    let entry = I18nEntry {
        key: "liked".to_string(),
        value: EntryValue::Select(SelectSet {
            variable: "gender".to_string(),
            cases,
        }),
        ..Default::default()
    };

    match &entry.value {
        EntryValue::Select(ss) => {
            assert_eq!(ss.variable, "gender");
            assert_eq!(ss.cases.len(), 3);
            assert_eq!(ss.cases["other"], "They liked this");
        }
        _ => panic!("Expected Select"),
    }
}

#[test]
fn format_capabilities_default_is_all_false() {
    let caps = FormatCapabilities::default();
    assert!(!caps.plurals);
    assert!(!caps.arrays);
    assert!(!caps.comments);
}

#[test]
fn create_array_entry() {
    let entry = I18nEntry {
        key: "planets".to_string(),
        value: EntryValue::Array(vec![
            "Mercury".to_string(),
            "Venus".to_string(),
            "Earth".to_string(),
        ]),
        ..Default::default()
    };

    match &entry.value {
        EntryValue::Array(arr) => {
            assert_eq!(arr.len(), 3);
            assert_eq!(arr[0], "Mercury");
        }
        _ => panic!("Expected Array"),
    }
}

#[test]
fn create_entry_with_device_variants() {
    let mut variants = IndexMap::new();
    variants.insert(
        DeviceType::Phone,
        EntryValue::Simple("Tap here".to_string()),
    );
    variants.insert(
        DeviceType::Tablet,
        EntryValue::Simple("Touch here".to_string()),
    );
    variants.insert(
        DeviceType::Desktop,
        EntryValue::Simple("Click here".to_string()),
    );

    let entry = I18nEntry {
        key: "action".to_string(),
        value: EntryValue::Simple("Interact here".to_string()),
        device_variants: Some(variants),
        ..Default::default()
    };

    let variants = entry.device_variants.as_ref().unwrap();
    assert_eq!(variants.len(), 3);
    assert_eq!(
        variants[&DeviceType::Phone],
        EntryValue::Simple("Tap here".to_string())
    );
}

#[test]
fn plural_set_with_exact_and_range_matches() {
    let mut exact = IndexMap::new();
    exact.insert(0, "No items at all".to_string());
    exact.insert(1, "Exactly one item".to_string());

    let ps = PluralSet {
        one: Some("{count} item".to_string()),
        other: "{count} items".to_string(),
        exact_matches: exact,
        range_matches: vec![
            PluralRange {
                from: Some(2),
                to: Some(5),
                inclusive: true,
                value: "A few items".to_string(),
            },
            PluralRange {
                from: Some(6),
                to: None,
                inclusive: true,
                value: "Many items".to_string(),
            },
        ],
        ..Default::default()
    };

    assert_eq!(ps.exact_matches.len(), 2);
    assert_eq!(ps.range_matches.len(), 2);
    assert_eq!(ps.range_matches[0].from, Some(2));
    assert_eq!(ps.range_matches[0].to, Some(5));
    assert!(ps.range_matches[1].to.is_none());
}

#[test]
fn resource_metadata_format_ids() {
    let formats = vec![
        FormatId::AndroidXml,
        FormatId::Xcstrings,
        FormatId::IosStrings,
        FormatId::Stringsdict,
        FormatId::Arb,
        FormatId::JsonStructured,
        FormatId::I18nextJson,
        FormatId::Xliff1,
        FormatId::Po,
        FormatId::YamlRails,
    ];
    assert_eq!(formats.len(), 10);
    // Default should be AndroidXml
    assert_eq!(FormatId::default(), FormatId::AndroidXml);
}

#[test]
fn format_extensions_android_xml() {
    let ext = FormatExtension::AndroidXml(AndroidXmlExt {
        formatted: Some(true),
        product: Some("phone".to_string()),
        xml_comments: vec!["header comment".to_string()],
    });

    match &ext {
        FormatExtension::AndroidXml(android) => {
            assert_eq!(android.formatted, Some(true));
            assert_eq!(android.product, Some("phone".to_string()));
            assert_eq!(android.xml_comments.len(), 1);
        }
        _ => panic!("Expected AndroidXml extension"),
    }
}

#[test]
fn format_extensions_arb() {
    let mut custom = IndexMap::new();
    custom.insert(
        "@@x-my-custom".to_string(),
        serde_json::Value::String("custom value".to_string()),
    );

    let ext = FormatExtension::Arb(ArbExt {
        message_type: Some("text".to_string()),
        custom_fields: custom,
    });

    match &ext {
        FormatExtension::Arb(arb) => {
            assert_eq!(arb.message_type, Some("text".to_string()));
            assert_eq!(arb.custom_fields.len(), 1);
        }
        _ => panic!("Expected Arb extension"),
    }
}

#[test]
fn data_loss_warning_construction() {
    let warning = DataLossWarning {
        severity: WarningSeverity::Error,
        message: "3 entries have plurals that will be lost".to_string(),
        affected_keys: vec!["items".to_string(), "messages".to_string(), "files".to_string()],
        lost_attribute: "plurals".to_string(),
        count: 3,
    };

    assert_eq!(warning.severity, WarningSeverity::Error);
    assert_eq!(warning.count, 3);
    assert_eq!(warning.affected_keys.len(), 3);
}

#[test]
fn all_comment_roles() {
    let roles = vec![
        CommentRole::Developer,
        CommentRole::Translator,
        CommentRole::Extracted,
        CommentRole::General,
    ];
    assert_eq!(roles.len(), 4);
    assert_ne!(CommentRole::Developer, CommentRole::Translator);
}

#[test]
fn all_translation_states() {
    let states = vec![
        TranslationState::New,
        TranslationState::Translated,
        TranslationState::NeedsReview,
        TranslationState::Reviewed,
        TranslationState::Final,
        TranslationState::Stale,
        TranslationState::NeedsTranslation,
        TranslationState::NeedsAdaptation,
        TranslationState::NeedsL10n,
        TranslationState::NeedsReviewAdaptation,
        TranslationState::NeedsReviewL10n,
        TranslationState::Vanished,
        TranslationState::Obsolete,
    ];
    assert_eq!(states.len(), 13);
}

#[test]
fn all_context_types() {
    let types = vec![
        ContextType::Disambiguation,
        ContextType::SourceFile,
        ContextType::LineNumber,
        ContextType::Element,
        ContextType::Description,
        ContextType::Custom("x-custom".to_string()),
    ];
    assert_eq!(types.len(), 6);
}

#[test]
fn all_placeholder_types() {
    let types = vec![
        PlaceholderType::String,
        PlaceholderType::Integer,
        PlaceholderType::Float,
        PlaceholderType::Double,
        PlaceholderType::DateTime,
        PlaceholderType::Currency,
        PlaceholderType::Object,
        PlaceholderType::Other("custom".to_string()),
    ];
    assert_eq!(types.len(), 8);
}

#[test]
fn kitchen_sink_ir() {
    // Build a resource with every possible field populated
    let mut entries = IndexMap::new();

    // Simple entry
    entries.insert(
        "simple".to_string(),
        I18nEntry {
            key: "simple".to_string(),
            value: EntryValue::Simple("Hello World".to_string()),
            comments: vec![
                Comment {
                    text: "A developer note".to_string(),
                    role: CommentRole::Developer,
                    priority: Some(5),
                    annotates: Some(AnnotationTarget::Source),
                },
                Comment {
                    text: "A translator note".to_string(),
                    role: CommentRole::Translator,
                    priority: None,
                    annotates: Some(AnnotationTarget::Target),
                },
            ],
            contexts: vec![ContextEntry {
                context_type: ContextType::Disambiguation,
                value: "greeting context".to_string(),
                purpose: Some("information".to_string()),
            }],
            source: Some("Hello World".to_string()),
            previous_source: Some("Hello".to_string()),
            previous_comment: Some("Old greeting".to_string()),
            placeholders: vec![],
            translatable: Some(true),
            state: Some(TranslationState::Translated),
            state_qualifier: Some("exact-match".to_string()),
            approved: Some(true),
            obsolete: false,
            max_width: Some(200),
            min_width: Some(50),
            max_height: Some(30),
            min_height: Some(10),
            size_unit: Some("pixel".to_string()),
            max_bytes: Some(1024),
            min_bytes: Some(5),
            source_references: vec![
                SourceRef {
                    file: "app.rs".to_string(),
                    line: Some(42),
                },
                SourceRef {
                    file: "lib.rs".to_string(),
                    line: None,
                },
            ],
            flags: vec!["c-format".to_string(), "no-wrap".to_string()],
            device_variants: None,
            alternatives: vec![AlternativeTranslation {
                value: "Hi World".to_string(),
                source: Some("Hello World".to_string()),
                match_quality: Some(90.0),
                origin: Some("TM".to_string()),
                alt_type: Some("fuzzy".to_string()),
            }],
            properties: {
                let mut p = IndexMap::new();
                p.insert("x-custom".to_string(), "value".to_string());
                p
            },
            resource_type: Some("string".to_string()),
            resource_name: Some("greeting".to_string()),
            format_ext: Some(FormatExtension::AndroidXml(AndroidXmlExt {
                formatted: Some(true),
                product: Some("phone".to_string()),
                xml_comments: vec![],
            })),
        },
    );

    // Plural entry
    entries.insert(
        "items".to_string(),
        I18nEntry {
            key: "items".to_string(),
            value: EntryValue::Plural(PluralSet {
                zero: Some("No items".to_string()),
                one: Some("{count} item".to_string()),
                two: Some("{count} items (dual)".to_string()),
                few: Some("{count} items (few)".to_string()),
                many: Some("{count} items (many)".to_string()),
                other: "{count} items".to_string(),
                exact_matches: IndexMap::new(),
                range_matches: Vec::new(),
                ordinal: false,
            }),
            ..Default::default()
        },
    );

    // Array entry
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

    // Select entry
    let mut cases = IndexMap::new();
    cases.insert("male".to_string(), "He".to_string());
    cases.insert("female".to_string(), "She".to_string());
    cases.insert("other".to_string(), "They".to_string());
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
            source_format: FormatId::Xliff1,
            locale: Some("en".to_string()),
            source_locale: Some("en".to_string()),
            headers: {
                let mut h = IndexMap::new();
                h.insert("Content-Type".to_string(), "text/plain".to_string());
                h
            },
            properties: {
                let mut p = IndexMap::new();
                p.insert("x-generator".to_string(), "test".to_string());
                p
            },
            encoding: Some("UTF-8".to_string()),
            direction: Some(TextDirection::Ltr),
            created_at: Some("2026-03-10".to_string()),
            modified_at: Some("2026-03-10".to_string()),
            created_by: Some("test".to_string()),
            modified_by: Some("test".to_string()),
            tool_name: Some("i18n-convert".to_string()),
            tool_version: Some("0.1.0".to_string()),
            format_ext: None,
        },
        entries,
    };

    // Verify the kitchen sink is fully populated
    assert_eq!(resource.entries.len(), 4);
    assert_eq!(resource.metadata.source_format, FormatId::Xliff1);
    assert_eq!(resource.metadata.locale, Some("en".to_string()));
    assert!(resource.metadata.direction.is_some());

    // Verify simple entry has all fields
    let simple = &resource.entries["simple"];
    assert_eq!(simple.comments.len(), 2);
    assert_eq!(simple.contexts.len(), 1);
    assert!(simple.source.is_some());
    assert!(simple.previous_source.is_some());
    assert!(simple.previous_comment.is_some());
    assert_eq!(simple.state, Some(TranslationState::Translated));
    assert!(simple.approved.is_some());
    assert!(simple.max_width.is_some());
    assert_eq!(simple.source_references.len(), 2);
    assert_eq!(simple.flags.len(), 2);
    assert_eq!(simple.alternatives.len(), 1);
    assert_eq!(simple.properties.len(), 1);
    assert!(simple.format_ext.is_some());
}
