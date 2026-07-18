use super::*;
use crate::manga::MangaCharacterPortraitRef;

#[test]
fn manga_pipeline_start_stage_input_preserves_optional_and_strict_rules() {
    assert_eq!(parse_manga_pipeline_start_stage(None).expect("none"), None);
    assert_eq!(
        parse_manga_pipeline_start_stage(Some("  ".to_string())).expect("blank"),
        None
    );
    assert_eq!(
        parse_manga_pipeline_start_stage(Some("page_planning".to_string())).expect("known stage"),
        Some(MangaPipelineStartStage::PagePlanning)
    );
    assert_eq!(
        parse_manga_pipeline_start_stage(Some("unknown".to_string()))
            .expect_err("unknown stage should reject")
            .to_string(),
        "参数无效：未知漫画流水线起始步骤: unknown"
    );
}

#[test]
fn manga_pipeline_stage_keys_are_owned_by_stage_enum() {
    assert_eq!(
        MangaPipelineStartStage::CharacterBinding.key(),
        "character_binding"
    );
    assert_eq!(
        MangaPipelineStartStage::SpeechSelection.key(),
        "speech_selection"
    );
    assert_eq!(
        MangaPipelineStartStage::VisualAnchorExtraction.key(),
        "visual_anchor_extraction"
    );
    assert_eq!(
        MangaPipelineStartStage::ItemExtraction.key(),
        "item_extraction"
    );
    assert_eq!(
        MangaPipelineStartStage::ComicSynthesis.key(),
        "comic_synthesis"
    );
    assert_eq!(MangaPipelineStartStage::PagePlanning.key(), "page_planning");
    assert_eq!(
        MangaPipelineStartStage::DrawingPrompt.key(),
        "drawing_prompt"
    );
}

#[test]
fn manga_pipeline_identity_and_fingerprint_preserve_source_and_portrait_ids() {
    let context = MangaReferenceContext {
        character_portraits: vec![MangaCharacterPortraitRef {
            profile_id: 1,
            portrait_id: 20,
            character_name: "林岚".to_string(),
            image_path: "portrait.png".to_string(),
            prompt: "portrait".to_string(),
            portrait_type: "primary".to_string(),
            expression: None,
            pose: None,
            is_primary: true,
        }],
    };

    assert_eq!(manga_pipeline_business_key(3, 7), "manga_pipeline:3:7");
    let fingerprint = manga_pipeline_fingerprint("正文", &context);
    assert_eq!(fingerprint["source"]["content_chars"], 2);
    assert_eq!(
        fingerprint["reference_assets"]["character_portrait_count"],
        1
    );
    assert_eq!(
        fingerprint["reference_assets"]["character_portrait_ids"],
        serde_json::json!([20])
    );
}
