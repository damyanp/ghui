use crate::data::test_helpers::TestData;
use crate::data::*;
use std::collections::HashSet;

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
