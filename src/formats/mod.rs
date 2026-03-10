use crate::ir::*;
use indexmap::IndexMap;
use thiserror::Error;

pub mod android_xml;
pub mod xcstrings;
pub mod ios_strings;
pub mod stringsdict;
pub mod arb;
pub mod json_structured;
pub mod i18next;
pub mod xliff1;
pub mod po;
pub mod yaml_rails;
pub mod xliff2;
pub mod resx;
pub mod java_properties;
pub mod php_laravel;
pub mod qt_linguist;
pub mod csv_format;
pub mod toml_format;
pub mod ini;
pub mod json5_format;
pub mod hjson;
pub mod tmx;
pub mod srt;
pub mod excel;
pub mod markdown;
pub mod ios_plist;
pub mod js_format;
pub mod ts_format;
pub mod neon;
pub mod plain_text;
pub mod yaml_plain;
pub mod ispring_xliff;
pub mod captivate_xml;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Invalid format: {0}")]
    InvalidFormat(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("XML error: {0}")]
    Xml(String),
    #[error("JSON error: {0}")]
    Json(String),
    #[error("YAML error: {0}")]
    Yaml(String),
}

#[derive(Error, Debug)]
pub enum WriteError {
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Confidence {
    None,
    Low,
    High,
    Definite,
}

pub trait FormatParser: Send + Sync {
    fn detect(&self, extension: &str, content: &[u8]) -> Confidence;
    fn parse(&self, content: &[u8]) -> Result<I18nResource, ParseError>;
    fn capabilities(&self) -> FormatCapabilities;
}

pub trait FormatWriter: Send + Sync {
    fn write(&self, resource: &I18nResource) -> Result<Vec<u8>, WriteError>;
    fn capabilities(&self) -> FormatCapabilities;
}

pub struct FormatEntry {
    pub id: &'static str,
    pub name: &'static str,
    pub extensions: &'static [&'static str],
    pub parser: Box<dyn FormatParser>,
    pub writer: Box<dyn FormatWriter>,
}

pub struct FormatRegistry {
    formats: IndexMap<&'static str, FormatEntry>,
}

impl FormatRegistry {
    pub fn new() -> Self {
        let mut formats = IndexMap::new();

        // Register all Tier 1 formats
        Self::register(
            &mut formats,
            "android-xml",
            "Android XML",
            &[".xml"],
            android_xml::Parser,
            android_xml::Writer,
        );
        Self::register(
            &mut formats,
            "xcstrings",
            "Xcode String Catalog",
            &[".xcstrings"],
            xcstrings::Parser,
            xcstrings::Writer,
        );
        Self::register(
            &mut formats,
            "ios-strings",
            "iOS Strings",
            &[".strings"],
            ios_strings::Parser,
            ios_strings::Writer,
        );
        Self::register(
            &mut formats,
            "stringsdict",
            "iOS Stringsdict",
            &[".stringsdict"],
            stringsdict::Parser,
            stringsdict::Writer,
        );
        Self::register(
            &mut formats,
            "arb",
            "Flutter ARB",
            &[".arb"],
            arb::Parser,
            arb::Writer,
        );
        Self::register(
            &mut formats,
            "json",
            "Structured JSON",
            &[".json"],
            json_structured::Parser,
            json_structured::Writer,
        );
        Self::register(
            &mut formats,
            "i18next",
            "i18next JSON",
            &[".json"],
            i18next::Parser,
            i18next::Writer,
        );
        Self::register(
            &mut formats,
            "xliff",
            "XLIFF 1.2",
            &[".xliff", ".xlf"],
            xliff1::Parser,
            xliff1::Writer,
        );
        Self::register(
            &mut formats,
            "po",
            "Gettext PO",
            &[".po", ".pot"],
            po::Parser,
            po::Writer,
        );
        Self::register(
            &mut formats,
            "yaml-rails",
            "YAML (Rails)",
            &[".yml", ".yaml"],
            yaml_rails::Parser,
            yaml_rails::Writer,
        );

        // Tier 2 formats
        Self::register(
            &mut formats,
            "xliff2",
            "XLIFF 2.0",
            &[".xliff", ".xlf"],
            xliff2::Parser,
            xliff2::Writer,
        );
        Self::register(
            &mut formats,
            "resx",
            ".NET RESX",
            &[".resx"],
            resx::Parser,
            resx::Writer,
        );
        Self::register(
            &mut formats,
            "java-properties",
            "Java Properties",
            &[".properties"],
            java_properties::Parser,
            java_properties::Writer,
        );
        Self::register(
            &mut formats,
            "php-laravel",
            "PHP/Laravel",
            &[".php"],
            php_laravel::Parser,
            php_laravel::Writer,
        );
        Self::register(
            &mut formats,
            "qt",
            "Qt Linguist",
            &[".ts"],
            qt_linguist::Parser,
            qt_linguist::Writer,
        );
        Self::register(
            &mut formats,
            "csv",
            "CSV",
            &[".csv", ".tsv"],
            csv_format::Parser,
            csv_format::Writer,
        );
        Self::register(
            &mut formats,
            "toml",
            "TOML",
            &[".toml"],
            toml_format::Parser,
            toml_format::Writer,
        );
        Self::register(
            &mut formats,
            "ini",
            "INI",
            &[".ini"],
            ini::Parser,
            ini::Writer,
        );
        Self::register(
            &mut formats,
            "json5",
            "JSON5",
            &[".json5"],
            json5_format::Parser,
            json5_format::Writer,
        );
        Self::register(
            &mut formats,
            "hjson",
            "HJSON",
            &[".hjson"],
            hjson::Parser,
            hjson::Writer,
        );
        Self::register(
            &mut formats,
            "tmx",
            "TMX",
            &[".tmx"],
            tmx::Parser,
            tmx::Writer,
        );
        Self::register(
            &mut formats,
            "srt",
            "SRT Subtitles",
            &[".srt"],
            srt::Parser,
            srt::Writer,
        );
        Self::register(
            &mut formats,
            "excel",
            "Excel",
            &[".xlsx", ".xls"],
            excel::Parser,
            excel::Writer,
        );
        Self::register(
            &mut formats,
            "markdown",
            "Markdown",
            &[".md"],
            markdown::Parser,
            markdown::Writer,
        );

        // Tier 3 formats
        Self::register(
            &mut formats,
            "ios-plist",
            "iOS Property List",
            &[".plist"],
            ios_plist::Parser,
            ios_plist::Writer,
        );
        Self::register(
            &mut formats,
            "javascript",
            "JavaScript",
            &[".js"],
            js_format::Parser,
            js_format::Writer,
        );
        Self::register(
            &mut formats,
            "typescript",
            "TypeScript",
            &[".ts"],
            ts_format::Parser,
            ts_format::Writer,
        );
        Self::register(
            &mut formats,
            "neon",
            "NEON",
            &[".neon"],
            neon::Parser,
            neon::Writer,
        );
        Self::register(
            &mut formats,
            "plain-text",
            "Plain Text",
            &[".txt"],
            plain_text::Parser,
            plain_text::Writer,
        );
        Self::register(
            &mut formats,
            "yaml-plain",
            "YAML (Plain)",
            &[".yml", ".yaml"],
            yaml_plain::Parser,
            yaml_plain::Writer,
        );

        // Vendor-specific formats
        Self::register(
            &mut formats,
            "ispring-xliff",
            "iSpring Suite XLIFF",
            &[".xliff", ".xlf"],
            ispring_xliff::Parser,
            ispring_xliff::Writer,
        );
        Self::register(
            &mut formats,
            "captivate-xml",
            "Adobe Captivate XML",
            &[".xml"],
            captivate_xml::Parser,
            captivate_xml::Writer,
        );

        Self { formats }
    }

    fn register(
        formats: &mut IndexMap<&'static str, FormatEntry>,
        id: &'static str,
        name: &'static str,
        extensions: &'static [&'static str],
        parser: impl FormatParser + 'static,
        writer: impl FormatWriter + 'static,
    ) {
        formats.insert(
            id,
            FormatEntry {
                id,
                name,
                extensions,
                parser: Box::new(parser),
                writer: Box::new(writer),
            },
        );
    }

    pub fn get(&self, id: &str) -> Option<&FormatEntry> {
        self.formats.get(id)
    }

    pub fn list(&self) -> Vec<&FormatEntry> {
        self.formats.values().collect()
    }

    pub fn detect(&self, extension: &str, content: &[u8]) -> Vec<(&str, Confidence)> {
        let mut results: Vec<_> = self
            .formats
            .iter()
            .map(|(id, entry)| (*id, entry.parser.detect(extension, content)))
            .filter(|(_, c)| *c != Confidence::None)
            .collect();
        results.sort_by(|a, b| b.1.cmp(&a.1));
        results
    }
}

impl Default for FormatRegistry {
    fn default() -> Self {
        Self::new()
    }
}
