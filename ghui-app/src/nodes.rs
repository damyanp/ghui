use github_graphql::data::{FieldOptionId, WorkItemId};
use serde::Serialize;
use ts_rs::TS;

pub(crate) mod recipe_builder;
pub(crate) use recipe_builder::RecipeNodeBuilder;

#[derive(Serialize, TS, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    pub level: u32,
    pub id: String,
    pub data: NodeData,
    pub has_children: bool,
    pub is_modified: bool,
    pub is_ghost: bool,
}

#[derive(Serialize, TS, Debug)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum NodeData {
    #[serde(rename_all = "camelCase")]
    WorkItem { work_item_id: WorkItemId },
    #[serde(rename_all = "camelCase")]
    Group {
        name: String,
        field_option_id: Option<FieldOptionId>,
    },
}
