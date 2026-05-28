use std::{cmp::Ordering, collections::HashMap};

use github_graphql::{
    data::{
        Field, FieldOptionId, Fields, Issue, IssueState, PullRequest, PullRequestState, WorkItem,
        WorkItemData, WorkItemId, WorkItems,
    },
    pivot::{Axis, MultiValueStrategy, PivotConfig, PivotField},
};

use crate::Filters;

use super::{Node, NodeData};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum SortValue {
    Index(usize),
    Text(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FieldValue {
    key: String,
    label: String,
    sort_value: SortValue,
    field_option_id: Option<FieldOptionId>,
}

struct Bucket {
    key: String,
    label: String,
    sort_value: SortValue,
    field_option_id: Option<FieldOptionId>,
    items: Vec<WorkItemId>,
}

pub(crate) struct RecipeNodeBuilder<'a> {
    fields: &'a Fields,
    work_items: &'a WorkItems,
    filters: &'a Filters,
    original_work_items: &'a HashMap<WorkItemId, WorkItem>,
    pivot_config: &'a PivotConfig,
    nodes: Vec<Node>,
}

impl<'a> RecipeNodeBuilder<'a> {
    pub fn new(
        fields: &'a Fields,
        work_items: &'a WorkItems,
        filters: &'a Filters,
        original_work_items: &'a HashMap<WorkItemId, WorkItem>,
        pivot_config: &'a PivotConfig,
    ) -> Self {
        Self {
            fields,
            work_items,
            filters,
            original_work_items,
            pivot_config,
            nodes: Vec::new(),
        }
    }

    pub fn build(&mut self) -> Vec<Node> {
        let items = self
            .work_items
            .iter()
            .filter(|id| self.should_include(id))
            .cloned()
            .collect();
        self.render_scope(items, &self.pivot_config.recipe, 0, "");
        std::mem::take(&mut self.nodes)
    }

    fn render_scope(&mut self, items: Vec<WorkItemId>, recipe: &[Axis], level: u32, path: &str) {
        if items.is_empty() {
            return;
        }

        if recipe.is_empty() {
            let mut items = items;
            items.sort_by(|a, b| self.item_title(a).cmp(self.item_title(b)));
            for id in items {
                self.push_item(&id, level, false, false, path);
            }
            return;
        }

        match &recipe[0] {
            Axis::Pivot(field) | Axis::Group(field) => {
                for bucket in self.bucket_by_field(items, field) {
                    let group_id = self.group_node_id(path, field, &bucket.key);
                    self.nodes.push(Node {
                        level,
                        id: group_id.clone(),
                        data: NodeData::Group {
                            name: bucket.label,
                            field_option_id: bucket.field_option_id,
                        },
                        has_children: !bucket.items.is_empty(),
                        is_modified: false,
                        is_ghost: false,
                    });
                    self.render_scope(bucket.items, &recipe[1..], level + 1, &group_id);
                }
            }
            Axis::Sort(field) => {
                self.render_scope(self.sort_items(items, field), &recipe[1..], level, path);
            }
            Axis::Hierarchy => self.render_hierarchy(items, &recipe[1..], level, path),
        }
    }

    fn render_hierarchy(
        &mut self,
        items: Vec<WorkItemId>,
        child_recipe: &[Axis],
        level: u32,
        path: &str,
    ) {
        let in_scope = std::collections::HashSet::<WorkItemId>::from_iter(items.iter().cloned());
        let roots = self.roots_in_scope(&items, &in_scope);

        if self.pivot_config.show_ghost_ancestors {
            let mut ghost_ids = std::collections::HashSet::new();
            for root in &roots {
                let mut parent = self.parent_id(root);
                while let Some(parent_id) = parent {
                    // Skip parents that don't exist in work_items: they can't
                    // be rendered and would silently swallow all descendants.
                    if self.item(&parent_id).is_none() {
                        break;
                    }
                    if in_scope.contains(&parent_id) || ghost_ids.contains(&parent_id) {
                        break;
                    }
                    ghost_ids.insert(parent_id.clone());
                    parent = self.parent_id(&parent_id);
                }
            }

            if !ghost_ids.is_empty() {
                let mut scope_ids = in_scope.clone();
                scope_ids.extend(ghost_ids.iter().cloned());
                let mut display_roots: Vec<_> = scope_ids.iter().cloned().collect();
                display_roots.retain(|id| {
                    self.parent_id(id)
                        .as_ref()
                        .is_none_or(|parent| !scope_ids.contains(parent))
                });
                self.render_tree(
                    display_roots,
                    &scope_ids,
                    &ghost_ids,
                    child_recipe,
                    level,
                    path,
                );
                return;
            }
        }

        self.render_tree(
            roots,
            &in_scope,
            &std::collections::HashSet::new(),
            child_recipe,
            level,
            path,
        );
    }

    fn render_tree(
        &mut self,
        mut roots: Vec<WorkItemId>,
        scope_ids: &std::collections::HashSet<WorkItemId>,
        ghost_ids: &std::collections::HashSet<WorkItemId>,
        child_recipe: &[Axis],
        level: u32,
        path: &str,
    ) {
        roots.sort_by(|a, b| self.item_title(a).cmp(self.item_title(b)));
        for root in roots {
            self.render_tree_node(&root, scope_ids, ghost_ids, child_recipe, level, path);
        }
    }

    fn render_tree_node(
        &mut self,
        id: &WorkItemId,
        scope_ids: &std::collections::HashSet<WorkItemId>,
        ghost_ids: &std::collections::HashSet<WorkItemId>,
        child_recipe: &[Axis],
        level: u32,
        path: &str,
    ) {
        let children = self.children_in_scope(id, scope_ids);
        self.push_item(
            id,
            level,
            ghost_ids.contains(id),
            !children.is_empty(),
            path,
        );

        if children.is_empty() {
            return;
        }

        let child_path = self.child_path(path, id);
        if child_recipe.is_empty() {
            let mut children = children;
            children.sort_by(|a, b| self.item_title(a).cmp(self.item_title(b)));
            for child in children {
                self.render_tree_node(
                    &child,
                    scope_ids,
                    ghost_ids,
                    child_recipe,
                    level + 1,
                    &child_path,
                );
            }
            return;
        }

        self.render_scope_in_tree(
            children,
            child_recipe,
            level + 1,
            scope_ids,
            ghost_ids,
            &child_path,
        );
    }

    fn render_scope_in_tree(
        &mut self,
        items: Vec<WorkItemId>,
        recipe: &[Axis],
        level: u32,
        scope_ids: &std::collections::HashSet<WorkItemId>,
        ghost_ids: &std::collections::HashSet<WorkItemId>,
        path: &str,
    ) {
        if items.is_empty() {
            return;
        }

        if recipe.is_empty() {
            let mut items = items;
            items.sort_by(|a, b| self.item_title(a).cmp(self.item_title(b)));
            for id in items {
                self.render_tree_node(&id, scope_ids, ghost_ids, recipe, level, path);
            }
            return;
        }

        match &recipe[0] {
            Axis::Pivot(field) | Axis::Group(field) => {
                for bucket in self.bucket_by_field(items, field) {
                    let group_id = self.group_node_id(path, field, &bucket.key);
                    self.nodes.push(Node {
                        level,
                        id: group_id.clone(),
                        data: NodeData::Group {
                            name: bucket.label,
                            field_option_id: bucket.field_option_id,
                        },
                        has_children: !bucket.items.is_empty(),
                        is_modified: false,
                        is_ghost: false,
                    });
                    self.render_scope_in_tree(
                        bucket.items,
                        &recipe[1..],
                        level + 1,
                        scope_ids,
                        ghost_ids,
                        &group_id,
                    );
                }
            }
            Axis::Sort(field) => {
                self.render_scope_in_tree(
                    self.sort_items(items, field),
                    &recipe[1..],
                    level,
                    scope_ids,
                    ghost_ids,
                    path,
                );
            }
            Axis::Hierarchy => {
                self.render_scope_in_tree(items, &recipe[1..], level, scope_ids, ghost_ids, path);
            }
        }
    }

    fn bucket_by_field(&self, items: Vec<WorkItemId>, field: &PivotField) -> Vec<Bucket> {
        let mut buckets = Vec::new();
        let mut bucket_indexes = HashMap::<String, usize>::new();

        for id in items {
            let values = self.field_values(&id, field);
            if values.is_empty() {
                let value = FieldValue {
                    key: "(none)".to_owned(),
                    label: "(none)".to_owned(),
                    sort_value: SortValue::Index(usize::MAX),
                    field_option_id: None,
                };
                let index = Self::bucket_index(&mut buckets, &mut bucket_indexes, &value);
                buckets[index].items.push(id);
                continue;
            }

            for value in values {
                let index = Self::bucket_index(&mut buckets, &mut bucket_indexes, &value);
                buckets[index].items.push(id.clone());
            }
        }

        buckets.sort_by(|a, b| {
            a.sort_value
                .cmp(&b.sort_value)
                .then_with(|| a.label.cmp(&b.label))
        });
        buckets
    }

    fn bucket_index(
        buckets: &mut Vec<Bucket>,
        bucket_indexes: &mut HashMap<String, usize>,
        value: &FieldValue,
    ) -> usize {
        if let Some(index) = bucket_indexes.get(&value.key) {
            return *index;
        }

        let index = buckets.len();
        buckets.push(Bucket {
            key: value.key.clone(),
            label: value.label.clone(),
            sort_value: value.sort_value.clone(),
            field_option_id: value.field_option_id.clone(),
            items: Vec::new(),
        });
        bucket_indexes.insert(value.key.clone(), index);
        index
    }

    fn sort_items(&self, mut items: Vec<WorkItemId>, field: &PivotField) -> Vec<WorkItemId> {
        items.sort_by(|a, b| self.compare_optional_field_values(a, b, field));
        items
    }

    fn compare_optional_field_values(
        &self,
        left: &WorkItemId,
        right: &WorkItemId,
        field: &PivotField,
    ) -> Ordering {
        match (
            self.field_values(left, field).into_iter().next(),
            self.field_values(right, field).into_iter().next(),
        ) {
            (Some(left), Some(right)) => left.sort_value.cmp(&right.sort_value),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        }
    }

    fn roots_in_scope(
        &self,
        items: &[WorkItemId],
        scope_ids: &std::collections::HashSet<WorkItemId>,
    ) -> Vec<WorkItemId> {
        items
            .iter()
            .filter(|id| {
                self.parent_id(id)
                    .as_ref()
                    .is_none_or(|parent| !scope_ids.contains(parent))
            })
            .cloned()
            .collect()
    }

    fn children_in_scope(
        &self,
        id: &WorkItemId,
        scope_ids: &std::collections::HashSet<WorkItemId>,
    ) -> Vec<WorkItemId> {
        self.item(id)
            .and_then(WorkItem::get_sub_issues)
            .map(|children| {
                children
                    .iter()
                    .filter(|child| scope_ids.contains(*child))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    fn field_values(&self, id: &WorkItemId, field: &PivotField) -> Vec<FieldValue> {
        let Some(item) = self.item(id) else {
            return Vec::new();
        };

        match field {
            PivotField::Status => {
                self.option_field_values(item.project_item.status.as_ref(), &self.fields.status)
            }
            PivotField::Blocked => self.option_field_values(
                item.project_item.blocked.expect_loaded().as_ref(),
                &self.fields.blocked,
            ),
            PivotField::Epic => {
                self.option_field_values(item.project_item.epic.as_ref(), &self.fields.epic)
            }
            PivotField::Iteration => self.option_field_values(
                item.project_item.iteration.expect_loaded().as_ref(),
                &self.fields.iteration,
            ),
            PivotField::Kind => self.option_field_values(
                item.project_item.kind.expect_loaded().as_ref(),
                &self.fields.kind,
            ),
            PivotField::Workstream => self.option_field_values(
                item.project_item.workstream.expect_loaded().as_ref(),
                &self.fields.workstream,
            ),
            PivotField::Estimate => {
                self.option_field_values(item.project_item.estimate.as_ref(), &self.fields.estimate)
            }
            PivotField::Priority => {
                self.option_field_values(item.project_item.priority.as_ref(), &self.fields.priority)
            }
            PivotField::Assignee => self.assignee_field_values(item),
            PivotField::Repository => item
                .repo_name_with_owner
                .as_ref()
                .map(|repo| vec![Self::text_field_value(repo.clone())])
                .unwrap_or_default(),
            PivotField::IssueType => match &item.data {
                WorkItemData::Issue(issue) => issue
                    .issue_type
                    .expect_loaded()
                    .as_ref()
                    .map(|issue_type| vec![Self::text_field_value(issue_type.clone())])
                    .unwrap_or_default(),
                WorkItemData::DraftIssue | WorkItemData::PullRequest(_) => Vec::new(),
            },
            PivotField::Type => vec![Self::type_field_value(&item.data)],
            PivotField::State => Self::state_field_values(&item.data),
        }
    }

    fn option_field_values<T>(
        &self,
        id: Option<&FieldOptionId>,
        field: &Field<T>,
    ) -> Vec<FieldValue> {
        id.map(|id| {
            vec![FieldValue {
                key: id.0.clone(),
                label: field.option_name(Some(id)).unwrap_or(&id.0).to_owned(),
                sort_value: SortValue::Index(field.option_index(Some(id))),
                field_option_id: Some(id.clone()),
            }]
        })
        .unwrap_or_default()
    }

    fn assignee_field_values(&self, item: &WorkItem) -> Vec<FieldValue> {
        let assignees = match &item.data {
            WorkItemData::Issue(Issue { assignees, .. }) => assignees.as_slice(),
            WorkItemData::PullRequest(PullRequest { assignees, .. }) => assignees.as_slice(),
            WorkItemData::DraftIssue => &[],
        };

        if assignees.is_empty() {
            return Vec::new();
        }

        match self.pivot_config.multi_value_strategy {
            MultiValueStrategy::Combined => {
                let mut assignees = assignees.to_vec();
                assignees.sort();
                let key = assignees.join("+");
                let label = assignees
                    .iter()
                    .map(|assignee| format!("@{assignee}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                vec![FieldValue {
                    key,
                    label: label.clone(),
                    sort_value: SortValue::Text(label),
                    field_option_id: None,
                }]
            }
            MultiValueStrategy::Explode => {
                let mut assignees = assignees.to_vec();
                assignees.sort();
                assignees
                    .into_iter()
                    .map(|assignee| {
                        let label = format!("@{assignee}");
                        FieldValue {
                            key: assignee,
                            label: label.clone(),
                            sort_value: SortValue::Text(label),
                            field_option_id: None,
                        }
                    })
                    .collect()
            }
        }
    }

    fn type_field_value(data: &WorkItemData) -> FieldValue {
        let value = match data {
            WorkItemData::Issue(_) => ("issue", 0),
            WorkItemData::PullRequest(_) => ("pullRequest", 1),
            WorkItemData::DraftIssue => ("draftIssue", 2),
        };
        FieldValue {
            key: value.0.to_owned(),
            label: value.0.to_owned(),
            sort_value: SortValue::Index(value.1),
            field_option_id: None,
        }
    }

    fn state_field_values(data: &WorkItemData) -> Vec<FieldValue> {
        let value = match data {
            WorkItemData::Issue(Issue { state, .. }) => Some(match state.expect_loaded() {
                IssueState::OPEN => ("OPEN".to_owned(), 0),
                IssueState::CLOSED => ("CLOSED".to_owned(), 1),
                IssueState::Other(value) => (value.clone(), 99),
            }),
            WorkItemData::PullRequest(PullRequest { state, .. }) => {
                Some(match state.expect_loaded() {
                    PullRequestState::OPEN => ("OPEN".to_owned(), 0),
                    PullRequestState::CLOSED => ("CLOSED".to_owned(), 1),
                    PullRequestState::MERGED => ("MERGED".to_owned(), 2),
                    PullRequestState::Other(value) => (value.clone(), 99),
                })
            }
            WorkItemData::DraftIssue => None,
        };

        value
            .map(|(label, sort_index)| {
                vec![FieldValue {
                    key: label.clone(),
                    label,
                    sort_value: SortValue::Index(sort_index),
                    field_option_id: None,
                }]
            })
            .unwrap_or_default()
    }

    fn text_field_value(value: String) -> FieldValue {
        FieldValue {
            key: value.clone(),
            label: value.clone(),
            sort_value: SortValue::Text(value),
            field_option_id: None,
        }
    }

    fn push_item(
        &mut self,
        id: &WorkItemId,
        level: u32,
        is_ghost: bool,
        has_children: bool,
        path: &str,
    ) {
        if self.item(id).is_none() {
            return;
        }
        self.nodes.push(Node {
            level,
            id: self.child_path(path, id),
            data: NodeData::WorkItem {
                work_item_id: id.clone(),
            },
            has_children,
            is_modified: self.original_work_items.contains_key(id),
            is_ghost,
        });
    }

    fn item(&self, id: &WorkItemId) -> Option<&WorkItem> {
        self.work_items.get(id)
    }

    fn item_title<'b>(&'b self, id: &'b WorkItemId) -> &'b str {
        self.item(id).map(|item| item.title.as_str()).unwrap_or("")
    }

    fn parent_id(&self, id: &WorkItemId) -> Option<WorkItemId> {
        if let Some(parent_id) = self.item(id).and_then(WorkItem::get_parent) {
            return Some(parent_id.clone());
        }

        self.work_items.iter().find_map(|candidate_id| {
            self.item(candidate_id)
                .and_then(WorkItem::get_sub_issues)
                .filter(|children| children.contains(id))
                .map(|_| candidate_id.clone())
        })
    }

    fn group_node_id(&self, path: &str, field: &PivotField, key: &str) -> String {
        if path.is_empty() {
            format!("path/{}={key}", Self::field_key(field))
        } else {
            format!("{path}/{}={key}", Self::field_key(field))
        }
    }

    fn child_path(&self, path: &str, id: &WorkItemId) -> String {
        if path.is_empty() {
            format!("path/{}", id.0)
        } else {
            format!("{path}/{}", id.0)
        }
    }

    fn field_key(field: &PivotField) -> &'static str {
        match field {
            PivotField::Status => "status",
            PivotField::Blocked => "blocked",
            PivotField::Epic => "epic",
            PivotField::Iteration => "iteration",
            PivotField::Kind => "kind",
            PivotField::Workstream => "workstream",
            PivotField::Estimate => "estimate",
            PivotField::Priority => "priority",
            PivotField::Assignee => "assignee",
            PivotField::Repository => "repository",
            PivotField::IssueType => "issue_type",
            PivotField::Type => "type",
            PivotField::State => "state",
        }
    }

    fn should_include(&self, work_item_id: &WorkItemId) -> bool {
        let work_item = self.item(work_item_id);
        if let Some(work_item) = work_item {
            if let WorkItem {
                data: WorkItemData::Issue(issue),
                ..
            } = work_item
            {
                for child_id in &issue.sub_issues {
                    if self.should_include(child_id) {
                        return true;
                    }
                }
            }
            self.filters.should_include(work_item)
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use github_graphql::{
        data::{
            Issue, IssueState, ProjectItem, PullRequest, PullRequestState, UpdateType, WorkItem,
            WorkItemData, WorkItemId, test_helpers::TestData,
        },
        pivot::{Axis, MultiValueStrategy, PivotConfig, PivotField, parse_recipe},
    };
    use serde_json::Value;

    use crate::Filters;

    use super::*;

    #[test]
    fn test_recipe_builder_preset_snapshots() {
        let presets: Value = serde_json::from_str(include_str!(
            "../../../github-graphql/tests/fixtures/recipes.json"
        ))
        .unwrap();
        let mut preset_names = presets
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        preset_names.sort();

        let fixture = build_snapshot_fixture();
        let actual = preset_names
            .iter()
            .map(|preset| {
                let config = PivotConfig {
                    recipe: parse_recipe(preset).unwrap(),
                    multi_value_strategy: MultiValueStrategy::Combined,
                    show_ghost_ancestors: true,
                };
                let nodes = build_recipe_nodes(&fixture.fields, &fixture.work_items, &config);

                // HARD STOP: a duplicate Node.id panics Svelte's keyed each
                // block at runtime. Fail loudly here, before the opaque
                // string compare below ever runs, so the failure message
                // names the preset and the colliding ids.
                let label = format!("preset {preset:?}");
                assert_node_ids_unique(&label, &nodes);

                let total = nodes.len();
                let unique = nodes.iter().map(|n| &n.id).collect::<HashSet<_>>().len();
                format!(
                    "== {preset} ==\ntotal={total} unique_ids={unique}\n{}",
                    format_nodes_string(&nodes),
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        assert_eq!(
            actual,
            r#"== Hierarchy ==
total=6 unique_ids=6
0 item 2 ghost=false children=true
1 item 1 ghost=false children=false
0 item 4 ghost=false children=true
1 item 3 ghost=false children=false
0 item 5 ghost=false children=false
0 item 6 ghost=false children=false

== Hierarchy → Group(Status) ==
total=8 unique_ids=8
0 item 2 ghost=false children=true
1 group path/2/status=id(Active) Active
2 item 1 ghost=false children=false
0 item 4 ghost=false children=true
1 group path/4/status=id(Closed) Closed
2 item 3 ghost=false children=false
0 item 5 ghost=false children=false
0 item 6 ghost=false children=false

== Hierarchy → Group(Workstream) ==
total=8 unique_ids=8
0 item 2 ghost=false children=true
1 group path/2/workstream=id(WS1) WS1
2 item 1 ghost=false children=false
0 item 4 ghost=false children=true
1 group path/4/workstream=id(WS2) WS2
2 item 3 ghost=false children=false
0 item 5 ghost=false children=false
0 item 6 ghost=false children=false

== Pivot(Assignee) → Group(Epic) ==
total=15 unique_ids=15
0 group path/assignee=(none) (none)
1 group path/assignee=(none)/epic=(none) (none)
2 item 6 ghost=false children=false
0 group path/assignee=alice @alice
1 group path/assignee=alice/epic=id(EpicA) EpicA
2 item 1 ghost=false children=false
2 item 2 ghost=false children=false
1 group path/assignee=alice/epic=id(EpicB) EpicB
2 item 3 ghost=false children=false
0 group path/assignee=alice+bob @alice, @bob
1 group path/assignee=alice+bob/epic=(none) (none)
2 item 4 ghost=false children=false
0 group path/assignee=bob @bob
1 group path/assignee=bob/epic=id(EpicB) EpicB
2 item 5 ghost=false children=false

== Pivot(Epic) → Hierarchy ==
total=10 unique_ids=10
0 group path/epic=id(EpicA) EpicA
1 item 2 ghost=false children=true
2 item 1 ghost=false children=false
0 group path/epic=id(EpicB) EpicB
1 item 4 ghost=true children=true
2 item 3 ghost=false children=false
1 item 5 ghost=false children=false
0 group path/epic=(none) (none)
1 item 4 ghost=false children=false
1 item 6 ghost=false children=false

== Pivot(IssueType) → Group(Status) ==
total=16 unique_ids=16
0 group path/issue_type=(none) (none)
1 group path/issue_type=(none)/status=id(Active) Active
2 item 5 ghost=false children=false
1 group path/issue_type=(none)/status=id(Planning) Planning
2 item 4 ghost=false children=false
1 group path/issue_type=(none)/status=(none) (none)
2 item 6 ghost=false children=false
0 group path/issue_type=Bug Bug
1 group path/issue_type=Bug/status=id(Active) Active
2 item 1 ghost=false children=false
0 group path/issue_type=Feature Feature
1 group path/issue_type=Feature/status=id(Closed) Closed
2 item 3 ghost=false children=false
0 group path/issue_type=Task Task
1 group path/issue_type=Task/status=id(Open) Open
2 item 2 ghost=false children=false

== Pivot(Iteration) → Hierarchy ==
total=10 unique_ids=10
0 group path/iteration=id(S1) S1
1 item 2 ghost=false children=true
2 item 1 ghost=false children=false
0 group path/iteration=id(S2) S2
1 item 4 ghost=true children=true
2 item 3 ghost=false children=false
0 group path/iteration=(none) (none)
1 item 4 ghost=false children=false
1 item 5 ghost=false children=false
1 item 6 ghost=false children=false

== Pivot(Repository) → Group(Epic) → Hierarchy ==
total=14 unique_ids=14
0 group path/repository=org/repo-a org/repo-a
1 group path/repository=org/repo-a/epic=id(EpicA) EpicA
2 item 2 ghost=false children=true
3 item 1 ghost=false children=false
0 group path/repository=org/repo-b org/repo-b
1 group path/repository=org/repo-b/epic=id(EpicB) EpicB
2 item 4 ghost=true children=true
3 item 3 ghost=false children=false
2 item 5 ghost=false children=false
1 group path/repository=org/repo-b/epic=(none) (none)
2 item 4 ghost=false children=false
0 group path/repository=org/repo-c org/repo-c
1 group path/repository=org/repo-c/epic=(none) (none)
2 item 6 ghost=false children=false

== Pivot(State) → Group(Epic) ==
total=14 unique_ids=14
0 group path/state=OPEN OPEN
1 group path/state=OPEN/epic=id(EpicA) EpicA
2 item 1 ghost=false children=false
2 item 2 ghost=false children=false
1 group path/state=OPEN/epic=id(EpicB) EpicB
2 item 5 ghost=false children=false
1 group path/state=OPEN/epic=(none) (none)
2 item 4 ghost=false children=false
0 group path/state=CLOSED CLOSED
1 group path/state=CLOSED/epic=id(EpicB) EpicB
2 item 3 ghost=false children=false
0 group path/state=(none) (none)
1 group path/state=(none)/epic=(none) (none)
2 item 6 ghost=false children=false

== Pivot(Status) ==
total=11 unique_ids=11
0 group path/status=id(Active) Active
1 item 1 ghost=false children=false
1 item 5 ghost=false children=false
0 group path/status=id(Open) Open
1 item 2 ghost=false children=false
0 group path/status=id(Closed) Closed
1 item 3 ghost=false children=false
0 group path/status=id(Planning) Planning
1 item 4 ghost=false children=false
0 group path/status=(none) (none)
1 item 6 ghost=false children=false

== Pivot(Status) → Group(Workstream) ==
total=17 unique_ids=17
0 group path/status=id(Active) Active
1 group path/status=id(Active)/workstream=id(WS1) WS1
2 item 1 ghost=false children=false
1 group path/status=id(Active)/workstream=id(WS2) WS2
2 item 5 ghost=false children=false
0 group path/status=id(Open) Open
1 group path/status=id(Open)/workstream=id(WS1) WS1
2 item 2 ghost=false children=false
0 group path/status=id(Closed) Closed
1 group path/status=id(Closed)/workstream=id(WS2) WS2
2 item 3 ghost=false children=false
0 group path/status=id(Planning) Planning
1 group path/status=id(Planning)/workstream=(none) (none)
2 item 4 ghost=false children=false
0 group path/status=(none) (none)
1 group path/status=(none)/workstream=(none) (none)
2 item 6 ghost=false children=false

== Pivot(Type) → Group(Status) ==
total=15 unique_ids=15
0 group path/type=issue issue
1 group path/type=issue/status=id(Active) Active
2 item 1 ghost=false children=false
1 group path/type=issue/status=id(Open) Open
2 item 2 ghost=false children=false
1 group path/type=issue/status=id(Closed) Closed
2 item 3 ghost=false children=false
1 group path/type=issue/status=id(Planning) Planning
2 item 4 ghost=false children=false
0 group path/type=pullRequest pullRequest
1 group path/type=pullRequest/status=id(Active) Active
2 item 5 ghost=false children=false
0 group path/type=draftIssue draftIssue
1 group path/type=draftIssue/status=(none) (none)
2 item 6 ghost=false children=false

== Sort(Epic) ==
total=6 unique_ids=6
0 item 1 ghost=false children=false
0 item 2 ghost=false children=false
0 item 3 ghost=false children=false
0 item 4 ghost=false children=false
0 item 5 ghost=false children=false
0 item 6 ghost=false children=false

== Sort(Priority) → Hierarchy ==
total=6 unique_ids=6
0 item 2 ghost=false children=true
1 item 1 ghost=false children=false
0 item 4 ghost=false children=true
1 item 3 ghost=false children=false
0 item 5 ghost=false children=false
0 item 6 ghost=false children=false"#
        );
    }

    #[test]
    fn test_recipe_builder_ghost_rows_for_mixed_epic_children() {
        let mut data = TestData::default();
        let child_a = data.build().epic("EpicA").add();
        let child_b = data.build().epic("EpicB").add();
        let parent = data
            .build()
            .epic("EpicA")
            .sub_issues(&[&child_a, &child_b])
            .add();
        set_title(&mut data, &child_a, "Alpha child");
        set_title(&mut data, &child_b, "Beta child");
        set_title(&mut data, &parent, "Parent");

        let config = PivotConfig {
            recipe: vec![Axis::Pivot(PivotField::Epic), Axis::Hierarchy],
            multi_value_strategy: MultiValueStrategy::Combined,
            show_ghost_ancestors: true,
        };

        assert_eq!(
            render_recipe_nodes(&data.fields, &data.work_items, &config),
            r#"0 group path/epic=id(EpicA) EpicA
1 item 3 ghost=false children=true
2 item 1 ghost=false children=false
0 group path/epic=id(EpicB) EpicB
1 item 3 ghost=true children=true
2 item 2 ghost=false children=false"#
        );
    }

    #[test]
    fn test_recipe_builder_multi_value_combined_vs_explode() {
        let mut data = TestData::default();
        let item = data.build().assignees(&["bob", "alice"]).add();
        set_title(&mut data, &item, "Shared");

        let combined = PivotConfig {
            recipe: vec![Axis::Pivot(PivotField::Assignee)],
            multi_value_strategy: MultiValueStrategy::Combined,
            show_ghost_ancestors: true,
        };
        let exploded = PivotConfig {
            recipe: vec![Axis::Pivot(PivotField::Assignee)],
            multi_value_strategy: MultiValueStrategy::Explode,
            show_ghost_ancestors: true,
        };

        assert_eq!(
            render_recipe_nodes(&data.fields, &data.work_items, &combined),
            r#"0 group path/assignee=alice+bob @alice, @bob
1 item 1 ghost=false children=false"#
        );
        assert_eq!(
            render_recipe_nodes(&data.fields, &data.work_items, &exploded),
            r#"0 group path/assignee=alice @alice
1 item 1 ghost=false children=false
0 group path/assignee=bob @bob
1 item 1 ghost=false children=false"#
        );
    }

    #[test]
    fn test_recipe_builder_without_ghost_ancestors_flattens_buckets() {
        let mut data = TestData::default();
        let child_a = data.build().epic("EpicA").add();
        let child_b = data.build().epic("EpicB").add();
        let parent = data
            .build()
            .epic("EpicA")
            .sub_issues(&[&child_a, &child_b])
            .add();
        set_title(&mut data, &child_a, "Alpha child");
        set_title(&mut data, &child_b, "Beta child");
        set_title(&mut data, &parent, "Parent");

        let config = PivotConfig {
            recipe: vec![Axis::Pivot(PivotField::Epic), Axis::Hierarchy],
            multi_value_strategy: MultiValueStrategy::Combined,
            show_ghost_ancestors: false,
        };

        assert_eq!(
            render_recipe_nodes(&data.fields, &data.work_items, &config),
            r#"0 group path/epic=id(EpicA) EpicA
1 item 3 ghost=false children=true
2 item 1 ghost=false children=false
0 group path/epic=id(EpicB) EpicB
1 item 2 ghost=false children=false"#
        );
    }

    /// Regression test: when a root item's `parent_id` points to a work item
    /// that is **not** in `work_items` (i.e. not part of the project), the
    /// ghost ancestor walk must stop rather than adding an unrenderable ghost.
    /// Previously the ghost was added to `scope_ids`, became the only display
    /// root, and its children (the real items) were never emitted — the group
    /// showed `has_children: true` but contained zero nodes.
    #[test]
    fn test_recipe_builder_ghost_ancestor_not_in_work_items_does_not_swallow_children() {
        let mut data = TestData::default();

        // child_a: in EpicA, parent_id points to a missing (out-of-project) issue
        let child_a = data.build().epic("EpicA").issue().add();
        let child_b = data.build().epic("EpicA").issue().add();

        // Give both items a parent_id that is NOT in work_items
        let missing_parent_id = WorkItemId("missing-parent".to_owned());
        for id in &[&child_a, &child_b] {
            if let WorkItemData::Issue(issue) = &mut data.work_items.get_mut(id).unwrap().data {
                issue.parent_id = Some(missing_parent_id.clone());
            }
        }
        set_title(&mut data, &child_a, "Alpha");
        set_title(&mut data, &child_b, "Beta");

        let config = PivotConfig {
            recipe: vec![Axis::Pivot(PivotField::Epic), Axis::Hierarchy],
            multi_value_strategy: MultiValueStrategy::Combined,
            show_ghost_ancestors: true,
        };

        // Both children must appear; the missing parent must NOT appear
        assert_eq!(
            render_recipe_nodes(&data.fields, &data.work_items, &config),
            r#"0 group path/epic=id(EpicA) EpicA
1 item 1 ghost=false children=false
1 item 2 ghost=false children=false"#
        );
    }

    /// Regression test for the `each_key_duplicate` Svelte crash that hit
    /// users running `Pivot(Epic) + Hierarchy` with `show_ghost_ancestors`
    /// on. With that recipe, the same work item legitimately appears in
    /// multiple Epic buckets — once as a real item in its own bucket and
    /// again as a ghost ancestor in every other Epic bucket whose items
    /// descend from it. Each [`Node`] must therefore carry a
    /// render-position-unique `id` (path-prefixed) so the frontend's keyed
    /// `{#each}` doesn't see duplicates and break expand/collapse.
    #[test]
    fn test_recipe_node_builder_no_duplicate_node_ids_with_ghost_ancestors_across_buckets() {
        let mut data = TestData::default();
        let child = data.build().epic("EpicB").add();
        let parent = data.build().epic("EpicA").sub_issues(&[&child]).add();
        set_title(&mut data, &child, "Child in EpicB");
        set_title(&mut data, &parent, "Parent in EpicA");

        let config = PivotConfig {
            recipe: vec![Axis::Pivot(PivotField::Epic), Axis::Hierarchy],
            multi_value_strategy: MultiValueStrategy::Combined,
            show_ghost_ancestors: true,
        };

        let filters = Filters::default();
        let original_work_items = HashMap::new();
        let mut builder = RecipeNodeBuilder::new(
            &data.fields,
            &data.work_items,
            &filters,
            &original_work_items,
            &config,
        );
        let nodes = builder.build();

        // Every Node.id must be unique — this is what the Svelte `{#each}`
        // keys on, and a collision causes `each_key_duplicate` at runtime.
        let ids: std::collections::HashSet<&str> = nodes.iter().map(|n| n.id.as_str()).collect();
        assert_eq!(
            ids.len(),
            nodes.len(),
            "duplicate Node.id values found in {nodes:#?}"
        );

        // The parent should appear in *both* Epic buckets: once as the real
        // item in EpicA, and once as a ghost ancestor in EpicB (anchoring
        // its child). This is the intended UX and the source of the
        // duplicate ids before the fix.
        let parent_occurrences: Vec<&Node> = nodes
            .iter()
            .filter(|n| match &n.data {
                NodeData::WorkItem { work_item_id } => work_item_id == &parent,
                _ => false,
            })
            .collect();
        assert_eq!(
            parent_occurrences.len(),
            2,
            "parent should appear in both Epic buckets (once real, once ghost)"
        );
        let ghost_flags: Vec<bool> = parent_occurrences.iter().map(|n| n.is_ghost).collect();
        assert!(
            ghost_flags.contains(&false) && ghost_flags.contains(&true),
            "parent should appear once as real (is_ghost=false) and once as ghost (is_ghost=true), got is_ghost flags {ghost_flags:?}"
        );
    }

    /// Structural invariants for the recipe node builder. Iterates the full
    /// matrix of (preset × ghost ancestors on/off × Combined/Explode) and
    /// asserts three invariants per combination:
    ///
    /// 1. `Node.id` uniqueness — Svelte's keyed `{#each}` blocks panic at
    ///    runtime if two siblings (or anywhere in the rendered list) share
    ///    the same key. This is the test that catches the
    ///    `each_key_duplicate` bug class the opaque string-snapshot test
    ///    failed to catch.
    /// 2. Level monotonicity — a flat depth-first tree may not jump down
    ///    more than one level between consecutive nodes; the first node
    ///    must be at level 0.
    /// 3. WorkItem id presence — every `NodeData::WorkItem` node references
    ///    a `WorkItemId` present in `work_items` (no stale/orphan refs).
    ///
    /// Failure messages always include preset name, ghost flag, and
    /// strategy so a CI failure points straight at the broken combination.
    #[test]
    fn test_recipe_builder_node_id_uniqueness_invariant() {
        let presets: Value = serde_json::from_str(include_str!(
            "../../../github-graphql/tests/fixtures/recipes.json"
        ))
        .unwrap();
        let mut preset_names = presets
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        preset_names.sort();

        let fixture = build_snapshot_fixture();

        for preset_name in &preset_names {
            for show_ghost_ancestors in [false, true] {
                for strategy in [MultiValueStrategy::Combined, MultiValueStrategy::Explode] {
                    let config = PivotConfig {
                        recipe: parse_recipe(preset_name).unwrap(),
                        multi_value_strategy: strategy.clone(),
                        show_ghost_ancestors,
                    };
                    let nodes = build_recipe_nodes(&fixture.fields, &fixture.work_items, &config);

                    let label = format!(
                        "preset={preset_name:?} ghost={show_ghost_ancestors} strategy={strategy:?}"
                    );

                    assert_node_ids_unique(&label, &nodes);
                    assert_levels_monotonic(&label, &nodes);
                    assert_work_item_ids_present(&label, &nodes, &fixture.work_items);
                    assert_has_children_consistent(&label, &nodes);
                }
            }
        }
    }

    fn build_snapshot_fixture() -> TestData {
        let mut data = TestData::default();

        let child_a = data
            .build()
            .epic("EpicA")
            .status("Active")
            .assignees(&["alice"])
            .add();
        let parent_a = data
            .build()
            .epic("EpicA")
            .status("Open")
            .assignees(&["alice"])
            .sub_issues(&[&child_a])
            .add();
        let child_b = data
            .build()
            .epic("EpicB")
            .status("Closed")
            .assignees(&["alice"])
            .add();
        let parent_none = data
            .build()
            .status("Planning")
            .assignees(&["alice", "bob"])
            .sub_issues(&[&child_b])
            .add();
        let pull_request = data.build().epic("EpicB").status("Active").add();
        let draft_issue = data.build().add();

        set_title(&mut data, &child_a, "Alpha child");
        set_title(&mut data, &parent_a, "Alpha parent");
        set_title(&mut data, &child_b, "Beta child");
        set_title(&mut data, &parent_none, "Beta parent");
        set_title(&mut data, &pull_request, "Repo pull request");
        set_title(&mut data, &draft_issue, "Scratch draft");

        set_workstream(&mut data, &child_a, Some("WS1"));
        set_workstream(&mut data, &parent_a, Some("WS1"));
        set_workstream(&mut data, &child_b, Some("WS2"));
        set_workstream(&mut data, &pull_request, Some("WS2"));

        set_iteration(&mut data, &child_a, Some("S1"));
        set_iteration(&mut data, &parent_a, Some("S1"));
        set_iteration(&mut data, &child_b, Some("S2"));

        set_priority(&mut data, &child_a, Some("Low"));
        set_priority(&mut data, &parent_a, Some("Medium"));
        set_priority(&mut data, &child_b, Some("High"));
        set_priority(&mut data, &parent_none, Some("High"));
        set_priority(&mut data, &pull_request, Some("Medium"));

        set_repository(&mut data, &child_a, Some("org/repo-a"));
        set_repository(&mut data, &parent_a, Some("org/repo-a"));
        set_repository(&mut data, &child_b, Some("org/repo-b"));
        set_repository(&mut data, &parent_none, Some("org/repo-b"));
        set_repository(&mut data, &pull_request, Some("org/repo-b"));
        set_repository(&mut data, &draft_issue, Some("org/repo-c"));

        set_issue_type(&mut data, &child_a, Some("Bug"));
        set_issue_type(&mut data, &parent_a, Some("Task"));
        set_issue_type(&mut data, &child_b, Some("Feature"));

        set_issue_state(&mut data, &child_b, IssueState::CLOSED);
        set_pull_request(&mut data, &pull_request, PullRequestState::OPEN, &["bob"]);
        set_draft_issue(&mut data, &draft_issue);

        data
    }

    fn build_recipe_nodes(
        fields: &Fields,
        work_items: &WorkItems,
        pivot_config: &PivotConfig,
    ) -> Vec<Node> {
        let filters = Filters::default();
        let original_work_items = HashMap::new();
        let mut builder = RecipeNodeBuilder::new(
            fields,
            work_items,
            &filters,
            &original_work_items,
            pivot_config,
        );
        builder.build()
    }

    fn format_nodes_string(nodes: &[Node]) -> String {
        nodes
            .iter()
            .map(|node| match &node.data {
                NodeData::Group { name, .. } => {
                    format!("{} group {} {}", node.level, node.id, name)
                }
                NodeData::WorkItem { work_item_id } => format!(
                    "{} item {} ghost={} children={}",
                    node.level, work_item_id.0, node.is_ghost, node.has_children
                ),
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn render_recipe_nodes(
        fields: &Fields,
        work_items: &WorkItems,
        pivot_config: &PivotConfig,
    ) -> String {
        let nodes = build_recipe_nodes(fields, work_items, pivot_config);
        format_nodes_string(&nodes)
    }

    /// Find duplicate `Node.id` values along with the count of occurrences.
    /// Returned in sorted order so failure messages are stable across runs.
    fn duplicate_node_ids(nodes: &[Node]) -> Vec<(String, usize)> {
        let mut counts: HashMap<&str, usize> = HashMap::new();
        for n in nodes {
            *counts.entry(n.id.as_str()).or_insert(0) += 1;
        }
        let mut dups: Vec<(String, usize)> = counts
            .into_iter()
            .filter(|(_, c)| *c > 1)
            .map(|(id, c)| (id.to_owned(), c))
            .collect();
        dups.sort();
        dups
    }

    /// Invariant: every `Node.id` produced by the builder must be unique within
    /// the result. Svelte's keyed `{#each}` blocks rely on this; duplicates
    /// cause runtime `each_key_duplicate` panics in `TreeTable.svelte`.
    fn assert_node_ids_unique(label: &str, nodes: &[Node]) {
        let unique = nodes.iter().map(|n| &n.id).collect::<HashSet<_>>().len();
        if unique != nodes.len() {
            panic!(
                "{label}: Node.id collision — total={} unique_ids={}. Duplicate ids: {:?}",
                nodes.len(),
                unique,
                duplicate_node_ids(nodes),
            );
        }
    }

    /// Invariant: the flat node list represents a depth-first traversal of a
    /// tree, so consecutive nodes may never *jump* down by more than one
    /// level. The first node must be at level 0.
    fn assert_levels_monotonic(label: &str, nodes: &[Node]) {
        if nodes.is_empty() {
            return;
        }
        assert_eq!(
            nodes[0].level, 0,
            "{label}: first node level must be 0, got {} (id {:?})",
            nodes[0].level, nodes[0].id,
        );
        for i in 1..nodes.len() {
            let prev = &nodes[i - 1];
            let curr = &nodes[i];
            assert!(
                curr.level <= prev.level + 1,
                "{label}: level jump at index {i}: prev (id={:?}, lvl={}) -> curr (id={:?}, lvl={}). Children must descend by exactly one level.",
                prev.id,
                prev.level,
                curr.id,
                curr.level,
            );
        }
    }

    /// Invariant: every `NodeData::WorkItem` node must reference a `WorkItemId`
    /// that exists in `work_items`. This catches stale or orphan refs (e.g. a
    /// ghost ancestor that survived after its underlying work item was
    /// removed).
    ///
    /// Assumption: on current main, `node.id` IS the `WorkItemId`. If a fix
    /// changes the id format to a path-prefixed string, this helper will need
    /// to extract the work item id (likely the trailing segment) instead.
    fn assert_work_item_ids_present(label: &str, nodes: &[Node], work_items: &WorkItems) {
        for n in nodes {
            if let NodeData::WorkItem { work_item_id } = &n.data {
                assert!(
                    work_items.get(work_item_id).is_some(),
                    "{label}: WorkItem node has id {:?} (work_item_id={:?}) which is not present in work_items",
                    n.id,
                    work_item_id,
                );
            }
        }
    }

    /// Invariant: every node whose `has_children` flag is `true` must be
    /// immediately followed (at depth `level + 1`) by at least one child
    /// node. This catches the class of bug where a group or item claims to
    /// have children but the renderer silently drops them — for example when
    /// a ghost ancestor that is not in `work_items` becomes the sole display
    /// root and swallows all real items beneath it.
    fn assert_has_children_consistent(label: &str, nodes: &[Node]) {
        for (i, node) in nodes.iter().enumerate() {
            if !node.has_children {
                continue;
            }
            let expected_child_level = node.level + 1;
            let has_child = nodes
                .get(i + 1)
                .is_some_and(|next| next.level == expected_child_level);
            assert!(
                has_child,
                "{label}: node at index {i} (id={:?}, level={}) has has_children=true but no child node follows at level {}",
                node.id, node.level, expected_child_level,
            );
        }
    }

    fn set_title(data: &mut TestData, id: &WorkItemId, title: &str) {
        data.work_items.get_mut(id).unwrap().title = title.to_owned();
    }

    fn set_repository(data: &mut TestData, id: &WorkItemId, repo: Option<&str>) {
        data.work_items.get_mut(id).unwrap().repo_name_with_owner = repo.map(str::to_owned);
    }

    fn set_issue_type(data: &mut TestData, id: &WorkItemId, issue_type: Option<&str>) {
        if let WorkItemData::Issue(issue) = &mut data.work_items.get_mut(id).unwrap().data {
            issue.issue_type = issue_type.map(str::to_owned).into();
        }
    }

    fn set_issue_state(data: &mut TestData, id: &WorkItemId, state: IssueState) {
        if let WorkItemData::Issue(issue) = &mut data.work_items.get_mut(id).unwrap().data {
            issue.state = state.into();
        }
    }

    fn set_pull_request(
        data: &mut TestData,
        id: &WorkItemId,
        state: PullRequestState,
        assignees: &[&str],
    ) {
        data.work_items.get_mut(id).unwrap().data = WorkItemData::PullRequest(PullRequest {
            state: state.into(),
            assignees: assignees
                .iter()
                .map(|assignee| (*assignee).to_owned())
                .collect(),
        });
    }

    fn set_draft_issue(data: &mut TestData, id: &WorkItemId) {
        data.work_items.get_mut(id).unwrap().data = WorkItemData::DraftIssue;
    }

    fn set_iteration(data: &mut TestData, id: &WorkItemId, value: Option<&str>) {
        let option = data.fields.iteration.option_id(value).cloned();
        data.work_items.get_mut(id).unwrap().project_item.iteration = option.into();
    }

    fn set_priority(data: &mut TestData, id: &WorkItemId, value: Option<&str>) {
        let option = data.fields.priority.option_id(value).cloned();
        data.work_items.get_mut(id).unwrap().project_item.priority = option;
    }

    fn set_workstream(data: &mut TestData, id: &WorkItemId, value: Option<&str>) {
        let option = data.fields.workstream.option_id(value).cloned();
        data.work_items.get_mut(id).unwrap().project_item.workstream = option.into();
    }

    #[test]
    fn test_recipe_node_builder_filters_closed() {
        let mut data = TestData::default();

        let child1_id = data.build().status("Closed").add();
        let child2_id = data.build().status("Closed").add();
        let parent1_id = data
            .build()
            .sub_issues(&[&child1_id, &child2_id])
            .status("Closed")
            .add();

        let grandchild1_id = data.build().status("Open").add();
        let child3_id = data
            .build()
            .sub_issues(&[&grandchild1_id])
            .status("Closed")
            .add();
        let parent2_id = data
            .build()
            .sub_issues(&[&child3_id])
            .status("Closed")
            .add();

        let filters = Filters {
            status: vec![Some(FieldOptionId("id(Closed)".into()))],
            ..Filters::default()
        };
        let original_work_items = HashMap::new();
        let config = PivotConfig {
            recipe: vec![Axis::Hierarchy],
            multi_value_strategy: MultiValueStrategy::Combined,
            show_ghost_ancestors: false,
        };
        let mut builder = RecipeNodeBuilder::new(
            &data.fields,
            &data.work_items,
            &filters,
            &original_work_items,
            &config,
        );
        let nodes = builder.build();

        let has_work_item = |id: &WorkItemId| {
            nodes.iter().any(|n| match &n.data {
                NodeData::WorkItem { work_item_id } => work_item_id == id,
                _ => false,
            })
        };

        assert!(!has_work_item(&parent1_id));
        assert!(!has_work_item(&child1_id));
        assert!(!has_work_item(&child2_id));

        assert!(has_work_item(&grandchild1_id));
        assert!(has_work_item(&child3_id));
        assert!(has_work_item(&parent2_id));
    }

    #[test]
    fn test_recipe_node_builder_new_item_after_update_appears() {
        let mut data = TestData::default();
        let existing_id = data.build().epic("EpicA").add();

        let new_item = WorkItem {
            id: WorkItemId("new-item".to_owned()),
            project_item: {
                let mut pi = ProjectItem::default_loaded();
                pi.epic = data.fields.epic.option_id(Some("EpicA")).cloned();
                pi
            },
            data: WorkItemData::Issue(Issue::default_loaded()),
            ..WorkItem::default_loaded()
        };
        let update_type = data.work_items.update(new_item);
        assert_eq!(update_type, UpdateType::ChangesHierarchy);

        let filters = Filters::default();
        let original_work_items = HashMap::new();
        let config = PivotConfig {
            recipe: vec![Axis::Hierarchy],
            multi_value_strategy: MultiValueStrategy::Combined,
            show_ghost_ancestors: false,
        };
        let mut builder = RecipeNodeBuilder::new(
            &data.fields,
            &data.work_items,
            &filters,
            &original_work_items,
            &config,
        );
        let nodes = builder.build();

        let new_item_id = WorkItemId("new-item".to_owned());
        let has_work_item = |id: &WorkItemId| {
            nodes.iter().any(|n| match &n.data {
                NodeData::WorkItem { work_item_id } => work_item_id == id,
                _ => false,
            })
        };

        assert!(
            has_work_item(&existing_id),
            "Existing item should still be in nodes"
        );
        assert!(
            has_work_item(&new_item_id),
            "New item added via update() should appear in nodes"
        );
    }

    #[test]
    fn test_recipe_node_builder_non_default_recipe_produces_correct_shape() {
        let mut data = TestData::default();
        let item1 = data.build().add();
        let item2 = data.build().add();
        let item3 = data.build().add();

        set_workstream(&mut data, &item1, Some("WS1"));
        set_workstream(&mut data, &item2, Some("WS1"));
        set_workstream(&mut data, &item3, Some("WS2"));

        set_title(&mut data, &item1, "Alpha");
        set_title(&mut data, &item2, "Beta");
        set_title(&mut data, &item3, "Gamma");

        let config = PivotConfig {
            recipe: vec![Axis::Pivot(PivotField::Workstream), Axis::Hierarchy],
            multi_value_strategy: MultiValueStrategy::Combined,
            show_ghost_ancestors: false,
        };

        let filters = Filters::default();
        let original_work_items = HashMap::new();
        let mut builder = RecipeNodeBuilder::new(
            &data.fields,
            &data.work_items,
            &filters,
            &original_work_items,
            &config,
        );
        let nodes = builder.build();

        // 2 group nodes (WS1, WS2) + 3 item nodes
        assert_eq!(nodes.len(), 5);

        let group_names: Vec<&str> = nodes
            .iter()
            .filter_map(|n| match &n.data {
                NodeData::Group { name, .. } => Some(name.as_str()),
                NodeData::WorkItem { .. } => None,
            })
            .collect();
        assert_eq!(group_names, vec!["WS1", "WS2"]);

        // Items in WS1 appear at level 1; item in WS2 also at level 1
        let find_item = |id: &WorkItemId| {
            nodes
                .iter()
                .find(|n| matches!(&n.data, NodeData::WorkItem { work_item_id } if work_item_id == id))
                .unwrap()
        };
        let position_item = |id: &WorkItemId| {
            nodes
                .iter()
                .position(|n| matches!(&n.data, NodeData::WorkItem { work_item_id } if work_item_id == id))
                .unwrap()
        };

        let item1_node = find_item(&item1);
        let item2_node = find_item(&item2);
        let item3_node = find_item(&item3);
        assert_eq!(item1_node.level, 1);
        assert_eq!(item2_node.level, 1);
        assert_eq!(item3_node.level, 1);

        // WS1 group comes before WS2 group
        let ws1_pos = nodes
            .iter()
            .position(|n| matches!(&n.data, NodeData::Group { name, .. } if name == "WS1"))
            .unwrap();
        let ws2_pos = nodes
            .iter()
            .position(|n| matches!(&n.data, NodeData::Group { name, .. } if name == "WS2"))
            .unwrap();
        assert!(ws1_pos < ws2_pos, "WS1 group should precede WS2 group");

        // item1 and item2 appear before item3 (they're under WS1 which sorts first)
        let item3_pos = position_item(&item3);
        assert!(
            position_item(&item1) < item3_pos,
            "WS1 items should appear before WS2 items"
        );
        assert!(
            position_item(&item2) < item3_pos,
            "WS1 items should appear before WS2 items"
        );
    }
}
