use super::output_sync::MangaPromptDraft;
use super::pipeline_prompt::{
    build_manga_step_user_message, manga_event_beats_user_message,
    manga_item_available_source_span_ids, manga_item_extraction_input,
    manga_page_drawing_prompt_page_input, manga_page_planning_input,
    manga_request_context_manifest_summary, manga_speech_selection_user_message,
    manga_visual_anchor_extraction_user_message, validate_manga_character_binding_source_span_ids,
    validate_manga_event_beats_source_span_ids, validate_manga_item_extraction_source_span_ids,
    validate_manga_speech_selection_source_span_ids, validate_manga_visual_anchor_source_span_ids,
};
use super::*;
use crate::chapter::source_spans::source_spans_for_content;
use serde_json::json;

async fn setup_db() -> DatabaseConnection {
    a4n_migration::test_support::connect_test_database().await
}

fn fixed_time(day: u32) -> chrono::NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2026, 5, day)
        .unwrap()
        .and_hms_opt(10, 0, 0)
        .unwrap()
}

#[tokio::test]
async fn sync_generated_outputs_preserves_rendered_panel_when_prompt_unchanged() {
    let db = Arc::new(setup_db().await);
    let service = MangaService::new(db.clone());
    let chapter_id = 8101;
    let project_id = 8201;
    let old_time = fixed_time(1);
    let kept = chapter_manga_prompt::ActiveModel {
        chapter_id: Set(chapter_id),
        project_id: Set(project_id),
        page_number: Set(1),
        panel_number: Set(1),
        prompt_type: Set(MANGA_PAGE_PROMPT_TYPE.to_string()),
        image_prompt: Set("镜头一".to_string()),
        layout: Set(Some(json!({"shot": 1}))),
        generated_image_id: Set(Some(9901)),
        status: Set("completed".to_string()),
        created_at: Set(old_time),
        updated_at: Set(old_time),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap();
    let stale = chapter_manga_prompt::ActiveModel {
        chapter_id: Set(chapter_id),
        project_id: Set(project_id),
        page_number: Set(9),
        panel_number: Set(9),
        prompt_type: Set(MANGA_PAGE_PROMPT_TYPE.to_string()),
        image_prompt: Set("旧镜头".to_string()),
        status: Set("pending".to_string()),
        created_at: Set(old_time),
        updated_at: Set(old_time),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap();

    let rows = service
        .sync_generated_outputs_by_chapter(
            project_id,
            chapter_id,
            vec![
                MangaPromptDraft {
                    chapter_id,
                    project_id,
                    page_number: 1,
                    panel_number: 1,
                    prompt_type: MANGA_PAGE_PROMPT_TYPE.to_string(),
                    image_prompt: "镜头一".to_string(),
                    dialogue_text: None,
                    characters_in_frame: None,
                    layout: Some(json!({"shot": 1})),
                    camera_angle: None,
                    dialogue_position: None,
                    status: "pending".to_string(),
                    generation_traces: Vec::new(),
                },
                MangaPromptDraft {
                    chapter_id,
                    project_id,
                    page_number: 1,
                    panel_number: 2,
                    prompt_type: MANGA_PAGE_PROMPT_TYPE.to_string(),
                    image_prompt: "镜头二".to_string(),
                    dialogue_text: None,
                    characters_in_frame: None,
                    layout: None,
                    camera_angle: None,
                    dialogue_position: None,
                    status: "pending".to_string(),
                    generation_traces: Vec::new(),
                },
            ],
        )
        .await
        .unwrap();

    assert_eq!(rows.len(), 2);
    let stored = chapter_manga_prompt::Entity::find()
        .filter(chapter_manga_prompt::Column::ChapterId.eq(chapter_id))
        .order_by_asc(chapter_manga_prompt::Column::PageNumber)
        .order_by_asc(chapter_manga_prompt::Column::PanelNumber)
        .all(db.as_ref())
        .await
        .unwrap();
    assert_eq!(stored.len(), 2);
    assert_eq!(stored[0].id, kept.id);
    assert_eq!(stored[0].generated_image_id, Some(9901));
    assert_eq!(stored[0].status, "completed");
    assert!(!stored.iter().any(|row| row.id == stale.id));
}

#[tokio::test]
async fn clear_pipeline_checkpoint_rejects_invalid_status_layout() {
    let db = Arc::new(setup_db().await);
    let service = MangaService::new(db.clone());
    let chapter_id = 8301;
    let project_id = 8302;
    let now = fixed_time(1);
    chapter_manga_prompt::ActiveModel {
        chapter_id: Set(chapter_id),
        project_id: Set(project_id),
        page_number: Set(0),
        panel_number: Set(0),
        prompt_type: Set(PIPELINE_STATUS_PROMPT_TYPE.to_string()),
        image_prompt: Set("状态".to_string()),
        layout: Set(Some(json!("invalid"))),
        status: Set("generating".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap();

    let err = service
        .clear_pipeline_checkpoint(project_id, chapter_id)
        .await
        .expect_err("invalid status layout must fail");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(err.to_string().contains("layout 必须是 JSON 对象"));
}

#[test]
fn build_manga_step_user_message_uses_json_payload() {
    let message = build_manga_step_user_message(
        "manga_event_beats",
        "抽取漫画事件卡片",
        json!({
            "chapter": {"content": "章节正文"}
        }),
        json!({
            "format": "json_object",
            "required_fields": ["events"]
        }),
    )
    .expect("manga step message should build");

    assert!(message.contains(r#""prompt_name": "manga_event_beats""#));
    assert!(message.contains(r#""input": {"#));
    assert!(message.contains(r#""output_contract": {"#));
    assert!(!message.contains("## 章节内容"));
}

#[test]
fn manga_event_beats_user_message_includes_source_spans() {
    let source_spans = source_spans_for_content(7, 1, "林晓雨推开门。陌生人站在雨里。");
    let message = manga_event_beats_user_message("林晓雨推开门。陌生人站在雨里。", &source_spans)
        .expect("event beats prompt should serialize");

    assert!(message.contains(r#""source_spans": ["#));
    assert!(message.contains(r#""source_span_rule""#));
    assert!(message.contains("source_span_ids"));
}

#[test]
fn validate_manga_event_and_character_stage_source_span_ids() {
    let source_spans = source_spans_for_content(7, 1, "林晓雨推开门。陌生人站在雨里。");
    let character_span_id = source_spans
        .iter()
        .find(|span| span.text.contains("林晓雨"))
        .expect("character span")
        .id
        .clone();
    let event_span_id = source_spans
        .iter()
        .find(|span| span.text.contains("陌生人"))
        .expect("event span")
        .id
        .clone();

    validate_manga_character_binding_source_span_ids(
        &json!({
            "characters": {
                "林晓雨": {
                    "name": "林晓雨",
                    "appearance": "短发，浅色外套",
                    "personality": "谨慎",
                    "role": "protagonist",
                    "gender": "female",
                    "age_description": "青年",
                    "source_span_ids": [character_span_id]
                }
            }
        }),
        &source_spans,
    )
    .expect("known character source span ids should pass");

    validate_manga_event_beats_source_span_ids(
        &json!({
            "events": [{
                "index": 0,
                "type": "description",
                "description": "陌生人站在雨里",
                "participants": ["林晓雨", "陌生人"],
                "importance": "normal",
                "is_climax": false,
                "source_span_ids": [event_span_id]
            }]
        }),
        &source_spans,
    )
    .expect("known event source span ids should pass");
}

#[test]
fn validate_manga_character_binding_source_span_ids_rejects_missing_or_unknown_id() {
    let source_spans = source_spans_for_content(7, 1, "林晓雨推开门。");
    let missing = validate_manga_character_binding_source_span_ids(
        &json!({
            "characters": {
                "林晓雨": {
                    "name": "林晓雨",
                    "appearance": "短发，浅色外套",
                    "personality": "谨慎",
                    "role": "protagonist",
                    "gender": "female",
                    "age_description": "青年"
                }
            }
        }),
        &source_spans,
    )
    .expect_err("missing source span ids must fail");
    assert!(missing.to_string().contains("source_span_ids"));

    let unknown = validate_manga_character_binding_source_span_ids(
        &json!({
            "characters": {
                "林晓雨": {
                    "name": "林晓雨",
                    "appearance": "短发，浅色外套",
                    "personality": "谨慎",
                    "role": "protagonist",
                    "gender": "female",
                    "age_description": "青年",
                    "source_span_ids": ["ch1:s99:f99"]
                }
            }
        }),
        &source_spans,
    )
    .expect_err("unknown source span ids must fail");
    assert!(unknown.to_string().contains("source_span_ids"));
    assert!(unknown.to_string().contains("无效"));
}

#[test]
fn manga_speech_selection_user_message_uses_event_windows() {
    let source_spans =
        source_spans_for_content(7, 1, "林晓雨低声说，你好。她忽然意识到这不是巧合。");
    let span_id = source_spans
        .iter()
        .find(|span| span.text.contains("你好"))
        .expect("dialogue span")
        .id
        .clone();
    let message = manga_speech_selection_user_message(
        json!({"林晓雨": {"name": "林晓雨"}}),
        json!([{"index": 0, "description": "林晓雨与陌生人对话", "source_span_ids": [span_id]}]),
        &source_spans,
        json!({"character_portraits": []}),
    )
    .expect("speech selection prompt should serialize");

    assert!(message.contains(r#""source_windows": ["#));
    assert!(message.contains(r#""available_source_spans": ["#));
    assert!(!message.contains(r#""content": "林晓雨低声说，你好。她忽然意识到这不是巧合。""#));
    assert!(message.contains(r#""source_span_rule""#));
    assert!(message.contains("input.chapter.available_source_spans"));
    assert!(!message.contains("input.chapter.source_spans"));
    assert!(!message.contains(r#""chapter_id""#));
    assert!(!message.contains(r#""chapter_number""#));
    assert!(!message.contains(r#""char_start""#));
    assert!(!message.contains(r#""char_end""#));
    assert!(!message.contains(r#""long_text""#));
    assert!(message.contains("source_span_id"));
}

#[test]
fn validate_manga_speech_selection_source_span_ids_accepts_known_ids() {
    let source_spans = source_spans_for_content(7, 1, "林晓雨低声说，你好。三天后，她回到公司。");
    let dialogue_span_id = source_spans
        .iter()
        .find(|span| span.text.contains("你好"))
        .expect("dialogue span")
        .id
        .clone();
    let narration_span_id = source_spans
        .iter()
        .find(|span| span.text.contains("三天后"))
        .expect("narration span")
        .id
        .clone();

    validate_manga_speech_selection_source_span_ids(
        &json!({
            "dialogues": [{
                "index": 0,
                "event_index": 0,
                "speaker": "林晓雨",
                "content": "你好",
                "emotion": "neutral",
                "target": null,
                "is_internal": false,
                "bubble_type": "normal",
                "source_span_id": dialogue_span_id
            }],
            "narrations": [{
                "index": 0,
                "event_index": 1,
                "content": "三天后",
                "narration_type": "time",
                "position": "top",
                "source_span_id": narration_span_id
            }]
        }),
        &source_spans,
    )
    .expect("known source spans should pass");
}

#[test]
fn validate_manga_speech_selection_source_span_ids_rejects_missing_or_unknown_id() {
    let source_spans = source_spans_for_content(7, 1, "林晓雨低声说，你好。");
    let missing = validate_manga_speech_selection_source_span_ids(
        &json!({
            "dialogues": [{
                "index": 0,
                "event_index": 0,
                "speaker": "林晓雨",
                "content": "你好",
                "emotion": "neutral",
                "target": null,
                "is_internal": false,
                "bubble_type": "normal"
            }],
            "narrations": []
        }),
        &source_spans,
    )
    .expect_err("missing source span id must fail");
    assert!(missing.to_string().contains("source_span_id"));

    let unknown = validate_manga_speech_selection_source_span_ids(
        &json!({
            "dialogues": [{
                "index": 0,
                "event_index": 0,
                "speaker": "林晓雨",
                "content": "你好",
                "emotion": "neutral",
                "target": null,
                "is_internal": false,
                "bubble_type": "normal",
                "source_span_id": "ch1:s99:f99"
            }],
            "narrations": []
        }),
        &source_spans,
    )
    .expect_err("unknown source span id must fail");
    assert!(unknown.to_string().contains("source_span_id 无效"));
}

#[test]
fn validate_manga_speech_selection_source_span_ids_rejects_invalid_event_index() {
    let source_spans = source_spans_for_content(7, 1, "林晓雨低声说，你好。");
    let dialogue_span_id = source_spans
        .iter()
        .find(|span| span.text.contains("你好"))
        .expect("dialogue span")
        .id
        .clone();

    let err = validate_manga_speech_selection_source_span_ids(
        &json!({
            "dialogues": [{
                "index": 0,
                "event_index": "0",
                "speaker": "林晓雨",
                "content": "你好",
                "emotion": "neutral",
                "target": null,
                "is_internal": false,
                "bubble_type": "normal",
                "source_span_id": dialogue_span_id
            }],
            "narrations": []
        }),
        &source_spans,
    )
    .expect_err("event_index must be numeric");

    assert!(
        err.to_string()
            .contains("manga_speech_selection.dialogues[0].event_index")
    );
    assert!(err.to_string().contains("非负整数"));
}

#[test]
fn manga_visual_anchor_extraction_user_message_uses_event_windows() {
    let source_spans = source_spans_for_content(7, 1, "林晓雨推开门。陌生人站在雨里。");
    let span_id = source_spans
        .iter()
        .find(|span| span.text.contains("陌生人"))
        .expect("scene span")
        .id
        .clone();
    let message = manga_visual_anchor_extraction_user_message(
        json!([{"index": 0, "description": "林晓雨推门", "source_span_ids": [span_id]}]),
        &source_spans,
    )
    .expect("visual anchor prompt should serialize");

    assert!(message.contains(r#""source_windows": ["#));
    assert!(message.contains(r#""available_source_spans": ["#));
    assert!(!message.contains(r#""content": "林晓雨推开门。陌生人站在雨里。""#));
    assert!(message.contains(r#""source_span_rule""#));
    assert!(message.contains("input.chapter.available_source_spans"));
    assert!(!message.contains("input.chapter.source_spans"));
    assert!(!message.contains(r#""chapter_id""#));
    assert!(!message.contains(r#""chapter_number""#));
    assert!(!message.contains(r#""char_start""#));
    assert!(!message.contains(r#""char_end""#));
    assert!(!message.contains(r#""long_text""#));
    assert!(message.contains("source_span_ids"));
}

#[test]
fn validate_manga_visual_anchor_source_span_ids_accepts_known_ids() {
    let source_spans = source_spans_for_content(7, 1, "林晓雨推开门。陌生人站在雨里。");
    let scene_span_id = source_spans
        .iter()
        .find(|span| span.text.contains("陌生人"))
        .expect("scene span")
        .id
        .clone();

    validate_manga_visual_anchor_source_span_ids(
        &json!({
            "scenes": [{
                "index": 0,
                "location": "门口",
                "location_en": "雨夜门口",
                "time_of_day": "night",
                "atmosphere": "紧张",
                "weather": "rainy",
                "lighting": "dim",
                "indoor_outdoor": "outdoor",
                "description": "门口雨幕中站着陌生人",
                "event_indices": [0],
                "source_span_ids": [scene_span_id]
            }]
        }),
        &source_spans,
    )
    .expect("known source span ids should pass");
}

#[test]
fn validate_manga_visual_anchor_source_span_ids_rejects_missing_or_unknown_id() {
    let source_spans = source_spans_for_content(7, 1, "林晓雨推开门。");
    let missing = validate_manga_visual_anchor_source_span_ids(
        &json!({
            "scenes": [{
                "index": 0,
                "location": "门口",
                "location_en": "雨夜门口",
                "time_of_day": "night",
                "atmosphere": "紧张",
                "weather": "rainy",
                "lighting": "dim",
                "indoor_outdoor": "outdoor",
                "description": "门口雨幕中站着陌生人",
                "event_indices": [0]
            }]
        }),
        &source_spans,
    )
    .expect_err("missing source span ids must fail");
    assert!(missing.to_string().contains("source_span_ids"));

    let unknown = validate_manga_visual_anchor_source_span_ids(
        &json!({
            "scenes": [{
                "index": 0,
                "location": "门口",
                "location_en": "雨夜门口",
                "time_of_day": "night",
                "atmosphere": "紧张",
                "weather": "rainy",
                "lighting": "dim",
                "indoor_outdoor": "outdoor",
                "description": "门口雨幕中站着陌生人",
                "event_indices": [0],
                "source_span_ids": ["ch1:s99:f99"]
            }]
        }),
        &source_spans,
    )
    .expect_err("unknown source span ids must fail");
    assert!(unknown.to_string().contains("source_span_ids"));
    assert!(unknown.to_string().contains("无效"));
}

#[test]
fn validate_manga_visual_anchor_source_span_ids_rejects_invalid_event_indices() {
    let source_spans = source_spans_for_content(7, 1, "林晓雨推开门。陌生人站在雨里。");
    let scene_span_id = source_spans
        .iter()
        .find(|span| span.text.contains("陌生人"))
        .expect("scene span")
        .id
        .clone();

    let invalid_type = validate_manga_visual_anchor_source_span_ids(
        &json!({
            "scenes": [{
                "index": 0,
                "location": "门口",
                "location_en": "雨夜门口",
                "time_of_day": "night",
                "atmosphere": "紧张",
                "weather": "rainy",
                "lighting": "dim",
                "indoor_outdoor": "outdoor",
                "description": "门口雨幕中站着陌生人",
                "event_indices": ["0"],
                "source_span_ids": [scene_span_id.clone()]
            }]
        }),
        &source_spans,
    )
    .expect_err("event_indices must contain numeric values");
    assert!(
        invalid_type
            .to_string()
            .contains("manga_visual_anchor_extraction.scenes[0].event_indices[0]")
    );
    assert!(invalid_type.to_string().contains("非负整数"));

    let duplicate = validate_manga_visual_anchor_source_span_ids(
        &json!({
            "scenes": [{
                "index": 0,
                "location": "门口",
                "location_en": "雨夜门口",
                "time_of_day": "night",
                "atmosphere": "紧张",
                "weather": "rainy",
                "lighting": "dim",
                "indoor_outdoor": "outdoor",
                "description": "门口雨幕中站着陌生人",
                "event_indices": [0, 0],
                "source_span_ids": [scene_span_id]
            }]
        }),
        &source_spans,
    )
    .expect_err("duplicate event_indices must fail");
    assert!(duplicate.to_string().contains("重复事件索引：0"));
}

#[test]
fn manga_item_extraction_input_exposes_available_source_span_ids() {
    let step2 = json!({
        "dialogues": [{"source_span_id": "ch1:s1:f1"}],
        "narrations": [{"source_span_id": "ch1:s2:f1"}]
    });
    let step3 = json!({
        "scenes": [{
            "source_span_ids": ["ch1:s2:f1", "ch1:s3:f1"]
        }]
    });

    let input = manga_item_extraction_input(step2.clone(), step3.clone());
    let ids = input["available_source_span_ids"].as_array().unwrap();

    assert_eq!(
        ids,
        &vec![json!("ch1:s1:f1"), json!("ch1:s2:f1"), json!("ch1:s3:f1")]
    );
    assert_eq!(
        manga_item_available_source_span_ids(&step2, &step3),
        vec!["ch1:s1:f1", "ch1:s2:f1", "ch1:s3:f1"]
    );
}

#[test]
fn validate_manga_item_extraction_source_span_ids_accepts_known_allowed_ids() {
    let source_spans = source_spans_for_content(7, 1, "林晓雨推开门。旧钥匙落在门边。");
    let item_span_id = source_spans
        .iter()
        .find(|span| span.text.contains("旧钥匙"))
        .expect("item span")
        .id
        .clone();
    let allowed_source_span_ids = vec![item_span_id.clone()];

    validate_manga_item_extraction_source_span_ids(
        &json!({
            "items": [{
                "name": "旧钥匙",
                "description": "落在门边的旧钥匙",
                "description_en": "暗铜色旧钥匙",
                "importance": "key_item",
                "first_appearance_event": 0,
                "visual_features": "暗铜色、边缘磨损",
                "source_span_ids": [item_span_id]
            }]
        }),
        &source_spans,
        &allowed_source_span_ids,
    )
    .expect("known allowed source span ids should pass");
}

#[test]
fn validate_manga_item_extraction_source_span_ids_rejects_missing_unknown_or_disallowed_id() {
    let source_spans = source_spans_for_content(7, 1, "林晓雨推开门。旧钥匙落在门边。");
    let door_span_id = source_spans
        .iter()
        .find(|span| span.text.contains("推开门"))
        .expect("door span")
        .id
        .clone();
    let item_span_id = source_spans
        .iter()
        .find(|span| span.text.contains("旧钥匙"))
        .expect("item span")
        .id
        .clone();

    let missing = validate_manga_item_extraction_source_span_ids(
        &json!({
            "items": [{
                "name": "旧钥匙",
                "description": "落在门边的旧钥匙",
                "description_en": "暗铜色旧钥匙",
                "importance": "key_item",
                "first_appearance_event": 0,
                "visual_features": "暗铜色、边缘磨损"
            }]
        }),
        &source_spans,
        std::slice::from_ref(&item_span_id),
    )
    .expect_err("missing source span ids must fail");
    assert!(missing.to_string().contains("source_span_ids"));

    let unknown = validate_manga_item_extraction_source_span_ids(
        &json!({
            "items": [{
                "name": "旧钥匙",
                "description": "落在门边的旧钥匙",
                "description_en": "暗铜色旧钥匙",
                "importance": "key_item",
                "first_appearance_event": 0,
                "visual_features": "暗铜色、边缘磨损",
                "source_span_ids": ["ch1:s99:f99"]
            }]
        }),
        &source_spans,
        std::slice::from_ref(&item_span_id),
    )
    .expect_err("unknown source span ids must fail");
    assert!(unknown.to_string().contains("source_span_ids"));
    assert!(unknown.to_string().contains("无效"));

    let disallowed = validate_manga_item_extraction_source_span_ids(
        &json!({
            "items": [{
                "name": "旧钥匙",
                "description": "落在门边的旧钥匙",
                "description_en": "暗铜色旧钥匙",
                "importance": "key_item",
                "first_appearance_event": 0,
                "visual_features": "暗铜色、边缘磨损",
                "source_span_ids": [door_span_id]
            }]
        }),
        &source_spans,
        std::slice::from_ref(&item_span_id),
    )
    .expect_err("source span ids outside available candidates must fail");
    assert!(disallowed.to_string().contains("available_source_span_ids"));
}

#[test]
fn validate_manga_item_extraction_source_span_ids_rejects_invalid_first_appearance_event() {
    let source_spans = source_spans_for_content(7, 1, "旧钥匙落在门边。");
    let item_span_id = source_spans
        .iter()
        .find(|span| span.text.contains("旧钥匙"))
        .expect("item span")
        .id
        .clone();

    let err = validate_manga_item_extraction_source_span_ids(
        &json!({
            "items": [{
                "name": "旧钥匙",
                "description": "落在门边的旧钥匙",
                "description_en": "暗铜色旧钥匙",
                "importance": "key_item",
                "first_appearance_event": "0",
                "visual_features": "暗铜色、边缘磨损",
                "source_span_ids": [item_span_id.clone()]
            }]
        }),
        &source_spans,
        std::slice::from_ref(&item_span_id),
    )
    .expect_err("first_appearance_event must be numeric");

    assert!(
        err.to_string()
            .contains("manga_item_extraction.items[0].first_appearance_event")
    );
    assert!(err.to_string().contains("非负整数"));
}

#[test]
fn manga_late_pipeline_inputs_do_not_repeat_chapter_content() {
    let items_input = manga_item_extraction_input(
        json!({"dialogues": [{"content": "原文对白"}]}),
        json!({"scenes": [{"description": "雨夜"}]}),
    );
    let page_planning = manga_page_planning_input(json!({
        "characters": {"陈恪": {"role": "主角"}},
        "events": [{"index": 0, "description": "暗棋初动"}],
        "comic_synthesis": {"chapter_summary": "陈恪发现线索"}
    }));

    assert!(items_input.get("chapter").is_none());
    assert!(page_planning.get("chapter").is_none());
    assert!(
        serde_json::to_string(&items_input)
            .unwrap()
            .contains("原文对白")
    );
    assert!(
        serde_json::to_string(&page_planning)
            .unwrap()
            .contains("暗棋初动")
    );
}

#[test]
fn manga_context_manifest_summary_redacts_canonical_content() {
    let manifest = json!({
        "schema_version": 1,
        "packs": [
            {
                "scope_kind": "chapter",
                "pack_kind": "manga_chapter_source",
                "source_refs_json": {"source_span_count": 1},
                "config_hash": "cfg",
                "required": true,
                "stable_prefix": true,
                "variants": [
                    {
                        "compression_level": "full_raw",
                        "canonical_content_json": {
                            "content": "这是一段不应进入调用摘要的完整章节原文"
                        }
                    }
                ]
            }
        ],
        "dynamic_user_content": "当前任务",
        "output_contract": null
    });

    let summary = manga_request_context_manifest_summary(Some(&manifest))
        .expect("summary should build")
        .expect("summary should exist");
    let summary_text = serde_json::to_string(&summary).expect("summary json");

    assert_eq!(summary["pack_count"], json!(1));
    assert_eq!(
        summary["packs"][0]["pack_kind"],
        json!("manga_chapter_source")
    );
    assert!(summary["packs"][0]["variants"][0]["canonical_content_chars"].is_number());
    assert!(
        summary["packs"][0]["variants"][0]["canonical_content_sha256"]
            .as_str()
            .is_some_and(|value| !value.is_empty())
    );
    assert!(!summary_text.contains("这是一段不应进入调用摘要的完整章节原文"));
    assert!(!summary_text.contains("canonical_content_json"));
}

#[test]
fn manga_page_drawing_prompt_page_input_keeps_only_page_scope_and_refs() {
    let page_plan = json!({
        "page_number": 2,
        "event_indices": [3, 4],
        "key_characters": ["陈恪", "小宦官"],
        "suggested_panel_count": 4
    });
    let extraction_summary = json!({
        "characters": {
            "陈恪": {"name": "陈恪"},
            "小宦官": {"name": "小宦官"},
            "李建成": {"name": "李建成"}
        },
        "events": [
            {"index": 3, "participants": ["陈恪", "小宦官"]},
            {"index": 8, "participants": ["陈恪", "李建成"]}
        ],
        "reference_assets": {
            "character_portraits": [
                {"character_name": "陈恪", "prompt": "青年书吏"},
                {"character_name": "李建成", "prompt": "太子冠服"}
            ],
            "character_portrait_summary": "陈恪: 青年书吏\n李建成: 太子冠服"
        },
        "speech": {
            "dialogues": [
                {"event_index": 3, "content": "今日是六月初三"},
                {"event_index": 8, "content": "你以为如何？"}
            ],
            "narrations": [
                {"event_index": 4, "content": "武德九年六月初三"}
            ]
        },
        "visual_anchors": {
            "scenes": [
                {"event_indices": [3, 4], "location": "书房"},
                {"event_indices": [8], "location": "显德殿"}
            ]
        },
        "items": [
            {"first_appearance_event": 4, "name": "青铜镜"},
            {"first_appearance_event": 8, "name": "奏章"}
        ],
        "comic_synthesis": {
            "chapter_summary": "陈恪穿越到唐初。",
            "mood_progression": ["迷茫", "震惊"]
        }
    });

    let input = manga_page_drawing_prompt_page_input(&page_plan, &extraction_summary);
    let context = &input["page_context"];

    assert!(context["characters"].get("陈恪").is_some());
    assert!(context["characters"].get("小宦官").is_some());
    assert!(context["characters"].get("李建成").is_none());
    assert_eq!(context["events"].as_array().unwrap().len(), 1);
    assert_eq!(context["dialogues"].as_array().unwrap().len(), 1);
    assert_eq!(context["narrations"].as_array().unwrap().len(), 1);
    assert_eq!(context["scenes"].as_array().unwrap().len(), 1);
    assert_eq!(context["items"].as_array().unwrap().len(), 1);
    let refs = input["reference_assets"]["character_portraits"]
        .as_array()
        .unwrap();
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0]["character_name"], json!("陈恪"));
}
