use super::{
    rules::{anchor_change_conflict, classify_anchor_change, severity_for_impacted_local_state},
    types::AnchorChange,
};
use a4n_core::models::project::text_anchor;
use chrono::Utc;

fn anchor(
    id: i64,
    snapshot_id: i64,
    source_chapter_id: i64,
    paragraph_index: i32,
    start_offset: i32,
    end_offset: i32,
    quote_hash: &str,
) -> text_anchor::Model {
    text_anchor::Model {
        id,
        project_id: 1,
        source_chapter_id: Some(source_chapter_id),
        source_snapshot_id: Some(snapshot_id),
        source_snapshot_chapter_id: Some(snapshot_id * 10 + source_chapter_id),
        source_kind: "chapter_content".to_string(),
        source_id: snapshot_id * 10 + source_chapter_id,
        paragraph_index,
        start_offset,
        end_offset,
        quote: "他推开门。".to_string(),
        quote_hash: quote_hash.to_string(),
        source_version_hash: "source-hash".to_string(),
        created_at: Utc::now().naive_utc(),
    }
}

#[test]
fn classify_anchor_change_detects_unchanged_and_moved() {
    let old = anchor(1, 10, 5, 0, 0, 5, "same");
    let unchanged = anchor(2, 11, 5, 0, 0, 5, "same");
    let moved = anchor(3, 11, 5, 0, 8, 13, "same");

    assert_eq!(
        classify_anchor_change(&old, &[unchanged]),
        AnchorChange::Unchanged
    );
    assert_eq!(
        classify_anchor_change(&old, &[moved]),
        AnchorChange::Moved(3)
    );
}

#[test]
fn classify_anchor_change_treats_missing_and_ambiguous_as_blocking() {
    let old = anchor(1, 10, 5, 0, 0, 5, "same");
    let first = anchor(2, 11, 5, 0, 8, 13, "same");
    let second = anchor(3, 11, 5, 1, 20, 25, "same");

    assert_eq!(classify_anchor_change(&old, &[]), AnchorChange::Missing);
    assert_eq!(
        classify_anchor_change(&old, &[first, second]),
        AnchorChange::Ambiguous
    );
    assert_eq!(
        anchor_change_conflict(&AnchorChange::Missing, "clean"),
        Some(("deleted", None, "blocking"))
    );
    assert_eq!(
        anchor_change_conflict(&AnchorChange::Ambiguous, "draft"),
        Some(("split", None, "blocking"))
    );
}

#[test]
fn severity_for_impacted_local_state_is_conservative() {
    assert_eq!(
        severity_for_impacted_local_state("clean", "updated"),
        "warning"
    );
    assert_eq!(
        severity_for_impacted_local_state("generated", "updated"),
        "blocking"
    );
    assert_eq!(
        severity_for_impacted_local_state("clean", "deleted"),
        "blocking"
    );
}
