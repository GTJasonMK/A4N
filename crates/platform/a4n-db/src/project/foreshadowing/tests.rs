use a4n_db_models::models::character::foreshadowing_index;
use chrono::Utc;
use serde_json::json;

use super::{
    contracts::ForeshadowingAnalysisDraft,
    rules::{
        ForeshadowingResolutionAction, foreshadowing_matches_analysis_draft,
        foreshadowing_related_characters_after_setup_resync, foreshadowing_resolution_action,
    },
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
fn resolution_action_is_idempotent_and_rejects_wrong_timeline() {
    let pending = foreshadowing("pending", None);
    assert_eq!(
        foreshadowing_resolution_action(&pending, 8),
        Ok(ForeshadowingResolutionAction::Resolve)
    );
    assert_eq!(
        foreshadowing_resolution_action(&foreshadowing("resolved", Some(8)), 8),
        Ok(ForeshadowingResolutionAction::Keep)
    );
    assert!(
        foreshadowing_resolution_action(&foreshadowing("resolved", Some(9)), 8)
            .unwrap_err()
            .contains("第 9 章回收")
    );
    let mut future_setup = pending;
    future_setup.setup_chapter = 8;
    assert!(
        foreshadowing_resolution_action(&future_setup, 8)
            .unwrap_err()
            .contains("不早于回收章节")
    );
}

#[test]
fn analysis_draft_only_fills_missing_related_characters() {
    let resolved = foreshadowing("resolved", Some(12));
    let matching = ForeshadowingAnalysisDraft {
        description: resolved.description.clone(),
        related_characters: resolved.related_characters.clone(),
        importance: resolved.importance,
    };
    assert!(foreshadowing_matches_analysis_draft(&resolved, &matching));

    let changed_characters = ForeshadowingAnalysisDraft {
        related_characters: Some(json!(["配角"])),
        ..matching.clone()
    };
    assert!(foreshadowing_matches_analysis_draft(
        &resolved,
        &changed_characters
    ));

    let mut missing = foreshadowing("pending", None);
    missing.related_characters = None;
    let discovered = ForeshadowingAnalysisDraft {
        description: missing.description.clone(),
        related_characters: Some(json!(["配角"])),
        importance: 5,
    };
    assert!(!foreshadowing_matches_analysis_draft(&missing, &discovered));
    assert_eq!(
        foreshadowing_related_characters_after_setup_resync(&missing, &discovered),
        Some(json!(["配角"]))
    );
}
