use indexmap::IndexMap;
use super::*;

#[derive(Debug, Clone, PartialEq)]
pub struct I18nResource {
    pub metadata: ResourceMetadata,
    pub entries: IndexMap<String, I18nEntry>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ResourceMetadata {
    pub source_format: FormatId,
    pub locale: Option<String>,
    pub source_locale: Option<String>,
    pub headers: IndexMap<String, String>,
    pub properties: IndexMap<String, String>,
    pub encoding: Option<String>,
    pub direction: Option<TextDirection>,
    pub created_at: Option<String>,
    pub modified_at: Option<String>,
    pub created_by: Option<String>,
    pub modified_by: Option<String>,
    pub tool_name: Option<String>,
    pub tool_version: Option<String>,
    pub format_ext: Option<FormatExtension>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FormatId {
    #[default]
    AndroidXml,
    Xcstrings,
    IosStrings,
    Stringsdict,
    Arb,
    JsonStructured,
    I18nextJson,
    Xliff1,
    Po,
    YamlRails,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextDirection {
    Ltr,
    Rtl,
    Auto,
}
