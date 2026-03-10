use indexmap::IndexMap;

/// Format-specific extension data for lossless round-trips.
/// Each variant holds data unique to that format.
/// Populated by parsers, consumed by writers of the same format.
/// Writers for other formats ignore extensions they don't understand.
#[derive(Debug, Clone, PartialEq)]
pub enum FormatExtension {
    AndroidXml(AndroidXmlExt),
    Xcstrings(XcstringsExt),
    IosStrings(IosStringsExt),
    Stringsdict(StringsdictExt),
    Arb(ArbExt),
    JsonStructured(JsonStructuredExt),
    I18nextJson(I18nextJsonExt),
    Xliff1(Xliff1Ext),
    Po(PoExt),
    YamlRails(YamlRailsExt),
}

// Tier 1 format extensions -- each starts minimal and grows as needed

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AndroidXmlExt {
    pub formatted: Option<bool>,
    pub product: Option<String>,
    pub xml_comments: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct XcstringsExt {
    pub extraction_state: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct IosStringsExt {}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct StringsdictExt {
    pub format_spec_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ArbExt {
    pub message_type: Option<String>,
    pub custom_fields: IndexMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct JsonStructuredExt {}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct I18nextJsonExt {
    pub context_separator: Option<String>,
    pub plural_separator: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Xliff1Ext {
    pub datatype: Option<String>,
    pub original: Option<String>,
    pub inline_elements: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PoExt {
    pub plural_forms_header: Option<String>,
    pub translator_comments: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct YamlRailsExt {}
