#![allow(dead_code)]

use std::{
    collections::{hash_map, HashMap},
    mem::Discriminant,
};

use github_graphql::{
    client::{
        graphql::{
            clear_project_field_value,
            custom_fields_query::{self, Field},
            set_project_field_value,
        },
        transport::GithubClient,
    },
    data::{self, HasFieldValue, SingleSelectFieldValue, WorkItemData, WorkItemId, WorkItems},
};

use crate::Result;

#[derive(Debug, clap::Args)]
pub struct Options {
    #[arg(value_enum, default_value_t = RunHygieneMode::DryRun)]
    mode: RunHygieneMode,
}

#[derive(Debug, Clone, clap::ValueEnum, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RunHygieneMode {
    DryRun,
    Commit,
    TestData,
}

pub async fn run(options: Options) -> Result {
    let client = crate::client();

    run_hygiene(&client, options.mode).await
}

async fn get_items(client: &GithubClient) -> Result<data::WorkItems> {
    let report_progress = |c, t| println!("Retrieved {c} of {t} items");
    data::WorkItems::from_client(client, report_progress).await
}

#[derive(Default, Debug, Eq, PartialEq)]
struct Changes {
    data: HashMap<ChangeKey, Change>,
}

impl Changes {
    fn add(&mut self, change: Change) {
        let old_value = self.data.insert(change.key(), change.clone());
        if let Some(old_value) = old_value {
            println!("WARNING! {:?} overrides {:?}", change, old_value);
        }
    }
}

impl<'a> IntoIterator for &'a Changes {
    type Item = &'a Change;

    type IntoIter = hash_map::Values<'a, ChangeKey, Change>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.values()
    }
}

#[derive(Debug, Eq, PartialEq, Hash)]
struct ChangeKey {
    pub work_item_id: WorkItemId,
    pub data_type: Discriminant<ChangeData>,
}

#[derive(Debug, Eq, PartialEq, Clone)]
struct Change {
    pub work_item_id: WorkItemId,
    pub data: ChangeData,
}

#[derive(Debug, Eq, PartialEq, Clone)]
enum ChangeData {
    Status(Option<String>),
    Blocked(Option<String>),
    Epic(Option<String>),
}

impl Change {
    fn key(&self) -> ChangeKey {
        ChangeKey {
            work_item_id: self.work_item_id.clone(),
            data_type: std::mem::discriminant(&self.data),
        }
    }

    fn describe(&self, work_items: &WorkItems) -> String {
        let work_item = work_items.get(&self.work_item_id).unwrap();

        let old_value = match self.data {
            ChangeData::Status(_) => work_item.project_item.status.field_value(),
            ChangeData::Blocked(_) => work_item.project_item.blocked.field_value(),
            ChangeData::Epic(_) => work_item.project_item.epic.field_value(),
        }
        .unwrap_or("<>");

        let name = match self.data {
            ChangeData::Status(_) => "Status",
            ChangeData::Blocked(_) => "Blocked",
            ChangeData::Epic(_) => "Epic",
        };

        let new_value = match &self.data {
            ChangeData::Status(value) => value.as_ref(),
            ChangeData::Blocked(value) => value.as_ref(),
            ChangeData::Epic(value) => value.as_ref(),
        }
        .map(|v| v.as_str())
        .unwrap_or("<>");

        format!("{}({} -> {})", name, old_value, new_value).to_owned()
    }
}

pub async fn run_hygiene(client: &GithubClient, mode: RunHygieneMode) -> Result {
    let items = match mode {
        RunHygieneMode::TestData => {
            let mut file = std::fs::File::open("all_items.json")?;
            data::WorkItems::from_graphql(serde_json::from_reader(&mut file)?)?
        }
        _ => get_items(client).await?,
    };

    println!("{} items", items.work_items.len());

    let changes = get_hygienic_changes(&items);

    commit_changes(client, &items, &changes, &mode).await?;

    Ok(())
}

fn get_hygienic_changes(items: &WorkItems) -> Changes {
    let mut changes = Changes::default();

    for item in items.work_items.values() {
        // Closed items should have status set to Closed
        if item.is_closed() && !item.project_item.status.matches("Closed") {
            changes.add(Change {
                work_item_id: item.id.clone(),
                data: ChangeData::Status(Some("Closed".to_owned())),
            });
        }

        // Map project milestones to epics
        if item.project_item.epic.is_none() {
            let new_epic = match item.project_item.project_milestone.field_value() {
                Some("3: ML preview requirements")
                | Some("4: ML preview planning")
                | Some("5: ML preview implementation") => Some("DML Demo"),
                Some("Graphics preview feature analysis") => Some("MiniEngine Demo"),
                Some("DXC: SM 6.9 Preview") => Some("SM 6.9 Preview"),
                Some("DXC: SM 6.9 Release") => Some("DXC 2025 Q4"),
                _ => None,
            };

            if let Some(new_epic) = new_epic {
                changes.add(Change {
                    work_item_id: item.id.clone(),
                    data: ChangeData::Epic(Some(new_epic.to_owned())),
                });
            }
        }
    }

    for root_item_id in items.get_roots() {
        sanitize_issue_hierarchy(items, &mut changes, &root_item_id, None);
    }

    fn sanitize_issue_hierarchy(
        items: &WorkItems,
        changes: &mut Changes,
        id: &WorkItemId,
        epic: Option<&str>,
    ) {
        if let Some(item) = items.get(id) {
            if item.project_item.epic.field_value() != epic {
                if let Some(epic) = epic {
                    if let Some(current_epic) = &item.project_item.epic {
                        println!("WARNING: {} - epic is '{}', should be '{}' - but not changing non-blank value",
                        describe_item(item), current_epic.name, epic);
                    } else {
                        changes.add(Change {
                            work_item_id: id.clone(),
                            data: ChangeData::Epic(Some(epic.to_owned())),
                        });
                    }
                }
            }

            if let WorkItemData::Issue(issue) = &item.data {
                for child_id in &issue.sub_issues {
                    sanitize_issue_hierarchy(
                        items,
                        changes,
                        child_id,
                        epic.or(item.project_item.epic.field_value()),
                    );
                }
            }
        }
    }

    changes
}

fn describe_item(item: &data::WorkItem) -> String {
    match &item.resource_path {
        Some(resource_path) => format!("https://github.com{}", resource_path),
        None => format!("[{}]", item.id.0),
    }
}

async fn commit_changes(
    client: &GithubClient,
    work_items: &WorkItems,
    changes: &Changes,
    mode: &RunHygieneMode,
) -> Result {
    let fields = custom_fields_query::get_fields(client).await?;

    for change in changes {
        let work_item = work_items.get(&change.work_item_id).unwrap();

        println!("{}", describe_item(work_item));

        let id = &work_item.project_item.id;

        match &change.data {
            ChangeData::Status(value) => apply_change(
                client,
                &fields.project_id,
                id,
                &work_item.project_item.status,
                &fields.status,
                value,
                mode,
            ),
            ChangeData::Blocked(value) => apply_change(
                client,
                &fields.project_id,
                id,
                &work_item.project_item.blocked,
                &fields.blocked,
                value,
                mode,
            ),
            ChangeData::Epic(value) => apply_change(
                client,
                &fields.project_id,
                id,
                &work_item.project_item.epic,
                &fields.epic,
                value,
                mode,
            ),
        }
        .await?;

        println!();
    }

    Ok(())
}

async fn apply_change(
    client: &GithubClient,
    project_id: &str,
    work_item_id: &data::ProjectItemId,
    old_value: &Option<SingleSelectFieldValue>,
    field: &Field,
    value: &Option<String>,
    mode: &RunHygieneMode,
) -> Result {
    let old_value = old_value.as_ref().map_or("<>", |v| v.name.as_str());
    let new_value_id = value.as_ref().and_then(|v| field.id(v));

    println!(
        "  {} {} -> {}({})",
        field.name,
        old_value,
        value.as_ref().map_or("<>", |v| v.as_str()),
        new_value_id.as_ref().map_or("<>", |v| v)
    );

    if *mode == RunHygieneMode::Commit {
        if let Some(new_value_id) = new_value_id {
            set_project_field_value::set(
                client,
                project_id,
                work_item_id.0.as_str(),
                field.id.as_str(),
                new_value_id,
            )
            .await?;
        } else {
            clear_project_field_value::clear(
                client,
                project_id,
                work_item_id.0.as_str(),
                field.id.as_str(),
            )
            .await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use github_graphql::data::{test_helpers::TestData, IssueState};

    use super::{get_hygienic_changes, Change, ChangeData, Changes};

    #[test]
    fn test_closed_issues_set_state_to_closed() {
        let mut data = TestData::default();

        data.build()
            .issue_state(IssueState::OPEN)
            .status("Active")
            .add();

        let closed_item_id = data
            .build()
            .issue_state(IssueState::CLOSED)
            .status("Active")
            .add();

        let actual_changes = get_hygienic_changes(&data.work_items);

        let mut expected_changes = Changes::default();
        expected_changes.add(Change {
            work_item_id: closed_item_id,
            data: ChangeData::Status(Some("Closed".to_owned())),
        });

        assert_eq!(actual_changes, expected_changes);
    }

    #[test]
    fn test_set_epic_from_project_milestone() {
        let mappings = [
            ("3: ML preview requirements", "DML Demo"),
            ("4: ML preview planning", "DML Demo"),
            ("5: ML preview implementation", "DML Demo"),
            ("Graphics preview feature analysis", "MiniEngine Demo"),
            ("DXC: SM 6.9 Preview", "SM 6.9 Preview"),
            ("DXC: SM 6.9 Release", "DXC 2025 Q4"),
        ];

        for (project_milestone, epic) in mappings {
            let mut data = TestData::default();

            // Existing epics shouldn't be changed
            data.build()
                .project_milestone(project_milestone)
                .epic("Do Not Change")
                .add();

            // Unrecognized milestones shouldn't change epic
            data.build()
                .project_milestone(format!("{}-XXX", project_milestone).as_str())
                .add();

            // Already matching ones shouldn't change
            data.build()
                .project_milestone(project_milestone)
                .epic(epic)
                .add();

            // But when there's a match and no epic is set, we should expect a
            // change
            let id = data.build().project_milestone(project_milestone).add();

            let actual_changes = get_hygienic_changes(&data.work_items);

            let mut expected_changes = Changes::default();
            expected_changes.add(Change {
                work_item_id: id,
                data: ChangeData::Epic(Some(epic.to_owned())),
            });

            assert_eq!(actual_changes, expected_changes);
        }
    }

    #[test]
    fn test_set_epic_from_parent() {
        let mut data = TestData::default();

        const RIGHT_EPIC: &str = "right epic";
        const WRONG_EPIC: &str = "wrong epic";

        let child_no_epic = data.build().add();
        let child_wrong_epic = data.build().epic(WRONG_EPIC).add();
        let child_right_epic = data.build().epic(RIGHT_EPIC).add();

        data.build()
            .epic(RIGHT_EPIC)
            .sub_issues(&[&child_no_epic, &child_wrong_epic, &child_right_epic])
            .add();

        let actual_changes = get_hygienic_changes(&data.work_items);

        let mut expected_changes = Changes::default();
        expected_changes.add(Change {
            work_item_id: child_no_epic,
            data: ChangeData::Epic(Some(RIGHT_EPIC.to_owned())),
        });

        assert_eq!(actual_changes, expected_changes);
    }

    #[test]
    fn test_set_epic_from_grandparent() {
        let mut data = TestData::default();

        const EPIC: &str = "epic";

        let child_a = data.build().add();
        let parent_a = data.build().epic(EPIC).sub_issues(&[&child_a]).add();

        let child_b = data.build().add();
        let parent_b = data.build().sub_issues(&[&child_b]).add();

        data.build()
            .epic(EPIC)
            .sub_issues(&[&parent_a, &parent_b])
            .add();

        let epic = ChangeData::Epic(Some(EPIC.to_owned()));

        let actual_changes = get_hygienic_changes(&data.work_items);

        let mut expected_changes = Changes::default();
        expected_changes.add(Change {
            work_item_id: child_a,
            data: epic.clone(),
        });
        expected_changes.add(Change {
            work_item_id: child_b,
            data: epic.clone(),
        });
        expected_changes.add(Change {
            work_item_id: parent_b,
            data: epic.clone(),
        });

        assert_eq!(actual_changes, expected_changes);
    }
}
