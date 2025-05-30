use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Serialize, Deserialize, TS, Clone)]
pub struct FieldId(pub String);

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize, TS, Clone, PartialOrd, Ord)]
pub struct FieldOptionId(pub String);

#[derive(Debug, Serialize, Deserialize, TS, Clone)]
pub struct Field {
    pub id: FieldId,
    pub name: String,
    pub field_type: FieldType,
    pub options: Vec<FieldOption>,
}

#[derive(Debug, Serialize, Deserialize, TS, Clone)]
pub struct FieldOption {
    pub id: FieldOptionId,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize, TS, Clone)]
pub enum FieldType {
    SingleSelect,
    Iteration,
}

#[derive(Debug, Serialize, Deserialize, TS, Clone)]
pub struct Fields {
    pub project_id: String,
    pub status: Field,
    pub blocked: Field,
    pub epic: Field,
    pub iteration: Field,
    pub project_milestone: Field,
    pub kind: Field,
}

impl Field {
    pub fn option_id(&self, name: Option<&str>) -> Option<&FieldOptionId> {
        name.and_then(|name| {
            self.options
                .iter()
                .find(|option| option.value == name)
                .map(|option| &option.id)
        })
    }

    pub fn option_name(&self, id: Option<&FieldOptionId>) -> Option<&str> {
        id.and_then(|id| {
            self.options
                .iter()
                .find(|option| option.id == *id)
                .map(|option| option.value.as_str())
        })
    }
}
