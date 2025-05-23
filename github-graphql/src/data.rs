use crate::{
    client::{
        graphql::{
            add_sub_issue, clear_project_field_value,
            custom_fields_query::{Field, Fields},
            set_project_field_value,
        },
        transport::Client,
    },
    Result,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{hash_map, HashMap},
    mem::{swap, Discriminant},
};
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

#[derive(Default, Deserialize, Serialize, Clone, Debug, Eq, PartialEq)]
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

    fn get_mut(&mut self, id: &WorkItemId) -> Option<&mut WorkItem> {
        self.work_items.get_mut(id)
    }

    pub fn get_roots(&self) -> Vec<WorkItemId> {
        let mut unreferenced_items: HashMap<&WorkItemId, usize> = HashMap::from_iter(
            self.ordered_items
                .iter()
                .enumerate()
                .map(|(index, item)| (item, index)),
        );

        for item in self.work_items.values() {
            if let Some(sub_issues) = item.get_sub_issues() {
                for issue_id in sub_issues {
                    unreferenced_items.remove(issue_id);
                }
            }
        }

        let mut unreferenced_items: Vec<_> = unreferenced_items.into_iter().collect();
        unreferenced_items.sort_by_key(|(_, index)| *index);

        unreferenced_items
            .into_iter()
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn convert_tracked_to_sub_issues(&self, id: &WorkItemId) -> Changes {
        let mut changes = Changes::default();

        if let Some(WorkItem {
            data: WorkItemData::Issue(parent_issue),
            ..
        }) = self.get(id)
        {
            for tracked_issue_id in &parent_issue.tracked_issues {
                if let Some(WorkItem {
                    data: WorkItemData::Issue(tracked_issue),
                    ..
                }) = self.get(tracked_issue_id)
                {
                    // only set parents on issues that aren't currently parented
                    if tracked_issue.parent_id.is_none() {
                        changes.add(Change {
                            work_item_id: tracked_issue_id.clone(),
                            data: ChangeData::SetParent(id.clone()),
                        });
                    }
                }
            }
        }

        changes
    }

    pub fn apply_changes(&mut self, changes: &Changes) -> HashMap<WorkItemId, WorkItem> {
        let mut originals = HashMap::<WorkItemId, WorkItem>::default();

        let mut remember_original = |work_item: Option<&WorkItem>| {
            if let Some(work_item) = work_item {
                let id = &work_item.id;
                if !originals.contains_key(id) {
                    originals.insert(id.clone(), work_item.clone());
                }
            }
        };

        for change in changes {
            let original = self.get(&change.work_item_id);
            if original.is_none() {
                println!(
                    "WARNING: change for '{0}' - work item not found",
                    change.work_item_id.0
                );
                continue;
            }

            remember_original(original);

            match &change.data {
                ChangeData::Status(_) => todo!(),
                ChangeData::Blocked(_) => todo!(),
                ChangeData::Epic(_) => todo!(),
                ChangeData::SetParent(new_parent_id) => {
                    remember_original(self.get(new_parent_id));

                    let child_id = &change.work_item_id;

                    // If there was an old parent we'll need to update it
                    let old_parent_id = if let Some(WorkItem {
                        data: WorkItemData::Issue(Issue { parent_id, .. }),
                        ..
                    }) = &self.get(child_id)
                    {
                        parent_id.to_owned()
                    } else {
                        None
                    };

                    if let Some(old_parent_id) = &old_parent_id {
                        remember_original(self.get(old_parent_id));
                        if let Some(old_parent) = self.get_mut(old_parent_id) {
                            if let WorkItemData::Issue(issue) = &mut old_parent.data {
                                issue.sub_issues.pop_if(|i| i == child_id);
                            } else {
                                println!("WARNING: old parent '{0}' not an issue", old_parent_id.0);
                            }
                        } else {
                            println!("WARNING: old parent '{0}' not found", old_parent_id.0);
                        }
                    }

                    if let Some(child) = self.get_mut(child_id) {
                        if let WorkItemData::Issue(issue) = &mut child.data {
                            issue.parent_id = Some(new_parent_id.clone());
                        } else {
                            println!("WARNING: child '{0}' not an issue", child_id.0);
                        }
                    } else {
                        println!("WARNING: child '{0}' not found", child_id.0);
                    }

                    if let Some(parent) = self.get_mut(new_parent_id) {
                        if let WorkItemData::Issue(issue) = &mut parent.data {
                            issue.sub_issues.push(child_id.clone());
                        } else {
                            println!("WARNING: new parent '{0}' not an issue", new_parent_id.0);
                        }
                    } else {
                        println!("WARNING: new parent '{0}' not found", new_parent_id.0);
                    }
                }
            }
        }

        originals
    }
}

#[derive(Default, Debug, Eq, PartialEq, Serialize, TS, Clone)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
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

    pub fn add_changes(&mut self, changes: Changes) {
        for change in changes.data.into_values() {
            self.add(change);
        }
    }

    pub async fn save(
        &mut self,
        client: &impl Client,
        fields: &Fields,
        work_items: &WorkItems,
        mode: SaveMode,
        report_progress: &impl Fn(&Change, usize, usize),
    ) -> Result<()> {
        let mut data = HashMap::default();
        swap(&mut data, &mut self.data);

        let change_count = data.len();

        for (change_number, (key, change)) in data.into_iter().enumerate() {
            let result = if let SaveMode::Commit = mode {
                change.save(client, fields, work_items).await
            } else {
                Ok(())
            };

            report_progress(&change, change_number, change_count);

            if result.is_err() {
                println!("WARNING: save for {:?} failed {result:?}", change.key());
                self.data.insert(key, change);
            }
        }

        Ok(())
    }
}

pub enum SaveMode {
    DryRun,
    Commit,
}

impl Change {
    async fn save(
        &self,
        client: &impl Client,
        fields: &Fields,
        work_items: &WorkItems,
    ) -> Result<()> {
        match &self.data {
            ChangeData::Status(value) => {
                self.save_field(
                    client,
                    work_items,
                    &fields.project_id,
                    &fields.status,
                    value,
                )
                .await
            }
            ChangeData::Blocked(value) => {
                self.save_field(
                    client,
                    work_items,
                    &fields.project_id,
                    &fields.blocked,
                    value,
                )
                .await
            }
            ChangeData::Epic(value) => {
                self.save_field(client, work_items, &fields.project_id, &fields.epic, value)
                    .await
            }
            ChangeData::SetParent(new_parent) => {
                add_sub_issue::add(client, &new_parent.0, &self.work_item_id.0).await
            }
        }
    }

    async fn save_field(
        &self,
        client: &impl Client,
        work_items: &WorkItems,
        project_id: &str,
        field: &Field,
        value: &Option<String>,
    ) -> Result<()> {
        if let Some(project_item_id) = work_items
            .get(&self.work_item_id)
            .map(|item| &item.project_item.id)
        {
            if let Some(new_value_id) = value.as_ref().and_then(|name| field.id(name)) {
                set_project_field_value::set(
                    client,
                    project_id,
                    project_item_id,
                    &field.id,
                    new_value_id,
                )
                .await?;
            } else {
                clear_project_field_value::clear(client, project_id, project_item_id, &field.id)
                    .await?;
            }
        }
        Ok(())
    }
}

impl<'a> IntoIterator for &'a Changes {
    type Item = &'a Change;

    type IntoIter = hash_map::Values<'a, ChangeKey, Change>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.values()
    }
}

#[derive(Debug, Eq, PartialEq, Hash, TS, Clone)]
#[ts(as = "String")]
pub struct ChangeKey {
    pub work_item_id: WorkItemId,
    pub data_type: Discriminant<ChangeData>,
}

impl serde::Serialize for ChangeKey {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(format!("{}-{:?}", self.work_item_id.0, self.data_type).as_str())
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    pub work_item_id: WorkItemId,
    pub data: ChangeData,
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase", tag = "type", content = "value")]
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
            // Set parent_id on any sub issues
            if let WorkItemData::Issue(issue) = &item.data {
                for sub_issue in &issue.sub_issues {
                    if let Some(WorkItem {
                        data: WorkItemData::Issue(sub_issue),
                        ..
                    }) = self.work_items.get_mut(sub_issue)
                    {
                        sub_issue.parent_id = Some(item.id.clone());
                    }
                }
            }

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

        pub fn issue(mut self) -> Self {
            self.get_issue();
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

    #[test]
    fn test_convert_tracked_to_sub_issues() {
        let mut data = TestData::default();

        let tracked_issue = data.build().issue().add();
        let sub_issue = data.build().issue().add();
        let issue_not_in_project = WorkItemId("not-in-project".to_owned());
        let issue_with_other_parent = data.build().issue().add();

        let parent = data
            .build()
            .tracked_issues(&[
                &tracked_issue,
                &sub_issue,
                &issue_not_in_project,
                &issue_with_other_parent,
            ])
            .sub_issues(&[&sub_issue])
            .add();

        let _other_parent = data.build().sub_issues(&[&issue_with_other_parent]).add();

        let mut expected_changes = Changes::default();
        expected_changes.add(Change {
            work_item_id: tracked_issue,
            data: ChangeData::SetParent(parent.clone()),
        });
        // sub_issue - we don't expect the parent to be changed for this because
        // it is already a sub-issue
        //
        // issue_not_in_project - we don't expect this to be changed because it
        // isn't in the project
        //
        // issue_with_other_parent - we don't expect this to be changed because
        // we only want to set new parents, not change existing ones.

        let actual_changes = data.work_items.convert_tracked_to_sub_issues(&parent);

        assert_eq!(actual_changes, expected_changes);
    }

    #[test]
    fn test_apply_changes_no_changes() {
        let mut data = TestData::default();
        data.build().add();
        data.build().add();

        let unmodified_work_items = data.work_items.clone();

        let changes = Default::default();

        let original_work_items = data.work_items.apply_changes(&changes);

        assert_eq!(unmodified_work_items, data.work_items);
        assert_eq!(original_work_items, Default::default());
    }

    #[test]
    fn test_apply_set_new_parent() {
        let mut data = TestData::default();
        let parent = data.build().issue().add();
        let child = data.build().issue().add();

        let mut changes = Changes::default();
        changes.add(Change {
            work_item_id: child.clone(),
            data: ChangeData::SetParent(parent.clone()),
        });

        // All the items have changed, so we expect to get back a map containing
        // all the originals
        let expected_original_work_items = data.work_items.work_items.clone();

        let actual_original_work_items = data.work_items.apply_changes(&changes);

        assert_eq!(expected_original_work_items, actual_original_work_items);

        let actual_sub_issues = data
            .work_items
            .get(&parent)
            .unwrap()
            .get_sub_issues()
            .unwrap();
        assert_eq!(&vec![child.clone()], actual_sub_issues);

        let actual_parent = match &data.work_items.get(&child).unwrap().data {
            WorkItemData::Issue(issue) => issue.parent_id.clone(),
            _ => panic!(),
        };
        assert_eq!(Some(parent.clone()), actual_parent);
    }

    #[test]
    fn test_apply_changes_update_parent() {
        let mut data = TestData::default();

        let child = data.build().issue().add();
        let old_parent = data.build().sub_issues(&[&child]).add();
        let new_parent = data.build().issue().add();

        let mut changes = Changes::default();
        changes.add(Change {
            work_item_id: child.clone(),
            data: ChangeData::SetParent(new_parent.clone()),
        });

        // All the items have changed, so we expect to get back a map containing
        // all the originals
        let expected_original_work_items = data.work_items.work_items.clone();

        let actual_original_work_items = data.work_items.apply_changes(&changes);

        assert_eq!(expected_original_work_items, actual_original_work_items);

        let actual_old_parent_sub_issues = data
            .work_items
            .get(&old_parent)
            .unwrap()
            .get_sub_issues()
            .unwrap();
        assert_eq!(actual_old_parent_sub_issues.len(), 0);

        let actual_new_parent_sub_issues = data
            .work_items
            .get(&new_parent)
            .unwrap()
            .get_sub_issues()
            .unwrap();

        assert_eq!(&vec![child.clone()], actual_new_parent_sub_issues);

        let actual_parent = match &data.work_items.get(&child).unwrap().data {
            WorkItemData::Issue(issue) => issue.parent_id.clone(),
            _ => panic!(),
        };
        assert_eq!(Some(new_parent.clone()), actual_parent);
    }

    #[test]
    fn test_apply_changes_item_not_found() {
        let mut data = TestData::default();
        let parent = data.build().issue().add();

        let mut changes = Changes::default();
        changes.add(Change {
            work_item_id: WorkItemId("id-that-does-not-exist".to_owned()),
            data: ChangeData::SetParent(parent.clone()),
        });

        // no items should change
        let work_items_before = data.work_items.work_items.clone();
        let expected_original_work_items = HashMap::default();
        let actual_original_work_items = data.work_items.apply_changes(&changes);

        assert_eq!(expected_original_work_items, actual_original_work_items);
        assert_eq!(work_items_before, data.work_items.work_items);
    }
}
