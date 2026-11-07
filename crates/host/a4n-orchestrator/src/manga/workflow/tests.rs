use super::*;
use crate::manga::pipeline::state::MangaPipelineStage;
use a4n_db::project::MangaCharacterPortraitRefSummary;

#[test]
fn manga_pipeline_stage_keys_are_owned_by_stage_enum() {
    assert_eq!(MangaPipelineStage::PageSlots.key(), "page_slots");
}

#[test]
fn checkpoint_fingerprints_keep_explicit_stages_separate() {
    use a4n_domain::manga::MangaPipelineArtifactStage as Stage;
    let full = manga_pipeline_run_fingerprint("正文", &[], &[], None);
    let storyboard = manga_pipeline_run_fingerprint("正文", &[], &[], Some(Stage::Storyboard));
    let image = manga_pipeline_run_fingerprint("正文", &[], &[], Some(Stage::Image));
    assert_ne!(full, storyboard);
    assert_ne!(storyboard, image);
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
    let fingerprint = manga_pipeline_run_fingerprint("正文", &character_portraits, &[], None);
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
