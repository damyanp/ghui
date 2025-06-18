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
            .project_milestone("Another Project Milestone")
            .add();

        // Already matching ones shouldn't change
        data.build()
            .project_milestone(project_milestone)
            .epic(epic)
            .add();

        // But when there's a match and no epic is set, we should expect a
        // change
        let id = data.build().project_milestone(project_milestone).add();

        let actual_changes = data.work_items.sanitize(&data.fields);

        let mut expected_changes = Changes::default();
        expected_changes.add(Change {
            work_item_id: id,
            data: ChangeData::Epic(data.fields.epic.option_id(epic.into()).cloned()),
        });

        assert_eq!(actual_changes, expected_changes);
    }
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
