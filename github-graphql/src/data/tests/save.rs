use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
};

use serde::{de::DeserializeOwned, Serialize};

use crate::{
    client::transport::Client,
    data::{
        test_helpers::TestData, Change, ChangeData, Changes, FieldOptionId, ProjectItemId, SaveMode,
    },
};

/// A mock Client that inspects each request's query string to determine the
/// mutation type and returns the corresponding pre-configured response.
#[derive(Clone)]
struct MockClient {
    responses: Arc<Mutex<HashMap<String, VecDeque<serde_json::Value>>>>,
}

impl MockClient {
    fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn on_mutation(self, keyword: &str, response: serde_json::Value) -> Self {
        self.responses
            .lock()
            .unwrap()
            .entry(keyword.to_string())
            .or_default()
            .push_back(response);
        self
    }
}

impl Client for MockClient {
    async fn request<Q, R>(&self, request: &Q) -> crate::Result<R>
    where
        Q: Serialize + Sync,
        R: DeserializeOwned,
    {
        let request_json = serde_json::to_value(request).unwrap();
        let query = request_json["query"].as_str().unwrap_or("");

        let mut responses = self.responses.lock().unwrap();
        let response = responses
            .iter_mut()
            .find_map(|(keyword, queue)| {
                if query.contains(keyword.as_str()) {
                    queue.pop_front()
                } else {
                    None
                }
            })
            .unwrap_or_else(|| panic!("MockClient: no response for query containing: {query}"));

        serde_json::from_value(response)
            .map_err(|e| crate::Error::GraphQlResponseUnexpected(e.to_string()))
    }
}

/// JSON response for `add_to_project` mutation.
fn add_to_project_response(project_item_id: &str) -> serde_json::Value {
    serde_json::json!({
        "data": {
            "addProjectV2ItemById": {
                "clientMutationId": null,
                "item": {
                    "id": project_item_id
                }
            }
        }
    })
}

/// JSON response for field mutations (set_project_single_select_field_value).
fn field_mutation_response() -> serde_json::Value {
    serde_json::json!({
        "data": {
            "updateProjectV2ItemFieldValue": {
                "clientMutationId": null
            }
        }
    })
}

fn noop_progress(_: &Change, _: usize, _: usize) {}

#[tokio::test]
async fn test_save_dry_run_returns_project_item_ids_for_existing_items() {
    let mut data = TestData::default();
    let id1 = data.build().status("Active").add();
    let id2 = data.build().status("Open").add();

    let project_item_id_1 = data.work_items.get(&id1).unwrap().project_item.id.clone();
    let project_item_id_2 = data.work_items.get(&id2).unwrap().project_item.id.clone();

    let mut changes = Changes::default();
    changes.add(Change {
        work_item_id: id1,
        data: ChangeData::Status(Some(FieldOptionId("id(Closed)".into()))),
    });
    changes.add(Change {
        work_item_id: id2,
        data: ChangeData::Status(Some(FieldOptionId("id(Closed)".into()))),
    });

    let client = MockClient::new();

    let mut result = changes
        .save(
            &client,
            &data.fields,
            &data.work_items,
            SaveMode::DryRun,
            &noop_progress,
        )
        .await
        .unwrap();

    result.sort_by(|a, b| a.0.cmp(&b.0));

    let mut expected = vec![project_item_id_1, project_item_id_2];
    expected.sort_by(|a, b| a.0.cmp(&b.0));

    assert_eq!(result, expected);
}

#[tokio::test]
async fn test_save_dry_run_skips_items_not_in_work_items() {
    let data = TestData::default();

    let mut changes = Changes::default();
    changes.add(Change {
        work_item_id: "nonexistent".to_string().into(),
        data: ChangeData::AddToProject,
    });

    let client = MockClient::new();

    let result = changes
        .save(
            &client,
            &data.fields,
            &data.work_items,
            SaveMode::DryRun,
            &noop_progress,
        )
        .await
        .unwrap();

    // AddToProject for nonexistent item: the WorkItemId can't be resolved to a
    // ProjectItemId (not in work_items), and DryRun doesn't call add_to_project,
    // so no ProjectItemId is captured.
    assert!(result.is_empty());
}

#[tokio::test]
async fn test_save_commit_add_to_project_returns_new_project_item_id() {
    let data = TestData::default();

    let mut changes = Changes::default();
    changes.add(Change {
        work_item_id: "new_item_1".to_string().into(),
        data: ChangeData::AddToProject,
    });

    let client =
        MockClient::new().on_mutation("addProjectV2ItemById", add_to_project_response("PVTI_new"));

    let result = changes
        .save(
            &client,
            &data.fields,
            &data.work_items,
            SaveMode::Commit,
            &noop_progress,
        )
        .await
        .unwrap();

    assert!(result.contains(&ProjectItemId("PVTI_new".into())));
}

#[tokio::test]
async fn test_save_commit_field_change_returns_project_item_id() {
    let mut data = TestData::default();
    let id = data.build().status("Active").add();
    let project_item_id = data.work_items.get(&id).unwrap().project_item.id.clone();

    let mut changes = Changes::default();
    changes.add(Change {
        work_item_id: id,
        data: ChangeData::Status(Some(FieldOptionId("id(Closed)".into()))),
    });

    let client =
        MockClient::new().on_mutation("updateProjectV2ItemFieldValue", field_mutation_response());

    let result = changes
        .save(
            &client,
            &data.fields,
            &data.work_items,
            SaveMode::Commit,
            &noop_progress,
        )
        .await
        .unwrap();

    assert!(result.contains(&project_item_id));
}

#[tokio::test]
async fn test_save_commit_mixed_changes_returns_all_project_item_ids() {
    let mut data = TestData::default();
    let existing_id = data.build().status("Active").add();
    let existing_project_item_id = data
        .work_items
        .get(&existing_id)
        .unwrap()
        .project_item
        .id
        .clone();

    let mut changes = Changes::default();
    changes.add(Change {
        work_item_id: existing_id,
        data: ChangeData::Status(Some(FieldOptionId("id(Closed)".into()))),
    });
    changes.add(Change {
        work_item_id: "new_item".to_string().into(),
        data: ChangeData::AddToProject,
    });

    let client = MockClient::new()
        .on_mutation("updateProjectV2ItemFieldValue", field_mutation_response())
        .on_mutation(
            "addProjectV2ItemById",
            add_to_project_response("PVTI_added"),
        );

    let result = changes
        .save(
            &client,
            &data.fields,
            &data.work_items,
            SaveMode::Commit,
            &noop_progress,
        )
        .await
        .unwrap();

    assert!(result.contains(&existing_project_item_id));
    assert!(result.contains(&ProjectItemId("PVTI_added".into())));
}

#[tokio::test]
async fn test_save_empty_changes_returns_empty() {
    let data = TestData::default();
    let mut changes = Changes::default();
    let client = MockClient::new();

    let result = changes
        .save(
            &client,
            &data.fields,
            &data.work_items,
            SaveMode::Commit,
            &noop_progress,
        )
        .await
        .unwrap();

    assert!(result.is_empty());
}
