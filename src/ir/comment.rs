#[derive(Debug, Clone, PartialEq)]
pub struct Comment {
    pub text: String,
    pub role: CommentRole,
    pub priority: Option<u8>,
    pub annotates: Option<AnnotationTarget>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentRole {
    Developer,
    Translator,
    Extracted,
    General,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnnotationTarget {
    Source,
    Target,
    General,
}
