use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Serialize, Deserialize, TS, Clone)]
pub struct FieldId(pub String);

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize, TS, Clone, PartialOrd, Ord)]
pub struct FieldOptionId(pub String);

#[derive(Debug, Serialize, Deserialize, TS, Clone)]
pub struct Field<T> {
    pub id: FieldId,
    pub name: String,
    pub options: Vec<FieldOption<T>>,
}

#[derive(Debug, Serialize, Deserialize, TS, Clone)]
pub struct FieldOption<T> {
    pub id: FieldOptionId,
    pub value: String,
    pub data: T,
}

#[derive(Debug, Serialize, Deserialize, TS, Clone)]
pub struct SingleSelect;

#[derive(Debug, Serialize, Deserialize, TS, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Iteration {
    pub start_date: String,
    pub duration: i64,
}

#[derive(Debug, Serialize, Deserialize, TS, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Fields {
    pub project_id: String,
    pub status: Field<SingleSelect>,
    pub blocked: Field<SingleSelect>,
    pub epic: Field<SingleSelect>,
    pub iteration: Field<Iteration>,
    pub kind: Field<SingleSelect>,
    pub estimate: Field<SingleSelect>,
    pub priority: Field<SingleSelect>,
}

impl<T> Field<T> {
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

    pub fn option_index(&self, id: Option<&FieldOptionId>) -> usize {
        id.and_then(|id| {
            self.options
                .iter()
                .enumerate()
                .find(|(_, option)| *id == option.id)
                .map(|(index, _)| index)
        })
        .unwrap_or(usize::MAX)
    }
}
