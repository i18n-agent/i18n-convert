use i18n_convert::formats::{FormatParser, FormatWriter, Confidence};
use i18n_convert::formats::xcstrings::{Parser, Writer};
use i18n_convert::ir::*;
use indexmap::IndexMap;

// ─── Helper ───────────────────────────────────────────────────────────

fn fixture(name: &str) -> Vec<u8> {
    let path = format!(
        "{}/tests/fixtures/xcstrings/{}",
        env!("CARGO_MANIFEST_DIR"),
        name
    );
    std::fs::read(&path).unwrap_or_else(|e| panic!("Failed to read fixture {}: {}", path, e))
}

fn parse(name: &str) -> I18nResource {
    let content = fixture(name);
    Parser.parse(&content).expect("parse failed")
}

fn round_trip(name: &str) -> I18nResource {
    let resource = parse(name);
    let written = Writer.write(&resource).expect("write failed");
    let re_parsed = Parser
        .parse(&written)
        .expect("re-parse after round-trip failed");
    re_parsed
}

/// Parse the raw JSON back into serde_json::Value for structural comparison.
fn write_to_json(resource: &I18nResource) -> serde_json::Value {
    let bytes = Writer.write(resource).expect("write failed");
    serde_json::from_slice(&bytes).expect("written output is not valid JSON")
}

// ─── Detection tests ──────────────────────────────────────────────────

#[test]
fn detect_xcstrings_extension() {
    assert_eq!(
        Parser.detect(".xcstrings", b"{}"),
        Confidence::Definite,
    );
}

#[test]
fn detect_json_with_xcstrings_content() {
    let content = br#"{"sourceLanguage":"en","strings":{}}"#;
    assert_eq!(
        Parser.detect(".json", content),
        Confidence::High,
    );
}

#[test]
fn detect_json_without_xcstrings_content() {
    let content = br#"{"key":"value"}"#;
    assert_eq!(
        Parser.detect(".json", content),
        Confidence::None,
    );
}

#[test]
fn detect_other_extension() {
    assert_eq!(
        Parser.detect(".xml", b"{}"),
        Confidence::None,
    );
}

// ─── Simple strings ───────────────────────────────────────────────────

#[test]
fn parse_simple_metadata() {
    let resource = parse("simple.xcstrings");
    assert_eq!(resource.metadata.source_format, FormatId::Xcstrings);
    assert_eq!(resource.metadata.locale, Some("en".to_string()));
    assert_eq!(resource.metadata.source_locale, Some("en".to_string()));

    // Check version in format_ext
    match &resource.metadata.format_ext {
        Some(FormatExtension::Xcstrings(ext)) => {
            assert_eq!(ext.version, Some("1.0".to_string()));
        }
        _ => panic!("Expected Xcstrings format extension"),
    }
}

#[test]
fn parse_simple_entries() {
    let resource = parse("simple.xcstrings");
    assert_eq!(resource.entries.len(), 3);

    // greeting
    let greeting = resource.entries.get("greeting").expect("missing greeting");
    assert_eq!(greeting.key, "greeting");
    match &greeting.value {
        EntryValue::Simple(v) => assert_eq!(v, "Hello"),
        _ => panic!("Expected Simple value for greeting"),
    }
    assert_eq!(greeting.state, Some(TranslationState::Translated));

    // farewell has a comment
    let farewell = resource.entries.get("farewell").expect("missing farewell");
    assert_eq!(farewell.comments.len(), 1);
    assert_eq!(farewell.comments[0].text, "Said when leaving");
    assert_eq!(farewell.comments[0].role, CommentRole::General);

    // app_name has extractionState
    let app_name = resource.entries.get("app_name").expect("missing app_name");
    match &app_name.format_ext {
        Some(FormatExtension::Xcstrings(ext)) => {
            assert_eq!(ext.extraction_state, Some("manual".to_string()));
        }
        _ => panic!("Expected Xcstrings format extension for app_name"),
    }
}

#[test]
fn parse_simple_stores_other_locales() {
    let resource = parse("simple.xcstrings");
    let greeting = resource.entries.get("greeting").expect("missing greeting");
    // The Japanese localization should be stored in properties
    assert!(
        greeting.properties.contains_key("xcstrings.localization.ja"),
        "Expected ja localization in properties"
    );
}

#[test]
fn roundtrip_simple() {
    let original = parse("simple.xcstrings");
    let rt = round_trip("simple.xcstrings");

    assert_eq!(original.entries.len(), rt.entries.len());
    assert_eq!(rt.metadata.locale, Some("en".to_string()));

    // Key values preserved
    for key in original.entries.keys() {
        let orig = &original.entries[key];
        let rted = rt.entries.get(key).expect("missing key in round-trip");
        assert_eq!(orig.value, rted.value, "value mismatch for key {}", key);
        assert_eq!(
            orig.comments.len(),
            rted.comments.len(),
            "comment count mismatch for key {}",
            key
        );
    }
}

#[test]
fn write_simple_produces_valid_json() {
    let resource = parse("simple.xcstrings");
    let json = write_to_json(&resource);

    assert_eq!(json["sourceLanguage"], "en");
    assert_eq!(json["version"], "1.0");
    assert!(json["strings"]["greeting"].is_object());
    assert_eq!(
        json["strings"]["greeting"]["localizations"]["en"]["stringUnit"]["value"],
        "Hello"
    );
}

// ─── Plural strings ───────────────────────────────────────────────────

#[test]
fn parse_plurals_en() {
    let resource = parse("plurals.xcstrings");
    let item_count = resource
        .entries
        .get("item_count")
        .expect("missing item_count");

    match &item_count.value {
        EntryValue::Plural(ps) => {
            assert_eq!(ps.one, Some("%lld item".to_string()));
            assert_eq!(ps.other, "%lld items");
            assert!(ps.zero.is_none());
            assert!(ps.two.is_none());
            assert!(ps.few.is_none());
            assert!(ps.many.is_none());
        }
        _ => panic!("Expected Plural for item_count, got {:?}", item_count.value),
    }
}

#[test]
fn parse_plurals_comment_preserved() {
    let resource = parse("plurals.xcstrings");
    let day_count = resource
        .entries
        .get("day_count")
        .expect("missing day_count");
    assert_eq!(day_count.comments.len(), 1);
    assert_eq!(day_count.comments[0].text, "Number of days remaining");
}

#[test]
fn parse_plurals_arabic_stored() {
    let resource = parse("plurals.xcstrings");
    let item_count = resource
        .entries
        .get("item_count")
        .expect("missing item_count");
    // Arabic localization should be in properties
    assert!(
        item_count
            .properties
            .contains_key("xcstrings.localization.ar"),
        "Expected ar localization in properties"
    );
}

#[test]
fn roundtrip_plurals() {
    let original = parse("plurals.xcstrings");
    let rt = round_trip("plurals.xcstrings");

    assert_eq!(original.entries.len(), rt.entries.len());

    let orig_item = &original.entries["item_count"];
    let rt_item = &rt.entries["item_count"];
    assert_eq!(orig_item.value, rt_item.value);

    // Arabic localization should survive round-trip
    assert!(
        rt_item
            .properties
            .contains_key("xcstrings.localization.ar"),
        "Arabic localization lost in round-trip"
    );
}

#[test]
fn write_plurals_structure() {
    let resource = parse("plurals.xcstrings");
    let json = write_to_json(&resource);

    let en_loc = &json["strings"]["item_count"]["localizations"]["en"];
    // Should be under variations.plural, not stringUnit
    assert!(en_loc["variations"]["plural"]["one"]["stringUnit"]["value"].is_string());
    assert_eq!(
        en_loc["variations"]["plural"]["one"]["stringUnit"]["value"],
        "%lld item"
    );
    assert_eq!(
        en_loc["variations"]["plural"]["other"]["stringUnit"]["value"],
        "%lld items"
    );
}

// ─── Device variants ──────────────────────────────────────────────────

#[test]
fn parse_device_variants() {
    let resource = parse("device_variants.xcstrings");
    let share = resource
        .entries
        .get("share_action")
        .expect("missing share_action");

    // The 'other' device should be the main value
    match &share.value {
        EntryValue::Simple(v) => assert_eq!(v, "Share"),
        _ => panic!(
            "Expected Simple value for share_action default, got {:?}",
            share.value
        ),
    }

    // Device variants should be populated
    let dv = share.device_variants.as_ref().expect("missing device_variants");
    assert!(dv.contains_key(&DeviceType::Phone));
    assert!(dv.contains_key(&DeviceType::Tablet));
    assert!(dv.contains_key(&DeviceType::Desktop));

    match dv.get(&DeviceType::Phone) {
        Some(EntryValue::Simple(v)) => assert_eq!(v, "Tap to share"),
        other => panic!("Expected Simple for Phone, got {:?}", other),
    }
    match dv.get(&DeviceType::Tablet) {
        Some(EntryValue::Simple(v)) => assert_eq!(v, "Tap or click to share"),
        other => panic!("Expected Simple for Tablet, got {:?}", other),
    }
    match dv.get(&DeviceType::Desktop) {
        Some(EntryValue::Simple(v)) => assert_eq!(v, "Click to share"),
        other => panic!("Expected Simple for Desktop, got {:?}", other),
    }
}

#[test]
fn parse_device_variants_extended() {
    let resource = parse("device_variants.xcstrings");
    let input = resource
        .entries
        .get("input_prompt")
        .expect("missing input_prompt");

    let dv = input.device_variants.as_ref().expect("missing device_variants");
    assert!(dv.contains_key(&DeviceType::Phone));
    assert!(dv.contains_key(&DeviceType::TV));
    assert!(dv.contains_key(&DeviceType::Watch));
}

#[test]
fn roundtrip_device_variants() {
    let original = parse("device_variants.xcstrings");
    let rt = round_trip("device_variants.xcstrings");

    assert_eq!(original.entries.len(), rt.entries.len());

    let orig_share = &original.entries["share_action"];
    let rt_share = &rt.entries["share_action"];

    assert_eq!(orig_share.value, rt_share.value);
    assert_eq!(
        orig_share.device_variants.as_ref().map(|dv| dv.len()),
        rt_share.device_variants.as_ref().map(|dv| dv.len()),
    );
}

#[test]
fn write_device_variants_structure() {
    let resource = parse("device_variants.xcstrings");
    let json = write_to_json(&resource);

    let en_loc = &json["strings"]["share_action"]["localizations"]["en"];
    let device = &en_loc["variations"]["device"];

    assert_eq!(device["iphone"]["stringUnit"]["value"], "Tap to share");
    assert_eq!(device["ipad"]["stringUnit"]["value"], "Tap or click to share");
    assert_eq!(device["mac"]["stringUnit"]["value"], "Click to share");
    assert_eq!(device["other"]["stringUnit"]["value"], "Share");
}

// ─── Full fixture (shouldTranslate, extractionState, state, comment, substitutions) ─

#[test]
fn parse_full_should_translate() {
    let resource = parse("full.xcstrings");
    let dnt = resource
        .entries
        .get("do_not_translate")
        .expect("missing do_not_translate");
    assert_eq!(dnt.translatable, Some(false));
}

#[test]
fn parse_full_extraction_state() {
    let resource = parse("full.xcstrings");
    let welcome = resource
        .entries
        .get("welcome_message")
        .expect("missing welcome_message");

    match &welcome.format_ext {
        Some(FormatExtension::Xcstrings(ext)) => {
            assert_eq!(
                ext.extraction_state,
                Some("extracted_with_value".to_string())
            );
        }
        _ => panic!("Expected Xcstrings extension"),
    }

    let stale = resource
        .entries
        .get("stale_entry")
        .expect("missing stale_entry");
    match &stale.format_ext {
        Some(FormatExtension::Xcstrings(ext)) => {
            assert_eq!(ext.extraction_state, Some("stale".to_string()));
        }
        _ => panic!("Expected Xcstrings extension"),
    }
}

#[test]
fn parse_full_states() {
    let resource = parse("full.xcstrings");

    // welcome_message -> translated
    let welcome = &resource.entries["welcome_message"];
    assert_eq!(welcome.state, Some(TranslationState::Translated));

    // new_untranslated -> new
    let new_entry = &resource.entries["new_untranslated"];
    assert_eq!(new_entry.state, Some(TranslationState::New));
}

#[test]
fn parse_full_comment() {
    let resource = parse("full.xcstrings");
    let welcome = &resource.entries["welcome_message"];
    assert_eq!(welcome.comments.len(), 1);
    assert_eq!(welcome.comments[0].text, "Shown on the home screen");
}

#[test]
fn parse_full_substitutions() {
    let resource = parse("full.xcstrings");
    let fif = resource
        .entries
        .get("files_in_folders")
        .expect("missing files_in_folders");

    match &fif.value {
        EntryValue::MultiVariablePlural(mvp) => {
            assert_eq!(mvp.pattern, "%#@files@ in %#@folders@");
            assert_eq!(mvp.variables.len(), 2);

            let files = mvp.variables.get("files").expect("missing 'files' variable");
            assert_eq!(files.arg_num, Some(1));
            assert_eq!(files.format_specifier, Some("lld".to_string()));
            assert_eq!(files.plural_set.one, Some("%arg file".to_string()));
            assert_eq!(files.plural_set.other, "%arg files");

            let folders = mvp
                .variables
                .get("folders")
                .expect("missing 'folders' variable");
            assert_eq!(folders.arg_num, Some(2));
            assert_eq!(folders.format_specifier, Some("lld".to_string()));
            assert_eq!(folders.plural_set.one, Some("%arg folder".to_string()));
            assert_eq!(folders.plural_set.other, "%arg folders");
        }
        _ => panic!(
            "Expected MultiVariablePlural for files_in_folders, got {:?}",
            fif.value
        ),
    }
}

#[test]
fn roundtrip_full() {
    let original = parse("full.xcstrings");
    let rt = round_trip("full.xcstrings");

    assert_eq!(original.entries.len(), rt.entries.len());

    // Check every entry's value survived
    for key in original.entries.keys() {
        let orig = &original.entries[key];
        let rted = rt.entries.get(key).expect(&format!("missing key {} in round-trip", key));
        assert_eq!(orig.value, rted.value, "value mismatch for key {}", key);
        assert_eq!(orig.state, rted.state, "state mismatch for key {}", key);
        assert_eq!(
            orig.translatable, rted.translatable,
            "translatable mismatch for key {}",
            key
        );
    }
}

#[test]
fn write_full_substitutions_structure() {
    let resource = parse("full.xcstrings");
    let json = write_to_json(&resource);

    let fif = &json["strings"]["files_in_folders"]["localizations"]["en"];
    assert_eq!(fif["stringUnit"]["value"], "%#@files@ in %#@folders@");
    assert!(fif["substitutions"]["files"].is_object());
    assert_eq!(fif["substitutions"]["files"]["argNum"], 1);
    assert_eq!(fif["substitutions"]["files"]["formatSpecifier"], "lld");
    assert_eq!(
        fif["substitutions"]["files"]["variations"]["plural"]["one"]["stringUnit"]["value"],
        "%arg file"
    );
}

#[test]
fn write_full_should_translate() {
    let resource = parse("full.xcstrings");
    let json = write_to_json(&resource);

    assert_eq!(json["strings"]["do_not_translate"]["shouldTranslate"], false);
    // Entries without shouldTranslate=false should not have the field
    assert!(json["strings"]["welcome_message"]["shouldTranslate"].is_null());
}

#[test]
fn write_full_extraction_state() {
    let resource = parse("full.xcstrings");
    let json = write_to_json(&resource);

    assert_eq!(
        json["strings"]["welcome_message"]["extractionState"],
        "extracted_with_value"
    );
    assert_eq!(
        json["strings"]["stale_entry"]["extractionState"],
        "stale"
    );
}

// ─── Construct from scratch and write ─────────────────────────────────

#[test]
fn construct_and_write_simple() {
    let mut entries = IndexMap::new();
    entries.insert(
        "hello".to_string(),
        I18nEntry {
            key: "hello".to_string(),
            value: EntryValue::Simple("Hello World".to_string()),
            state: Some(TranslationState::Translated),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Xcstrings,
            locale: Some("en".to_string()),
            source_locale: Some("en".to_string()),
            format_ext: Some(FormatExtension::Xcstrings(XcstringsExt {
                extraction_state: None,
                version: Some("1.0".to_string()),
            })),
            ..Default::default()
        },
        entries,
    };

    let json = write_to_json(&resource);
    assert_eq!(json["sourceLanguage"], "en");
    assert_eq!(json["version"], "1.0");
    assert_eq!(
        json["strings"]["hello"]["localizations"]["en"]["stringUnit"]["value"],
        "Hello World"
    );
    assert_eq!(
        json["strings"]["hello"]["localizations"]["en"]["stringUnit"]["state"],
        "translated"
    );
}

#[test]
fn construct_and_write_plural() {
    let mut entries = IndexMap::new();
    entries.insert(
        "items".to_string(),
        I18nEntry {
            key: "items".to_string(),
            value: EntryValue::Plural(PluralSet {
                one: Some("%lld item".to_string()),
                other: "%lld items".to_string(),
                ..Default::default()
            }),
            state: Some(TranslationState::Translated),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Xcstrings,
            locale: Some("en".to_string()),
            source_locale: Some("en".to_string()),
            format_ext: Some(FormatExtension::Xcstrings(XcstringsExt {
                extraction_state: None,
                version: Some("1.0".to_string()),
            })),
            ..Default::default()
        },
        entries,
    };

    let json = write_to_json(&resource);
    let en = &json["strings"]["items"]["localizations"]["en"];
    assert_eq!(
        en["variations"]["plural"]["one"]["stringUnit"]["value"],
        "%lld item"
    );
    assert_eq!(
        en["variations"]["plural"]["other"]["stringUnit"]["value"],
        "%lld items"
    );
    // Should not have a top-level stringUnit
    assert!(en["stringUnit"].is_null());
}

#[test]
fn construct_and_write_device_variants() {
    let mut dv = IndexMap::new();
    dv.insert(DeviceType::Phone, EntryValue::Simple("Tap".to_string()));
    dv.insert(DeviceType::Desktop, EntryValue::Simple("Click".to_string()));

    let mut entries = IndexMap::new();
    entries.insert(
        "action".to_string(),
        I18nEntry {
            key: "action".to_string(),
            value: EntryValue::Simple("Interact".to_string()),
            device_variants: Some(dv),
            state: Some(TranslationState::Translated),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Xcstrings,
            locale: Some("en".to_string()),
            source_locale: Some("en".to_string()),
            format_ext: Some(FormatExtension::Xcstrings(XcstringsExt {
                extraction_state: None,
                version: Some("1.0".to_string()),
            })),
            ..Default::default()
        },
        entries,
    };

    let json = write_to_json(&resource);
    let device = &json["strings"]["action"]["localizations"]["en"]["variations"]["device"];
    assert_eq!(device["iphone"]["stringUnit"]["value"], "Tap");
    assert_eq!(device["mac"]["stringUnit"]["value"], "Click");
    assert_eq!(device["other"]["stringUnit"]["value"], "Interact");
}

// ─── Capabilities ─────────────────────────────────────────────────────

#[test]
fn capabilities_correct() {
    let caps = Parser.capabilities();
    assert!(caps.plurals);
    assert!(caps.comments);
    assert!(caps.translatable_flag);
    assert!(caps.translation_state);
    assert!(caps.device_variants);
    assert!(!caps.arrays);
    assert!(!caps.context);
    assert!(!caps.source_string);
    assert!(!caps.nested_keys);
    assert!(!caps.inline_markup);
    assert!(!caps.select_gender);
    assert!(!caps.alternatives);
    assert!(!caps.source_references);
    assert!(!caps.custom_properties);

    // Writer capabilities should match parser
    let writer_caps = Writer.capabilities();
    assert_eq!(caps.plurals, writer_caps.plurals);
    assert_eq!(caps.device_variants, writer_caps.device_variants);
}
