use crate::data::*;

#[test]
fn test_undo_add_change() {
    let mut changes = Changes::default();
    let mut history = UndoHistory::default();
    let change = Change {
        work_item_id: WorkItemId("item1".to_owned()),
        data: ChangeData::Status(Some(FieldOptionId("status1".to_owned()))),
    };

    history.track_add(&mut changes, change.clone());
    assert_eq!(changes.len(), 1);
    assert!(history.can_undo());
    assert!(!history.can_redo());

    history.undo(&mut changes);
    assert_eq!(changes.len(), 0);
    assert!(!history.can_undo());
    assert!(history.can_redo());

    history.redo(&mut changes);
    assert_eq!(changes.len(), 1);
    assert!(history.can_undo());
    assert!(!history.can_redo());
}

#[test]
fn test_undo_remove_change() {
    let mut changes = Changes::default();
    let mut history = UndoHistory::default();
    let change = Change {
        work_item_id: WorkItemId("item1".to_owned()),
        data: ChangeData::Status(Some(FieldOptionId("status1".to_owned()))),
    };

    history.track_add(&mut changes, change.clone());
    history.track_remove(&mut changes, change.clone());
    assert_eq!(changes.len(), 0);

    // Undo the remove - should restore the change
    history.undo(&mut changes);
    assert_eq!(changes.len(), 1);

    // Undo the add - should be empty again
    history.undo(&mut changes);
    assert_eq!(changes.len(), 0);
    assert!(!history.can_undo());
}

#[test]
fn test_undo_clear_changes() {
    let mut changes = Changes::default();
    let mut history = UndoHistory::default();
    history.track_add(
        &mut changes,
        Change {
            work_item_id: WorkItemId("item1".to_owned()),
            data: ChangeData::Status(Some(FieldOptionId("s1".to_owned()))),
        },
    );
    history.track_add(
        &mut changes,
        Change {
            work_item_id: WorkItemId("item2".to_owned()),
            data: ChangeData::Epic(Some(FieldOptionId("e1".to_owned()))),
        },
    );

    assert_eq!(changes.len(), 2);

    history.track_clear(&mut changes);
    assert_eq!(changes.len(), 0);
    assert!(history.can_undo());

    history.undo(&mut changes);
    assert_eq!(changes.len(), 2);
    assert!(history.can_redo());

    history.redo(&mut changes);
    assert_eq!(changes.len(), 0);
}

#[test]
fn test_undo_overwrite_change() {
    let mut changes = Changes::default();
    let mut history = UndoHistory::default();
    let change1 = Change {
        work_item_id: WorkItemId("item1".to_owned()),
        data: ChangeData::Status(Some(FieldOptionId("status1".to_owned()))),
    };
    let change2 = Change {
        work_item_id: WorkItemId("item1".to_owned()),
        data: ChangeData::Status(Some(FieldOptionId("status2".to_owned()))),
    };

    history.track_add(&mut changes, change1.clone());
    history.track_add(&mut changes, change2.clone());
    assert_eq!(changes.len(), 1);

    // Undo the second add - should restore the first change value
    history.undo(&mut changes);
    assert_eq!(changes.len(), 1);
    // Verify the first change is back by checking it's iterable and matches
    let restored_changes: Vec<&Change> = changes.into_iter().collect();
    assert_eq!(restored_changes.len(), 1);
    assert_eq!(restored_changes[0], &change1);

    // Undo the first add - should be empty
    history.undo(&mut changes);
    assert_eq!(changes.len(), 0);
}

#[test]
fn test_multiple_undo_redo() {
    let mut changes = Changes::default();
    let mut history = UndoHistory::default();
    let c1 = Change {
        work_item_id: WorkItemId("item1".to_owned()),
        data: ChangeData::Status(Some(FieldOptionId("s1".to_owned()))),
    };
    let c2 = Change {
        work_item_id: WorkItemId("item2".to_owned()),
        data: ChangeData::Epic(Some(FieldOptionId("e1".to_owned()))),
    };

    history.track_add(&mut changes, c1.clone());
    history.track_add(&mut changes, c2.clone());
    assert_eq!(changes.len(), 2);

    history.undo(&mut changes);
    assert_eq!(changes.len(), 1);

    history.undo(&mut changes);
    assert_eq!(changes.len(), 0);

    history.redo(&mut changes);
    assert_eq!(changes.len(), 1);

    history.redo(&mut changes);
    assert_eq!(changes.len(), 2);
}

#[test]
fn test_new_change_clears_redo_stack() {
    let mut changes = Changes::default();
    let mut history = UndoHistory::default();
    let c1 = Change {
        work_item_id: WorkItemId("item1".to_owned()),
        data: ChangeData::Status(Some(FieldOptionId("s1".to_owned()))),
    };
    let c2 = Change {
        work_item_id: WorkItemId("item2".to_owned()),
        data: ChangeData::Epic(Some(FieldOptionId("e1".to_owned()))),
    };

    history.track_add(&mut changes, c1.clone());
    history.undo(&mut changes);
    assert!(history.can_redo());

    history.track_add(&mut changes, c2.clone());
    assert!(!history.can_redo());
}

#[test]
fn test_undo_add_changes_batch() {
    let mut changes = Changes::default();
    let mut history = UndoHistory::default();

    // Pre-populate with a change to add first
    history.track_add(
        &mut changes,
        Change {
            work_item_id: WorkItemId("item0".to_owned()),
            data: ChangeData::Status(Some(FieldOptionId("s0".to_owned()))),
        },
    );
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

    history.track_add_changes(&mut changes, batch);
    assert_eq!(changes.len(), 3);

    // Undo should reverse the entire batch at once
    history.undo(&mut changes);
    assert_eq!(changes.len(), 1);

    // Redo should restore the entire batch at once
    history.redo(&mut changes);
    assert_eq!(changes.len(), 3);
}

#[test]
fn test_undo_save_changes() {
    let mut changes = Changes::default();
    let mut history = UndoHistory::default();

    // Add two changes
    history.track_add(
        &mut changes,
        Change {
            work_item_id: WorkItemId("item1".to_owned()),
            data: ChangeData::Status(Some(FieldOptionId("s1".to_owned()))),
        },
    );
    history.track_add(
        &mut changes,
        Change {
            work_item_id: WorkItemId("item2".to_owned()),
            data: ChangeData::Epic(Some(FieldOptionId("e1".to_owned()))),
        },
    );
    assert_eq!(changes.len(), 2);

    // Simulate a successful save: snapshot, then clear data (as save() does)
    let pre_save = changes.clone();
    changes = Changes::default(); // all changes saved successfully
    history.track_save(&changes, pre_save);

    assert_eq!(changes.len(), 0);
    assert!(history.can_undo());

    // Undo the save: restores all changes as unsaved local changes
    history.undo(&mut changes);
    assert_eq!(changes.len(), 2);
    assert!(history.can_redo());

    // Redo the save: changes are cleared again
    history.redo(&mut changes);
    assert_eq!(changes.len(), 0);
}

#[test]
fn test_undo_save_with_partial_failure() {
    let mut changes = Changes::default();
    let mut history = UndoHistory::default();

    let change1 = Change {
        work_item_id: WorkItemId("item1".to_owned()),
        data: ChangeData::Status(Some(FieldOptionId("s1".to_owned()))),
    };
    let change2 = Change {
        work_item_id: WorkItemId("item2".to_owned()),
        data: ChangeData::Epic(Some(FieldOptionId("e1".to_owned()))),
    };

    history.track_add(&mut changes, change1.clone());
    history.track_add(&mut changes, change2.clone());
    assert_eq!(changes.len(), 2);

    // Simulate partial save: item1 saved, item2 failed (remains in changes)
    let pre_save = changes.clone();
    changes = Changes::default();
    changes.add(change2.clone()); // item2 failed, stays in changes
    history.track_save(&changes, pre_save);

    assert_eq!(changes.len(), 1);
    assert!(history.can_undo());

    // Undo the save: restores all original changes
    history.undo(&mut changes);
    assert_eq!(changes.len(), 2);

    // Redo: goes back to only the failed change
    history.redo(&mut changes);
    assert_eq!(changes.len(), 1);
    let remaining: Vec<&Change> = changes.into_iter().collect();
    assert_eq!(remaining[0], &change2);
}

#[test]
fn test_save_empty_changes_no_undo() {
    let changes = Changes::default();
    let mut history = UndoHistory::default();

    // Save with no changes should not add an undo entry
    let pre_save = changes.clone();
    history.track_save(&changes, pre_save);

    assert!(!history.can_undo());
}

#[test]
fn test_save_all_failed_no_undo() {
    let mut changes = Changes::default();
    let mut history = UndoHistory::default();

    let change = Change {
        work_item_id: WorkItemId("item1".to_owned()),
        data: ChangeData::Status(Some(FieldOptionId("s1".to_owned()))),
    };

    history.track_add(&mut changes, change.clone());

    // Simulate all saves failing: changes are unchanged after save attempt
    let pre_save = changes.clone();
    // changes remains the same (all failed)
    history.track_save(&changes, pre_save);

    // Should only have the undo for the original add, not for the no-op save
    assert!(history.can_undo());
    history.undo(&mut changes);
    assert_eq!(changes.len(), 0);
    assert!(!history.can_undo()); // No more undo entries
}

#[test]
fn test_save_clears_redo_stack() {
    let mut changes = Changes::default();
    let mut history = UndoHistory::default();

    let change = Change {
        work_item_id: WorkItemId("item1".to_owned()),
        data: ChangeData::Status(Some(FieldOptionId("s1".to_owned()))),
    };

    history.track_add(&mut changes, change.clone());
    history.undo(&mut changes);
    assert!(history.can_redo());

    // Re-add and save
    history.track_add(&mut changes, change.clone());
    let pre_save = changes.clone();
    changes = Changes::default();
    history.track_save(&changes, pre_save);

    // Save should have cleared the redo stack
    assert!(!history.can_redo());
}
