use indexmap::IndexMap;

#[derive(Debug, Clone, PartialEq)]
pub enum EntryValue {
    Simple(String),
    Plural(PluralSet),
    Array(Vec<String>),
    Select(SelectSet),
    MultiVariablePlural(MultiVariablePlural),
}

#[derive(Debug, Clone, PartialEq)]
pub struct PluralSet {
    pub zero: Option<String>,
    pub one: Option<String>,
    pub two: Option<String>,
    pub few: Option<String>,
    pub many: Option<String>,
    pub other: String,
    pub exact_matches: IndexMap<u64, String>,
    pub range_matches: Vec<PluralRange>,
    pub ordinal: bool,
}

impl Default for PluralSet {
    fn default() -> Self {
        Self {
            zero: None,
            one: None,
            two: None,
            few: None,
            many: None,
            other: String::new(),
            exact_matches: IndexMap::new(),
            range_matches: Vec::new(),
            ordinal: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PluralRange {
    pub from: Option<i64>,
    pub to: Option<i64>,
    pub inclusive: bool,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectSet {
    pub variable: String,
    pub cases: IndexMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MultiVariablePlural {
    pub pattern: String,
    pub variables: IndexMap<String, PluralVariable>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PluralVariable {
    pub name: String,
    pub format_specifier: Option<String>,
    pub arg_num: Option<u32>,
    pub plural_set: PluralSet,
}
