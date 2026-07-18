use serde_json::json;

use super::{
    MANGA_PAGE_DRAWING_PROMPT_V2, MANGA_PAGE_PLAN_V3, MangaPageDrawingPromptV2, MangaPagePacingV2,
    MangaPagePlanV3, MangaPageRoleV2, MangaPanelFunctionV2, MangaShootingBeatRefV3,
    MangaTextLayerKindV2, MangaTextPlacementV2,
};

#[test]
fn root_page_plan_and_drawing_prompt_reject_unknown_fields() {
    let plan = json!({
        "schema_version": MANGA_PAGE_PLAN_V3,
        "shooting_script_id": 1,
        "shooting_script_revision": 2,
        "pages": [],
        "extra": true
    });
    assert!(serde_json::from_value::<MangaPagePlanV3>(plan).is_err());

    let prompt = json!({
        "schema_version": MANGA_PAGE_DRAWING_PROMPT_V2,
        "page_number": 1,
        "image_prompt": "构图",
        "extra": true
    });
    assert!(serde_json::from_value::<MangaPageDrawingPromptV2>(prompt).is_err());
}

#[test]
fn shooting_beat_reference_rejects_unknown_fields_and_preserves_order_traits() {
    let invalid = json!({
        "scene_index": 1,
        "shot_index": 2,
        "beat_index": 3,
        "extra": true
    });
    assert!(serde_json::from_value::<MangaShootingBeatRefV3>(invalid).is_err());

    let first = MangaShootingBeatRefV3 {
        scene_index: 1,
        shot_index: 1,
        beat_index: 2,
    };
    let second = MangaShootingBeatRefV3 {
        scene_index: 1,
        shot_index: 2,
        beat_index: 1,
    };
    assert!(first < second);
}

#[test]
fn manga_enums_preserve_snake_case_wire_values() {
    assert_eq!(
        serde_json::to_value(MangaPageRoleV2::Opening).expect("page role wire value"),
        json!("opening")
    );
    assert_eq!(
        serde_json::to_value(MangaPagePacingV2::Explosive).expect("page pacing wire value"),
        json!("explosive")
    );
    assert_eq!(
        serde_json::to_value(MangaPanelFunctionV2::Revelation).expect("panel function wire value"),
        json!("revelation")
    );
    assert_eq!(
        serde_json::to_value(MangaTextLayerKindV2::Sfx).expect("text layer kind wire value"),
        json!("sfx")
    );
    assert_eq!(
        serde_json::to_value(MangaTextPlacementV2::UpperCenter).expect("text placement wire value"),
        json!("upper_center")
    );
}
