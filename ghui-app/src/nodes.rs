use github_graphql::data::{FieldOptionId, Fields, WorkItem, WorkItemData, WorkItemId, WorkItems};
use serde::Serialize;
use std::collections::HashMap;
use ts_rs::TS;

use crate::Filters;

#[derive(Serialize, TS, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Node {
    pub level: u32,
    pub id: String,
    pub data: NodeData,
    pub has_children: bool,
    pub is_modified: bool,
}

#[derive(Serialize, TS, Debug)]
#[serde(rename_all = "camelCase", tag = "type")]
pub(crate) enum NodeData {
    WorkItem,
    Group { name: String },
}

pub(crate) struct NodeBuilder<'a> {
    fields: &'a Fields,
    work_items: &'a WorkItems,
    filters: &'a Filters,
    original_work_items: &'a HashMap<WorkItemId, WorkItem>,
    nodes: Vec<Node>,
}

impl<'a> NodeBuilder<'a> {
    pub fn new(
        fields: &'a Fields,
        work_items: &'a WorkItems,
        filters: &'a Filters,
        original_work_items: &'a HashMap<WorkItemId, WorkItem>,
    ) -> Self {
        NodeBuilder {
            fields,
            work_items,
            filters,
            original_work_items,
            nodes: Vec::new(),
        }
    }

    pub fn build(&mut self) -> Vec<Node> {
        self.add_nodes(&self.work_items.get_roots(), 0, "");
        std::mem::take(&mut self.nodes)
    }

    fn add_nodes(&mut self, items: &[WorkItemId], level: u32, path: &str) {
        let items = self.apply_filters(items);

        // For now, group by "Epic"
        let group = |id| {
            self.work_items
                .get(id)
                .and_then(|item| item.project_item.epic.flatten())
        };

        let mut group_item: Vec<_> = items.iter().map(|id| (group(id), *id)).collect();
        group_item.sort_by_key(|a| a.0);

        let has_multiple_groups =
            !(group_item.is_empty() || group_item.iter().all(|i| i.0 == group_item[0].0));

        let mut current_group: Option<Option<&FieldOptionId>> = None;
        let mut current_path = path.to_owned();

        for (key, id) in group_item {
            if has_multiple_groups {
                let start_new = current_group
                    .as_ref()
                    .is_none_or(|group| group.as_ref() != key.as_ref());

                if start_new {
                    let name = self
                        .fields
                        .epic
                        .option_name(key)
                        .unwrap_or("None")
                        .to_owned();
                    let id = format!("{}{}", path, name);

                    current_group = Some(key);
                    current_path = format!("{}/", id);

                    self.nodes.push(Node {
                        level,
                        id,
                        data: NodeData::Group { name },
                        has_children: true,
                        is_modified: false,
                    });
                }
            }

            let level = if has_multiple_groups {
                level + 1
            } else {
                level
            };

            self.add_node(id, level, current_path.as_str());
        }
    }

    fn apply_filters(&self, work_items: &'a [WorkItemId]) -> Vec<&'a WorkItemId> {
        Vec::from_iter(work_items.iter().filter(|i| self.should_include(i)))
    }

    fn should_include(&self, work_item_id: &WorkItemId) -> bool {
        // NOTE: this works harder than it should. Consider memoizing the
        // results for each work_item_id.

        let work_item = self.work_items.get(work_item_id);
        if let Some(work_item) = work_item {
            if let WorkItem {
                data: WorkItemData::Issue(issue),
                ..
            } = work_item
            {
                for child_id in &issue.sub_issues {
                    if self.should_include(child_id) {
                        // as soon as we find a descendant that should be
                        // visible we know that this item must be visible
                        return true;
                    }
                }
            }
            self.filters.should_include(self.fields, work_item)
        } else {
            false
        }
    }

    fn add_node(&mut self, id: &WorkItemId, level: u32, path: &str) {
        if let Some(item) = self.work_items.get(id) {
            let children = if let WorkItemData::Issue(issue) = &item.data {
                // Note it is important to use sub_issues here (rather than try
                // and generate the hierarchy from the issue's parents) because
                // the order of sub_issues is significant.
                issue.sub_issues.clone()
            } else {
                Vec::default()
            };

            self.nodes.push(Node {
                level,
                id: id.0.clone(),
                data: NodeData::WorkItem,
                has_children: !children.is_empty(),
                is_modified: self.original_work_items.contains_key(id),
            });

            self.add_nodes(
                &children,
                level + 1,
                format!("{}{}/", path, item.id.0).as_str(),
            );
        }
    }
}

#[cfg(test)]
mod nodebuilder_tests {
    use super::*;
    use github_graphql::data::test_helpers::TestData;
    use std::collections::HashMap;

    #[test]
    fn test_node_builder_single_item() {
        let mut data = TestData::default();
        let id = data.build().epic("EpicA").add();
        let work_items = data.work_items;
        let filters = Filters::default();
        let original_work_items = HashMap::new();
        let mut builder =
            NodeBuilder::new(&data.fields, &work_items, &filters, &original_work_items);
        let nodes = builder.build();
        // Only one node (the work item) should be present, no group node
        assert_eq!(nodes.len(), 1);
        assert!(matches!(nodes[0].data, NodeData::WorkItem));
        assert_eq!(nodes[0].id, id.0);
        assert_eq!(nodes[0].level, 0);
    }

    #[test]
    fn test_node_builder_grouping() {
        let mut data = TestData::default();
        let id1 = data.build().epic("EpicA").add();
        let id2 = data.build().epic("EpicB").add();
        let work_items = data.work_items;
        let filters = Filters::default();
        let original_work_items = HashMap::new();
        let mut builder =
            NodeBuilder::new(&data.fields, &work_items, &filters, &original_work_items);
        let nodes = builder.build();
        // Should have two groups and two work items, in order: Group(EpicA), WorkItem(1), Group(EpicB), WorkItem(2)
        println!("{:?}", work_items.work_items.values());
        println!("{nodes:?}");
        assert_eq!(nodes.len(), 4);
        assert!(matches!(nodes[0].data, NodeData::Group { ref name } if name == "EpicA"));
        assert!(matches!(nodes[1].data, NodeData::WorkItem));
        assert_eq!(nodes[1].id, id1.0);
        assert!(matches!(nodes[2].data, NodeData::Group { ref name } if name == "EpicB"));
        assert!(matches!(nodes[3].data, NodeData::WorkItem));
        assert_eq!(nodes[3].id, id2.0);
    }

    #[test]
    fn test_node_builder_hierarchy() {
        let mut data = TestData::default();
        let id1 = data.build().epic("EpicA").add();
        let id2 = data.build().epic("EpicA").sub_issues(&[&id1]).add();
        let work_items = data.work_items;
        let filters = Filters::default();
        let original_work_items = HashMap::new();
        let mut builder =
            NodeBuilder::new(&data.fields, &work_items, &filters, &original_work_items);
        let nodes = builder.build();
        // Should have two work items, no group node, in order: WorkItem(2), WorkItem(1)
        assert_eq!(nodes.len(), 2);
        assert!(matches!(nodes[0].data, NodeData::WorkItem));
        assert_eq!(nodes[0].id, id2.0);
        assert!(matches!(nodes[1].data, NodeData::WorkItem));
        assert_eq!(nodes[1].id, id1.0);
        // Child should be at a deeper level
        let parent_level = nodes.iter().find(|n| n.id == id2.0).unwrap().level;
        let child_level = nodes.iter().find(|n| n.id == id1.0).unwrap().level;
        assert!(child_level > parent_level);
    }

    #[test]
    fn test_node_build_no_filters() {
        let mut data = TestData::default();

        let closed_item = data.build().status("Closed").add();

        let filters = Filters::default();
        let original_work_items = HashMap::new();
        let mut builder = NodeBuilder::new(
            &data.fields,
            &data.work_items,
            &filters,
            &original_work_items,
        );
        let nodes = builder.build();

        // The closed parent and its closed children should be filtered out
        assert!(nodes.iter().any(|n| n.id == closed_item.0));
    }

    #[test]
    fn test_node_builder_filters_closed() {
        let mut data = TestData::default();

        // Create a closed parent with two closed children
        let child1_id = data.build().status("Closed").add();
        let child2_id = data.build().status("Closed").add();
        let parent1_id = data
            .build()
            .sub_issues(&[&child1_id, &child2_id])
            .status("Closed")
            .add();

        // Create a closed parent, with closed child and open granchild
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

        let work_items = data.work_items;
        let filters = Filters { hide_closed: true };
        let original_work_items = HashMap::new();
        let mut builder =
            NodeBuilder::new(&data.fields, &work_items, &filters, &original_work_items);
        let nodes = builder.build();

        // The closed parent and its closed children should be filtered out
        assert!(!nodes.iter().any(|n| n.id == parent1_id.0));
        assert!(!nodes.iter().any(|n| n.id == child1_id.0));
        assert!(!nodes.iter().any(|n| n.id == child2_id.0));

        // The open grandchild should be present, and so should its ancestors
        assert!(nodes.iter().any(|n| n.id == grandchild1_id.0));
        assert!(nodes.iter().any(|n| n.id == child3_id.0));
        assert!(nodes.iter().any(|n| n.id == parent2_id.0));
    }
}
