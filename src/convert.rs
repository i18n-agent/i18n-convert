use crate::ir::*;

/// Check for potential data loss when converting an IR resource to a target format.
/// Compares what the IR contains against what the target format supports.
pub fn check_data_loss(
    resource: &I18nResource,
    target: &FormatCapabilities,
) -> Vec<DataLossWarning> {
    let mut warnings = Vec::new();

    let mut plural_keys = Vec::new();
    let mut array_keys = Vec::new();
    let mut comment_keys = Vec::new();
    let mut context_keys = Vec::new();
    let mut source_keys = Vec::new();
    let mut state_keys = Vec::new();
    let mut width_keys = Vec::new();
    let mut device_keys = Vec::new();
    let mut select_keys = Vec::new();
    let mut alt_keys = Vec::new();
    let mut ref_keys = Vec::new();
    let mut prop_keys = Vec::new();

    for (_, entry) in &resource.entries {
        if !target.plurals
            && matches!(
                entry.value,
                EntryValue::Plural(_) | EntryValue::MultiVariablePlural(_)
            )
        {
            plural_keys.push(entry.key.clone());
        }
        if !target.arrays && matches!(entry.value, EntryValue::Array(_)) {
            array_keys.push(entry.key.clone());
        }
        if !target.comments && !entry.comments.is_empty() {
            comment_keys.push(entry.key.clone());
        }
        if !target.context && !entry.contexts.is_empty() {
            context_keys.push(entry.key.clone());
        }
        if !target.source_string && entry.source.is_some() {
            source_keys.push(entry.key.clone());
        }
        if !target.translation_state && entry.state.is_some() {
            state_keys.push(entry.key.clone());
        }
        if !target.max_width && entry.max_width.is_some() {
            width_keys.push(entry.key.clone());
        }
        if !target.device_variants && entry.device_variants.is_some() {
            device_keys.push(entry.key.clone());
        }
        if !target.select_gender && matches!(entry.value, EntryValue::Select(_)) {
            select_keys.push(entry.key.clone());
        }
        if !target.alternatives && !entry.alternatives.is_empty() {
            alt_keys.push(entry.key.clone());
        }
        if !target.source_references && !entry.source_references.is_empty() {
            ref_keys.push(entry.key.clone());
        }
        if !target.custom_properties && !entry.properties.is_empty() {
            prop_keys.push(entry.key.clone());
        }
    }

    fn push_warning(
        warnings: &mut Vec<DataLossWarning>,
        keys: Vec<String>,
        attr: &str,
        sev: WarningSeverity,
    ) {
        if !keys.is_empty() {
            let count = keys.len();
            warnings.push(DataLossWarning {
                severity: sev,
                message: format!("{count} entries have {attr} that will be lost"),
                affected_keys: keys,
                lost_attribute: attr.to_string(),
                count,
            });
        }
    }

    push_warning(
        &mut warnings,
        plural_keys,
        "plurals",
        WarningSeverity::Error,
    );
    push_warning(&mut warnings, array_keys, "arrays", WarningSeverity::Error);
    push_warning(
        &mut warnings,
        select_keys,
        "select/gender",
        WarningSeverity::Error,
    );
    push_warning(
        &mut warnings,
        comment_keys,
        "comments",
        WarningSeverity::Warning,
    );
    push_warning(
        &mut warnings,
        context_keys,
        "context",
        WarningSeverity::Warning,
    );
    push_warning(
        &mut warnings,
        source_keys,
        "source strings",
        WarningSeverity::Warning,
    );
    push_warning(
        &mut warnings,
        state_keys,
        "translation state",
        WarningSeverity::Info,
    );
    push_warning(
        &mut warnings,
        width_keys,
        "max width constraints",
        WarningSeverity::Info,
    );
    push_warning(
        &mut warnings,
        device_keys,
        "device variants",
        WarningSeverity::Error,
    );
    push_warning(
        &mut warnings,
        alt_keys,
        "alternative translations",
        WarningSeverity::Info,
    );
    push_warning(
        &mut warnings,
        ref_keys,
        "source references",
        WarningSeverity::Info,
    );
    push_warning(
        &mut warnings,
        prop_keys,
        "custom properties",
        WarningSeverity::Info,
    );

    warnings
}
