use super::*;

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
                "beat_refs": [{"unit_id": "unit-1", "beat_id": "beat-1"}],
                "source_span_ids": [scene_span_id]
            }]
        }),
        &source_spans,
        &known_beats(&source_spans, 1),
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
                "beat_refs": [{"unit_id": "unit-1", "beat_id": "beat-1"}]
            }]
        }),
        &source_spans,
        &known_beats(&source_spans, 1),
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
                "beat_refs": [{"unit_id": "unit-1", "beat_id": "beat-1"}],
                "source_span_ids": ["ch1:s99:f99"]
            }]
        }),
        &source_spans,
        &known_beats(&source_spans, 1),
    )
    .expect_err("unknown source span ids must fail");
    assert!(unknown.to_string().contains("source_span_ids"));
    assert!(unknown.to_string().contains("无效"));
}

#[test]
fn validate_manga_visual_anchor_source_span_ids_rejects_invalid_beat_refs() {
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
                "beat_refs": [{"unit_id": "unit-1", "beat_id": 0}],
                "source_span_ids": [scene_span_id.clone()]
            }]
        }),
        &source_spans,
        &known_beats(&source_spans, 1),
    )
    .expect_err("beat_refs must contain typed refs");
    assert!(
        invalid_type
            .to_string()
            .contains("manga_visual_anchor_extraction.scenes[0].beat_refs[0]")
    );
    assert!(invalid_type.to_string().contains("expected a string"));

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
                "beat_refs": [
                    {"unit_id": "unit-1", "beat_id": "beat-1"},
                    {"unit_id": "unit-1", "beat_id": "beat-1"}
                ],
                "source_span_ids": [scene_span_id]
            }]
        }),
        &source_spans,
        &known_beats(&source_spans, 1),
    )
    .expect_err("duplicate beat_refs must fail");
    assert!(duplicate.to_string().contains("重复改编节拍引用"));
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
                "first_appearance_beat_ref": {"unit_id": "unit-1", "beat_id": "beat-1"},
                "visual_features": "暗铜色、边缘磨损",
                "source_span_ids": [item_span_id]
            }]
        }),
        &source_spans,
        &allowed_source_span_ids,
        &known_beats(&source_spans, 1),
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
                "first_appearance_beat_ref": {"unit_id": "unit-1", "beat_id": "beat-1"},
                "visual_features": "暗铜色、边缘磨损"
            }]
        }),
        &source_spans,
        std::slice::from_ref(&item_span_id),
        &known_beats(&source_spans, 1),
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
                "first_appearance_beat_ref": {"unit_id": "unit-1", "beat_id": "beat-1"},
                "visual_features": "暗铜色、边缘磨损",
                "source_span_ids": ["ch1:s99:f99"]
            }]
        }),
        &source_spans,
        std::slice::from_ref(&item_span_id),
        &known_beats(&source_spans, 1),
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
                "first_appearance_beat_ref": {"unit_id": "unit-1", "beat_id": "beat-1"},
                "visual_features": "暗铜色、边缘磨损",
                "source_span_ids": [door_span_id]
            }]
        }),
        &source_spans,
        std::slice::from_ref(&item_span_id),
        &known_beats(&source_spans, 1),
    )
    .expect_err("source span ids outside available candidates must fail");
    assert!(disallowed.to_string().contains("available_source_span_ids"));
}

#[test]
fn validate_manga_item_extraction_source_span_ids_rejects_invalid_first_appearance_beat_ref() {
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
                "first_appearance_beat_ref": {"unit_id": "unit-1", "beat_id": 0},
                "visual_features": "暗铜色、边缘磨损",
                "source_span_ids": [item_span_id.clone()]
            }]
        }),
        &source_spans,
        std::slice::from_ref(&item_span_id),
        &known_beats(&source_spans, 1),
    )
    .expect_err("first_appearance_beat_ref must be typed");

    assert!(
        err.to_string()
            .contains("manga_item_extraction.items[0].first_appearance_beat_ref")
    );
    assert!(err.to_string().contains("expected a string"));
}
