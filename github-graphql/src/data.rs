use std::{
    collections::{hash_map, HashMap, HashSet},
    mem::Discriminant,
};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Default, PartialEq, Eq, Debug, Serialize, Deserialize, Clone, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkItem {
    pub id: WorkItemId,
    pub title: String,
    pub updated_at: Option<String>,
    pub resource_path: Option<String>,
    pub repository: Option<String>,
    pub data: WorkItemData,
    pub project_item: ProjectItem,
}

impl WorkItem {
    fn get_sub_issues(&self) -> Option<&Vec<WorkItemId>> {
        if let WorkItem {
            data: WorkItemData::Issue(Issue { sub_issues, .. }),
            ..
        } = self
        {
            Some(sub_issues)
        } else {
            None
        }
    }

    pub fn is_closed(&self) -> bool {
        match &self.data {
            WorkItemData::DraftIssue => false,
            WorkItemData::Issue(issue) => issue.state == IssueState::CLOSED,
            WorkItemData::PullRequest(pull_request) => {
                pull_request.state == PullRequestState::MERGED
                    || pull_request.state == PullRequestState::CLOSED
            }
        }
    }
}

#[derive(Default, PartialEq, Debug, Eq, Hash, Clone, Serialize, Deserialize, TS)]
pub struct WorkItemId(pub String);

impl From<String> for WorkItemId {
    fn from(value: String) -> Self {
        WorkItemId(value)
    }
}

#[derive(Default, PartialEq, Eq, Debug, Serialize, Deserialize, Clone, TS)]
#[serde(tag = "type")]
#[serde(rename_all = "camelCase")]
pub enum WorkItemData {
    #[default]
    DraftIssue,
    Issue(Issue),
    PullRequest(PullRequest),
}

#[derive(Default, PartialEq, Eq, Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    pub parent_id: Option<WorkItemId>,
    pub state: IssueState,
    pub sub_issues: Vec<WorkItemId>,
    pub tracked_issues: Vec<WorkItemId>,
}

#[derive(Default, PartialEq, Debug, Eq, Hash, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all_fields = "camelCase")]
pub enum IssueState {
    CLOSED,
    #[default]
    OPEN,
    Other(String),
}

#[derive(Default, PartialEq, Eq, Debug, Serialize, Deserialize, Clone, TS)]
#[serde(rename_all = "camelCase")]
pub struct PullRequest {
    pub state: PullRequestState,
}

#[derive(Default, PartialEq, Debug, Eq, Hash, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all_fields = "camelCase")]
pub enum PullRequestState {
    CLOSED,
    #[default]
    MERGED,
    OPEN,
    Other(String),
}

#[derive(Default, PartialEq, Eq, Debug, Serialize, Deserialize, Clone, TS)]
#[serde(rename_all = "camelCase")]
pub struct ProjectItem {
    pub id: ProjectItemId,
    pub updated_at: String,
    pub status: Option<SingleSelectFieldValue>,
    pub iteration: Option<IterationFieldValue>,
    pub blocked: Option<SingleSelectFieldValue>,
    pub kind: Option<SingleSelectFieldValue>,
    pub epic: Option<SingleSelectFieldValue>,
    pub workstream: Option<SingleSelectFieldValue>,
    pub project_milestone: Option<SingleSelectFieldValue>,
}

#[derive(Default, PartialEq, Eq, Debug, Serialize, Deserialize, Clone, TS)]
#[serde(rename_all = "camelCase")]
pub struct SingleSelectFieldValue {
    pub option_id: String,
    pub name: String,
}
impl SingleSelectFieldValue {
    fn from_name(name: &str) -> SingleSelectFieldValue {
        SingleSelectFieldValue {
            name: name.to_owned(),
            ..Default::default()
        }
    }
}

#[derive(Default, PartialEq, Eq, Debug, Serialize, Deserialize, Clone, TS)]
#[serde(rename_all = "camelCase")]
pub struct IterationFieldValue {
    pub iteration_id: String,
    pub title: String,
}

pub trait HasFieldValue {
    fn matches(&self, value: &str) -> bool;
    fn field_value(&self) -> Option<&str>;
}

impl HasFieldValue for Option<SingleSelectFieldValue> {
    fn matches(&self, value: &str) -> bool {
        match self {
            Some(SingleSelectFieldValue { name, .. }) => name == value,
            None => false,
        }
    }

    fn field_value(&self) -> Option<&str> {
        match self {
            Some(SingleSelectFieldValue { name, .. }) => Some(name.as_str()),
            None => None,
        }
    }
}

#[derive(Default, PartialEq, Debug, Eq, Hash, Clone, Serialize, Deserialize, TS)]
pub struct ProjectItemId(pub String);

impl From<String> for ProjectItemId {
    fn from(value: String) -> Self {
        ProjectItemId(value)
    }
}

#[derive(Default, Deserialize, Serialize, Clone)]
pub struct WorkItems {
    ordered_items: Vec<WorkItemId>,
    pub work_items: HashMap<WorkItemId, WorkItem>,
}

impl WorkItems {
    pub fn add(&mut self, item: WorkItem) {
        let issue_id = item.id.clone();

        self.work_items.insert(issue_id.clone(), item);
        self.ordered_items.push(issue_id);
    }

    pub fn iter(&self) -> std::slice::Iter<'_, WorkItemId> {
        self.ordered_items.iter()
    }

    pub fn get(&self, id: &WorkItemId) -> Option<&WorkItem> {
        self.work_items.get(id)
    }

    pub fn get_roots(&self) -> Vec<WorkItemId> {
        let mut unreferenced_items: HashSet<&WorkItemId> =
            HashSet::from_iter(self.ordered_items.iter());

        for item in self.work_items.values() {
            if let Some(sub_issues) = item.get_sub_issues() {
                for issue_id in sub_issues {
                    unreferenced_items.remove(issue_id);
                }
            }
        }

        unreferenced_items.into_iter().cloned().collect()
    }
}

#[derive(Default, Debug, Eq, PartialEq)]
pub struct Changes {
    data: HashMap<ChangeKey, Change>,
}

impl Changes {
    pub fn add(&mut self, change: Change) {
        let old_value = self.data.insert(change.key(), change.clone());
        if let Some(old_value) = old_value {
            if change != old_value {
                println!("WARNING! {:?} overrides {:?}", change, old_value);
            }
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
pub struct ChangeKey {
    pub work_item_id: WorkItemId,
    pub data_type: Discriminant<ChangeData>,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Change {
    pub work_item_id: WorkItemId,
    pub data: ChangeData,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum ChangeData {
    Status(Option<String>),
    Blocked(Option<String>),
    Epic(Option<String>),
    SetParent(WorkItemId),
}

impl Change {
    fn key(&self) -> ChangeKey {
        ChangeKey {
            work_item_id: self.work_item_id.clone(),
            data_type: std::mem::discriminant(&self.data),
        }
    }

    pub fn describe(&self, work_items: &WorkItems) -> String {
        let work_item = work_items.get(&self.work_item_id).unwrap();

        let old_value = match self.data {
            ChangeData::Status(_) => work_item.project_item.status.field_value(),
            ChangeData::Blocked(_) => work_item.project_item.blocked.field_value(),
            ChangeData::Epic(_) => work_item.project_item.epic.field_value(),
            ChangeData::SetParent(_) => match &work_item.data {
                WorkItemData::Issue(issue) => issue.parent_id.as_ref().map(|v| v.0.as_str()),
                _ => None,
            },
        }
        .unwrap_or("<>");

        let name = match self.data {
            ChangeData::Status(_) => "Status",
            ChangeData::Blocked(_) => "Blocked",
            ChangeData::Epic(_) => "Epic",
            ChangeData::SetParent(_) => "SetParent",
        };

        let new_value = match &self.data {
            ChangeData::Status(value) => value.as_ref(),
            ChangeData::Blocked(value) => value.as_ref(),
            ChangeData::Epic(value) => value.as_ref(),
            ChangeData::SetParent(value) => Some(&value.0),
        }
        .map(|v| v.as_str())
        .unwrap_or("<>");

        format!("{}({} -> {})", name, old_value, new_value).to_owned()
    }
}

pub mod test_helpers {
    use super::*;

    #[derive(Default)]
    pub struct TestData {
        pub work_items: WorkItems,
        next_id: i32,
    }

    impl TestData {
        pub fn next_id(&mut self) -> WorkItemId {
            self.next_id += 1;
            WorkItemId::from(format!("{}", self.next_id))
        }

        pub fn add_work_item(&mut self, item: WorkItem) {
            self.work_items.add(item);
        }

        pub fn build(&mut self) -> TestDataWorkItemBuilder {
            let id = self.next_id();

            TestDataWorkItemBuilder {
                data: self,
                item: WorkItem {
                    id,
                    ..Default::default()
                },
            }
        }

        pub fn add_blank_issue<const S: usize, const T: usize>(
            &mut self,
            sub_issues: [&WorkItemId; S],
            tracked_issues: [&WorkItemId; T],
        ) -> WorkItemId {
            self.build()
                .sub_issues(&sub_issues)
                .tracked_issues(&tracked_issues)
                .add()
        }
    }

    pub struct TestDataWorkItemBuilder<'a> {
        data: &'a mut TestData,
        item: WorkItem,
    }

    impl TestDataWorkItemBuilder<'_> {
        pub fn add(self) -> WorkItemId {
            let id = self.item.id.clone();
            self.data.add_work_item(self.item);
            id
        }

        pub fn issue_state(mut self, state: IssueState) -> Self {
            self.get_issue().state = state;
            self
        }

        pub fn sub_issues(mut self, ids: &[&WorkItemId]) -> Self {
            self.get_issue().sub_issues = to_project_item_ref_vec(ids);
            self
        }

        pub fn tracked_issues(mut self, ids: &[&WorkItemId]) -> Self {
            self.get_issue().tracked_issues = to_project_item_ref_vec(ids);
            self
        }

        fn get_issue(&mut self) -> &mut Issue {
            if let WorkItemData::DraftIssue = self.item.data {
                self.item.data = WorkItemData::Issue(Issue::default());
            }

            if let WorkItemData::PullRequest(_) = self.item.data {
                panic!("Cannot set Issue field on PullRequest")
            }

            if let WorkItemData::Issue(issue) = &mut self.item.data {
                return issue;
            }

            panic!("This shouldn't happen");
        }

        pub fn status(mut self, name: &str) -> Self {
            self.item.project_item.status = Some(SingleSelectFieldValue::from_name(name));
            self
        }

        pub fn project_milestone(mut self, name: &str) -> Self {
            self.item.project_item.project_milestone =
                Some(SingleSelectFieldValue::from_name(name));
            self
        }

        pub fn epic(mut self, name: &str) -> Self {
            self.item.project_item.epic = Some(SingleSelectFieldValue::from_name(name));
            self
        }
    }

    fn to_project_item_ref_vec(ids: &[&WorkItemId]) -> Vec<WorkItemId> {
        ids.iter().map(|id| (*id).to_owned()).collect()
    }

    impl WorkItem {
        pub fn new_blank_issue(sub_issues: &[&WorkItemId], tracked_issues: &[&WorkItemId]) -> Self {
            WorkItem {
                data: WorkItemData::Issue(Issue {
                    sub_issues: to_project_item_ref_vec(sub_issues),
                    tracked_issues: to_project_item_ref_vec(tracked_issues),
                    ..Default::default()
                }),
                ..Default::default()
            }
        }
    }
}

#[cfg(test)]
pub mod tests {
    use std::collections::HashSet;
    use test_helpers::TestData;

    use super::*;

    #[test]
    fn test_resolve() {
        let mut data = TestData::default();

        let a = data.add_blank_issue([], []);
        let b = data.add_blank_issue([], []);

        let c = data.add_blank_issue([&a], [&a]);
        let d = data.add_blank_issue([&a, &b], [&a, &b]);

        let unresolvable = data.next_id();

        let root1 = data.add_blank_issue([&c], [&d, &unresolvable]);
        let root2 = data.add_blank_issue([&a], [&d, &unresolvable]);
        let root3 = data.add_blank_issue([&c, &unresolvable], [&b, &unresolvable]);

        let roots: HashSet<WorkItemId> = HashSet::from_iter(data.work_items.get_roots());

        // Roots only looks at sub_issues
        assert_eq!(4, roots.len());
        assert!(roots.contains(&d));
        assert!(roots.contains(&root1));
        assert!(roots.contains(&root2));
        assert!(roots.contains(&root3));
    }
}
