#[macro_export]
#[rustfmt::skip]
macro_rules! gql {
    ($name:ident, $query_file:expr) => {
        #[derive(GraphQLQuery)]
        #[graphql(schema_path = "src/schema.docs.graphql",                    
                  query_path = $query_file,
                  response_derives = "Debug, Serialize, Eq, PartialEq",
                  variables_derives = "Debug, Clone")]
        pub(crate) struct $name;
    };
}

#[allow(clippy::upper_case_acronyms)]
type URI = String;

type DateTime = String;

pub mod mutators;
pub use mutators::{
    add_sub_issue, add_to_project, clear_project_field_value, set_issue_type,
    set_project_field_value,
};

pub mod custom_fields_query;
pub use custom_fields_query::{
    get_custom_fields, FieldConfig, FieldConfigOnProjectV2IterationField,
    FieldConfigOnProjectV2SingleSelectField,
};

pub mod get_issue_types;

pub mod get_all_items;
pub mod get_items;
pub use get_all_items::get_all_items;

mod viewer_info;
pub use viewer_info::{get_viewer_info, ViewerInfo};

mod get_resource_id_query;
pub use get_resource_id_query::get_resource_id;

pub mod get_project_item_ids;
