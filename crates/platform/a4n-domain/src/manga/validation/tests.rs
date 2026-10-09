use super::super::contracts::*;
use super::super::{
    MangaPageGenerationLayoutV3, MangaPagePromptSourceSummary, MangaReferenceBindingSummary,
};
use crate::media_authoring::AdaptationBeatRef;
use serde_json::json;

fn beat_ref() -> AdaptationBeatRef {
    AdaptationBeatRef {
        unit_id: "door".into(),
        beat_id: "open".into(),
    }
}

fn valid_plan() -> MangaPagePlanV4 {
    MangaPagePlanV4 {
        schema_version: MangaPagePlanSchemaVersion::V4,
        adaptation_plan_id: 3,
        adaptation_plan_revision: 1,
        pages: vec![MangaPageV4 {
            page_number: 1,
            beat_refs: vec![beat_ref()],
            role: MangaPageRoleV2::Opening,
            pacing: MangaPagePacingV2::Slow,
            importance: MangaPageImportanceV2::High,
            reading_flow: MangaReadingFlowV2::ZPath,
            page_goal: "让读者跟随角色发现门后异常".to_string(),
            visual_thesis: "门框把角色压在狭窄前景中".to_string(),
            entry_state: "门关闭".to_string(),
            exit_state: "门打开".to_string(),
            dominant_panel_number: 1,
            panels: vec![MangaPanelV2 {
                panel_number: 1,
                beat_refs: vec![beat_ref()],
                function: MangaPanelFunctionV2::Revelation,
                size: MangaPanelSizeV2::Dominant,
                shot_scale: MangaShotScaleV2::Wide,
                camera_angle: MangaCameraAngleV2::EyeLevel,
                participants: vec!["角色".to_string()],
                composition: "门框前景包围角色，门缝亮光指向视线终点".to_string(),
                observable_action: "角色压下门把并看向门缝".to_string(),
                reaction: Some("角色屏住呼吸".to_string()),
                spatial_relationship: "角色在门外，未知空间在画面深处".to_string(),
                continuity_from_previous: None,
                text_layers: vec![MangaTextLayerV2 {
                    layer_index: 1,
                    kind: MangaTextLayerKindV2::Sfx,
                    speaker: None,
                    text: "咔哒".to_string(),
                    placement: MangaTextPlacementV2::UpperCenter,
                    avoid_covering: vec!["门把".to_string()],
                }],
            }],
        }],
    }
}

#[test]
fn validates_structured_page_plan() {
    valid_plan().validate().expect("valid manga page plan");
}

#[test]
fn accepts_page_state_paraphrases() {
    let mut plan = valid_plan();
    let mut next_page = plan.pages[0].clone();
    next_page.page_number = 2;
    next_page.entry_state = "门已经打开，角色停在门口".to_string();
    next_page.exit_state = "角色进入门内".to_string();
    plan.pages.push(next_page);

    plan.validate()
        .expect("natural-language continuity must not require textual identity");
}

#[test]
fn rejects_missing_single_dominant_panel() {
    let mut plan = valid_plan();
    plan.pages[0].panels[0].size = MangaPanelSizeV2::Large;

    let error = plan.validate().expect_err("dominant panel is required");
    assert_eq!(error.path, "pages[0].panels");
}

#[test]
fn rejects_panel_beat_outside_page() {
    let mut plan = valid_plan();
    plan.pages[0].panels[0].beat_refs[0].beat_id = "other".into();

    let error = plan.validate().expect_err("foreign beat ref must fail");
    assert_eq!(error.path, "pages[0].panels[0].beat_refs");
}

#[test]
fn page_generation_layout_keeps_adaptation_identity_only_in_source() {
    let page = valid_plan().pages.remove(0);
    let mut layout = MangaPageGenerationLayoutV3 {
        schema_version: MangaPageGenerationSchemaVersion::V3,
        page_plan: page,
        page_plan_ref: None,
        drawing_prompt: MangaPageDrawingPromptV2 {
            schema_version: MangaPageDrawingPromptSchemaVersion::V2,
            page_number: 1,
            image_prompt: "整页漫画".to_string(),
        },
        reference_bindings: vec![MangaReferenceBindingSummary {
            character_name: "主角".to_string(),
            bound_asset: None,
            user_added_assets: Vec::new(),
        }],
        prompt_fingerprint: json!({"sha256": "fingerprint"}),
        source: MangaPagePromptSourceSummary {
            root_project_id: 1,
            source_chapter_id: 2,
            source_snapshot_id: Some(4),
            source_snapshot_chapter_id: Some(5),
            source_revision_key: "snapshot:4:chapter:5:content".to_string(),
            content_sha256: "content".to_string(),
            adaptation_plan_id: 3,
            adaptation_plan_revision: 1,
            adaptation_plan_sha256: "plan".to_string(),
        },
    };

    layout.validate(1).expect("canonical layout should pass");
    let serialized = serde_json::to_value(&layout).expect("layout should serialize");
    assert!(serialized.get("adaptation_plan_id").is_none());
    assert!(serialized.get("adaptation_plan_revision").is_none());
    assert!(!serialized.to_string().contains("shooting_script"));

    layout.source.adaptation_plan_id = 0;
    let error = layout
        .validate(1)
        .expect_err("invalid source adaptation identity must fail");
    assert_eq!(error.path, "source.adaptation_plan_id");
}
