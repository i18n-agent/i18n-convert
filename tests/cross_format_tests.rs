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
        "i18next output should contain plural-suffixed keys, got: {output_str}"
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
            "Key '{key}' should survive ARB -> JSON"
        );
        // Compare just the simple string value
        if let EntryValue::Simple(original) = &entry.value {
            if let EntryValue::Simple(converted) = &reparsed.entries[key].value {
                assert_eq!(original, converted, "Value for '{key}' should survive");
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
                "Key '{key}' should survive PO -> JSON"
            );
            assert_eq!(
                entry.value, reparsed.entries[key].value,
                "Value for key '{key}' should match"
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
                "Key '{key}' should survive XLIFF -> ARB"
            );
            if let EntryValue::Simple(converted) = &reparsed.entries[key].value {
                assert_eq!(original, converted, "Value for '{key}' should survive");
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
                "Key '{key}' should survive iOS strings -> Android XML"
            );
            if let EntryValue::Simple(converted) = &reparsed.entries[key].value {
                assert_eq!(original, converted, "Value for '{key}' should survive");
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
                "Key '{key}' should survive YAML -> JSON"
            );
            if let EntryValue::Simple(converted) = &reparsed.entries[key].value {
                assert_eq!(original, converted, "Value for '{key}' should survive");
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

// ── 10. Markdown → JSON (key conflicts resolved, no panic) ──────────────

#[test]
fn markdown_to_json_no_panic_on_key_conflicts() {
    let input = std::fs::read("tests/fixtures/markdown/simple.md").unwrap();
    let resource = markdown::Parser.parse(&input).unwrap();
    // This used to panic with "Expected nested object in key path"
    let output = json_structured::Writer.write(&resource).unwrap();
    let output_str = String::from_utf8(output).unwrap();

    assert!(output_str.contains("Hello and welcome"));
    assert!(output_str.contains("getting-started"));
    assert!(output_str.contains("reset"));
}

// ── 11. Markdown → TOML (key conflicts resolved, no error) ─────────────

#[test]
fn markdown_to_toml_no_error_on_key_conflicts() {
    let input = std::fs::read("tests/fixtures/markdown/simple.md").unwrap();
    let resource = markdown::Parser.parse(&input).unwrap();
    // This used to error with "Key path conflict"
    let output = toml_format::Writer.write(&resource).unwrap();
    let output_str = String::from_utf8(output).unwrap();

    assert!(output_str.contains("Hello and welcome"));
    assert!(output_str.contains("getting-started"));
}

// ── 12. Markdown → YAML (key conflicts resolved, no silent data loss) ──

#[test]
fn markdown_to_yaml_no_silent_data_loss() {
    let input = std::fs::read("tests/fixtures/markdown/simple.md").unwrap();
    let resource = markdown::Parser.parse(&input).unwrap();
    let entry_count = resource.entries.len();
    // This used to silently drop entries
    let output = yaml_plain::Writer.write(&resource).unwrap();
    let reparsed = yaml_plain::Parser.parse(&output).unwrap();

    assert!(
        reparsed.entries.len() >= entry_count,
        "Expected at least {entry_count} entries after YAML roundtrip, got {}",
        reparsed.entries.len()
    );
}

// ── 13. Markdown → Plain Text (all entries output, not just first) ──────

#[test]
fn markdown_to_plain_text_all_entries_output() {
    let input = std::fs::read("tests/fixtures/markdown/simple.md").unwrap();
    let resource = markdown::Parser.parse(&input).unwrap();
    // This used to only output the first entry
    let output = plain_text::Writer.write(&resource).unwrap();
    let output_str = String::from_utf8(output).unwrap();

    assert!(output_str.contains("Hello and welcome"));
    assert!(
        output_str.contains("reset") || output_str.contains("Reset"),
        "Should contain FAQ content, not just first section"
    );
}

// ── 14. INI writer: root-level keys appear before sections ──────────────

#[test]
fn ini_writer_root_keys_before_sections() {
    let mut entries = indexmap::IndexMap::new();

    entries.insert(
        "section.nested_key".to_string(),
        I18nEntry {
            key: "section.nested_key".to_string(),
            value: EntryValue::Simple("nested value".to_string()),
            format_ext: Some(FormatExtension::Ini(IniExt {
                section: Some("section".to_string()),
                delimiter: Some('='),
                comment_char: None,
            })),
            ..Default::default()
        },
    );

    entries.insert(
        "root_key".to_string(),
        I18nEntry {
            key: "root_key".to_string(),
            value: EntryValue::Simple("root value".to_string()),
            format_ext: Some(FormatExtension::Ini(IniExt {
                section: None,
                delimiter: Some('='),
                comment_char: None,
            })),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Ini,
            format_ext: Some(FormatExtension::Ini(IniExt {
                section: None,
                delimiter: Some('='),
                comment_char: None,
            })),
            ..Default::default()
        },
        entries,
    };

    let output = ini::Writer.write(&resource).unwrap();
    let output_str = String::from_utf8(output).unwrap();

    let root_pos = output_str
        .find("root_key")
        .expect("root_key should be in output");
    let section_pos = output_str
        .find("[section]")
        .expect("[section] should be in output");
    assert!(
        root_pos < section_pos,
        "Root keys should appear before section headers. Output:\n{output_str}"
    );

    let reparsed = ini::Parser.parse(output_str.as_bytes()).unwrap();
    assert_eq!(
        reparsed.entries["root_key"].value,
        EntryValue::Simple("root value".to_string()),
    );
}

// ── 15. JSON writer: key conflict promotion to _content ─────────────────

#[test]
fn json_writer_handles_leaf_and_branch_conflict() {
    let mut entries = indexmap::IndexMap::new();
    entries.insert(
        "parent".to_string(),
        I18nEntry {
            key: "parent".to_string(),
            value: EntryValue::Simple("parent value".to_string()),
            ..Default::default()
        },
    );
    entries.insert(
        "parent.child".to_string(),
        I18nEntry {
            key: "parent.child".to_string(),
            value: EntryValue::Simple("child value".to_string()),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::JsonStructured,
            ..Default::default()
        },
        entries,
    };

    let output = json_structured::Writer.write(&resource).unwrap();
    let output_str = String::from_utf8(output).unwrap();

    assert!(output_str.contains("parent value"));
    assert!(output_str.contains("child value"));
}

// ── 16. TOML writer: key conflict promotion to _content ─────────────────

#[test]
fn toml_writer_handles_leaf_and_branch_conflict() {
    let mut entries = indexmap::IndexMap::new();
    entries.insert(
        "parent".to_string(),
        I18nEntry {
            key: "parent".to_string(),
            value: EntryValue::Simple("parent value".to_string()),
            ..Default::default()
        },
    );
    entries.insert(
        "parent.child".to_string(),
        I18nEntry {
            key: "parent.child".to_string(),
            value: EntryValue::Simple("child value".to_string()),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Toml,
            ..Default::default()
        },
        entries,
    };

    let output = toml_format::Writer.write(&resource).unwrap();
    let output_str = String::from_utf8(output).unwrap();

    assert!(output_str.contains("parent value"));
    assert!(output_str.contains("child value"));
}

// ── 17. YAML writer: key conflict promotion to _content ─────────────────

#[test]
fn yaml_writer_handles_leaf_and_branch_conflict() {
    let mut entries = indexmap::IndexMap::new();
    entries.insert(
        "parent".to_string(),
        I18nEntry {
            key: "parent".to_string(),
            value: EntryValue::Simple("parent value".to_string()),
            ..Default::default()
        },
    );
    entries.insert(
        "parent.child".to_string(),
        I18nEntry {
            key: "parent.child".to_string(),
            value: EntryValue::Simple("child value".to_string()),
            ..Default::default()
        },
    );

    let resource = I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::YamlPlain,
            ..Default::default()
        },
        entries,
    };

    let output = yaml_plain::Writer.write(&resource).unwrap();
    let output_str = String::from_utf8(output).unwrap();

    assert!(output_str.contains("parent value"));
    assert!(output_str.contains("child value"));
}
