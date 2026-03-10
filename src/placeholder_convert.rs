/// Convert placeholder syntax between formats.
///
/// Each format has its own placeholder style:
///   Android:   %1$s, %2$d
///   iOS:       %@, %1$@
///   ARB/ICU:   {name}
///   i18next:   {{name}}
///   Laravel:   :name
///   PO:        %(name)s
///   YAML:      %{name}

use std::sync::LazyLock;
use regex::Regex;

static RE_ANDROID_POSITIONAL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"%(\d+)\$[sdifegacoxX@]").expect("valid regex pattern")
});

static RE_ANDROID_SIMPLE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"%([sdifegacoxX@])").expect("valid regex pattern")
});

static RE_ICU_POSITIONAL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\{(\d+)\}").expect("valid regex pattern")
});

static RE_ICU_NAMED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\{(\w+)\}").expect("valid regex pattern")
});

static RE_I18NEXT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\{\{(\w+)\}\}").expect("valid regex pattern")
});

static RE_ICU_ALPHA: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\{([a-zA-Z_]\w*)\}").expect("valid regex pattern")
});

static RE_YAML_RAILS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"%\{(\w+)\}").expect("valid regex pattern")
});

/// Convert Android positional placeholders to ICU-style.
/// %1$s -> {0}, %2$d -> {1}, %s -> {0}
pub fn android_to_icu(s: &str) -> String {
    let result = RE_ANDROID_POSITIONAL
        .replace_all(s, |caps: &regex::Captures| {
            let pos: usize = caps[1].parse::<usize>().expect("regex guarantees digits") - 1;
            format!("{{{pos}}}")
        })
        .to_string();

    // Handle non-positional %s, %d etc. (map to {0})
    RE_ANDROID_SIMPLE.replace_all(&result, "{0}").to_string()
}

/// Convert ICU-style positional placeholders to Android.
/// {0} -> %1$s, {1} -> %2$s
pub fn icu_to_android(s: &str) -> String {
    RE_ICU_POSITIONAL
        .replace_all(s, |caps: &regex::Captures| {
            let pos: usize = caps[1].parse::<usize>().expect("regex guarantees digits") + 1;
            format!("%{pos}$s")
        })
        .to_string()
}

/// Convert ICU-style placeholders to i18next.
/// {name} -> {{name}}, {0} -> {{0}}
pub fn icu_to_i18next(s: &str) -> String {
    RE_ICU_NAMED.replace_all(s, "{{$1}}").to_string()
}

/// Convert i18next interpolation to ICU-style.
/// {{name}} -> {name}
pub fn i18next_to_icu(s: &str) -> String {
    RE_I18NEXT.replace_all(s, "{$1}").to_string()
}

/// Convert ICU-style named placeholders to YAML Rails.
/// {name} -> %{name}
pub fn icu_to_yaml_rails(s: &str) -> String {
    RE_ICU_ALPHA.replace_all(s, "%{$1}").to_string()
}

/// Convert YAML Rails interpolation to ICU-style.
/// %{name} -> {name}
pub fn yaml_rails_to_icu(s: &str) -> String {
    RE_YAML_RAILS.replace_all(s, "{$1}").to_string()
}

/// Convert Android positional placeholders to i18next.
/// %1$s -> {{0}}, %2$d -> {{1}}
pub fn android_to_i18next(s: &str) -> String {
    icu_to_i18next(&android_to_icu(s))
}

/// Convert i18next interpolation to Android positional.
/// {{name}} -> %1$s (named placeholders become positional)
pub fn i18next_to_android(s: &str) -> String {
    icu_to_android(&i18next_to_icu(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_android_to_icu_positional() {
        assert_eq!(android_to_icu("%1$s"), "{0}");
        assert_eq!(android_to_icu("%2$d"), "{1}");
        assert_eq!(android_to_icu("Hello %1$s, you have %2$d items"), "Hello {0}, you have {1} items");
    }

    #[test]
    fn test_android_to_icu_simple() {
        assert_eq!(android_to_icu("%s"), "{0}");
        assert_eq!(android_to_icu("%d"), "{0}");
    }

    #[test]
    fn test_icu_to_android() {
        assert_eq!(icu_to_android("{0}"), "%1$s");
        assert_eq!(icu_to_android("{1}"), "%2$s");
        assert_eq!(icu_to_android("Hello {0}, you have {1} items"), "Hello %1$s, you have %2$s items");
    }

    #[test]
    fn test_icu_to_i18next() {
        assert_eq!(icu_to_i18next("{name}"), "{{name}}");
        assert_eq!(icu_to_i18next("Hello {name}, you have {count} items"), "Hello {{name}}, you have {{count}} items");
    }

    #[test]
    fn test_i18next_to_icu() {
        assert_eq!(i18next_to_icu("{{name}}"), "{name}");
        assert_eq!(i18next_to_icu("Hello {{name}}, you have {{count}} items"), "Hello {name}, you have {count} items");
    }

    #[test]
    fn test_icu_to_yaml_rails() {
        assert_eq!(icu_to_yaml_rails("{name}"), "%{name}");
        assert_eq!(icu_to_yaml_rails("Hello {name}"), "Hello %{name}");
    }

    #[test]
    fn test_yaml_rails_to_icu() {
        assert_eq!(yaml_rails_to_icu("%{name}"), "{name}");
        assert_eq!(yaml_rails_to_icu("Hello %{name}"), "Hello {name}");
    }

    #[test]
    fn test_android_to_i18next() {
        assert_eq!(android_to_i18next("%1$s"), "{{0}}");
    }

    #[test]
    fn test_no_placeholders_unchanged() {
        assert_eq!(android_to_icu("Hello World"), "Hello World");
        assert_eq!(icu_to_android("Hello World"), "Hello World");
        assert_eq!(icu_to_i18next("Hello World"), "Hello World");
        assert_eq!(i18next_to_icu("Hello World"), "Hello World");
    }

    #[test]
    fn test_icu_named_not_positional_in_android_conversion() {
        // Named ICU placeholders like {name} should NOT be changed by icu_to_android
        // because {name} doesn't match \{\d+\}
        assert_eq!(icu_to_android("{name}"), "{name}");
    }

    #[test]
    fn test_roundtrip_android_icu() {
        let original = "Hello %1$s, you have %2$s items";
        let icu = android_to_icu(original);
        assert_eq!(icu, "Hello {0}, you have {1} items");
        let back = icu_to_android(&icu);
        assert_eq!(back, "Hello %1$s, you have %2$s items");
    }

    #[test]
    fn test_roundtrip_icu_i18next() {
        let original = "Hello {name}";
        let i18next = icu_to_i18next(original);
        assert_eq!(i18next, "Hello {{name}}");
        let back = i18next_to_icu(&i18next);
        assert_eq!(back, "Hello {name}");
    }

    #[test]
    fn test_roundtrip_icu_yaml_rails() {
        let original = "Hello {name}";
        let rails = icu_to_yaml_rails(original);
        assert_eq!(rails, "Hello %{name}");
        let back = yaml_rails_to_icu(&rails);
        assert_eq!(back, "Hello {name}");
    }
}
