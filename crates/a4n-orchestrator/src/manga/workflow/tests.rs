use super::*;
use crate::manga::MangaCharacterPortraitRef;
use crate::manga::pipeline_state::MangaPipelineStage;

#[test]
fn manga_pipeline_stage_keys_are_owned_by_stage_enum() {
    assert_eq!(
        MangaPipelineStage::CharacterBinding.key(),
        "character_binding"
    );
    assert_eq!(
        MangaPipelineStage::SpeechSelection.key(),
        "speech_selection"
    );
    assert_eq!(
        MangaPipelineStage::VisualAnchorExtraction.key(),
        "visual_anchor_extraction"
    );
    assert_eq!(MangaPipelineStage::ItemExtraction.key(), "item_extraction");
    assert_eq!(MangaPipelineStage::ComicSynthesis.key(), "comic_synthesis");
    assert_eq!(MangaPipelineStage::PagePlanning.key(), "page_planning");
    assert_eq!(MangaPipelineStage::DrawingPrompt.key(), "drawing_prompt");
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
        fingerprint["reference_assets"]["character_portraits"][0]["portrait_id"],
        20
    );
    assert_eq!(
        fingerprint["reference_assets"]["character_portraits"][0]["prompt"],
        "portrait"
    );
}
