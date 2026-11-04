// SPDX-License-Identifier: GPL-3.0-or-later
use gameclip_core::retention::{
    ClipInfo, ClipStatus, FolderKind, RetentionPlan, Warning, plan_deletions, retention_groups,
};
use time::{Duration, OffsetDateTime};

const GB: u64 = 1_000_000_000;

fn clip(row_id: u32, gb: u64, age_days: i64) -> ClipInfo {
    ClipInfo {
        row_id,
        folder: FolderKind::Clips,
        size_bytes: gb * GB,
        created_at: OffsetDateTime::UNIX_EPOCH + Duration::days(1000 - age_days),
        starred: false,
        status: ClipStatus::Ok,
        placeholder: false,
        in_current_folder: true,
    }
}

#[test]
fn under_limit_deletes_nothing() {
    let clips = [clip(1, 4, 3), clip(2, 4, 2)];
    assert_eq!(
        plan_deletions(&clips, 10 * GB, None),
        RetentionPlan::default()
    );
}

#[test]
fn deletes_oldest_first_until_under_limit() {
    let clips = [clip(1, 4, 1), clip(2, 4, 3), clip(3, 4, 2)];
    let plan = plan_deletions(&clips, 10 * GB, None);
    assert_eq!(plan.delete, vec![2]);
    assert_eq!(plan.warning, None);
}

#[test]
fn starred_clips_are_never_deleted() {
    let mut old = clip(1, 4, 5);
    old.starred = true;
    let clips = [old, clip(2, 4, 3), clip(3, 4, 1)];
    assert_eq!(plan_deletions(&clips, 10 * GB, None).delete, vec![2]);
}

#[test]
fn new_clip_is_protected() {
    let clips = [clip(1, 6, 2), clip(2, 6, 0)];
    assert_eq!(plan_deletions(&clips, 10 * GB, Some(1)).delete, vec![2]);
}

#[test]
fn foreign_and_old_folder_clips_do_not_count() {
    let mut foreign = clip(1, 8, 5);
    foreign.status = ClipStatus::Foreign;
    let mut elsewhere = clip(2, 8, 4);
    elsewhere.in_current_folder = false;
    let mut pending = clip(3, 8, 3);
    pending.status = ClipStatus::Pending;
    let clips = [foreign, elsewhere, pending, clip(4, 4, 1)];
    assert_eq!(
        plan_deletions(&clips, 10 * GB, None),
        RetentionPlan::default()
    );
}

#[test]
fn placeholders_count_zero_and_are_never_deleted() {
    let mut cloud = clip(1, 8, 9);
    cloud.placeholder = true;
    let clips = [cloud, clip(2, 6, 2), clip(3, 6, 1)];
    let plan = plan_deletions(&clips, 10 * GB, None);
    assert_eq!(plan.delete, vec![2]);
}

#[test]
fn all_starred_gives_warning_and_deletes_nothing() {
    let mut a = clip(1, 6, 2);
    a.starred = true;
    let mut b = clip(2, 6, 1);
    b.starred = true;
    let plan = plan_deletions(&[a, b], 10 * GB, None);
    assert!(plan.delete.is_empty());
    assert_eq!(plan.warning, Some(Warning::AllStarred));
}

#[test]
fn clip_larger_than_limit_gives_warning() {
    let plan = plan_deletions(&[clip(1, 12, 0)], 10 * GB, Some(1));
    assert!(plan.delete.is_empty());
    assert_eq!(plan.warning, Some(Warning::ClipLargerThanLimit));
}

#[test]
fn shared_folder_uses_one_group_with_smaller_limit() {
    let groups = retention_groups(10 * GB, 8 * GB, true);
    assert_eq!(
        groups,
        vec![(vec![FolderKind::Clips, FolderKind::Exports], 8 * GB)]
    );

    // 6 GB Clips + 6 GB Exporte, gemischt nach Alter: die ältesten 4 GB gehen.
    let mut e1 = clip(10, 2, 6);
    e1.folder = FolderKind::Exports;
    let mut e2 = clip(11, 2, 4);
    e2.folder = FolderKind::Exports;
    let mut e3 = clip(12, 2, 2);
    e3.folder = FolderKind::Exports;
    let clips = [clip(1, 2, 5), clip(2, 2, 3), clip(3, 2, 1), e1, e2, e3];
    let plan = plan_deletions(&clips, groups[0].1, None);
    assert_eq!(plan.delete, vec![10, 1]);
}

#[test]
fn separate_folders_use_two_groups() {
    let groups = retention_groups(10 * GB, 8 * GB, false);
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[1], (vec![FolderKind::Exports], 8 * GB));
}
