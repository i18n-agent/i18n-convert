use i18n_convert::formats::android_xml;
use i18n_convert::formats::FormatParser;
use i18n_convert::formats::FormatWriter;
use i18n_convert::ir::*;

fn load_fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!("tests/fixtures/android_xml/{name}")).unwrap()
}

#[test]
fn parse_simple_strings() {
    let content = load_fixture("simple.xml");
    let resource = android_xml::Parser.parse(&content).unwrap();
    assert_eq!(resource.entries.len(), 3);
    assert_eq!(
        resource.entries["app_name"].value,
        EntryValue::Simple("My App".to_string())
    );
    assert_eq!(resource.entries["untranslatable"].translatable, Some(false));
    // Comment should be captured
    assert!(!resource.entries["app_name"].comments.is_empty());
}

#[test]
fn parse_plurals() {
    let content = load_fixture("plurals.xml");
    let resource = android_xml::Parser.parse(&content).unwrap();
    match &resource.entries["items"].value {
        EntryValue::Plural(ps) => {
            assert_eq!(ps.zero, Some("No items".to_string()));
            assert_eq!(ps.one, Some("%d item".to_string()));
            assert_eq!(ps.other, "%d items");
        }
        _ => panic!("Expected Plural"),
    }
}

#[test]
fn parse_arrays() {
    let content = load_fixture("arrays.xml");
    let resource = android_xml::Parser.parse(&content).unwrap();
    match &resource.entries["planets"].value {
        EntryValue::Array(arr) => {
            assert_eq!(arr.len(), 3);
            assert_eq!(arr[0], "Mercury");
        }
        _ => panic!("Expected Array"),
    }
}

#[test]
fn roundtrip_full() {
    let content = load_fixture("full.xml");
    let resource = android_xml::Parser.parse(&content).unwrap();
    let written = android_xml::Writer.write(&resource).unwrap();
    let reparsed = android_xml::Parser.parse(&written).unwrap();

    assert_eq!(resource.entries.len(), reparsed.entries.len());
    for (key, entry) in &resource.entries {
        let reparsed_entry = &reparsed.entries[key];
        assert_eq!(
            entry.value, reparsed_entry.value,
            "Value mismatch for key: {key}"
        );
        assert_eq!(
            entry.translatable, reparsed_entry.translatable,
            "Translatable mismatch for key: {key}"
        );
    }
}
