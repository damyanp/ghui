use std::collections::{HashMap, HashSet};
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

    let actual_changes = data.work_items.sanitize(&data.fields);

    let mut expected_changes = Changes::default();

    let closed_option = data.fields.status.option_id(Some("Closed")).cloned();

    expected_changes.add(Change {
        work_item_id: closed_item_id,
        data: ChangeData::Status(closed_option),
    });

    assert_eq!(actual_changes, expected_changes);
}

#[test]
fn test_set_epic_from_parent() {
    let mut data = TestData::default();

    const RIGHT_EPIC: &str = "DML Demo";
    const WRONG_EPIC: &str = "MiniEngine Demo";

    let child_no_epic = data.build().add();
    let child_wrong_epic = data.build().epic(WRONG_EPIC).add();
    let child_right_epic = data.build().epic(RIGHT_EPIC).add();

    data.build()
        .epic(RIGHT_EPIC)
        .sub_issues(&[&child_no_epic, &child_wrong_epic, &child_right_epic])
        .add();

    let actual_changes = data.work_items.sanitize(&data.fields);

    let mut expected_changes = Changes::default();
    expected_changes.add(Change {
        work_item_id: child_no_epic,
        data: ChangeData::Epic(data.fields.epic.option_id(RIGHT_EPIC.into()).cloned()),
    });

    assert_eq!(actual_changes, expected_changes);
}

#[test]
fn test_set_epic_from_grandparent() {
    let mut data = TestData::default();

    const EPIC: &str = "DML Demo";

    let child_a = data.build().add();
    let parent_a = data.build().epic(EPIC).sub_issues(&[&child_a]).add();

    let child_b = data.build().add();
    let parent_b = data.build().sub_issues(&[&child_b]).add();

    data.build()
        .epic(EPIC)
        .sub_issues(&[&parent_a, &parent_b])
        .add();

    let epic = ChangeData::Epic(data.fields.epic.option_id(EPIC.into()).cloned());

    let actual_changes = data.work_items.sanitize(&data.fields);

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

#[test]
fn test_set_workstream_from_parent() {
    let mut data = TestData::default();

    const WS_PARENT: &str = "WS1";
    const WS_WRONG: &str = "WS2";

    let child_blank = data.build().add();
    let child_wrong = data.build().workstream(WS_WRONG).add();
    let child_right = data.build().workstream(WS_PARENT).add();

    data.build()
        .workstream(WS_PARENT)
        .sub_issues(&[&child_blank, &child_wrong, &child_right])
        .add();

    let ws = ChangeData::Workstream(data.fields.workstream.option_id(WS_PARENT.into()).cloned());

    let actual_changes = data.work_items.sanitize(&data.fields);

    let mut expected_changes = Changes::default();
    expected_changes.add(Change {
        work_item_id: child_blank,
        data: ws.clone(),
    });
    expected_changes.add(Change {
        work_item_id: child_wrong,
        data: ws,
    });

    assert_eq!(actual_changes, expected_changes);
}

#[test]
fn test_set_workstream_on_blank_parent_from_child() {
    let mut data = TestData::default();

    const WS: &str = "WS1";

    let _child_with_ws = data.build().workstream(WS).add();
    let _child_blank = data.build().add();

    let actual_changes = data.work_items.sanitize(&data.fields);

    // Parent is blank: sanitize does not change workstreams.
    assert_eq!(actual_changes, Changes::default());
}

#[test]
fn test_blank_parent_with_conflicting_children_workstreams_no_change() {
    let mut data = TestData::default();

    let child_ws1 = data.build().workstream("WS1").add();
    let child_ws2 = data.build().workstream("WS2").add();

    data.build().sub_issues(&[&child_ws1, &child_ws2]).add();

    let actual_changes = data.work_items.sanitize(&data.fields);

    // Parent is blank: sanitize does not change workstreams.
    assert_eq!(actual_changes, Changes::default());
}

#[test]
fn test_assigned_issue_with_no_status_gets_planning() {
    let mut data = TestData::default();

    let assigned_item_id = data.build().assignees(&["user1"]).add();

    let actual_changes = data.work_items.sanitize(&data.fields);

    let mut expected_changes = Changes::default();
    let planning_option = data.fields.status.option_id(Some("Planning")).cloned();
    expected_changes.add(Change {
        work_item_id: assigned_item_id,
        data: ChangeData::Status(planning_option),
    });

    assert_eq!(actual_changes, expected_changes);
}

#[test]
fn test_assigned_issue_with_status_set_no_change() {
    let mut data = TestData::default();

    data.build()
        .assignees(&["user1"])
        .status("Active")
        .add();

    let actual_changes = data.work_items.sanitize(&data.fields);

    assert_eq!(actual_changes, Changes::default());
}

#[test]
fn test_unassigned_issue_with_no_status_no_change() {
    let mut data = TestData::default();

    data.build().add();

    let actual_changes = data.work_items.sanitize(&data.fields);

    assert_eq!(actual_changes, Changes::default());
}

#[test]
fn test_closed_assigned_issue_gets_closed_not_planning() {
    let mut data = TestData::default();

    let closed_assigned_id = data
        .build()
        .issue_state(IssueState::CLOSED)
        .assignees(&["user1"])
        .add();

    let actual_changes = data.work_items.sanitize(&data.fields);

    let mut expected_changes = Changes::default();
    let closed_option = data.fields.status.option_id(Some("Closed")).cloned();
    expected_changes.add(Change {
        work_item_id: closed_assigned_id,
        data: ChangeData::Status(closed_option),
    });

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

    let expected_original_work_items =
        HashMap::from_iter([(child.clone(), data.work_items.get(&child).unwrap().clone())]);

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
    let other_child_1 = data.build().issue().add();
    let other_child_2 = data.build().issue().add();
    let old_parent = data
        .build()
        .sub_issues(&[&other_child_1, &child, &other_child_2])
        .add();
    let new_parent = data.build().issue().add();

    let mut changes = Changes::default();
    changes.add(Change {
        work_item_id: child.clone(),
        data: ChangeData::SetParent(new_parent.clone()),
    });

    let expected_original_work_items = HashMap::from_iter([(
        child.clone(),
        data.work_items.get(&child).unwrap().to_owned(),
    )]);

    let actual_original_work_items = data.work_items.apply_changes(&changes);

    assert_eq!(expected_original_work_items, actual_original_work_items);

    let actual_old_parent_sub_issues = data
        .work_items
        .get(&old_parent)
        .unwrap()
        .get_sub_issues()
        .unwrap();
    assert_eq!(actual_old_parent_sub_issues.len(), 2);

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

#[test]
fn test_undo_add_change() {
    let mut changes = Changes::default();
    let change = Change {
        work_item_id: WorkItemId("item1".to_owned()),
        data: ChangeData::Status(Some(FieldOptionId("status1".to_owned()))),
    };

    changes.add(change.clone());
    assert_eq!(changes.len(), 1);
    assert!(changes.can_undo());
    assert!(!changes.can_redo());

    changes.undo();
    assert_eq!(changes.len(), 0);
    assert!(!changes.can_undo());
    assert!(changes.can_redo());

    changes.redo();
    assert_eq!(changes.len(), 1);
    assert!(changes.can_undo());
    assert!(!changes.can_redo());
}

#[test]
fn test_undo_remove_change() {
    let mut changes = Changes::default();
    let change = Change {
        work_item_id: WorkItemId("item1".to_owned()),
        data: ChangeData::Status(Some(FieldOptionId("status1".to_owned()))),
    };

    changes.add(change.clone());
    changes.remove(change.clone());
    assert_eq!(changes.len(), 0);

    // Undo the remove - should restore the change
    changes.undo();
    assert_eq!(changes.len(), 1);

    // Undo the add - should be empty again
    changes.undo();
    assert_eq!(changes.len(), 0);
    assert!(!changes.can_undo());
}

#[test]
fn test_undo_clear_changes() {
    let mut changes = Changes::default();
    changes.add(Change {
        work_item_id: WorkItemId("item1".to_owned()),
        data: ChangeData::Status(Some(FieldOptionId("s1".to_owned()))),
    });
    changes.add(Change {
        work_item_id: WorkItemId("item2".to_owned()),
        data: ChangeData::Epic(Some(FieldOptionId("e1".to_owned()))),
    });

    assert_eq!(changes.len(), 2);

    changes.clear();
    assert_eq!(changes.len(), 0);
    assert!(changes.can_undo());

    changes.undo();
    assert_eq!(changes.len(), 2);
    assert!(changes.can_redo());

    changes.redo();
    assert_eq!(changes.len(), 0);
}

#[test]
fn test_undo_overwrite_change() {
    let mut changes = Changes::default();
    let change1 = Change {
        work_item_id: WorkItemId("item1".to_owned()),
        data: ChangeData::Status(Some(FieldOptionId("status1".to_owned()))),
    };
    let change2 = Change {
        work_item_id: WorkItemId("item1".to_owned()),
        data: ChangeData::Status(Some(FieldOptionId("status2".to_owned()))),
    };

    changes.add(change1.clone());
    changes.add(change2.clone());
    assert_eq!(changes.len(), 1);

    // Undo the second add - should restore the first change value
    changes.undo();
    assert_eq!(changes.len(), 1);
    // Verify the first change is back by checking it's iterable and matches
    let restored_changes: Vec<&Change> = changes.into_iter().collect();
    assert_eq!(restored_changes.len(), 1);
    assert_eq!(restored_changes[0], &change1);

    // Undo the first add - should be empty
    changes.undo();
    assert_eq!(changes.len(), 0);
}

#[test]
fn test_multiple_undo_redo() {
    let mut changes = Changes::default();
    let c1 = Change {
        work_item_id: WorkItemId("item1".to_owned()),
        data: ChangeData::Status(Some(FieldOptionId("s1".to_owned()))),
    };
    let c2 = Change {
        work_item_id: WorkItemId("item2".to_owned()),
        data: ChangeData::Epic(Some(FieldOptionId("e1".to_owned()))),
    };

    changes.add(c1.clone());
    changes.add(c2.clone());
    assert_eq!(changes.len(), 2);

    changes.undo();
    assert_eq!(changes.len(), 1);

    changes.undo();
    assert_eq!(changes.len(), 0);

    changes.redo();
    assert_eq!(changes.len(), 1);

    changes.redo();
    assert_eq!(changes.len(), 2);
}

#[test]
fn test_new_change_clears_redo_stack() {
    let mut changes = Changes::default();
    let c1 = Change {
        work_item_id: WorkItemId("item1".to_owned()),
        data: ChangeData::Status(Some(FieldOptionId("s1".to_owned()))),
    };
    let c2 = Change {
        work_item_id: WorkItemId("item2".to_owned()),
        data: ChangeData::Epic(Some(FieldOptionId("e1".to_owned()))),
    };

    changes.add(c1.clone());
    changes.undo();
    assert!(changes.can_redo());

    changes.add(c2.clone());
    assert!(!changes.can_redo());
}

#[test]
fn test_undo_add_changes_batch() {
    let mut changes = Changes::default();

    // Pre-populate with a change to add first
    changes.add(Change {
        work_item_id: WorkItemId("item0".to_owned()),
        data: ChangeData::Status(Some(FieldOptionId("s0".to_owned()))),
    });
    assert_eq!(changes.len(), 1);

    let mut batch = Changes::default();
    batch.add(Change {
        work_item_id: WorkItemId("item1".to_owned()),
        data: ChangeData::Status(Some(FieldOptionId("s1".to_owned()))),
    });
    batch.add(Change {
        work_item_id: WorkItemId("item2".to_owned()),
        data: ChangeData::Epic(Some(FieldOptionId("e1".to_owned()))),
    });

    changes.add_changes(batch);
    assert_eq!(changes.len(), 3);

    // Undo should reverse the entire batch at once
    changes.undo();
    assert_eq!(changes.len(), 1);

    // Redo should restore the entire batch at once
    changes.redo();
    assert_eq!(changes.len(), 3);
}
