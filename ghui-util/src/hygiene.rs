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
    data::{self, HasFieldValue, SingleSelectFieldValue},
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

#[derive(Debug)]
struct Change<'a> {
    work_item: &'a data::WorkItem,
    status: Option<Option<String>>,
    blocked: Option<Option<String>>,
    epic: Option<Option<String>>,
}

impl<'a> Change<'a> {
    fn new(work_item: &'a data::WorkItem) -> Self {
        Change {
            work_item,
            status: None,
            blocked: None,
            epic: None,
        }
    }

    fn describe(&self) -> String {
        let mut s = Vec::new();

        if let Some(status) = &self.status {
            s.push(format!(
                "status({} -> {})",
                self.work_item
                    .project_item
                    .status
                    .field_value()
                    .unwrap_or("<>"),
                status.as_ref().map(|i| i.as_str()).unwrap_or("<>")
            ));
        }

        if let Some(blocked) = &self.blocked {
            s.push(format!(
                "blocked({} -> {})",
                self.work_item
                    .project_item
                    .status
                    .field_value()
                    .unwrap_or("<>"),
                blocked.as_ref().map(|i| i.as_str()).unwrap_or("<>")
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

    commit_changes(client, changes, &mode).await?;

    Ok(())
}

fn get_hygienic_changes<'a>(items: &'a data::WorkItems) -> impl Iterator<Item = Change<'a>> {
    items
        .work_items
        .values()
        .map(|item| {
            let mut change = Change::new(item);

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

async fn commit_changes<'a, I>(client: &GithubClient, changes: I, mode: &RunHygieneMode) -> Result
where
    I: std::iter::IntoIterator<Item = Change<'a>>,
{
    let fields = custom_fields_query::get_fields(client).await?;

    for change in changes {
        println!("{}", describe_item(change.work_item));

        let id = &change.work_item.project_item.id;

        if let Some(status) = &change.status {
            apply_change(
                client,
                &fields.project_id,
                id,
                &change.work_item.project_item.status,
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
                &change.work_item.project_item.blocked,
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
                &change.work_item.project_item.epic,
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
    use github_graphql::data::{test_helpers::TestData, IssueState};

    use super::{get_hygienic_changes, Change};

    #[test]
    fn test_closed_issues_set_state_to_closed() {
        let mut data = TestData::new();

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
        assert_eq!(change.work_item.id, closed_item_id);
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
            let mut data = TestData::new();

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
            assert_eq!(change.work_item.id, id);
            assert_eq!(change.status, None);
            assert_eq!(change.blocked, None);
            assert_eq!(change.epic, Some(Some(epic.to_owned())));
        }
    }
}
