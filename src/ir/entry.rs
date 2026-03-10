use super::*;
use indexmap::IndexMap;

#[derive(Debug, Clone, PartialEq)]
pub struct I18nEntry {
    pub key: String,
    pub value: EntryValue,
    pub comments: Vec<Comment>,
    pub contexts: Vec<ContextEntry>,
    pub source: Option<String>,
    pub previous_source: Option<String>,
    pub previous_comment: Option<String>,
    pub placeholders: Vec<Placeholder>,
    pub translatable: Option<bool>,
    pub state: Option<TranslationState>,
    pub state_qualifier: Option<String>,
    pub approved: Option<bool>,
    pub obsolete: bool,
    pub max_width: Option<u32>,
    pub min_width: Option<u32>,
    pub max_height: Option<u32>,
    pub min_height: Option<u32>,
    pub size_unit: Option<String>,
    pub max_bytes: Option<u32>,
    pub min_bytes: Option<u32>,
    pub source_references: Vec<SourceRef>,
    pub flags: Vec<String>,
    pub device_variants: Option<IndexMap<DeviceType, EntryValue>>,
    pub alternatives: Vec<AlternativeTranslation>,
    pub properties: IndexMap<String, String>,
    pub resource_type: Option<String>,
    pub resource_name: Option<String>,
    pub format_ext: Option<FormatExtension>,
}

impl Default for I18nEntry {
    fn default() -> Self {
        Self {
            key: String::new(),
            value: EntryValue::Simple(String::new()),
            comments: Vec::new(),
            contexts: Vec::new(),
            source: None,
            previous_source: None,
            previous_comment: None,
            placeholders: Vec::new(),
            translatable: None,
            state: None,
            state_qualifier: None,
            approved: None,
            obsolete: false,
            max_width: None,
            min_width: None,
            max_height: None,
            min_height: None,
            size_unit: None,
            max_bytes: None,
            min_bytes: None,
            source_references: Vec::new(),
            flags: Vec::new(),
            device_variants: None,
            alternatives: Vec::new(),
            properties: IndexMap::new(),
            resource_type: None,
            resource_name: None,
            format_ext: None,
        }
    }
}
