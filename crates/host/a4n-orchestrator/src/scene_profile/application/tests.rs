use super::parsing::{
    parse_scene_entity_states, parse_scene_occurrences, parse_scene_profiles,
    parse_scene_stage_response, parse_scene_transitions,
};
use super::stages::{build_scene_profile_candidates_request, build_scene_profile_stage_request};
use super::types::{
    SCENE_CANDIDATE_ACTUAL_OCCURRENCE_RULE, SCENE_OCCURRENCE_COVERAGE_RULE,
    SCENE_OCCURRENCE_PROMPT, SCENE_PROFILE_CANDIDATES_PROMPT, SCENE_TRANSITION_REFERENCE_RULE,
    SceneProfileSyncRequestContext,
};
use crate::scene_profile::SceneSyncInput;
use a4n_domain::chapter::ChapterRecord;
use a4n_domain::source_spans::source_spans_for_chapters;
use a4n_domain::text_anchor::TextAnchorRecord;
use a4n_error::AppError;
use a4n_prompt::rendering::PromptRenderer;
use async_trait::async_trait;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

struct SceneSessionPromptRenderer;

#[async_trait]
impl PromptRenderer for SceneSessionPromptRenderer {
    async fn render_prompt(
        &self,
        template_name: &str,
        _variables: &std::collections::HashMap<String, Value>,
        _project_id: Option<i64>,
    ) -> Result<String, AppError> {
        Ok(format!("rendered prompt for {template_name}"))
    }

    async fn render_prompt_for_call(
        &self,
        _: &str,
        template_name: &str,
        variables: &std::collections::HashMap<String, Value>,
        project_id: Option<i64>,
    ) -> Result<String, AppError> {
        self.render_prompt(template_name, variables, project_id)
            .await
    }
}

fn chapter_for_test() -> ChapterRecord {
    let now = chrono::Utc::now().naive_utc();
    ChapterRecord {
        id: 1,
        project_id: 1,
        outline_id: None,
        chapter_number: 3,
        title: Some("旧书房".to_string()),
        content: "他推门走进旧书房，木书桌靠着窗。青铜钥匙放在抽屉里。随后他离开旧书房。"
            .to_string(),
        word_count: 42,
        status: "draft".to_string(),
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
        existing_scene_profiles: Arc::new(Vec::new()),
        character_profiles: Arc::new(Vec::new()),
        object_profiles: Arc::new(Vec::new()),
        existing_scene_ids: BTreeSet::new(),
        character_ids: Arc::new(BTreeSet::from([10])),
        object_ids: Arc::new(BTreeSet::from([20])),
        character_names: Arc::new(BTreeMap::from([(10, "进入者".to_string())])),
        object_names: Arc::new(BTreeMap::from([(20, "青铜钥匙".to_string())])),
    }
}

fn anchors_for_test(chapters: &[ChapterRecord]) -> Vec<TextAnchorRecord> {
    let now = chrono::Utc::now().naive_utc();
    source_spans_for_chapters(chapters)
        .iter()
        .enumerate()
        .map(|(index, span)| TextAnchorRecord {
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

fn anchor_id_containing(anchors: &[TextAnchorRecord], text: &str) -> i64 {
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

fn parse_scene_profile_sync_content(
    content: &str,
    context: &SceneProfileSyncRequestContext,
) -> Result<SceneSyncInput, String> {
    let value: Value = serde_json::from_str(content).map_err(|error| error.to_string())?;
    let profiles = parse_scene_profiles(&value, context)?;
    let mut input = SceneSyncInput {
        profiles: profiles.drafts,
        occurrences: Vec::new(),
        transitions: Vec::new(),
        entity_states: Vec::new(),
    };
    input.occurrences = parse_scene_occurrences(&value, context, &input.profiles)?;
    input.transitions = parse_scene_transitions(&value, context, &input)?;
    input.entity_states = parse_scene_entity_states(&value, context, &input)?;
    Ok(input)
}

#[tokio::test]
async fn scene_stages_share_one_chapter_prefix_without_merging_stage_instructions() {
    let context = context_for_test();
    let scenes = build_scene_profile_stage_request(
        &SceneSessionPromptRenderer,
        &context,
        None,
        SCENE_PROFILE_CANDIDATES_PROMPT,
        "识别场景",
        json!({}),
        json!({"required_root_field": "scenes"}),
    )
    .await
    .expect("scene request");
    let occurrences = build_scene_profile_stage_request(
        &SceneSessionPromptRenderer,
        &context,
        None,
        SCENE_OCCURRENCE_PROMPT,
        "抽取出场",
        json!({"scenes": []}),
        json!({"required_root_field": "occurrences"}),
    )
    .await
    .expect("occurrence request");

    assert_eq!(scenes.messages[0].content, occurrences.messages[0].content);
    assert!(
        scenes.messages[1]
            .content
            .contains(SCENE_PROFILE_CANDIDATES_PROMPT)
    );
    assert!(
        occurrences.messages[1]
            .content
            .contains(SCENE_OCCURRENCE_PROMPT)
    );
    let scene_pack = &scenes.context_manifest_json.as_ref().unwrap()["packs"][0];
    let occurrence_pack = &occurrences.context_manifest_json.as_ref().unwrap()["packs"][0];
    assert_eq!(scene_pack["config_hash"], occurrence_pack["config_hash"]);
    assert_eq!(scene_pack["variants"], occurrence_pack["variants"]);
}

#[tokio::test]
async fn model_contract_matrix_scene_candidates_request_declares_integer_evidence_ids() {
    let request = build_scene_profile_candidates_request(
        &SceneSessionPromptRenderer,
        &context_for_test(),
        None,
    )
    .await
    .expect("candidate request");
    a4n_test_support::model_contract::emit_schema(
        SCENE_PROFILE_CANDIDATES_PROMPT,
        request
            .response_format
            .as_ref()
            .unwrap()
            .json_schema
            .as_ref()
            .unwrap()
            .schema
            .clone(),
    );
    let manifest = request.context_manifest_json.as_ref().expect("manifest");
    let task = manifest["dynamic_user_content"].as_str().expect("task");
    let payload = a4n_prompt::request::json_user_message_payload(task)
        .and_then(|text| serde_json::from_str::<Value>(text).ok())
        .expect("candidate output contract");
    let evidence_type = &payload["output_contract"]["field_types"]["evidence_anchor_ids"];
    assert_eq!(evidence_type["type"], "array");
    assert_eq!(evidence_type["items"]["type"], "integer");
    assert_eq!(evidence_type["items"]["minimum"], 1);
    assert_eq!(evidence_type["minItems"], 1);
}

#[tokio::test]
async fn scene_task_and_native_schema_share_all_required_fields() {
    let request = build_scene_profile_stage_request(
        &SceneSessionPromptRenderer,
        &context_for_test(),
        None,
        SCENE_OCCURRENCE_PROMPT,
        "抽取出场",
        json!({"scenes": []}),
        json!({
            "required_root_field": "occurrences",
            "item_required_fields": ["summary", "entry_state", "exit_state"],
            "field_types": {
                "summary": {"type": "string"},
                "entry_state": {"type": "object"},
                "exit_state": {"type": "object"}
            }
        }),
    )
    .await
    .unwrap();
    let manifest = request.context_manifest_json.as_ref().unwrap();
    let task = manifest["dynamic_user_content"].as_str().unwrap();
    let payload: Value =
        serde_json::from_str(a4n_prompt::request::json_user_message_payload(task).unwrap())
            .unwrap();
    let schema = &request
        .response_format
        .as_ref()
        .unwrap()
        .json_schema
        .as_ref()
        .unwrap()
        .schema;
    let required = &schema["properties"]["occurrences"]["items"]["required"];
    assert_eq!(
        payload["output_contract"]["item_required_fields"],
        *required
    );
    assert_eq!(required, &json!(["summary", "entry_state", "exit_state"]));
    assert!(task.contains("空对象或空数组也不得省略"));
}

#[test]
fn parses_valid_scene_profile_sync_content() {
    let context = context_for_test();
    let content = valid_scene_sync_content(&context).to_string();

    let input = parse_scene_profile_sync_content(&content, &context)
        .expect("valid scene sync content should parse");

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
fn occurrence_states_are_required_objects_and_preserve_nested_facts() {
    let context = context_for_test();
    let content = valid_scene_sync_content(&context);
    let profiles = parse_scene_profiles(&content, &context).unwrap();
    for field in ["entry_state", "exit_state"] {
        let mut missing = json!({"occurrences": content["occurrences"].clone()});
        missing["occurrences"][0]
            .as_object_mut()
            .unwrap()
            .remove(field);
        let error = parse_scene_occurrences(&missing, &context, &profiles.drafts).unwrap_err();
        assert_eq!(error, format!("occurrences[0].{field} 缺少字段"));
        for invalid in [
            Value::Null,
            json!("未知"),
            json!([]),
            json!(false),
            json!(1),
        ] {
            missing["occurrences"][0][field] = invalid;
            let error = parse_scene_occurrences(&missing, &context, &profiles.drafts).unwrap_err();
            assert_eq!(error, format!("occurrences[0].{field} 必须是 JSON 对象"));
        }
    }
    let mut valid = json!({"occurrences": content["occurrences"].clone()});
    let exit = json!({"door": {"locked": true}, "remaining_objects": ["青铜钥匙"]});
    valid["occurrences"][0]["entry_state"] = json!({});
    valid["occurrences"][0]["exit_state"] = exit.clone();
    let parsed = parse_scene_occurrences(&valid, &context, &profiles.drafts).unwrap();
    assert_eq!(parsed[0].entry_state, json!({}));
    assert_eq!(parsed[0].exit_state, exit);
}

#[test]
fn scene_stage_parsing_enforces_field_types_before_accepting_output() {
    let context = context_for_test();
    let content = valid_scene_sync_content(&context);
    let known = parse_scene_profile_sync_content(&content.to_string(), &context).unwrap();
    for (root_field, field, bad_value) in [
        ("scenes", "visual_description", json!({"text": "旧书房"})),
        ("occurrences", "timeline_position", json!(1)),
        ("transitions", "time_cost", json!(5)),
        ("entity_states", "state_after", json!("抽屉里")),
    ] {
        let mut stage = json!({root_field: content[root_field].clone()});
        let parse = |value: &Value| match root_field {
            "scenes" => parse_scene_profiles(value, &context).map(drop),
            "occurrences" => parse_scene_occurrences(value, &context, &known.profiles).map(drop),
            "transitions" => parse_scene_transitions(value, &context, &known).map(drop),
            "entity_states" => parse_scene_entity_states(value, &context, &known).map(drop),
            _ => unreachable!("fixture stage"),
        };
        let (output, ()) =
            parse_scene_stage_response(&stage.to_string(), root_field, root_field, parse)
                .expect("valid stage should return validated drafts");
        assert_eq!(json!(output), content[root_field]);

        stage[root_field][0][field] = bad_value;
        let error = parse_scene_stage_response(&stage.to_string(), root_field, root_field, parse)
            .expect_err("invalid stage fields must fail before checkpointing");
        assert!(
            error.contains(&format!("{root_field}[0].{field}")),
            "{error}"
        );
    }
}

#[test]
fn scene_occurrence_timeline_accepts_text_or_null_without_coercion() {
    let context = context_for_test();
    let content = valid_scene_sync_content(&context);
    let profiles = parse_scene_profiles(&content, &context).unwrap();
    let mut stage = json!({"occurrences": content["occurrences"].clone()});
    for value in [Value::Null, json!("次日清晨")] {
        stage["occurrences"][0]["timeline_position"] = value.clone();
        let (output, drafts) = parse_scene_stage_response(
            &stage.to_string(),
            SCENE_OCCURRENCE_PROMPT,
            "occurrences",
            |value| parse_scene_occurrences(value, &context, &profiles.drafts),
        )
        .expect("text and null are valid timeline positions");
        assert_eq!(output[0]["timeline_position"], value);
        assert_eq!(drafts[0].timeline_position.as_deref(), value.as_str());
    }
    for value in [
        json!(1),
        json!(1.5),
        json!(true),
        json!({"order": 1}),
        json!([]),
    ] {
        stage["occurrences"][0]["timeline_position"] = value;
        let error = parse_scene_stage_response(
            &stage.to_string(),
            SCENE_OCCURRENCE_PROMPT,
            "occurrences",
            |value| parse_scene_occurrences(value, &context, &profiles.drafts),
        )
        .expect_err("other JSON types must not be coerced to text");
        assert_eq!(
            error,
            "occurrences[0].timeline_position 必须是字符串或 null"
        );
    }
}

#[test]
fn scene_stages_reject_reference_errors_before_returning_drafts() {
    let context = context_for_test();
    let content = valid_scene_sync_content(&context);
    let known = parse_scene_profile_sync_content(&content.to_string(), &context).unwrap();
    for (field, reference, expected) in [
        ("occurrences", "scene_ref", "没有对应 occurrences"),
        (
            "transitions",
            "to_scene_ref",
            "引用了不存在的 scenes[].scene_ref",
        ),
        (
            "entity_states",
            "occurrence_ref",
            "引用了不存在的 occurrences[].occurrence_ref",
        ),
    ] {
        let mut stage = json!({field: content[field].clone()});
        stage[field][0][reference] = json!("missing");
        let error =
            parse_scene_stage_response(&stage.to_string(), field, field, |value| match field {
                "occurrences" => {
                    parse_scene_occurrences(value, &context, &known.profiles).map(drop)
                }
                "transitions" => parse_scene_transitions(value, &context, &known).map(drop),
                "entity_states" => parse_scene_entity_states(value, &context, &known).map(drop),
                _ => unreachable!("fixture stage"),
            })
            .expect_err("a referenced stage must be valid before it can be checkpointed");
        assert!(error.contains(expected), "{error}");
    }
}

#[test]
fn every_scene_stage_rejects_non_integer_source_ids() {
    let context = context_for_test();
    let content = valid_scene_sync_content(&context);
    for (root, field) in [
        ("scenes", "evidence_anchor_ids"),
        ("occurrences", "source_anchor_ids"),
        ("transitions", "source_anchor_ids"),
        ("entity_states", "source_anchor_ids"),
    ] {
        for invalid in [
            json!("1"),
            json!("⟦1⟧"),
            json!(1.0),
            json!(true),
            Value::Null,
        ] {
            let mut value = content.clone();
            value[root][0][field] = json!([invalid]);
            let error = parse_scene_profile_sync_content(&value.to_string(), &context)
                .expect_err("invalid IDs must not reach persistence or checkpoints");
            assert_eq!(error, format!("{root}[0].{field}[0] 必须是 JSON 整数数字"));
        }
    }
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
        "/../../../prompts/novel/05_analysis/scene_profile_candidates.md"
    ));
    let occurrences_prompt = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../prompts/novel/05_analysis/scene_occurrence_extraction.md"
    ));
    let transitions_prompt = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../prompts/novel/05_analysis/scene_transition_extraction.md"
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
