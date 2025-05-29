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
                ..WorkItem::default_loaded()
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
        self.get_issue().state = state.into();
        self
    }

    pub fn sub_issues(mut self, ids: &[&WorkItemId]) -> Self {
        self.get_issue().sub_issues = to_project_item_ref_vec(ids);
        self
    }

    pub fn tracked_issues(mut self, ids: &[&WorkItemId]) -> Self {
        self.get_issue().tracked_issues = to_project_item_ref_vec(ids).into();
        self
    }

    pub fn issue(mut self) -> Self {
        self.get_issue();
        self
    }

    fn get_issue(&mut self) -> &mut Issue {
        if let WorkItemData::DraftIssue = self.item.data {
            self.item.data = WorkItemData::Issue(Issue::default_loaded());
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
        self.item.project_item.status = Some(SingleSelectFieldValue::from_name(name)).into();
        self
    }

    pub fn project_milestone(mut self, name: &str) -> Self {
        self.item.project_item.project_milestone =
            Some(SingleSelectFieldValue::from_name(name)).into();
        self
    }

    pub fn epic(mut self, name: &str) -> Self {
        self.item.project_item.epic = Some(SingleSelectFieldValue::from_name(name)).into();
        self
    }
}

fn to_project_item_ref_vec(ids: &[&WorkItemId]) -> Vec<WorkItemId> {
    ids.iter().map(|id| (*id).to_owned()).collect()
}

impl WorkItem {
    pub fn default_loaded() -> Self {
        WorkItem {
            project_item: ProjectItem::default_loaded(),
            ..Default::default()
        }
    }
    pub fn new_blank_issue(sub_issues: &[&WorkItemId], tracked_issues: &[&WorkItemId]) -> Self {
        WorkItem {
            data: WorkItemData::Issue(Issue {
                sub_issues: to_project_item_ref_vec(sub_issues),
                tracked_issues: to_project_item_ref_vec(tracked_issues).into(),
                ..Issue::default_loaded()
            }),
            ..WorkItem::default_loaded()
        }
    }
}

impl SingleSelectFieldValue {
    fn from_name(name: &str) -> SingleSelectFieldValue {
        SingleSelectFieldValue {
            name: name.to_owned(),
            ..Default::default()
        }
    }
}
