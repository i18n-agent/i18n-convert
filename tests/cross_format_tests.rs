use i18n_convert::convert::check_data_loss;
use i18n_convert::formats::*;
use i18n_convert::ir::*;

// ── 1. Android XML -> JSON (simple strings survive) ────────────────────────

#[test]
fn android_xml_to_json_simple_strings_survive() {
    let input = std::fs::read("tests/fixtures/android_xml/simple.xml").unwrap();
    let resource = android_xml::Parser.parse(&input).unwrap();
    let output = json_structured::Writer.write(&resource).unwrap();
    let reparsed = json_structured::Parser.parse(&output).unwrap();

    // Simple string values should survive the round-trip
    assert_eq!(
        resource.entries["app_name"].value,
        reparsed.entries["app_name"].value
    );
    assert_eq!(
        resource.entries["greeting"].value,
        reparsed.entries["greeting"].value
    );
    assert_eq!(
        resource.entries["untranslatable"].value,
        reparsed.entries["untranslatable"].value
    );
}

// ── 2. Android XML -> ARB (plurals convert via ICU) ────────────────────────

#[test]
fn android_xml_to_arb_plurals_convert() {
    let input = std::fs::read("tests/fixtures/android_xml/plurals.xml").unwrap();
    let resource = android_xml::Parser.parse(&input).unwrap();

    // ARB supports plurals, so no data loss warning for plurals
    let warnings = check_data_loss(&resource, &arb::Writer.capabilities());
    assert!(
        !warnings.iter().any(|w| w.lost_attribute == "plurals"),
        "ARB supports plurals, should not warn about plural loss"
    );

    // Write to ARB and verify we can parse back
    let output = arb::Writer.write(&resource).unwrap();
    let reparsed = arb::Parser.parse(&output).unwrap();

    // The plural entry should still be present (stored as ICU in ARB)
    assert!(
        reparsed.entries.contains_key("items"),
        "Plural key 'items' should survive in ARB output"
    );
}

// ── 3. Android XML -> i18next (plurals convert to suffixed keys) ───────────

#[test]
fn android_xml_to_i18next_plurals_convert_to_suffixed_keys() {
    let input = std::fs::read("tests/fixtures/android_xml/plurals.xml").unwrap();
    let resource = android_xml::Parser.parse(&input).unwrap();

    // i18next supports plurals, so no data loss warning for plurals
    let warnings = check_data_loss(&resource, &i18next::Writer.capabilities());
    assert!(
        !warnings.iter().any(|w| w.lost_attribute == "plurals"),
        "i18next supports plurals, should not warn about plural loss"
    );

    // Write to i18next
    let output = i18next::Writer.write(&resource).unwrap();
    let output_str = String::from_utf8(output.clone()).unwrap();

    // i18next should create suffixed keys like items_one, items_other
    assert!(
        output_str.contains("items_one") || output_str.contains("items_other"),
        "i18next output should contain plural-suffixed keys, got: {}",
        output_str
    );

    // Parse back and verify plural structure
    let reparsed = i18next::Parser.parse(&output).unwrap();
    assert!(
        reparsed.entries.contains_key("items"),
        "Plural key 'items' should be regrouped in i18next"
    );
    assert!(
        matches!(reparsed.entries["items"].value, EntryValue::Plural(_)),
        "items should be parsed back as Plural"
    );
}

// ── 4. ARB -> JSON (metadata stripped, values survive) ─────────────────────

#[test]
fn arb_to_json_metadata_stripped_values_survive() {
    let input = std::fs::read("tests/fixtures/arb/metadata.arb").unwrap();
    let resource = arb::Parser.parse(&input).unwrap();
    let output = json_structured::Writer.write(&resource).unwrap();
    let reparsed = json_structured::Parser.parse(&output).unwrap();

    // Values should survive
    for (key, entry) in &resource.entries {
        assert!(
            reparsed.entries.contains_key(key),
            "Key '{}' should survive ARB -> JSON",
            key
        );
        // Compare just the simple string value
        if let EntryValue::Simple(original) = &entry.value {
            if let EntryValue::Simple(converted) = &reparsed.entries[key].value {
                assert_eq!(
                    original, converted,
                    "Value for '{}' should survive",
                    key
                );
            }
        }
    }
}

#[test]
fn arb_to_json_warns_about_comments() {
    let input = std::fs::read("tests/fixtures/arb/metadata.arb").unwrap();
    let resource = arb::Parser.parse(&input).unwrap();

    // JSON doesn't support comments; check if any entries have comments
    let has_comments = resource.entries.values().any(|e| !e.comments.is_empty());
    if has_comments {
        let warnings = check_data_loss(&resource, &json_structured::Writer.capabilities());
        assert!(
            warnings.iter().any(|w| w.lost_attribute == "comments"),
            "Should warn about comment loss when converting ARB with metadata to JSON"
        );
    }
}

// ── 5. PO -> JSON (comments warned, values survive) ────────────────────────

#[test]
fn po_to_json_values_survive() {
    let input = std::fs::read("tests/fixtures/po/simple.po").unwrap();
    let resource = po::Parser.parse(&input).unwrap();
    let output = json_structured::Writer.write(&resource).unwrap();
    let reparsed = json_structured::Parser.parse(&output).unwrap();

    // Simple string values should survive
    for (key, entry) in &resource.entries {
        if let EntryValue::Simple(_) = &entry.value {
            assert!(
                reparsed.entries.contains_key(key),
                "Key '{}' should survive PO -> JSON",
                key
            );
            assert_eq!(
                entry.value,
                reparsed.entries[key].value,
                "Value for key '{}' should match",
                key
            );
        }
    }
}

#[test]
fn po_to_json_warns_about_comments() {
    let input = std::fs::read("tests/fixtures/po/comments.po").unwrap();
    let resource = po::Parser.parse(&input).unwrap();
    let warnings = check_data_loss(&resource, &json_structured::Writer.capabilities());

    assert!(
        warnings.iter().any(|w| w.lost_attribute == "comments"),
        "Converting PO with comments to JSON should warn about comment loss"
    );
}

#[test]
fn po_to_json_warns_about_source_strings() {
    let input = std::fs::read("tests/fixtures/po/comments.po").unwrap();
    let resource = po::Parser.parse(&input).unwrap();

    // PO entries can have source references
    let has_sources = resource
        .entries
        .values()
        .any(|e| !e.source_references.is_empty());
    if has_sources {
        let warnings = check_data_loss(&resource, &json_structured::Writer.capabilities());
        assert!(
            warnings
                .iter()
                .any(|w| w.lost_attribute == "source references"),
            "Converting PO with source references to JSON should warn"
        );
    }
}

// ── 6. XLIFF -> ARB (source string warned, target values survive) ──────────

#[test]
fn xliff_to_arb_target_values_survive() {
    let input = std::fs::read("tests/fixtures/xliff1/simple.xliff").unwrap();
    let resource = xliff1::Parser.parse(&input).unwrap();
    let output = arb::Writer.write(&resource).unwrap();
    let reparsed = arb::Parser.parse(&output).unwrap();

    // Target values (which become entry.value) should survive
    for (key, entry) in &resource.entries {
        if let EntryValue::Simple(original) = &entry.value {
            if original.is_empty() {
                continue; // Skip entries with empty target
            }
            assert!(
                reparsed.entries.contains_key(key),
                "Key '{}' should survive XLIFF -> ARB",
                key
            );
            if let EntryValue::Simple(converted) = &reparsed.entries[key].value {
                assert_eq!(
                    original, converted,
                    "Value for '{}' should survive",
                    key
                );
            }
        }
    }
}

#[test]
fn xliff_to_arb_warns_about_source_strings() {
    let input = std::fs::read("tests/fixtures/xliff1/simple.xliff").unwrap();
    let resource = xliff1::Parser.parse(&input).unwrap();

    // XLIFF entries have source strings; ARB doesn't natively support source_string
    let has_source = resource.entries.values().any(|e| e.source.is_some());
    assert!(has_source, "XLIFF entries should have source strings");

    let warnings = check_data_loss(&resource, &arb::Writer.capabilities());
    assert!(
        warnings
            .iter()
            .any(|w| w.lost_attribute == "source strings"),
        "Converting XLIFF to ARB should warn about source string loss"
    );
}

// ── 7. iOS strings -> Android XML (simple strings survive) ─────────────────

#[test]
fn ios_strings_to_android_xml_simple_strings_survive() {
    let input = std::fs::read("tests/fixtures/ios_strings/simple.strings").unwrap();
    let resource = ios_strings::Parser.parse(&input).unwrap();
    let output = android_xml::Writer.write(&resource).unwrap();
    let reparsed = android_xml::Parser.parse(&output).unwrap();

    // Simple string values should survive
    for (key, entry) in &resource.entries {
        if let EntryValue::Simple(original) = &entry.value {
            assert!(
                reparsed.entries.contains_key(key),
                "Key '{}' should survive iOS strings -> Android XML",
                key
            );
            if let EntryValue::Simple(converted) = &reparsed.entries[key].value {
                assert_eq!(
                    original, converted,
                    "Value for '{}' should survive",
                    key
                );
            }
        }
    }
}

// ── 8. YAML Rails -> JSON (nested keys flatten/unflatten) ──────────────────

#[test]
fn yaml_rails_to_json_nested_keys_survive() {
    let input = std::fs::read("tests/fixtures/yaml_rails/nested.yml").unwrap();
    let resource = yaml_rails::Parser.parse(&input).unwrap();
    let output = json_structured::Writer.write(&resource).unwrap();
    let reparsed = json_structured::Parser.parse(&output).unwrap();

    // All entries from YAML should survive in JSON (both support nested keys)
    for (key, entry) in &resource.entries {
        if let EntryValue::Simple(original) = &entry.value {
            assert!(
                reparsed.entries.contains_key(key),
                "Key '{}' should survive YAML -> JSON",
                key
            );
            if let EntryValue::Simple(converted) = &reparsed.entries[key].value {
                assert_eq!(
                    original, converted,
                    "Value for '{}' should survive",
                    key
                );
            }
        }
    }
}

// ── 9. Stringsdict -> xcstrings (multi-variable plurals survive) ───────────

#[test]
fn stringsdict_to_xcstrings_multi_variable_plurals_survive() {
    let input = std::fs::read("tests/fixtures/stringsdict/multi_var.stringsdict").unwrap();
    let resource = stringsdict::Parser.parse(&input).unwrap();

    // xcstrings supports plurals, so no data loss warning
    let warnings = check_data_loss(&resource, &xcstrings::Writer.capabilities());
    assert!(
        !warnings.iter().any(|w| w.lost_attribute == "plurals"),
        "xcstrings supports plurals, should not warn"
    );

    // Write to xcstrings and parse back
    let output = xcstrings::Writer.write(&resource).unwrap();
    let reparsed = xcstrings::Parser.parse(&output).unwrap();

    // The multi-variable plural entry should survive
    assert!(
        reparsed.entries.contains_key("files_in_folders"),
        "Multi-variable plural key should survive stringsdict -> xcstrings"
    );

    // Should still be a multi-variable plural (or at minimum a plural)
    match &reparsed.entries["files_in_folders"].value {
        EntryValue::MultiVariablePlural(mvp) => {
            assert!(
                mvp.variables.len() >= 2,
                "Should preserve multiple variables, got {}",
                mvp.variables.len()
            );
        }
        EntryValue::Plural(_) => {
            // Acceptable fallback -- at least plural data survived
        }
        other => {
            panic!(
                "Expected MultiVariablePlural or Plural, got {:?}",
                std::mem::discriminant(other)
            );
        }
    }
}
