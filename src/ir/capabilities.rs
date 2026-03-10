#[derive(Debug, Clone, PartialEq, Default)]
pub struct FormatCapabilities {
    pub plurals: bool,
    pub arrays: bool,
    pub comments: bool,
    pub context: bool,
    pub source_string: bool,
    pub translatable_flag: bool,
    pub translation_state: bool,
    pub max_width: bool,
    pub device_variants: bool,
    pub select_gender: bool,
    pub nested_keys: bool,
    pub inline_markup: bool,
    pub alternatives: bool,
    pub source_references: bool,
    pub custom_properties: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DataLossWarning {
    pub severity: WarningSeverity,
    pub message: String,
    pub affected_keys: Vec<String>,
    pub lost_attribute: String,
    pub count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarningSeverity {
    Info,
    Warning,
    Error,
}
