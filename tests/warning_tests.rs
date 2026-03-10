use i18n_convert::convert::check_data_loss;
use i18n_convert::formats::*;
use i18n_convert::ir::*;
use indexmap::IndexMap;

// ── Helper functions ───────────────────────────────────────────────────────

fn ir_with_plurals() -> I18nResource {
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
    I18nResource {
        metadata: ResourceMetadata::default(),
        entries,
    }
}

fn ir_with_comments() -> I18nResource {
    let mut entries = IndexMap::new();
    entries.insert(
        "greeting".to_string(),
        I18nEntry {
            key: "greeting".to_string(),
            value: EntryValue::Simple("Hello".to_string()),
            comments: vec![Comment {
                text: "A developer note".to_string(),
                role: CommentRole::Developer,
                priority: None,
                annotates: None,
            }],
            ..Default::default()
        },
    );
    I18nResource {
        metadata: ResourceMetadata::default(),
        entries,
    }
}

fn ir_with_source_strings() -> I18nResource {
    let mut entries = IndexMap::new();
    entries.insert(
        "greeting".to_string(),
        I18nEntry {
            key: "greeting".to_string(),
            value: EntryValue::Simple("Hallo".to_string()),
            source: Some("Hello".to_string()),
            ..Default::default()
        },
    );
    I18nResource {
        metadata: ResourceMetadata::default(),
        entries,
    }
}

fn ir_with_translation_state() -> I18nResource {
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
    I18nResource {
        metadata: ResourceMetadata::default(),
        entries,
    }
}

fn ir_with_all_android_features() -> I18nResource {
    let mut entries = IndexMap::new();
    entries.insert(
        "greeting".to_string(),
        I18nEntry {
            key: "greeting".to_string(),
            value: EntryValue::Simple("Hello".to_string()),
            comments: vec![Comment {
                text: "Greeting message".to_string(),
                role: CommentRole::Developer,
                priority: None,
                annotates: None,
            }],
            translatable: Some(true),
            ..Default::default()
        },
    );
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
    I18nResource {
        metadata: ResourceMetadata::default(),
        entries,
    }
}

fn ir_with_multiple_lossy_entries(count: usize) -> I18nResource {
    let mut entries = IndexMap::new();
    for i in 0..count {
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
    I18nResource {
        metadata: ResourceMetadata::default(),
        entries,
    }
}

// ── 1. iOS strings cannot hold plurals -> Error warning ────────────────────

#[test]
fn ios_strings_cannot_hold_plurals() {
    let ir = ir_with_plurals();
    let warnings = check_data_loss(&ir, &ios_strings::Writer.capabilities());
    let plural_warning = warnings
        .iter()
        .find(|w| w.lost_attribute == "plurals")
        .expect("Should warn about plural loss");
    assert_eq!(
        plural_warning.severity,
        WarningSeverity::Error,
        "Plural loss is content loss, should be Error severity"
    );
}

// ── 2. JSON structured cannot hold comments -> Warning ─────────────────────

#[test]
fn json_cannot_hold_comments() {
    let ir = ir_with_comments();
    let warnings = check_data_loss(&ir, &json_structured::Writer.capabilities());
    let comment_warning = warnings
        .iter()
        .find(|w| w.lost_attribute == "comments")
        .expect("Should warn about comment loss");
    assert_eq!(
        comment_warning.severity,
        WarningSeverity::Warning,
        "Comment loss is metadata loss, should be Warning severity"
    );
}

// ── 3. JSON cannot hold source strings -> Warning ──────────────────────────

#[test]
fn json_cannot_hold_source_strings() {
    let ir = ir_with_source_strings();
    let warnings = check_data_loss(&ir, &json_structured::Writer.capabilities());
    let source_warning = warnings
        .iter()
        .find(|w| w.lost_attribute == "source strings")
        .expect("Should warn about source string loss");
    assert_eq!(
        source_warning.severity,
        WarningSeverity::Warning,
        "Source string loss is metadata loss, should be Warning severity"
    );
}

// ── 4. No warnings when converting same format ─────────────────────────────

#[test]
fn no_warnings_for_same_format_android_xml() {
    let ir = ir_with_all_android_features();
    let warnings = check_data_loss(&ir, &android_xml::Writer.capabilities());
    assert!(
        warnings.is_empty(),
        "No warnings expected when features match target capabilities, got: {:?}",
        warnings
            .iter()
            .map(|w| &w.lost_attribute)
            .collect::<Vec<_>>()
    );
}

#[test]
fn no_warnings_for_same_format_xliff() {
    // Build an IR with features XLIFF supports
    let mut entries = IndexMap::new();
    entries.insert(
        "greeting".to_string(),
        I18nEntry {
            key: "greeting".to_string(),
            value: EntryValue::Simple("Hallo".to_string()),
            source: Some("Hello".to_string()),
            comments: vec![Comment {
                text: "A note".to_string(),
                role: CommentRole::Developer,
                priority: None,
                annotates: None,
            }],
            state: Some(TranslationState::Translated),
            max_width: Some(100),
            source_references: vec![SourceRef {
                file: "main.c".to_string(),
                line: Some(42),
            }],
            alternatives: vec![AlternativeTranslation {
                value: "Hi".to_string(),
                source: None,
                match_quality: None,
                origin: None,
                alt_type: None,
            }],
            properties: {
                let mut p = IndexMap::new();
                p.insert("custom_key".to_string(), "custom_val".to_string());
                p
            },
            contexts: vec![ContextEntry {
                context_type: ContextType::Description,
                value: "Context info".to_string(),
                purpose: None,
            }],
            ..Default::default()
        },
    );

    let ir = I18nResource {
        metadata: ResourceMetadata::default(),
        entries,
    };
    let warnings = check_data_loss(&ir, &xliff1::Writer.capabilities());
    assert!(
        warnings.is_empty(),
        "No warnings expected for XLIFF-compatible IR, got: {:?}",
        warnings
            .iter()
            .map(|w| &w.lost_attribute)
            .collect::<Vec<_>>()
    );
}

// ── 5. Warning count matches affected entry count ──────────────────────────

#[test]
fn warning_count_matches_affected_entry_count() {
    let ir = ir_with_multiple_lossy_entries(7);
    let warnings = check_data_loss(&ir, &json_structured::Writer.capabilities());
    let plural_warning = warnings
        .iter()
        .find(|w| w.lost_attribute == "plurals")
        .expect("Should have plural warning");
    assert_eq!(
        plural_warning.count, 7,
        "Warning count should match number of affected entries"
    );
    assert_eq!(
        plural_warning.affected_keys.len(),
        7,
        "Affected keys list should have 7 entries"
    );
}

#[test]
fn warning_count_single_entry() {
    let ir = ir_with_plurals();
    let warnings = check_data_loss(&ir, &json_structured::Writer.capabilities());
    let plural_warning = warnings
        .iter()
        .find(|w| w.lost_attribute == "plurals")
        .expect("Should have plural warning");
    assert_eq!(plural_warning.count, 1);
    assert_eq!(plural_warning.affected_keys, vec!["items"]);
}

// ── 6. Severity levels are correct ─────────────────────────────────────────

#[test]
fn severity_error_for_content_loss_plurals() {
    let ir = ir_with_plurals();
    let warnings = check_data_loss(&ir, &json_structured::Writer.capabilities());
    let w = warnings
        .iter()
        .find(|w| w.lost_attribute == "plurals")
        .unwrap();
    assert_eq!(w.severity, WarningSeverity::Error);
}

#[test]
fn severity_error_for_content_loss_arrays() {
    let mut entries = IndexMap::new();
    entries.insert(
        "colors".to_string(),
        I18nEntry {
            key: "colors".to_string(),
            value: EntryValue::Array(vec!["Red".to_string(), "Blue".to_string()]),
            ..Default::default()
        },
    );
    let ir = I18nResource {
        metadata: ResourceMetadata::default(),
        entries,
    };
    let warnings = check_data_loss(&ir, &arb::Writer.capabilities());
    let w = warnings
        .iter()
        .find(|w| w.lost_attribute == "arrays")
        .unwrap();
    assert_eq!(w.severity, WarningSeverity::Error);
}

#[test]
fn severity_error_for_content_loss_select() {
    let mut entries = IndexMap::new();
    let mut cases = IndexMap::new();
    cases.insert("male".to_string(), "He".to_string());
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
    let ir = I18nResource {
        metadata: ResourceMetadata::default(),
        entries,
    };
    let warnings = check_data_loss(&ir, &json_structured::Writer.capabilities());
    let w = warnings
        .iter()
        .find(|w| w.lost_attribute == "select/gender")
        .unwrap();
    assert_eq!(w.severity, WarningSeverity::Error);
}

#[test]
fn severity_error_for_content_loss_device_variants() {
    let mut entries = IndexMap::new();
    let mut variants = IndexMap::new();
    variants.insert(DeviceType::Phone, EntryValue::Simple("Tap".to_string()));
    entries.insert(
        "action".to_string(),
        I18nEntry {
            key: "action".to_string(),
            value: EntryValue::Simple("Interact".to_string()),
            device_variants: Some(variants),
            ..Default::default()
        },
    );
    let ir = I18nResource {
        metadata: ResourceMetadata::default(),
        entries,
    };
    let warnings = check_data_loss(&ir, &json_structured::Writer.capabilities());
    let w = warnings
        .iter()
        .find(|w| w.lost_attribute == "device variants")
        .unwrap();
    assert_eq!(w.severity, WarningSeverity::Error);
}

#[test]
fn severity_warning_for_metadata_loss_comments() {
    let ir = ir_with_comments();
    let warnings = check_data_loss(&ir, &json_structured::Writer.capabilities());
    let w = warnings
        .iter()
        .find(|w| w.lost_attribute == "comments")
        .unwrap();
    assert_eq!(w.severity, WarningSeverity::Warning);
}

#[test]
fn severity_warning_for_metadata_loss_context() {
    let mut entries = IndexMap::new();
    entries.insert(
        "greeting".to_string(),
        I18nEntry {
            key: "greeting".to_string(),
            value: EntryValue::Simple("Hello".to_string()),
            contexts: vec![ContextEntry {
                context_type: ContextType::Disambiguation,
                value: "UI button".to_string(),
                purpose: None,
            }],
            ..Default::default()
        },
    );
    let ir = I18nResource {
        metadata: ResourceMetadata::default(),
        entries,
    };
    let warnings = check_data_loss(&ir, &json_structured::Writer.capabilities());
    let w = warnings
        .iter()
        .find(|w| w.lost_attribute == "context")
        .unwrap();
    assert_eq!(w.severity, WarningSeverity::Warning);
}

#[test]
fn severity_warning_for_metadata_loss_source_strings() {
    let ir = ir_with_source_strings();
    let warnings = check_data_loss(&ir, &json_structured::Writer.capabilities());
    let w = warnings
        .iter()
        .find(|w| w.lost_attribute == "source strings")
        .unwrap();
    assert_eq!(w.severity, WarningSeverity::Warning);
}

#[test]
fn severity_info_for_workflow_data_translation_state() {
    let ir = ir_with_translation_state();
    let warnings = check_data_loss(&ir, &json_structured::Writer.capabilities());
    let w = warnings
        .iter()
        .find(|w| w.lost_attribute == "translation state")
        .unwrap();
    assert_eq!(w.severity, WarningSeverity::Info);
}

#[test]
fn severity_info_for_workflow_data_alternatives() {
    let mut entries = IndexMap::new();
    entries.insert(
        "greeting".to_string(),
        I18nEntry {
            key: "greeting".to_string(),
            value: EntryValue::Simple("Hello".to_string()),
            alternatives: vec![AlternativeTranslation {
                value: "Hi".to_string(),
                source: None,
                match_quality: Some(0.9),
                origin: None,
                alt_type: None,
            }],
            ..Default::default()
        },
    );
    let ir = I18nResource {
        metadata: ResourceMetadata::default(),
        entries,
    };
    let warnings = check_data_loss(&ir, &json_structured::Writer.capabilities());
    let w = warnings
        .iter()
        .find(|w| w.lost_attribute == "alternative translations")
        .unwrap();
    assert_eq!(w.severity, WarningSeverity::Info);
}

#[test]
fn severity_info_for_workflow_data_source_references() {
    let mut entries = IndexMap::new();
    entries.insert(
        "greeting".to_string(),
        I18nEntry {
            key: "greeting".to_string(),
            value: EntryValue::Simple("Hello".to_string()),
            source_references: vec![SourceRef {
                file: "main.c".to_string(),
                line: Some(10),
            }],
            ..Default::default()
        },
    );
    let ir = I18nResource {
        metadata: ResourceMetadata::default(),
        entries,
    };
    let warnings = check_data_loss(&ir, &json_structured::Writer.capabilities());
    let w = warnings
        .iter()
        .find(|w| w.lost_attribute == "source references")
        .unwrap();
    assert_eq!(w.severity, WarningSeverity::Info);
}

#[test]
fn severity_info_for_workflow_data_max_width() {
    let mut entries = IndexMap::new();
    entries.insert(
        "greeting".to_string(),
        I18nEntry {
            key: "greeting".to_string(),
            value: EntryValue::Simple("Hello".to_string()),
            max_width: Some(200),
            ..Default::default()
        },
    );
    let ir = I18nResource {
        metadata: ResourceMetadata::default(),
        entries,
    };
    let warnings = check_data_loss(&ir, &json_structured::Writer.capabilities());
    let w = warnings
        .iter()
        .find(|w| w.lost_attribute == "max width constraints")
        .unwrap();
    assert_eq!(w.severity, WarningSeverity::Info);
}

#[test]
fn severity_info_for_workflow_data_custom_properties() {
    let mut entries = IndexMap::new();
    let mut props = IndexMap::new();
    props.insert("custom_key".to_string(), "custom_value".to_string());
    entries.insert(
        "greeting".to_string(),
        I18nEntry {
            key: "greeting".to_string(),
            value: EntryValue::Simple("Hello".to_string()),
            properties: props,
            ..Default::default()
        },
    );
    let ir = I18nResource {
        metadata: ResourceMetadata::default(),
        entries,
    };
    let warnings = check_data_loss(&ir, &json_structured::Writer.capabilities());
    let w = warnings
        .iter()
        .find(|w| w.lost_attribute == "custom properties")
        .unwrap();
    assert_eq!(w.severity, WarningSeverity::Info);
}
