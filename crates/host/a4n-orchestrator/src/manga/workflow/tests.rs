use super::*;
use crate::manga::pipeline::state::MangaPipelineStage;
use crate::manga::{
    manga_completed_stage_can_continue, manga_completed_stage_order, manga_stop_after_order,
};
use a4n_db::project::MangaCharacterPortraitRefSummary;

#[test]
fn manga_pipeline_stage_keys_are_owned_by_stage_enum() {
    assert_eq!(MangaPipelineStage::PageSlots.key(), "page_slots");
}

#[test]
fn completed_manga_pipeline_stages_reuse_forward_or_same_page_stage() {
    use a4n_api_contract::manga::MangaPipelineStopAfter as StopAfter;

    assert_eq!(manga_completed_stage_order("page_content"), Some(0));
    assert_eq!(manga_completed_stage_order("adaptation"), Some(1));
    assert_eq!(manga_completed_stage_order("storyboard"), Some(2));
    assert_eq!(manga_completed_stage_order("reference_discovery"), Some(3));
    assert_eq!(manga_completed_stage_order("drawing_prompt"), Some(4));
    assert_eq!(manga_completed_stage_order("completed"), Some(5));
    assert!(manga_completed_stage_order("page_content").unwrap() < manga_stop_after_order(None));
    assert!(
        manga_completed_stage_order("adaptation").unwrap()
            < manga_stop_after_order(Some(StopAfter::Storyboard))
    );
    assert!(
        manga_completed_stage_order("storyboard").unwrap()
            >= manga_stop_after_order(Some(StopAfter::Storyboard))
    );
    assert!(manga_completed_stage_can_continue(2, 2, true));
    assert!(!manga_completed_stage_can_continue(2, 2, false));
    assert!(!manga_completed_stage_can_continue(2, 1, true));
    assert!(manga_completed_stage_can_continue(5, 5, true));
    assert!(!manga_completed_stage_can_continue(5, 5, false));
}

#[test]
fn manga_pipeline_identity_and_fingerprint_preserve_source_and_portrait_ids() {
    let character_portraits = vec![MangaCharacterPortraitRefSummary {
        profile_id: 1,
        portrait_id: 20,
        character_name: "林岚".to_string(),
        image_path: "portrait.png".to_string(),
        prompt: "portrait".to_string(),
        portrait_type: "primary".to_string(),
        expression: None,
        pose: None,
    }];

    assert_eq!(manga_pipeline_business_key(3, 7), "manga_pipeline:3:7");
    let fingerprint = manga_pipeline_fingerprint("正文", &character_portraits, &[]);
    assert_eq!(fingerprint["source"]["content_chars"], 2);
    assert_eq!(
        fingerprint["reference_assets"]["character_portraits"][0]["portrait_id"],
        20
    );
    assert_eq!(
        fingerprint["reference_assets"]["character_portraits"][0]["prompt"],
        "portrait"
    );
    assert_eq!(
        fingerprint["reference_assets"]["character_portraits"][0]["is_primary"],
        true
    );
}
