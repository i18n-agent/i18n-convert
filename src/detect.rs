use crate::formats::{Confidence, FormatRegistry};
use std::path::Path;

/// Auto-detect the format of an input file by inspecting its extension and content.
/// Returns a list of (format_id, confidence) pairs sorted by confidence descending.
pub fn detect_format<'a>(
    registry: &'a FormatRegistry,
    path: &Path,
    content: &[u8],
) -> Vec<(&'a str, Confidence)> {
    let ext = path
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    registry.detect(&ext, content)
}

/// Get the best-matching format for a file, if any.
/// Returns None if no format could be detected.
pub fn detect_best<'a>(
    registry: &'a FormatRegistry,
    path: &Path,
    content: &[u8],
) -> Option<&'a str> {
    let results = detect_format(registry, path, content);
    results.into_iter().next().map(|(id, _)| id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_android_xml_file() {
        let registry = FormatRegistry::new();
        let path = Path::new("strings.xml");
        let content = br#"<?xml version="1.0"?><resources><string name="a">b</string></resources>"#;
        let result = detect_best(&registry, path, content);
        assert_eq!(result, Some("android-xml"));
    }

    #[test]
    fn detect_xliff_file() {
        let registry = FormatRegistry::new();
        let path = Path::new("translations.xliff");
        let content = br#"<?xml version="1.0"?><xliff version="1.2"><file></file></xliff>"#;
        let result = detect_best(&registry, path, content);
        assert_eq!(result, Some("xliff"));
    }

    #[test]
    fn detect_arb_file() {
        let registry = FormatRegistry::new();
        let path = Path::new("app_en.arb");
        let content = br#"{"@@locale": "en", "greeting": "Hello"}"#;
        let result = detect_best(&registry, path, content);
        assert_eq!(result, Some("arb"));
    }

    #[test]
    fn detect_po_file() {
        let registry = FormatRegistry::new();
        let path = Path::new("messages.po");
        let content = b"msgid \"\"\nmsgstr \"\"\n\nmsgid \"hello\"\nmsgstr \"world\"\n";
        let result = detect_best(&registry, path, content);
        assert_eq!(result, Some("po"));
    }

    #[test]
    fn detect_unknown_file() {
        let registry = FormatRegistry::new();
        let path = Path::new("unknown.xyz");
        let result = detect_best(&registry, path, b"random content");
        assert_eq!(result, None);
    }

    #[test]
    fn detect_xcstrings_file() {
        let registry = FormatRegistry::new();
        let path = Path::new("Localizable.xcstrings");
        let content = br#"{"sourceLanguage": "en", "strings": {}}"#;
        let result = detect_best(&registry, path, content);
        assert_eq!(result, Some("xcstrings"));
    }

    #[test]
    fn detect_ios_strings_file() {
        let registry = FormatRegistry::new();
        let path = Path::new("Localizable.strings");
        let content = br#""key" = "value";"#;
        let result = detect_best(&registry, path, content);
        assert_eq!(result, Some("ios-strings"));
    }

    #[test]
    fn detect_stringsdict_file() {
        let registry = FormatRegistry::new();
        let path = Path::new("Localizable.stringsdict");
        let content = b"<?xml version=\"1.0\"?><plist></plist>";
        let result = detect_best(&registry, path, content);
        assert_eq!(result, Some("stringsdict"));
    }

    #[test]
    fn detect_yaml_rails_file() {
        let registry = FormatRegistry::new();
        let path = Path::new("en.yml");
        let content = b"en:\n  greeting: Hello\n";
        let result = detect_best(&registry, path, content);
        assert_eq!(result, Some("yaml-rails"));
    }

    #[test]
    fn detect_xml_with_xliff_content_not_android() {
        let registry = FormatRegistry::new();
        let path = Path::new("file.xml");
        let content = br#"<?xml version="1.0"?><xliff version="1.2"><file></file></xliff>"#;
        let results = detect_format(&registry, path, content);
        // XLIFF should rank higher than android-xml for XML with <xliff> content
        assert_eq!(results[0].0, "xliff");
        assert_eq!(results[0].1, Confidence::Definite);
    }

    #[test]
    fn detect_plain_json_as_high() {
        let registry = FormatRegistry::new();
        let path = Path::new("en.json");
        let content = br#"{"greeting": "Hello", "farewell": "Goodbye"}"#;
        let result = detect_best(&registry, path, content);
        assert_eq!(result, Some("json"));
        let results = detect_format(&registry, path, content);
        assert_eq!(results[0].1, Confidence::High);
    }

    #[test]
    fn detect_i18next_json_definite_with_plural_pairs() {
        let registry = FormatRegistry::new();
        let path = Path::new("en.json");
        let content = br#"{"item_one": "{{count}} item", "item_other": "{{count}} items", "dog_one": "{{count}} dog", "dog_other": "{{count}} dogs"}"#;
        let result = detect_best(&registry, path, content);
        assert_eq!(result, Some("i18next"));
        let results = detect_format(&registry, path, content);
        assert_eq!(results[0].1, Confidence::Definite);
    }

    #[test]
    fn detect_i18next_json_high_with_single_pair() {
        let registry = FormatRegistry::new();
        let path = Path::new("en.json");
        let content = br#"{"item_one": "{{count}} item", "item_other": "{{count}} items", "greeting": "Hello"}"#;
        let result = detect_best(&registry, path, content);
        assert_eq!(result, Some("i18next"));
        let results = detect_format(&registry, path, content);
        assert_eq!(results[0].1, Confidence::High);
    }

    #[test]
    fn detect_json_not_confused_by_other_suffix_in_values() {
        let registry = FormatRegistry::new();
        let path = Path::new("en.json");
        // Values contain "_one" and "_other" but keys don't have plural pairs
        let content = br#"{"message": "buy_one get_other free"}"#;
        let result = detect_best(&registry, path, content);
        assert_eq!(result, Some("json"));
    }

    #[test]
    fn detect_valid_json_not_hjson() {
        let registry = FormatRegistry::new();
        let path = Path::new("en.json");
        // Valid JSON with "# " in a value should NOT match as HJSON
        let content = br#"{"comment": "see section # 3 for details"}"#;
        let results = detect_format(&registry, path, content);
        for (id, _) in &results {
            assert_ne!(*id, "hjson");
        }
    }

    #[test]
    fn detect_i18next_nested_plural_pairs() {
        let registry = FormatRegistry::new();
        let path = Path::new("translation.json");
        let content = br#"{"common": {"item_one": "{{count}} item", "item_other": "{{count}} items"}, "nav": {"home": "Home"}}"#;
        let result = detect_best(&registry, path, content);
        assert_eq!(result, Some("i18next"));
    }
}
