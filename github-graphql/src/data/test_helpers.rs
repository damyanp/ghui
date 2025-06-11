use super::*;

pub struct TestData {
    pub work_items: WorkItems,
    pub fields: Fields,
    next_id: i32,
}

impl Default for TestData {
    fn default() -> Self {
        Self {
            work_items: Default::default(),
            fields: Fields::test(),
            next_id: Default::default(),
        }
    }
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
        let id = self.data.fields.status.option_id(Some(name)).cloned();
        self.item.project_item.status = id;
        self
    }

    pub fn project_milestone(mut self, name: &str) -> Self {
        let id = self
            .data
            .fields
            .project_milestone
            .option_id(Some(name))
            .cloned();
        self.item.project_item.project_milestone = id.into();
        self
    }

    pub fn epic(mut self, name: &str) -> Self {
        let id = self.data.fields.epic.option_id(Some(name)).cloned();
        assert!(id.is_some());
        self.item.project_item.epic = id;
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

pub trait FieldOptionTestData {
    fn generate() -> Self;
}

impl FieldOptionTestData for SingleSelect {
    fn generate() -> Self {
        SingleSelect
    }
}

impl FieldOptionTestData for Iteration {
    fn generate() -> Self {
        Iteration {
            start_date: "2025-01-01".to_owned(),
            duration: 1,
        }
    }
}

impl<T: FieldOptionTestData> Field<T> {
    pub fn test(name: &str, options: &[&str]) -> Self {
        Field {
            id: FieldId(format!("id{name}")),
            name: name.to_owned(),
            options: options
                .iter()
                .map(|name| FieldOption {
                    id: FieldOptionId(format!("id({name})")),
                    value: name.to_string(),
                    data: T::generate(),
                })
                .collect(),
        }
    }
}

impl Fields {
    pub fn test() -> Self {
        Fields {
            project_id: "project_id".to_owned(),
            status: Field::test("status", &["Active", "Open", "Closed"]),
            blocked: Field::test("blocked", &["PR"]),
            epic: Field::test(
                "epic",
                &[
                    "DML Demo",
                    "MiniEngine Demo",
                    "SM 6.9 Preview",
                    "DXC 2025 Q4",
                    "Do Not Change",
                    "EpicA",
                    "EpicB",
                ],
            ),
            iteration: Field::test("iteration", &["S1", "S2"]),
            project_milestone: Field::test(
                "Project Milestone",
                &[
                    "3: ML preview requirements",
                    "4: ML preview planning",
                    "5: ML preview implementation",
                    "Graphics preview feature analysis",
                    "DXC: SM 6.9 Preview",
                    "DXC: SM 6.9 Release",
                    "Another Project Milestone",
                ],
            ),
            kind: Field::test("Kind", &["Bug", "Task"]),
        }
    }
}
