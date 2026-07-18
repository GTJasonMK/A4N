use super::*;
use a4n_core::models::generation::generation_spec;
use a4n_prompt::tera_engine::TeraPromptEngine;
use chrono::NaiveDateTime;
use std::path::Path;

#[test]
fn run_galgame_asset_generation_input_rejects_duplicate_spec_ids() {
    let input = RunGalgameAssetGenerationInput {
        project_id: 1,
        generation_spec_ids: vec![10, 10],
        config_id: None,
        limit: None,
    };

    let err = validate_run_galgame_asset_generation_input(&input)
        .expect_err("duplicate generation_spec_ids must fail before querying specs");

    assert!(err.contains("generation_spec_ids 不能包含重复 ID: 10"));
}

#[tokio::test]
async fn galgame_background_prompt_rejects_missing_required_scene_title() {
    let spec = background_visual_spec(
        10,
        Some(100),
        Some(30),
        Some(40),
        json!({
            "scene": {
                "location": "车站",
                "time_of_day": "夜晚",
                "background_note": "雨夜月台"
            }
        }),
    );
    let plan = GalgameVisualSpecPlan::from_spec(&spec).expect("background visual plan");

    let err = render_galgame_image_prompt(&prompt_engine(), &spec, &plan)
        .await
        .expect_err("background prompt should reject missing title");

    assert!(err.contains("scene.title 缺失"));
}

#[tokio::test]
async fn galgame_background_prompt_renders_registered_template() {
    let spec = background_visual_spec(
        10,
        Some(100),
        Some(30),
        Some(40),
        json!({
            "canonical_key": "galgame:10:background:scene:100",
            "scene": {
                "id": 100,
                "title": "雨夜车站",
                "location": "旧车站月台",
                "time_of_day": "夜晚",
                "background_note": "雨后的冷色灯光和空旷月台"
            },
            "source": {
                "source_snapshot_chapter_id": 40,
                "source_chapter_id": 30
            }
        }),
    );
    let plan = GalgameVisualSpecPlan::from_spec(&spec).expect("background visual plan");
    let prompt = render_galgame_image_prompt(&prompt_engine(), &spec, &plan)
        .await
        .expect("background prompt should render");

    assert!(prompt.contains("视觉小说 Galgame 背景图"));
    assert!(prompt.contains("场景标题：雨夜车站"));
    assert!(prompt.contains("地点：旧车站月台"));
    assert!(prompt.contains("背景规划：雨后的冷色灯光和空旷月台"));
    assert!(!prompt.contains("Scene title:"));
    assert!(!prompt.contains("no watermark"));
}

#[tokio::test]
async fn galgame_event_cg_prompt_rejects_missing_cg_note() {
    let spec = cg_visual_spec(
        10,
        Some(1000),
        Some(30),
        Some(40),
        json!({
            "scene": {
                "title": "雨夜车站",
                "location": "车站"
            },
            "line": {
                "text": "她回过头。"
            }
        }),
    );
    let plan = GalgameVisualSpecPlan::from_spec(&spec).expect("cg visual plan");

    let err = render_galgame_image_prompt(&prompt_engine(), &spec, &plan)
        .await
        .expect_err("event CG prompt should reject missing cg_note");

    assert!(err.contains("cg_note 缺失"));
}

#[tokio::test]
async fn galgame_event_cg_prompt_omits_optional_null_fields_without_defaults() {
    let spec = cg_visual_spec(
        10,
        Some(1000),
        Some(30),
        Some(40),
        json!({
            "scene": {
                "title": "雨夜车站",
                "location": null,
                "time_of_day": null
            },
            "line": {
                "text": "她回过头。",
                "expression": null,
                "pose": null
            },
            "cg_note": "两人在雨中对视",
            "cg_importance": "high"
        }),
    );
    let plan = GalgameVisualSpecPlan::from_spec(&spec).expect("cg visual plan");
    let prompt = render_galgame_image_prompt(&prompt_engine(), &spec, &plan)
        .await
        .expect("event CG prompt should allow explicit null optional fields");

    assert!(prompt.contains("场景标题：雨夜车站"));
    assert!(prompt.contains("事件文本：她回过头。"));
    assert!(prompt.contains("CG 方向：两人在雨中对视"));
    assert!(prompt.contains("重要程度：high"));
    assert!(!prompt.contains("neutral"));
    assert!(!prompt.contains("natural"));
    assert!(!prompt.contains("未指定"));
}
