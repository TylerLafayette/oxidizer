use darling::{FromMeta};

#[derive(Debug,FromMeta)]
pub struct RelationAttr {
    pub model: String,
    pub key: String,
}

#[derive(Debug,FromMeta,Clone)]
pub struct IndexAttr {
    pub name: String,
    pub columns: String,
    #[darling(default)]
    pub unique: bool,
}

#[derive(Debug,FromMeta,Clone)]
pub struct EntityAttr {
    pub table_name: Option<String>,
}