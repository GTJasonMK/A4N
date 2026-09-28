use serde_json::json;

use super::{
    MANGA_PAGE_DRAWING_PROMPT_V2, MANGA_PAGE_PLAN_V4, MangaCameraAngleV2,
    MangaPageDrawingPromptSchemaVersion, MangaPageDrawingPromptV2,
    MangaPageGenerationSchemaVersion, MangaPagePacingV2, MangaPagePlanSchemaVersion,
    MangaPagePlanV4, MangaPageRoleV2, MangaPanelFunctionV2, MangaShotScaleV2, MangaTextLayerKindV2,
    MangaTextPlacementV2,
};
use crate::media_authoring::AdaptationBeatRef;

#[test]
fn manga_schema_versions_reject_retired_values() {
    assert!(
        serde_json::from_value::<MangaPagePlanSchemaVersion>(json!("manga_page_plan_v3")).is_err()
    );
    assert!(
        serde_json::from_value::<MangaPageDrawingPromptSchemaVersion>(json!(
            "manga_page_drawing_prompt_v1"
        ))
        .is_err()
    );
    assert!(
        serde_json::from_value::<MangaPageGenerationSchemaVersion>(json!(
            "manga_page_generation_v2"
        ))
        .is_err()
    );
}

#[test]
fn root_page_plan_and_drawing_prompt_reject_unknown_fields() {
    let plan = json!({
        "schema_version": MANGA_PAGE_PLAN_V4,
        "adaptation_plan_id": 1,
        "adaptation_plan_revision": 2,
        "pages": [],
        "extra": true
    });
    assert!(serde_json::from_value::<MangaPagePlanV4>(plan).is_err());

    let prompt = json!({
        "schema_version": MANGA_PAGE_DRAWING_PROMPT_V2,
        "page_number": 1,
        "image_prompt": "构图",
        "extra": true
    });
    assert!(serde_json::from_value::<MangaPageDrawingPromptV2>(prompt).is_err());
}

#[test]
fn adaptation_beat_reference_rejects_unknown_fields_and_preserves_identity() {
    let invalid = json!({
        "unit_id": "door",
        "beat_id": "open",
        "extra": true
    });
    assert!(serde_json::from_value::<AdaptationBeatRef>(invalid).is_err());

    let first = AdaptationBeatRef {
        unit_id: "door".into(),
        beat_id: "open".into(),
    };
    let second = AdaptationBeatRef {
        unit_id: "door".into(),
        beat_id: "react".into(),
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
    assert_eq!(
        serde_json::from_value::<MangaShotScaleV2>(json!("medium_wide"))
            .expect("shooting-script shot scale remains valid in manga"),
        MangaShotScaleV2::MediumWide
    );
    assert_eq!(
        serde_json::from_value::<MangaCameraAngleV2>(json!("subjective"))
            .expect("shooting-script camera angle remains valid in manga"),
        MangaCameraAngleV2::Subjective
    );
}

#[test]
fn manga_text_placement_rejects_noncanonical_wire_values() {
    for (alias, placement, canonical) in [
        ("upper_left", MangaTextPlacementV2::TopLeft, "top_left"),
        ("upper_right", MangaTextPlacementV2::TopRight, "top_right"),
        (
            "top_center",
            MangaTextPlacementV2::UpperCenter,
            "upper_center",
        ),
        (
            "lower_left",
            MangaTextPlacementV2::BottomLeft,
            "bottom_left",
        ),
        (
            "lower_right",
            MangaTextPlacementV2::BottomRight,
            "bottom_right",
        ),
        (
            "bottom_center",
            MangaTextPlacementV2::LowerCenter,
            "lower_center",
        ),
    ] {
        assert!(
            serde_json::from_value::<MangaTextPlacementV2>(json!(alias)).is_err(),
            "noncanonical placement {alias} must be rejected"
        );
        assert_eq!(
            serde_json::to_value(placement).expect("placement should serialize"),
            json!(canonical)
        );
    }
}
