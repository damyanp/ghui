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

    data.build().assignees(&["user1"]).status("Active").add();

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
