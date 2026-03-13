use crate::data::test_helpers::TestData;
use crate::data::*;
use std::collections::HashMap;

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
