#[derive(Debug, Clone, PartialEq)]
pub struct ContextEntry {
    pub context_type: ContextType,
    pub value: String,
    pub purpose: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ContextType {
    Disambiguation,
    SourceFile,
    LineNumber,
    Element,
    Description,
    Custom(String),
}
