use super::*;

#[test]
fn validate_manga_character_stage_source_span_ids() {
    let source_spans = source_spans_for_content(7, 1, "林晓雨推开门。陌生人站在雨里。");
    let character_span_id = source_spans
        .iter()
        .find(|span| span.text.contains("林晓雨"))
        .expect("character span")
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
}

#[test]
fn validate_manga_character_binding_rejects_name_different_from_object_key() {
    let source_spans = source_spans_for_content(7, 1, "林晓雨推开门。");
    let character_span_id = source_spans
        .iter()
        .find(|span| span.text.contains("林晓雨"))
        .expect("character span")
        .id
        .clone();

    let error = validate_manga_character_binding_source_span_ids(
        &json!({
            "characters": {
                "林晓雨": {
                    "name": "陌生人",
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
    .expect_err("character name and object key must use one canonical identity");

    assert!(error.to_string().contains("name 必须与角色对象 key 一致"));
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
                "beat_ref": {"scene_index": 1, "shot_index": 1, "beat_index": 1},
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
                "beat_ref": {"scene_index": 1, "shot_index": 1, "beat_index": 2},
                "content": "三天后",
                "narration_type": "time",
                "position": "top",
                "source_span_id": narration_span_id
            }]
        }),
        &source_spans,
        &std::collections::HashSet::from([(1, 1, 1), (1, 1, 2)]),
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
                "beat_ref": {"scene_index": 1, "shot_index": 1, "beat_index": 1},
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
        &std::collections::HashSet::from([(1, 1, 1)]),
    )
    .expect_err("missing source span id must fail");
    assert!(missing.to_string().contains("source_span_id"));

    let unknown = validate_manga_speech_selection_source_span_ids(
        &json!({
            "dialogues": [{
                "index": 0,
                "beat_ref": {"scene_index": 1, "shot_index": 1, "beat_index": 1},
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
        &std::collections::HashSet::from([(1, 1, 1)]),
    )
    .expect_err("unknown source span id must fail");
    assert!(unknown.to_string().contains("source_span_id 无效"));
}

#[test]
fn validate_manga_speech_selection_source_span_ids_rejects_invalid_beat_ref() {
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
                "beat_ref": {"scene_index": 1, "shot_index": 1, "beat_index": "0"},
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
        &std::collections::HashSet::from([(1, 1, 1)]),
    )
    .expect_err("beat_ref must be typed");

    assert!(
        err.to_string()
            .contains("manga_speech_selection.dialogues[0].beat_ref.beat_index")
    );
    assert!(err.to_string().contains("正整数"));
}
