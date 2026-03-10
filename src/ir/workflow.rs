#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranslationState {
    New,
    Translated,
    NeedsReview,
    Reviewed,
    Final,
    Stale,
    NeedsTranslation,
    NeedsAdaptation,
    NeedsL10n,
    NeedsReviewAdaptation,
    NeedsReviewL10n,
    Vanished,
    Obsolete,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AlternativeTranslation {
    pub value: String,
    pub source: Option<String>,
    pub match_quality: Option<f32>,
    pub origin: Option<String>,
    pub alt_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SourceRef {
    pub file: String,
    pub line: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DeviceType {
    Default,
    Phone,
    Tablet,
    Desktop,
    Watch,
    TV,
    Vision,
    IPod,
    Other(String),
}
