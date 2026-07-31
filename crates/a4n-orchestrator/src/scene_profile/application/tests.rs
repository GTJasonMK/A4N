use super::parsing::parse_scene_profile_sync_content;
use super::types::{
    SCENE_CANDIDATE_ACTUAL_OCCURRENCE_RULE, SCENE_OCCURRENCE_COVERAGE_RULE,
    SCENE_TRANSITION_REFERENCE_RULE, SceneProfileSyncRequestContext,
};
use a4n_core::models::chapter::chapter;
use a4n_core::models::project::text_anchor;
use a4n_core::source_spans::source_spans_for_chapters;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

fn chapter_for_test() -> chapter::Model {
    let now = chrono::Utc::now().naive_utc();
    chapter::Model {
        id: 1,
        project_id: 1,
        outline_id: None,
        chapter_number: 3,
        title: Some("旧书房".to_string()),
        content: "他推门走进旧书房，木书桌靠着窗。青铜钥匙放在抽屉里。随后他离开旧书房。"
            .to_string(),
        word_count: 42,
        status: chapter::STATUS_DRAFT.to_string(),
        summary: None,
        style_tags: None,
        analysis_data: Some(json!({"metadata": {}})),
        created_at: now,
        updated_at: now,
    }
}

fn context_for_test() -> SceneProfileSyncRequestContext {
    let chapter = chapter_for_test();
    let text_anchors = anchors_for_test(std::slice::from_ref(&chapter));
    SceneProfileSyncRequestContext {
        chapter,
        text_anchors,
        existing_scene_profiles: Vec::new(),
        character_profiles: Vec::new(),
        object_profiles: Vec::new(),
        existing_scene_ids: BTreeSet::new(),
        character_ids: BTreeSet::from([10]),
        object_ids: BTreeSet::from([20]),
        character_names: BTreeMap::from([(10, "进入者".to_string())]),
        object_names: BTreeMap::from([(20, "青铜钥匙".to_string())]),
    }
}

fn anchors_for_test(chapters: &[chapter::Model]) -> Vec<text_anchor::Model> {
    let now = chrono::Utc::now().naive_utc();
    source_spans_for_chapters(chapters)
        .iter()
        .enumerate()
        .map(|(index, span)| text_anchor::Model {
            id: i64::try_from(index + 1).unwrap(),
            project_id: 1,
            source_chapter_id: Some(span.chapter_id),
            source_snapshot_id: None,
            source_snapshot_chapter_id: None,
            source_kind: "chapter_content".to_string(),
            source_id: span.chapter_id,
            paragraph_index: i32::try_from(span.sentence_index).unwrap(),
            start_offset: i32::try_from(span.char_start).unwrap(),
            end_offset: i32::try_from(span.char_end).unwrap(),
            quote: span.text.clone(),
            quote_hash: format!("hash-{}", index + 1),
            source_version_hash: "source-version".to_string(),
            created_at: now,
        })
        .collect()
}

fn anchor_id_containing(anchors: &[text_anchor::Model], text: &str) -> i64 {
    anchors
        .iter()
        .find(|anchor| anchor.quote.contains(text))
        .map(|anchor| anchor.id)
        .unwrap_or_else(|| panic!("missing text anchor containing {text}"))
}

fn valid_scene_sync_content(context: &SceneProfileSyncRequestContext) -> Value {
    let enter_anchor = anchor_id_containing(&context.text_anchors, "推门走进旧书房");
    let desk_anchor = anchor_id_containing(&context.text_anchors, "木书桌靠着窗");
    let key_anchor = anchor_id_containing(&context.text_anchors, "青铜钥匙放在抽屉里");
    json!({
        "scenes": [
            {
                "scene_ref": "scene_1",
                "scene_profile_id": null,
                "name": "旧书房",
                "aliases": [],
                "scene_type": "room",
                "importance": "primary",
                "parent_scene_id": null,
                "base_description": "一间旧书房",
                "visual_description": "木书桌靠窗，抽屉里有青铜钥匙",
                "spatial_layout": "门在入口，木书桌靠窗，抽屉在桌下",
                "atmosphere": "安静、旧物感强",
                "entry_points": ["门"],
                "rules": [],
                "evidence_anchor_ids": [enter_anchor]
            }
        ],
        "occurrences": [
            {
                "occurrence_ref": "occ_1",
                "scene_ref": "scene_1",
                "timeline_position": null,
                "summary": "他进入旧书房并注意到抽屉里的钥匙",
                "mood": "谨慎",
                "weather_or_lighting": null,
                "source_anchor_ids": [desk_anchor],
                "characters": [{"profile_id": 10, "role": "进入书房"}],
                "objects": [{"object_profile_id": 20, "role": "放在抽屉里"}],
                "key_events": ["进入旧书房"],
                "entry_state": {},
                "exit_state": {"left": true}
            }
        ],
        "transitions": [
            {
                "from_scene_ref": null,
                "to_scene_ref": "scene_1",
                "occurrence_ref": "occ_1",
                "movement_type": "enter",
                "route_description": "推门进入旧书房",
                "time_cost": null,
                "distance_hint": null,
                "participants": [10],
                "object_refs": [],
                "source_anchor_ids": [key_anchor]
            }
        ],
        "entity_states": [
            {
                "scene_ref": "scene_1",
                "occurrence_ref": "occ_1",
                "entity_type": "object",
                "entity_id": 20,
                "state_key": "position",
                "state_before": {},
                "state_after": {"position": "抽屉里"},
                "position_description": "抽屉里",
                "action_description": "钥匙位于书桌抽屉",
                "source_anchor_ids": [key_anchor]
            }
        ]
    })
}

#[test]
fn parses_valid_scene_profile_sync_content() {
    let context = context_for_test();
    let content = valid_scene_sync_content(&context).to_string();

    let parsed = parse_scene_profile_sync_content(&content, &context)
        .expect("valid scene sync content should parse");
    let input = parsed.input;

    assert_eq!(input.profiles.len(), 1);
    assert_eq!(input.occurrences.len(), 1);
    assert_eq!(input.transitions.len(), 1);
    assert_eq!(input.entity_states.len(), 1);
    assert_eq!(input.profiles[0].first_seen_chapter, 3);
    assert_eq!(input.profiles[0].last_seen_chapter, 3);
    assert_eq!(input.occurrences[0].occurrence_order, 1);
    assert_eq!(input.transitions[0].transition_order, 1);
    assert_eq!(input.entity_states[0].entity_name_snapshot, "青铜钥匙");
    assert_eq!(input.profiles[0].evidence[0], "他推门走进旧书房");
    assert!(input.profiles[0].image_prompt_seed.contains("旧书房"));
    assert!(input.profiles[0].image_prompt_seed.contains("木书桌靠窗"));
    assert!(
        input.profiles[0]
            .image_prompt_seed
            .contains("安静、旧物感强")
    );
}

#[test]
fn rejects_scene_type_and_movement_type_aliases() {
    let context = context_for_test();
    let mut value = valid_scene_sync_content(&context);
    value["scenes"][0]["scene_type"] = json!("corridor");
    value["transitions"][0]["movement_type"] = json!("move");

    let err = parse_scene_profile_sync_content(&value.to_string(), &context)
        .expect_err("scene and transition aliases should fail visibly");

    assert!(err.contains("scene_type 无效：corridor"));
}

#[test]
fn rejects_scene_entity_type_and_state_key_aliases() {
    let context = context_for_test();
    let mut value = valid_scene_sync_content(&context);
    value["entity_states"][0]["entity_type"] = json!("item");
    value["entity_states"][0]["state_key"] = json!("location");

    let err = parse_scene_profile_sync_content(&value.to_string(), &context)
        .expect_err("entity state aliases should fail visibly");

    assert!(err.contains("entity_type 无效：item"));
}

#[test]
fn derives_entity_name_snapshot_and_rejects_model_supplied_value() {
    let context = context_for_test();
    let mut value = valid_scene_sync_content(&context);
    value["entity_states"][0]["entity_name_snapshot"] = json!({
        "name": "模型误输出对象"
    });

    let err = parse_scene_profile_sync_content(&value.to_string(), &context)
        .expect_err("entity_name_snapshot is system-derived and must not be accepted");

    assert!(err.contains("未支持字段 `entity_name_snapshot`"));
}

#[test]
fn rejects_scene_profile_without_occurrence() {
    let context = context_for_test();
    let mut value = valid_scene_sync_content(&context);
    value["occurrences"] = json!([]);

    let err = parse_scene_profile_sync_content(&value.to_string(), &context)
        .expect_err("scene without occurrence should be rejected");

    assert!(err.contains("没有对应 occurrences"));
}

#[test]
fn scene_stage_contracts_explain_scene_occurrence_invariant() {
    assert!(SCENE_CANDIDATE_ACTUAL_OCCURRENCE_RULE.contains("实际出现"));
    assert!(SCENE_CANDIDATE_ACTUAL_OCCURRENCE_RULE.contains("至少一条 occurrences"));
    assert!(SCENE_OCCURRENCE_COVERAGE_RULE.contains("每一个 scene_ref"));
    assert!(SCENE_OCCURRENCE_COVERAGE_RULE.contains("occurrences: []"));

    let candidates_prompt = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../prompts/novel/05_analysis/scene_profile_candidates.md"
    ));
    let occurrences_prompt = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../prompts/novel/05_analysis/scene_occurrence_extraction.md"
    ));
    let transitions_prompt = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../prompts/novel/05_analysis/scene_transition_extraction.md"
    ));
    assert!(candidates_prompt.contains("未实际出场"));
    assert!(candidates_prompt.contains("至少一条场景出场记录"));
    assert!(occurrences_prompt.contains("必须为每个 `scene_ref` 至少输出一条出场记录"));
    assert!(occurrences_prompt.contains("必须覆盖输入 `scenes[]`"));
    assert!(SCENE_TRANSITION_REFERENCE_RULE.contains("to_scene_ref"));
    assert!(SCENE_TRANSITION_REFERENCE_RULE.contains("不得为 null"));
    assert!(transitions_prompt.contains("`to_scene_ref` 必须"));
    assert!(transitions_prompt.contains("不得为 `null`"));
    assert!(transitions_prompt.contains("目标场景时，不要输出该条转场"));
}

#[test]
fn rejects_entity_state_with_mismatched_occurrence_scene() {
    let context = context_for_test();
    let mut value = valid_scene_sync_content(&context);
    value["scenes"].as_array_mut().expect("scenes").push(json!({
        "scene_ref": "scene_2",
        "scene_profile_id": null,
        "name": "走廊",
        "aliases": [],
        "scene_type": "route",
        "importance": "secondary",
        "parent_scene_id": null,
        "base_description": "旧书房外的走廊",
        "visual_description": "狭窄走廊",
        "spatial_layout": "连接旧书房",
        "atmosphere": "昏暗",
        "entry_points": ["旧书房门"],
        "rules": [],
        "evidence_anchor_ids": [anchor_id_containing(&context.text_anchors, "随后他离开旧书房")]
    }));
    value["occurrences"]
        .as_array_mut()
        .expect("occurrences")
        .push(json!({
            "occurrence_ref": "occ_2",
            "scene_ref": "scene_2",
            "timeline_position": null,
            "summary": "他离开旧书房进入走廊",
            "mood": null,
            "weather_or_lighting": null,
            "source_anchor_ids": [anchor_id_containing(&context.text_anchors, "随后他离开旧书房")],
            "characters": [{"profile_id": 10, "role": "离开书房"}],
            "objects": [],
            "key_events": [],
            "entry_state": {},
            "exit_state": {}
        }));
    value["entity_states"][0]["scene_ref"] = json!("scene_2");
    value["entity_states"][0]["occurrence_ref"] = json!("occ_1");

    let err = parse_scene_profile_sync_content(&value.to_string(), &context)
        .expect_err("entity state scene/occurrence mismatch should be rejected");

    assert!(err.contains("occurrence_ref 必须指向 scene_ref 对应的出场"));
}

#[test]
fn rejects_model_supplied_scene_derived_fields() {
    let context = context_for_test();
    let mut value = valid_scene_sync_content(&context);
    value["scenes"][0]["first_seen_chapter"] = json!(3);

    let err = parse_scene_profile_sync_content(&value.to_string(), &context)
        .expect_err("model supplied scene chapter field should fail");

    assert!(err.contains("未支持字段 `first_seen_chapter`"));
}

#[test]
fn rejects_model_supplied_scene_order_fields() {
    let context = context_for_test();
    let mut value = valid_scene_sync_content(&context);
    value["occurrences"][0]["occurrence_order"] = json!(1);

    let err = parse_scene_profile_sync_content(&value.to_string(), &context)
        .expect_err("model supplied occurrence order should fail");

    assert!(err.contains("未支持字段 `occurrence_order`"));
}

#[test]
fn rejects_unknown_scene_transition_movement_type() {
    let context = context_for_test();
    let mut value = valid_scene_sync_content(&context);
    value["transitions"][0]["movement_type"] = json!("dream_merge");

    let err = parse_scene_profile_sync_content(&value.to_string(), &context)
        .expect_err("unknown movement_type should fail");

    assert!(err.contains("transitions[0].movement_type 无效"));
    assert!(err.contains("travel"));
}

#[test]
fn rejects_unknown_scene_type() {
    let context = context_for_test();
    let mut value = valid_scene_sync_content(&context);
    value["scenes"][0]["scene_type"] = json!("memory");

    let err = parse_scene_profile_sync_content(&value.to_string(), &context)
        .expect_err("unknown scene_type should fail");

    assert!(err.contains("scenes[0].scene_type 无效"));
    assert!(err.contains("route"));
}

#[test]
fn rejects_unknown_scene_entity_type() {
    let context = context_for_test();
    let mut value = valid_scene_sync_content(&context);
    value["entity_states"][0]["entity_type"] = json!("memory");

    let err = parse_scene_profile_sync_content(&value.to_string(), &context)
        .expect_err("unknown entity_type should fail");

    assert!(err.contains("entity_states[0].entity_type 无效"));
    assert!(err.contains("character"));
}

#[test]
fn rejects_unknown_scene_entity_state_key() {
    let context = context_for_test();
    let mut value = valid_scene_sync_content(&context);
    value["entity_states"][0]["state_key"] = json!("memory");

    let err = parse_scene_profile_sync_content(&value.to_string(), &context)
        .expect_err("unknown state_key should fail");

    assert!(err.contains("entity_states[0].state_key 无效"));
    assert!(err.contains("position"));
}
