use i18n_convert::convert::*;
use i18n_convert::ir::*;
use indexmap::IndexMap;

#[test]
fn generate_warnings_for_unsupported_plurals() {
    let mut entries = IndexMap::new();
    entries.insert(
        "items".to_string(),
        I18nEntry {
            key: "items".to_string(),
            value: EntryValue::Plural(PluralSet {
                one: Some("1 item".to_string()),
                other: "items".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata::default(),
        entries,
    };

    // Target format that doesn't support plurals
    let target_caps = FormatCapabilities {
        plurals: false,
        ..Default::default()
    };

    let warnings = check_data_loss(&resource, &target_caps);
    assert!(!warnings.is_empty());
    assert!(warnings.iter().any(|w| w.lost_attribute == "plurals"));
    assert_eq!(warnings[0].count, 1);
    assert_eq!(warnings[0].severity, WarningSeverity::Error);
}

#[test]
fn generate_warnings_for_unsupported_arrays() {
    let mut entries = IndexMap::new();
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

    let resource = I18nResource {
        metadata: ResourceMetadata::default(),
        entries,
    };

    let target_caps = FormatCapabilities {
        arrays: false,
        ..Default::default()
    };

    let warnings = check_data_loss(&resource, &target_caps);
    assert!(warnings.iter().any(|w| w.lost_attribute == "arrays"));
}

#[test]
fn generate_warnings_for_unsupported_comments() {
    let mut entries = IndexMap::new();
    entries.insert(
        "greeting".to_string(),
        I18nEntry {
            key: "greeting".to_string(),
            value: EntryValue::Simple("Hello".to_string()),
            comments: vec![Comment {
                text: "This is a greeting".to_string(),
                role: CommentRole::Developer,
                priority: None,
                annotates: None,
            }],
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata::default(),
        entries,
    };

    let target_caps = FormatCapabilities {
        comments: false,
        ..Default::default()
    };

    let warnings = check_data_loss(&resource, &target_caps);
    assert!(warnings.iter().any(|w| w.lost_attribute == "comments"));
    // Comments are Warning severity, not Error
    let comment_warning = warnings
        .iter()
        .find(|w| w.lost_attribute == "comments")
        .unwrap();
    assert_eq!(comment_warning.severity, WarningSeverity::Warning);
}

#[test]
fn generate_warnings_for_unsupported_translation_state() {
    let mut entries = IndexMap::new();
    entries.insert(
        "greeting".to_string(),
        I18nEntry {
            key: "greeting".to_string(),
            value: EntryValue::Simple("Hello".to_string()),
            state: Some(TranslationState::NeedsReview),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata::default(),
        entries,
    };

    let target_caps = FormatCapabilities {
        translation_state: false,
        ..Default::default()
    };

    let warnings = check_data_loss(&resource, &target_caps);
    assert!(warnings
        .iter()
        .any(|w| w.lost_attribute == "translation state"));
    let state_warning = warnings
        .iter()
        .find(|w| w.lost_attribute == "translation state")
        .unwrap();
    assert_eq!(state_warning.severity, WarningSeverity::Info);
}

#[test]
fn no_warnings_when_all_features_supported() {
    let mut entries = IndexMap::new();
    entries.insert(
        "greeting".to_string(),
        I18nEntry {
            key: "greeting".to_string(),
            value: EntryValue::Simple("Hello".to_string()),
            comments: vec![Comment {
                text: "A note".to_string(),
                role: CommentRole::Developer,
                priority: None,
                annotates: None,
            }],
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata::default(),
        entries,
    };

    // Target supports everything
    let target_caps = FormatCapabilities {
        plurals: true,
        arrays: true,
        comments: true,
        context: true,
        source_string: true,
        translatable_flag: true,
        translation_state: true,
        max_width: true,
        device_variants: true,
        select_gender: true,
        nested_keys: true,
        inline_markup: true,
        alternatives: true,
        source_references: true,
        custom_properties: true,
    };

    let warnings = check_data_loss(&resource, &target_caps);
    assert!(warnings.is_empty());
}

#[test]
fn no_warnings_for_empty_resource() {
    let resource = I18nResource {
        metadata: ResourceMetadata::default(),
        entries: IndexMap::new(),
    };

    let target_caps = FormatCapabilities::default(); // nothing supported
    let warnings = check_data_loss(&resource, &target_caps);
    assert!(warnings.is_empty());
}

#[test]
fn multiple_entries_count_correctly() {
    let mut entries = IndexMap::new();
    for i in 0..5 {
        entries.insert(
            format!("item_{i}"),
            I18nEntry {
                key: format!("item_{i}"),
                value: EntryValue::Plural(PluralSet {
                    one: Some(format!("{i} item")),
                    other: format!("{i} items"),
                    ..Default::default()
                }),
                ..Default::default()
            },
        );
    }

    let resource = I18nResource {
        metadata: ResourceMetadata::default(),
        entries,
    };

    let target_caps = FormatCapabilities::default();
    let warnings = check_data_loss(&resource, &target_caps);
    let plural_warning = warnings
        .iter()
        .find(|w| w.lost_attribute == "plurals")
        .unwrap();
    assert_eq!(plural_warning.count, 5);
    assert_eq!(plural_warning.affected_keys.len(), 5);
}

#[test]
fn select_gender_warning() {
    let mut entries = IndexMap::new();
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
        metadata: ResourceMetadata::default(),
        entries,
    };

    let target_caps = FormatCapabilities {
        select_gender: false,
        ..Default::default()
    };

    let warnings = check_data_loss(&resource, &target_caps);
    assert!(warnings
        .iter()
        .any(|w| w.lost_attribute == "select/gender"));
}

#[test]
fn device_variants_warning() {
    let mut entries = IndexMap::new();
    let mut variants = IndexMap::new();
    variants.insert(
        DeviceType::Phone,
        EntryValue::Simple("Tap".to_string()),
    );

    entries.insert(
        "action".to_string(),
        I18nEntry {
            key: "action".to_string(),
            value: EntryValue::Simple("Interact".to_string()),
            device_variants: Some(variants),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata::default(),
        entries,
    };

    let target_caps = FormatCapabilities {
        device_variants: false,
        ..Default::default()
    };

    let warnings = check_data_loss(&resource, &target_caps);
    assert!(warnings
        .iter()
        .any(|w| w.lost_attribute == "device variants"));
}
