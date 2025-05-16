#![allow(dead_code)]

use github_graphql::{
    client::{
        graphql::{
            clear_project_field_value,
            custom_fields_query::{self, Field},
            get_all_items, project_items, set_project_field_value,
        },
        transport::GithubClient,
    },
    data::{self, HasFieldValue, SingleSelectFieldValue, WorkItemId, WorkItems},
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
    let variables = project_items::Variables {
        page_size: 100,
        after: None,
    };
    let report_progress = |c, t| println!("Retrieved {c} of {t} items");
    let items = get_all_items::<project_items::ProjectItems, GithubClient>(
        client,
        variables,
        report_progress,
    )
    .await?;

    data::WorkItems::from_graphql(items)
}

#[derive(Debug, Default, Eq, Hash, PartialEq)]
struct Change {
    work_item_id: WorkItemId,
    status: Option<Option<String>>,
    blocked: Option<Option<String>>,
    epic: Option<Option<String>>,
}

impl Change {
    fn new(id: WorkItemId) -> Self {
        Change {
            work_item_id: id,
            ..Default::default()
        }
    }

    fn describe(&self, work_items: &WorkItems) -> String {
        let work_item = work_items.get(&self.work_item_id).unwrap();

        let mut s = Vec::new();

        if let Some(status) = &self.status {
            s.push(format!(
                "status({} -> {})",
                work_item.project_item.status.field_value().unwrap_or("<>"),
                status.as_ref().map(|i| i.as_str()).unwrap_or("<>")
            ));
        }

        if let Some(blocked) = &self.blocked {
            s.push(format!(
                "blocked({} -> {})",
                work_item.project_item.status.field_value().unwrap_or("<>"),
                blocked.as_ref().map(|i| i.as_str()).unwrap_or("<>")
            ));
        }

        if let Some(epic) = &self.epic {
            s.push(format!(
                "epic({} -> {})",
                work_item.project_item.epic.field_value().unwrap_or("<>"),
                epic.as_ref().map(|i| i.as_str()).unwrap_or("<>")
            ));
        }

        s.join(", ")
    }

    fn has_changes(&self) -> bool {
        self.status.is_some() || self.blocked.is_some() || self.epic.is_some()
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

    commit_changes(client, &items, changes, &mode).await?;

    Ok(())
}

fn get_hygienic_changes(items: &WorkItems) -> impl Iterator<Item = Change> + use<'_> {
    items
        .work_items
        .values()
        .map(|item| {
            let mut change = Change::new(item.id.clone());

            // Closed items should have status set to Closed
            if item.is_closed() && !item.project_item.status.matches("Closed") {
                change.status = Some(Some("Closed".to_owned()));
            }

            // Map project milestones to epics
            if item.project_item.epic.is_none() {
                change.epic = match item.project_item.project_milestone.field_value() {
                    Some("3: ML preview requirements")
                    | Some("4: ML preview planning")
                    | Some("5: ML preview implementation") => Some(Some("DML Demo".to_owned())),
                    Some("Graphics preview feature analysis") => {
                        Some(Some("MiniEngine Demo".to_owned()))
                    }
                    Some("DXC: SM 6.9 Preview") => Some(Some("SM 6.9 Preview".to_owned())),
                    Some("DXC: SM 6.9 Release") => Some(Some("DXC 2025 Q4".to_owned())),
                    _ => None,
                };
            }

            change
        })
        .filter(|change| change.has_changes())
}

fn describe_item(item: &data::WorkItem) -> String {
    match &item.resource_path {
        Some(resource_path) => format!("https://github.com{}", resource_path),
        None => format!("[{}]", item.id.0),
    }
}

async fn commit_changes<I>(
    client: &GithubClient,
    work_items: &WorkItems,
    changes: I,
    mode: &RunHygieneMode,
) -> Result
where
    I: std::iter::IntoIterator<Item = Change>,
{
    let fields = custom_fields_query::get_fields(client).await?;

    for change in changes {
        let work_item = work_items.get(&change.work_item_id).unwrap();

        println!("{}", describe_item(work_item));

        let id = &work_item.project_item.id;

        if let Some(status) = &change.status {
            apply_change(
                client,
                &fields.project_id,
                id,
                &work_item.project_item.status,
                &fields.status,
                status,
                mode,
            )
            .await?;
        }

        if let Some(blocked) = &change.blocked {
            apply_change(
                client,
                &fields.project_id,
                id,
                &work_item.project_item.blocked,
                &fields.blocked,
                blocked,
                mode,
            )
            .await?;
        }

        if let Some(epic) = &change.epic {
            apply_change(
                client,
                &fields.project_id,
                id,
                &work_item.project_item.epic,
                &fields.epic,
                epic,
                mode,
            )
            .await?;
        }

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
        "  {}({}) {} -> {}({})",
        field.name,
        field.id,
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
    use std::collections::HashSet;

    use github_graphql::data::{test_helpers::TestData, IssueState};

    use super::{get_hygienic_changes, Change};

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

        let changes: Vec<Change> = get_hygienic_changes(&data.work_items).collect();

        assert_eq!(changes.len(), 1);

        let change = &changes[0];
        assert_eq!(change.work_item_id, closed_item_id);
        assert_eq!(change.status, Some(Some("Closed".to_owned())));
        assert_eq!(change.blocked, None);
        assert_eq!(change.epic, None);
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

            let changes: Vec<Change> = get_hygienic_changes(&data.work_items).collect();

            assert_eq!(changes.len(), 1);

            let change = &changes[0];
            assert_eq!(change.work_item_id, id);
            assert_eq!(change.status, None);
            assert_eq!(change.blocked, None);
            assert_eq!(change.epic, Some(Some(epic.to_owned())));
        }
    }

    #[test]
    #[ignore]
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

        let expected_changes: HashSet<Change> = HashSet::from_iter([Change {
            work_item_id: child_no_epic,
            epic: Some(Some(RIGHT_EPIC.to_owned())),
            ..Default::default()
        }]);

        let actual_changes: HashSet<Change> =
            HashSet::from_iter(get_hygienic_changes(&data.work_items));

        assert_eq!(actual_changes, expected_changes);
    }

    #[test]
    #[ignore]
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

        let epic = Some(Some(EPIC.to_owned()));

        let expected_changes: HashSet<Change> = HashSet::from_iter([
            Change {
                work_item_id: child_a,
                epic: epic.clone(),
                ..Default::default()
            },
            Change {
                work_item_id: child_b,
                epic: epic.clone(),
                ..Default::default()
            },
            Change {
                work_item_id: parent_b,
                epic: epic.clone(),
                ..Default::default()
            },
        ]);

        let actual_changes: HashSet<Change> =
            HashSet::from_iter(get_hygienic_changes(&data.work_items));

        assert_eq!(actual_changes, expected_changes);
    }
}
