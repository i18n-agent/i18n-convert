use indexmap::IndexMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Placeholder {
    pub name: String,
    pub original_syntax: String,
    pub placeholder_type: Option<PlaceholderType>,
    pub position: Option<usize>,
    pub example: Option<String>,
    pub description: Option<String>,
    pub format: Option<String>,
    pub optional_parameters: Option<IndexMap<String, String>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlaceholderType {
    String,
    Integer,
    Float,
    Double,
    DateTime,
    Currency,
    Object,
    Other(String),
}
