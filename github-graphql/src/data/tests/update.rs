use crate::data::test_helpers::TestData;
use crate::data::*;

#[test]
fn test_update_existing_item_no_change() {
    let mut data = TestData::default();
    let id = data.build().status("Active").assignees(&["user1"]).add();
    let item = data.work_items.get(&id).unwrap().clone();

    // Updating with the same item should be NoUpdate
    let update_type = data.work_items.update(item);
    assert_eq!(update_type, UpdateType::NoUpdate);
}

#[test]
fn test_update_existing_item_title_change() {
    let mut data = TestData::default();
    let id = data.build().status("Active").add();
    let mut item = data.work_items.get(&id).unwrap().clone();
    item.title = "New Title".to_owned();

    let update_type = data.work_items.update(item);
    assert_eq!(update_type, UpdateType::SimpleChange);
}

#[test]
fn test_update_existing_item_assignees_change() {
    let mut data = TestData::default();
    let id = data.build().assignees(&["user1"]).add();
    let mut item = data.work_items.get(&id).unwrap().clone();
    if let WorkItemData::Issue(issue) = &mut item.data {
        issue.assignees = vec!["user2".to_owned()];
    }

    // Assignees changes trigger ChangesHierarchy (see get_issue_update_type)
    let update_type = data.work_items.update(item);
    assert_eq!(update_type, UpdateType::ChangesHierarchy);
}

#[test]
fn test_update_existing_item_status_change() {
    let mut data = TestData::default();
    let id = data.build().status("Active").add();
    let mut item = data.work_items.get(&id).unwrap().clone();
    item.project_item.status = data.fields.status.option_id(Some("Closed")).cloned();

    // Status is a project_item field that may be used for grouping/filtering
    // (see get_project_item_update_type)
    let update_type = data.work_items.update(item);
    assert_eq!(update_type, UpdateType::ChangesHierarchy);
}

#[test]
fn test_update_existing_item_epic_change() {
    let mut data = TestData::default();
    let id = data.build().epic("EpicA").add();
    let mut item = data.work_items.get(&id).unwrap().clone();
    item.project_item.epic = data.fields.epic.option_id(Some("EpicB")).cloned();

    // Epic is used for grouping in NodeBuilder, so changing it affects
    // the hierarchy
    let update_type = data.work_items.update(item);
    assert_eq!(update_type, UpdateType::ChangesHierarchy);
}

#[test]
fn test_update_new_item_returns_changes_hierarchy() {
    let mut data = TestData::default();
    data.build().add();

    // Create a new item not yet in work_items
    let new_item = WorkItem {
        id: WorkItemId("new-item".to_owned()),
        title: "New Item".to_owned(),
        ..WorkItem::default_loaded()
    };

    let update_type = data.work_items.update(new_item.clone());
    assert_eq!(update_type, UpdateType::ChangesHierarchy);
}

#[test]
fn test_update_new_item_added_to_ordered_items() {
    let mut data = TestData::default();
    let existing_id = data.build().add();

    let new_item = WorkItem {
        id: WorkItemId("new-item".to_owned()),
        title: "New Item".to_owned(),
        ..WorkItem::default_loaded()
    };

    data.work_items.update(new_item.clone());

    // New item should be in the HashMap
    assert!(data.work_items.get(&new_item.id).is_some());

    // New item should also appear in ordered iteration (and thus in get_roots)
    let roots = data.work_items.get_roots();
    assert!(roots.contains(&existing_id));
    assert!(roots.contains(&new_item.id));
}

#[test]
fn test_update_new_item_appears_in_iteration() {
    let mut data = TestData::default();
    data.build().add();

    let new_id = WorkItemId("new-item".to_owned());
    let new_item = WorkItem {
        id: new_id.clone(),
        ..WorkItem::default_loaded()
    };

    data.work_items.update(new_item);

    // The new item should be accessible via iter()
    let ids: Vec<&WorkItemId> = data.work_items.iter().collect();
    assert!(ids.contains(&&new_id));
}

#[test]
fn test_update_existing_item_issue_state_change() {
    let mut data = TestData::default();
    let id = data.build().issue_state(IssueState::OPEN).add();
    let mut item = data.work_items.get(&id).unwrap().clone();
    if let WorkItemData::Issue(issue) = &mut item.data {
        issue.state = IssueState::CLOSED.into();
    }

    let update_type = data.work_items.update(item);
    assert_eq!(update_type, UpdateType::ChangesHierarchy);
}

#[test]
fn test_update_existing_item_sub_issues_change() {
    let mut data = TestData::default();
    let child = data.build().issue().add();
    let id = data.build().issue().add();
    let mut item = data.work_items.get(&id).unwrap().clone();
    if let WorkItemData::Issue(issue) = &mut item.data {
        issue.sub_issues = vec![child.clone()];
    }

    let update_type = data.work_items.update(item);
    assert_eq!(update_type, UpdateType::ChangesHierarchy);
}

#[test]
fn test_update_existing_item_tracked_issues_change() {
    let mut data = TestData::default();
    let tracked = data.build().issue().add();
    let id = data.build().issue().add();
    let mut item = data.work_items.get(&id).unwrap().clone();
    if let WorkItemData::Issue(issue) = &mut item.data {
        issue.tracked_issues = vec![tracked.clone()].into();
    }

    let update_type = data.work_items.update(item);
    assert_eq!(update_type, UpdateType::SimpleChange);
}

#[test]
fn test_update_preserves_existing_ordered_items() {
    let mut data = TestData::default();
    let id1 = data.build().add();
    let id2 = data.build().add();
    let id3 = data.build().add();

    // Update an existing item
    let mut item = data.work_items.get(&id2).unwrap().clone();
    item.title = "Updated Title".to_owned();
    data.work_items.update(item);

    // All items should still be in ordered_items and accessible
    let ids: Vec<&WorkItemId> = data.work_items.iter().collect();
    assert_eq!(ids.len(), 3);
    assert!(ids.contains(&&id1));
    assert!(ids.contains(&&id2));
    assert!(ids.contains(&&id3));
}

#[test]
fn test_update_multiple_new_items_all_appear() {
    let mut data = TestData::default();
    let existing = data.build().add();

    let new1 = WorkItem {
        id: WorkItemId("new-1".to_owned()),
        ..WorkItem::default_loaded()
    };
    let new2 = WorkItem {
        id: WorkItemId("new-2".to_owned()),
        ..WorkItem::default_loaded()
    };

    data.work_items.update(new1.clone());
    data.work_items.update(new2.clone());

    let roots = data.work_items.get_roots();
    assert_eq!(roots.len(), 3);
    assert!(roots.contains(&existing));
    assert!(roots.contains(&new1.id));
    assert!(roots.contains(&new2.id));
}
