use a4n_core::models::character::foreshadowing_index;
use chrono::Utc;
use serde_json::json;

use super::{
    contracts::ForeshadowingAnalysisDraft,
    rules::{foreshadowing_matches_analysis_draft, foreshadowing_status_after_setup_resync},
};

fn foreshadowing(status: &str, payoff_chapter: Option<i32>) -> foreshadowing_index::Model {
    let now = Utc::now().naive_utc();
    foreshadowing_index::Model {
        id: 1,
        project_id: 7,
        description: "门后的脚步声".to_string(),
        setup_chapter: 2,
        payoff_chapter,
        status: status.to_string(),
        related_characters: Some(json!(["主角"])),
        importance: 4,
        created_at: now,
        updated_at: now,
    }
}

#[test]
fn foreshadowing_setup_resync_preserves_terminal_status() {
    assert_eq!(
        foreshadowing_status_after_setup_resync("resolved", Some(12), "pending"),
        "resolved"
    );
    assert_eq!(
        foreshadowing_status_after_setup_resync("abandoned", None, "pending"),
        "abandoned"
    );
    assert_eq!(
        foreshadowing_status_after_setup_resync("pending", None, "active"),
        "active"
    );
}

#[test]
fn analysis_draft_match_uses_preserved_status_and_payload_fields() {
    let resolved = foreshadowing("resolved", Some(12));
    let matching = ForeshadowingAnalysisDraft {
        description: resolved.description.clone(),
        status: "pending".to_string(),
        related_characters: resolved.related_characters.clone(),
        importance: resolved.importance,
    };
    assert!(foreshadowing_matches_analysis_draft(&resolved, &matching));

    let changed_characters = ForeshadowingAnalysisDraft {
        related_characters: Some(json!(["配角"])),
        ..matching.clone()
    };
    assert!(!foreshadowing_matches_analysis_draft(
        &resolved,
        &changed_characters
    ));

    let pending = foreshadowing("pending", None);
    let changed_status = ForeshadowingAnalysisDraft {
        description: pending.description.clone(),
        status: "active".to_string(),
        related_characters: pending.related_characters.clone(),
        importance: pending.importance,
    };
    assert!(!foreshadowing_matches_analysis_draft(
        &pending,
        &changed_status
    ));
}
