use crate::ir::*;
use super::*;
use serde_json::Value;

pub struct Parser;
pub struct Writer;

// ─── Device key mapping ───────────────────────────────────────────────

fn device_key_to_type(key: &str) -> DeviceType {
    match key {
        "iphone" => DeviceType::Phone,
        "ipad" => DeviceType::Tablet,
        "ipod" => DeviceType::IPod,
        "mac" => DeviceType::Desktop,
        "appletv" => DeviceType::TV,
        "applewatch" => DeviceType::Watch,
        "applevision" => DeviceType::Vision,
        "other" => DeviceType::Default,
        _ => DeviceType::Other(key.to_string()),
    }
}

fn device_type_to_key(dt: &DeviceType) -> &str {
    match dt {
        DeviceType::Phone => "iphone",
        DeviceType::Tablet => "ipad",
        DeviceType::IPod => "ipod",
        DeviceType::Desktop => "mac",
        DeviceType::TV => "appletv",
        DeviceType::Watch => "applewatch",
        DeviceType::Vision => "applevision",
        DeviceType::Default => "other",
        DeviceType::Other(s) => s.as_str(),
    }
}

// ─── State mapping ────────────────────────────────────────────────────

fn state_from_str(s: &str) -> Option<TranslationState> {
    match s {
        "new" => Some(TranslationState::New),
        "translated" => Some(TranslationState::Translated),
        "needs_review" => Some(TranslationState::NeedsReview),
        "stale" => Some(TranslationState::Stale),
        _ => None,
    }
}

fn state_to_str(s: &TranslationState) -> &'static str {
    match s {
        TranslationState::New => "new",
        TranslationState::Translated => "translated",
        TranslationState::NeedsReview => "needs_review",
        TranslationState::Stale => "stale",
        // Map other states to the closest xcstrings equivalent
        TranslationState::Reviewed | TranslationState::Final => "translated",
        TranslationState::NeedsTranslation
        | TranslationState::NeedsAdaptation
        | TranslationState::NeedsL10n
        | TranslationState::NeedsReviewAdaptation
        | TranslationState::NeedsReviewL10n => "needs_review",
        TranslationState::Vanished | TranslationState::Obsolete => "stale",
    }
}

// ─── Parse helpers ────────────────────────────────────────────────────

/// Parse a stringUnit JSON object into (state, value).
fn parse_string_unit(su: &Value) -> Option<(Option<TranslationState>, String)> {
    let obj = su.as_object()?;
    let value = obj.get("value")?.as_str()?.to_string();
    let state = obj
        .get("state")
        .and_then(|s| s.as_str())
        .and_then(state_from_str);
    Some((state, value))
}

/// Parse plural variations into a PluralSet.
fn parse_plural_variations(plural_obj: &Value) -> Option<PluralSet> {
    let map = plural_obj.as_object()?;
    let mut ps = PluralSet::default();

    for (category, variant) in map {
        let su = variant.get("stringUnit")?;
        let (_, value) = parse_string_unit(su)?;
        match category.as_str() {
            "zero" => ps.zero = Some(value),
            "one" => ps.one = Some(value),
            "two" => ps.two = Some(value),
            "few" => ps.few = Some(value),
            "many" => ps.many = Some(value),
            "other" => ps.other = value,
            _ => {}
        }
    }
    Some(ps)
}

/// Parse device variations into an IndexMap<DeviceType, EntryValue>.
fn parse_device_variations(
    device_obj: &Value,
) -> Option<IndexMap<DeviceType, EntryValue>> {
    let map = device_obj.as_object()?;
    let mut variants = IndexMap::new();

    for (device_key, variant) in map {
        let su = variant.get("stringUnit")?;
        let (_, value) = parse_string_unit(su)?;
        variants.insert(
            device_key_to_type(device_key),
            EntryValue::Simple(value),
        );
    }
    Some(variants)
}

/// Parse substitutions into MultiVariablePlural.
fn parse_substitutions(
    subs_obj: &Value,
    pattern: &str,
) -> Option<MultiVariablePlural> {
    let map = subs_obj.as_object()?;
    let mut variables = IndexMap::new();

    for (name, sub) in map {
        let sub_obj = sub.as_object()?;
        let arg_num = sub_obj.get("argNum").and_then(|v| v.as_u64()).map(|n| n as u32);
        let format_specifier = sub_obj
            .get("formatSpecifier")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let plural_set = sub_obj
            .get("variations")
            .and_then(|v| v.get("plural"))
            .and_then(parse_plural_variations)?;

        variables.insert(
            name.clone(),
            PluralVariable {
                name: name.clone(),
                format_specifier,
                arg_num,
                plural_set,
            },
        );
    }

    Some(MultiVariablePlural {
        pattern: pattern.to_string(),
        variables,
    })
}

// ─── Write helpers ────────────────────────────────────────────────────

fn string_unit_json(state: Option<&TranslationState>, value: &str) -> Value {
    let mut su = serde_json::Map::new();
    let st = state.map(state_to_str).unwrap_or("translated");
    su.insert("state".to_string(), Value::String(st.to_string()));
    su.insert("value".to_string(), Value::String(value.to_string()));
    Value::Object(su)
}

fn plural_set_to_json(ps: &PluralSet, state: Option<&TranslationState>) -> Value {
    let mut plural = serde_json::Map::new();
    let cats: &[(&str, &Option<String>)] = &[
        ("zero", &ps.zero),
        ("one", &ps.one),
        ("two", &ps.two),
        ("few", &ps.few),
        ("many", &ps.many),
    ];
    for (cat, val) in cats {
        if let Some(v) = val {
            let mut variant = serde_json::Map::new();
            variant.insert("stringUnit".to_string(), string_unit_json(state, v));
            plural.insert(cat.to_string(), Value::Object(variant));
        }
    }
    // 'other' is always present
    {
        let mut variant = serde_json::Map::new();
        variant.insert("stringUnit".to_string(), string_unit_json(state, &ps.other));
        plural.insert("other".to_string(), Value::Object(variant));
    }
    Value::Object(plural)
}

// ─── Parser implementation ────────────────────────────────────────────

impl FormatParser for Parser {
    fn detect(&self, extension: &str, content: &[u8]) -> Confidence {
        if extension == ".xcstrings" {
            return Confidence::Definite;
        }
        if extension == ".json" {
            if let Ok(s) = std::str::from_utf8(content) {
                if s.contains("\"sourceLanguage\"") && s.contains("\"strings\"") {
                    return Confidence::High;
                }
            }
        }
        Confidence::None
    }

    fn parse(&self, content: &[u8]) -> Result<I18nResource, ParseError> {
        let root: Value = serde_json::from_slice(content)
            .map_err(|e| ParseError::Json(e.to_string()))?;

        let root_obj = root
            .as_object()
            .ok_or_else(|| ParseError::InvalidFormat("Root must be a JSON object".into()))?;

        let source_language = root_obj
            .get("sourceLanguage")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let version = root_obj
            .get("version")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let strings = root_obj
            .get("strings")
            .and_then(|v| v.as_object())
            .ok_or_else(|| ParseError::InvalidFormat("Missing 'strings' dictionary".into()))?;

        let mut entries = IndexMap::new();

        for (key, string_def) in strings {
            let def = match string_def.as_object() {
                Some(o) => o,
                None => continue,
            };

            let comment = def.get("comment").and_then(|v| v.as_str()).map(|s| s.to_string());
            let extraction_state = def
                .get("extractionState")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let should_translate = def.get("shouldTranslate").and_then(|v| v.as_bool());

            // We merge localizations from all locales. For the source locale we
            // build the entry value; for additional locales we store them in the
            // entry's properties as serialized JSON so a round-trip can reconstruct
            // the full file. But the primary IR entry represents the source locale
            // content (or the first locale we encounter).

            let localizations = def.get("localizations").and_then(|v| v.as_object());

            // Determine the primary locale to use for the entry value.
            // Prefer source_language; if missing pick first available.
            let primary_locale = source_language
                .as_deref()
                .and_then(|sl| {
                    localizations
                        .and_then(|locs| locs.get(sl))
                        .map(|_| sl)
                });

            let mut entry_value = EntryValue::Simple(String::new());
            let mut entry_state: Option<TranslationState> = None;
            let mut device_variants: Option<IndexMap<DeviceType, EntryValue>> = None;

            if let Some(locs) = localizations {
                // Pick the primary locale's localization for the entry value
                let primary_loc = primary_locale
                    .and_then(|pl| locs.get(pl))
                    .or_else(|| locs.values().next());

                if let Some(loc) = primary_loc {
                    let loc_obj = loc.as_object();

                    // Check for substitutions (MultiVariablePlural)
                    if let Some(subs) = loc_obj.and_then(|o| o.get("substitutions")) {
                        // Get the pattern from the stringUnit value
                        let pattern = loc_obj
                            .and_then(|o| o.get("stringUnit"))
                            .and_then(|su| su.get("value"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("");

                        if let Some(su) = loc_obj.and_then(|o| o.get("stringUnit")) {
                            entry_state = su
                                .get("state")
                                .and_then(|s| s.as_str())
                                .and_then(state_from_str);
                        }

                        if let Some(mvp) = parse_substitutions(subs, pattern) {
                            entry_value = EntryValue::MultiVariablePlural(mvp);
                        }
                    }
                    // Check for stringUnit (simple value)
                    else if let Some(su) = loc_obj.and_then(|o| o.get("stringUnit")) {
                        if let Some((state, value)) = parse_string_unit(su) {
                            entry_state = state;
                            entry_value = EntryValue::Simple(value);
                        }
                    }
                    // Check for variations
                    else if let Some(variations) = loc_obj.and_then(|o| o.get("variations")) {
                        let var_obj = variations.as_object();

                        // Plural variations
                        if let Some(plural) = var_obj.and_then(|o| o.get("plural")) {
                            if let Some(ps) = parse_plural_variations(plural) {
                                // Try to get state from the 'other' variant
                                entry_state = plural
                                    .get("other")
                                    .and_then(|v| v.get("stringUnit"))
                                    .and_then(|su| su.get("state"))
                                    .and_then(|s| s.as_str())
                                    .and_then(state_from_str);
                                entry_value = EntryValue::Plural(ps);
                            }
                        }
                        // Device variations
                        else if let Some(device) = var_obj.and_then(|o| o.get("device")) {
                            if let Some(dv) = parse_device_variations(device) {
                                // Try to get state from 'other' device variant
                                entry_state = device
                                    .get("other")
                                    .and_then(|v| v.get("stringUnit"))
                                    .and_then(|su| su.get("state"))
                                    .and_then(|s| s.as_str())
                                    .and_then(state_from_str);
                                // The 'other' device variant becomes the main entry value
                                let default_val = dv
                                    .get(&DeviceType::Default)
                                    .cloned()
                                    .unwrap_or(EntryValue::Simple(String::new()));
                                entry_value = default_val;
                                // Store non-default variants
                                let non_default: IndexMap<DeviceType, EntryValue> = dv
                                    .into_iter()
                                    .filter(|(dt, _)| *dt != DeviceType::Default)
                                    .collect();
                                if !non_default.is_empty() {
                                    device_variants = Some(non_default);
                                }
                            }
                        }
                    }
                }

                // Serialize non-primary localizations into properties for round-trip
                for (locale, loc_data) in locs {
                    if Some(locale.as_str()) == primary_locale || primary_locale.is_none() {
                        // We only skip the exact primary locale we already parsed
                        if Some(locale.as_str()) == primary_locale {
                            continue;
                        }
                        if locs.values().next().map(|v| std::ptr::eq(v, loc_data)).unwrap_or(false) {
                            continue;
                        }
                    }
                    // Store additional localizations as JSON in properties
                    let json_str = serde_json::to_string(loc_data).unwrap_or_default();
                    // We use a well-known prefix so the writer can reconstruct them
                    let prop_key = format!("xcstrings.localization.{}", locale);
                    // Will be set below on entry
                    entries.entry(key.clone()).or_insert_with(|| I18nEntry {
                        key: key.clone(),
                        ..Default::default()
                    });
                    // We will update properties after creating the entry
                    if let Some(e) = entries.get_mut(key) {
                        e.properties.insert(prop_key, json_str);
                    }
                }
            }

            // Create or update the entry
            let entry = entries.entry(key.clone()).or_insert_with(|| I18nEntry {
                key: key.clone(),
                ..Default::default()
            });

            entry.value = entry_value;
            entry.state = entry_state;
            entry.device_variants = device_variants;

            if let Some(c) = comment {
                entry.comments.push(Comment {
                    text: c,
                    role: CommentRole::General,
                    priority: None,
                    annotates: None,
                });
            }

            if let Some(false) = should_translate {
                entry.translatable = Some(false);
            }

            let ext = XcstringsExt {
                extraction_state,
                version: None, // version is file-level, not entry-level
            };
            entry.format_ext = Some(FormatExtension::Xcstrings(ext));
        }

        let metadata = ResourceMetadata {
            source_format: FormatId::Xcstrings,
            locale: source_language.clone(),
            source_locale: source_language,
            format_ext: Some(FormatExtension::Xcstrings(XcstringsExt {
                extraction_state: None,
                version,
            })),
            ..Default::default()
        };

        Ok(I18nResource { metadata, entries })
    }

    fn capabilities(&self) -> FormatCapabilities {
        FormatCapabilities {
            plurals: true,
            arrays: false,
            comments: true,
            context: false,
            source_string: false,
            translatable_flag: true,
            translation_state: true,
            max_width: false,
            device_variants: true,
            select_gender: false,
            nested_keys: false,
            inline_markup: false,
            alternatives: false,
            source_references: false,
            custom_properties: false,
        }
    }
}

// ─── Writer implementation ────────────────────────────────────────────

impl FormatWriter for Writer {
    fn write(&self, resource: &I18nResource) -> Result<Vec<u8>, WriteError> {
        let mut root = serde_json::Map::new();

        // sourceLanguage
        let source_lang = resource
            .metadata
            .source_locale
            .as_deref()
            .or(resource.metadata.locale.as_deref())
            .unwrap_or("en");
        root.insert(
            "sourceLanguage".to_string(),
            Value::String(source_lang.to_string()),
        );

        // strings
        let mut strings = serde_json::Map::new();

        for (key, entry) in &resource.entries {
            let mut string_def = serde_json::Map::new();

            // comment
            if let Some(comment) = entry.comments.first() {
                string_def.insert(
                    "comment".to_string(),
                    Value::String(comment.text.clone()),
                );
            }

            // extractionState
            if let Some(FormatExtension::Xcstrings(ext)) = &entry.format_ext {
                if let Some(es) = &ext.extraction_state {
                    string_def.insert(
                        "extractionState".to_string(),
                        Value::String(es.clone()),
                    );
                }
            }

            // shouldTranslate
            if entry.translatable == Some(false) {
                string_def.insert(
                    "shouldTranslate".to_string(),
                    Value::Bool(false),
                );
            }

            // localizations
            let mut localizations = serde_json::Map::new();

            // Build the primary locale localization
            let primary_locale = source_lang;
            let state_ref = entry.state.as_ref();

            match &entry.value {
                EntryValue::Simple(value) => {
                    let mut loc = serde_json::Map::new();
                    loc.insert(
                        "stringUnit".to_string(),
                        string_unit_json(state_ref, value),
                    );
                    localizations.insert(
                        primary_locale.to_string(),
                        Value::Object(loc),
                    );
                }
                EntryValue::Plural(ps) => {
                    let mut loc = serde_json::Map::new();
                    let mut variations = serde_json::Map::new();
                    variations.insert(
                        "plural".to_string(),
                        plural_set_to_json(ps, state_ref),
                    );
                    loc.insert("variations".to_string(), Value::Object(variations));
                    localizations.insert(
                        primary_locale.to_string(),
                        Value::Object(loc),
                    );
                }
                EntryValue::MultiVariablePlural(mvp) => {
                    let mut loc = serde_json::Map::new();

                    // stringUnit with the pattern
                    loc.insert(
                        "stringUnit".to_string(),
                        string_unit_json(state_ref, &mvp.pattern),
                    );

                    // substitutions
                    let mut subs = serde_json::Map::new();
                    for (name, var) in &mvp.variables {
                        let mut sub = serde_json::Map::new();
                        if let Some(arg_num) = var.arg_num {
                            sub.insert(
                                "argNum".to_string(),
                                Value::Number(serde_json::Number::from(arg_num)),
                            );
                        }
                        if let Some(fs) = &var.format_specifier {
                            sub.insert(
                                "formatSpecifier".to_string(),
                                Value::String(fs.clone()),
                            );
                        }
                        let mut variations = serde_json::Map::new();
                        variations.insert(
                            "plural".to_string(),
                            plural_set_to_json(&var.plural_set, state_ref),
                        );
                        sub.insert("variations".to_string(), Value::Object(variations));
                        subs.insert(name.clone(), Value::Object(sub));
                    }
                    loc.insert("substitutions".to_string(), Value::Object(subs));

                    localizations.insert(
                        primary_locale.to_string(),
                        Value::Object(loc),
                    );
                }
                EntryValue::Array(_) | EntryValue::Select(_) => {
                    // xcstrings doesn't support arrays or select; write as simple string fallback
                    let fallback = match &entry.value {
                        EntryValue::Array(arr) => arr.join(", "),
                        EntryValue::Select(sel) => {
                            sel.cases.get("other").cloned().unwrap_or_default()
                        }
                        _ => String::new(),
                    };
                    let mut loc = serde_json::Map::new();
                    loc.insert(
                        "stringUnit".to_string(),
                        string_unit_json(state_ref, &fallback),
                    );
                    localizations.insert(
                        primary_locale.to_string(),
                        Value::Object(loc),
                    );
                }
            }

            // Write device variants
            if let Some(dv) = &entry.device_variants {
                // If we already wrote the primary locale as a simple/plural value,
                // we need to rewrite it as device variations
                let has_device_variants = !dv.is_empty();
                if has_device_variants {
                    let mut device_map = serde_json::Map::new();

                    // Write non-default variants
                    for (dt, val) in dv {
                        let dk = device_type_to_key(dt);
                        let text = match val {
                            EntryValue::Simple(s) => s.clone(),
                            _ => String::new(),
                        };
                        let mut variant = serde_json::Map::new();
                        variant.insert(
                            "stringUnit".to_string(),
                            string_unit_json(state_ref, &text),
                        );
                        device_map.insert(dk.to_string(), Value::Object(variant));
                    }

                    // Write 'other' (default) from entry.value
                    let default_text = match &entry.value {
                        EntryValue::Simple(s) => s.clone(),
                        _ => String::new(),
                    };
                    let mut variant = serde_json::Map::new();
                    variant.insert(
                        "stringUnit".to_string(),
                        string_unit_json(state_ref, &default_text),
                    );
                    device_map.insert("other".to_string(), Value::Object(variant));

                    let mut variations = serde_json::Map::new();
                    variations.insert("device".to_string(), Value::Object(device_map));

                    let mut loc = serde_json::Map::new();
                    loc.insert("variations".to_string(), Value::Object(variations));
                    // Override the primary locale with device variations
                    localizations.insert(
                        primary_locale.to_string(),
                        Value::Object(loc),
                    );
                }
            }

            // Restore additional localizations from properties
            for (prop_key, prop_val) in &entry.properties {
                if let Some(locale) = prop_key.strip_prefix("xcstrings.localization.") {
                    if let Ok(loc_data) = serde_json::from_str::<Value>(prop_val) {
                        localizations.insert(locale.to_string(), loc_data);
                    }
                }
            }

            if !localizations.is_empty() {
                string_def.insert(
                    "localizations".to_string(),
                    Value::Object(localizations),
                );
            }

            strings.insert(key.clone(), Value::Object(string_def));
        }

        root.insert("strings".to_string(), Value::Object(strings));

        // version
        let version = resource
            .metadata
            .format_ext
            .as_ref()
            .and_then(|ext| match ext {
                FormatExtension::Xcstrings(x) => x.version.as_deref(),
                _ => None,
            })
            .unwrap_or("1.0");
        root.insert("version".to_string(), Value::String(version.to_string()));

        let json = serde_json::to_string_pretty(&Value::Object(root))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        Ok(json.into_bytes())
    }

    fn capabilities(&self) -> FormatCapabilities {
        Parser.capabilities()
    }
}
