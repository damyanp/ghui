use crate::data::test_helpers::TestData;
use crate::data::*;

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

    let report = data.work_items.sanitize(&data.fields);

    let mut expected_changes = Changes::default();

    let closed_option = data.fields.status.option_id(Some("Closed")).cloned();

    expected_changes.add(Change {
        work_item_id: closed_item_id,
        data: ChangeData::Status(closed_option),
    });

    assert_eq!(report.changes, expected_changes);
    assert!(report.epic_conflicts.is_empty());
}

#[test]
fn test_set_epic_from_parent() {
    let mut data = TestData::default();

    const RIGHT_EPIC: &str = "DML Demo";
    const LATER_EPIC: &str = "MiniEngine Demo";
    const EARLIER_EPIC: &str = "DML Demo"; // same position = at or before

    let child_no_epic = data.build().add();
    let child_later_epic = data.build().epic(LATER_EPIC).add();
    let child_right_epic = data.build().epic(RIGHT_EPIC).add();

    data.build()
        .epic(RIGHT_EPIC)
        .sub_issues(&[&child_no_epic, &child_later_epic, &child_right_epic])
        .add();

    let report = data.work_items.sanitize(&data.fields);

    let mut expected_changes = Changes::default();
    expected_changes.add(Change {
        work_item_id: child_no_epic,
        data: ChangeData::Epic(data.fields.epic.option_id(RIGHT_EPIC.into()).cloned()),
    });

    assert_eq!(report.changes, expected_changes);

    // child_later_epic has a later Epic than parent — it must appear in conflicts.
    assert_eq!(report.epic_conflicts.len(), 1);
    assert_eq!(report.epic_conflicts[0].work_item_id, child_later_epic);
    assert_eq!(
        report.epic_conflicts[0].proposed_epic,
        data.fields
            .epic
            .option_id(RIGHT_EPIC.into())
            .cloned()
            .unwrap()
    );
    assert_eq!(
        report.epic_conflicts[0].current_epic,
        data.fields
            .epic
            .option_id(LATER_EPIC.into())
            .cloned()
            .unwrap()
    );
    // child_right_epic has the same Epic — no conflict.
    let _ = child_right_epic;
    let _ = EARLIER_EPIC;
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

    let report = data.work_items.sanitize(&data.fields);

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

    assert_eq!(report.changes, expected_changes);
    assert!(report.epic_conflicts.is_empty());
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

    let report = data.work_items.sanitize(&data.fields);

    let mut expected_changes = Changes::default();
    expected_changes.add(Change {
        work_item_id: child_blank,
        data: ws.clone(),
    });
    expected_changes.add(Change {
        work_item_id: child_wrong,
        data: ws,
    });

    assert_eq!(report.changes, expected_changes);
    assert!(report.epic_conflicts.is_empty());
}

#[test]
fn test_set_workstream_on_blank_parent_from_child() {
    let mut data = TestData::default();

    const WS: &str = "WS1";

    let _child_with_ws = data.build().workstream(WS).add();
    let _child_blank = data.build().add();

    let report = data.work_items.sanitize(&data.fields);

    // Parent is blank: sanitize does not change workstreams.
    assert_eq!(report.changes, Changes::default());
    assert!(report.epic_conflicts.is_empty());
}

#[test]
fn test_add_missing_parent_to_project() {
    let mut data = TestData::default();

    let child = data.build().issue().add();
    let missing_parent = WorkItemId("missing-parent".to_owned());

    if let Some(WorkItem {
        data: WorkItemData::Issue(issue),
        ..
    }) = data.work_items.get_mut(&child)
    {
        issue.parent_id = Some(missing_parent.clone());
    }

    let report = data.work_items.sanitize(&data.fields);

    let mut expected_changes = Changes::default();
    expected_changes.add(Change {
        work_item_id: missing_parent,
        data: ChangeData::AddToProject,
    });

    assert_eq!(report.changes, expected_changes);
    assert!(report.epic_conflicts.is_empty());
}

#[test]
fn test_add_shared_missing_parent_to_project_once() {
    let mut data = TestData::default();

    let child_a = data.build().issue().add();
    let child_b = data.build().issue().add();
    let missing_parent = WorkItemId("missing-parent".to_owned());

    for child in [&child_a, &child_b] {
        if let Some(WorkItem {
            data: WorkItemData::Issue(issue),
            ..
        }) = data.work_items.get_mut(child)
        {
            issue.parent_id = Some(missing_parent.clone());
        }
    }

    let report = data.work_items.sanitize(&data.fields);

    let mut expected_changes = Changes::default();
    expected_changes.add(Change {
        work_item_id: missing_parent,
        data: ChangeData::AddToProject,
    });

    assert_eq!(report.changes, expected_changes);
    assert!(report.epic_conflicts.is_empty());
}

#[test]
fn test_add_missing_grandparent_to_project() {
    let mut data = TestData::default();

    let child = data.build().issue().add();
    let parent = data.build().sub_issues(&[&child]).add();
    let missing_grandparent = WorkItemId("missing-grandparent".to_owned());

    if let Some(WorkItem {
        data: WorkItemData::Issue(issue),
        ..
    }) = data.work_items.get_mut(&parent)
    {
        issue.parent_id = Some(missing_grandparent.clone());
    }

    let report = data.work_items.sanitize(&data.fields);

    let mut expected_changes = Changes::default();
    expected_changes.add(Change {
        work_item_id: missing_grandparent,
        data: ChangeData::AddToProject,
    });

    assert_eq!(report.changes, expected_changes);
    assert!(report.epic_conflicts.is_empty());
}

#[test]
fn test_blank_parent_with_conflicting_children_workstreams_no_change() {
    let mut data = TestData::default();

    let child_ws1 = data.build().workstream("WS1").add();
    let child_ws2 = data.build().workstream("WS2").add();

    data.build().sub_issues(&[&child_ws1, &child_ws2]).add();

    let report = data.work_items.sanitize(&data.fields);

    // Parent is blank: sanitize does not change workstreams.
    assert_eq!(report.changes, Changes::default());
    assert!(report.epic_conflicts.is_empty());
}

#[test]
fn test_assigned_issue_with_no_status_gets_planning() {
    let mut data = TestData::default();

    let assigned_item_id = data.build().assignees(&["user1"]).add();

    let report = data.work_items.sanitize(&data.fields);

    let mut expected_changes = Changes::default();
    let planning_option = data.fields.status.option_id(Some("Planning")).cloned();
    expected_changes.add(Change {
        work_item_id: assigned_item_id,
        data: ChangeData::Status(planning_option),
    });

    assert_eq!(report.changes, expected_changes);
    assert!(report.epic_conflicts.is_empty());
}

#[test]
fn test_assigned_issue_with_status_set_no_change() {
    let mut data = TestData::default();

    data.build().assignees(&["user1"]).status("Active").add();

    let report = data.work_items.sanitize(&data.fields);

    assert_eq!(report.changes, Changes::default());
    assert!(report.epic_conflicts.is_empty());
}

#[test]
fn test_unassigned_issue_with_no_status_no_change() {
    let mut data = TestData::default();

    data.build().add();

    let report = data.work_items.sanitize(&data.fields);

    assert_eq!(report.changes, Changes::default());
    assert!(report.epic_conflicts.is_empty());
}

#[test]
fn test_closed_assigned_issue_gets_closed_not_planning() {
    let mut data = TestData::default();

    let closed_assigned_id = data
        .build()
        .issue_state(IssueState::CLOSED)
        .assignees(&["user1"])
        .add();

    let report = data.work_items.sanitize(&data.fields);

    let mut expected_changes = Changes::default();
    let closed_option = data.fields.status.option_id(Some("Closed")).cloned();
    expected_changes.add(Change {
        work_item_id: closed_assigned_id,
        data: ChangeData::Status(closed_option),
    });

    assert_eq!(report.changes, expected_changes);
    assert!(report.epic_conflicts.is_empty());
}

#[test]
fn test_epic_conflict_recorded_not_changed() {
    let mut data = TestData::default();

    const PARENT_EPIC: &str = "DML Demo";
    const CHILD_EPIC: &str = "MiniEngine Demo";

    let child_id = data.build().epic(CHILD_EPIC).add();
    data.build()
        .epic(PARENT_EPIC)
        .sub_issues(&[&child_id])
        .add();

    let report = data.work_items.sanitize(&data.fields);

    // No change should be staged for the child — the existing Epic is preserved.
    assert_eq!(report.changes, Changes::default());

    // But the conflict is recorded for the user to review.
    assert_eq!(report.epic_conflicts.len(), 1);
    let conflict = &report.epic_conflicts[0];
    assert_eq!(conflict.work_item_id, child_id);
    assert_eq!(
        conflict.current_epic,
        data.fields
            .epic
            .option_id(CHILD_EPIC.into())
            .cloned()
            .unwrap()
    );
    assert_eq!(
        conflict.proposed_epic,
        data.fields
            .epic
            .option_id(PARENT_EPIC.into())
            .cloned()
            .unwrap()
    );
}

#[test]
fn test_child_epic_before_parent_no_conflict() {
    let mut data = TestData::default();

    // Parent has a later epic; child has an earlier one — no conflict.
    const PARENT_EPIC: &str = "MiniEngine Demo"; // index 1
    const CHILD_EPIC: &str = "DML Demo"; // index 0

    let child_id = data.build().epic(CHILD_EPIC).add();
    data.build()
        .epic(PARENT_EPIC)
        .sub_issues(&[&child_id])
        .add();

    let report = data.work_items.sanitize(&data.fields);

    assert_eq!(report.changes, Changes::default());
    assert!(report.epic_conflicts.is_empty());
}

#[test]
fn test_child_epic_propagates_to_grandchild() {
    let mut data = TestData::default();

    // Grandparent has a later epic; child has an earlier one.
    // Grandchild (blank) should inherit the child's epic, not the grandparent's.
    const GRANDPARENT_EPIC: &str = "SM 6.9 Preview"; // index 2
    const CHILD_EPIC: &str = "DML Demo"; // index 0

    let grandchild = data.build().add();
    let child = data
        .build()
        .epic(CHILD_EPIC)
        .sub_issues(&[&grandchild])
        .add();
    data.build()
        .epic(GRANDPARENT_EPIC)
        .sub_issues(&[&child])
        .add();

    let report = data.work_items.sanitize(&data.fields);

    let mut expected_changes = Changes::default();
    expected_changes.add(Change {
        work_item_id: grandchild,
        data: ChangeData::Epic(data.fields.epic.option_id(CHILD_EPIC.into()).cloned()),
    });

    assert_eq!(report.changes, expected_changes);
    assert!(report.epic_conflicts.is_empty());
}

#[test]
fn test_child_epic_after_parent_with_grandchild() {
    let mut data = TestData::default();

    // Parent has earlier epic; child has a later one (conflict).
    // Grandchild is blank — should inherit the child's epic (nearest ancestor).
    const PARENT_EPIC: &str = "DML Demo"; // index 0
    const CHILD_EPIC: &str = "SM 6.9 Preview"; // index 2

    let grandchild = data.build().add();
    let child = data
        .build()
        .epic(CHILD_EPIC)
        .sub_issues(&[&grandchild])
        .add();
    data.build().epic(PARENT_EPIC).sub_issues(&[&child]).add();

    let report = data.work_items.sanitize(&data.fields);

    // Grandchild inherits child's epic
    let mut expected_changes = Changes::default();
    expected_changes.add(Change {
        work_item_id: grandchild,
        data: ChangeData::Epic(data.fields.epic.option_id(CHILD_EPIC.into()).cloned()),
    });

    assert_eq!(report.changes, expected_changes);

    // Child's epic is after parent's — conflict recorded
    assert_eq!(report.epic_conflicts.len(), 1);
    assert_eq!(report.epic_conflicts[0].work_item_id, child);
}
