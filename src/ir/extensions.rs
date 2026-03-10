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
    Xliff2(Xliff2Ext),
    Resx(ResxExt),
    JavaProperties(JavaPropertiesExt),
    PhpLaravel(PhpLaravelExt),
    QtLinguist(QtLinguistExt),
    Csv(CsvExt),
    Toml(TomlExt),
    Ini(IniExt),
    Json5(Json5Ext),
    Hjson(HjsonExt),
    Tmx(TmxExt),
    Srt(SrtExt),
    Excel(ExcelExt),
    Markdown(MarkdownExt),
    // Tier 3
    IosPlist(IosPlistExt),
    JavaScript(JavaScriptExt),
    TypeScript(TypeScriptExt),
    Neon(NeonExt),
    PlainText(PlainTextExt),
    YamlPlain(YamlPlainExt),
    // Vendor-specific
    IspringXliff(IspringXliffExt),
    CaptivateXml(CaptivateXmlExt),
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

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Xliff2Ext {
    pub can_resegment: Option<bool>,
    pub original_data: IndexMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ResxExt {
    pub mimetype: Option<String>,
    pub type_name: Option<String>,
    pub schema: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct JavaPropertiesExt {
    pub separator: Option<char>,
    pub comment_char: Option<char>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PhpLaravelExt {
    pub quote_style: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct QtLinguistExt {
    pub numerus: Option<bool>,
    pub extra_elements: IndexMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CsvExt {
    pub delimiter: Option<char>,
    pub key_column: Option<String>,
    pub value_column: Option<String>,
    pub has_bom: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TomlExt {
    pub table_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct IniExt {
    pub section: Option<String>,
    pub delimiter: Option<char>,
    pub comment_char: Option<char>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Json5Ext {
    pub trailing_commas: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct HjsonExt {}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TmxExt {
    pub seg_type: Option<String>,
    pub o_tmf: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SrtExt {
    pub sequence_number: Option<u32>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ExcelExt {
    pub sheet_name: Option<String>,
    pub key_column: Option<u32>,
    pub value_column: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MarkdownExt {
    pub front_matter: Option<String>,
}

// Tier 3 format extensions

#[derive(Debug, Clone, PartialEq, Default)]
pub struct IosPlistExt {
    pub plist_format: Option<String>, // "xml1" or "binary1"
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct JavaScriptExt {
    pub export_style: Option<String>, // "module.exports", "export default", "exports"
    pub quote_style: Option<char>,    // '\'' or '"'
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TypeScriptExt {
    pub export_style: Option<String>, // "export default", "export const"
    pub type_annotation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct NeonExt {}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlainTextExt {
    pub line_ending: Option<String>, // "\n" or "\r\n"
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct YamlPlainExt {}

// Vendor-specific format extensions

#[derive(Debug, Clone, PartialEq, Default)]
pub struct IspringXliffExt {
    pub xliff_version: Option<String>, // "1.2" or "2.1"
    pub source_language: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CaptivateXmlExt {
    pub slide_id: Option<String>,
    pub item_id: Option<String>,
    pub css_style: Option<String>,
}
