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
