use std::collections::BTreeSet;

use a4n_core::{
    models::{chapter::chapter, character::protagonist_profile, project::text_anchor},
    source_spans::source_spans_for_chapters,
};
use serde_json::{Value, json};

use crate::character::{
    discovery_rules::{
        select_character_discovery_chapters, validate_discovered_character_chapter_scope,
    },
    discovery_workflow_rules::{
        character_discovery_call_fingerprint_json, character_discovery_workflow_run_input,
    },
};

use super::{
    context::character_llm_chapter_context,
    contracts::CharacterDiscoveryInput,
    execution::required_character_discovery_input,
    identity::existing_profile_for_prompt,
    output::{discovered_character_base_attributes, parse_character_discovery_output},
    request::build_character_discovery_user_prompt,
};

fn discovery_chapter(chapter_number: i32, status: &str, content: &str) -> chapter::Model {
    let now = chrono::Utc::now().naive_utc();
    chapter::Model {
        id: i64::from(chapter_number),
        project_id: 1,
        outline_id: None,
        chapter_number,
        title: Some(format!("第{chapter_number}章")),
        content: content.to_string(),
        word_count: content.chars().count() as i32,
        status: status.to_string(),
        summary: None,
        style_tags: None,
        analysis_data: None,
        created_at: now,
        updated_at: now,
    }
}

fn discovery_anchor_context(content: &str) -> (Vec<chapter::Model>, Vec<text_anchor::Model>) {
    let chapters = vec![discovery_chapter(1, "draft", content)];
    (chapters.clone(), anchors_for_test(&chapters))
}

fn anchors_for_test(chapters: &[chapter::Model]) -> Vec<text_anchor::Model> {
    let now = chrono::Utc::now().naive_utc();
    source_spans_for_chapters(chapters)
        .iter()
        .enumerate()
        .map(|(index, span)| text_anchor::Model {
            id: i64::try_from(index + 1).expect("test anchor id should fit i64"),
            project_id: 1,
            source_chapter_id: Some(span.chapter_id),
            source_snapshot_id: None,
            source_snapshot_chapter_id: None,
            source_kind: "chapter_content".to_string(),
            source_id: span.chapter_id,
            paragraph_index: i32::try_from(span.sentence_index)
                .expect("test sentence index should fit i32"),
            start_offset: i32::try_from(span.char_start).expect("test start offset should fit i32"),
            end_offset: i32::try_from(span.char_end).expect("test end offset should fit i32"),
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

fn canonical_discovery_content(evidence_anchor_id: i64) -> String {
    json!({
        "characters": [{
            "name": "沈砚",
            "aliases": ["沈先生", "沈砚"],
            "importance": "primary",
            "role": "主角",
            "explicit_attributes": {
                "identity": "书院弟子",
                "appearance": "青衫少年"
            },
            "implicit_attributes": {
                "cautious": "遇事先观察",
                "personality": "冷静谨慎"
            },
            "social_attributes": {
                "书院": "弟子"
            },
            "evidence_anchor_id": evidence_anchor_id
        }]
    })
    .to_string()
}

#[test]
fn input_contract_rejects_non_positive_project_and_chapter() {
    let project_error = required_character_discovery_input(&CharacterDiscoveryInput {
        project_id: 0,
        chapter_number: None,
    })
    .expect_err("non-positive project id must fail");
    assert!(project_error.to_string().contains("项目 ID 必须大于 0"));

    let chapter_error = required_character_discovery_input(&CharacterDiscoveryInput {
        project_id: 1,
        chapter_number: Some(0),
    })
    .expect_err("non-positive chapter number must fail");
    assert!(
        chapter_error
            .to_string()
            .contains("角色发现章节号必须大于 0")
    );
}

#[test]
fn workflow_and_call_fingerprints_validate_input_contracts() {
    let request = a4n_llm::provider::ChatRequest::new(
        vec![a4n_llm::provider::ChatMessage::user("发现角色")],
        None,
        None,
        None,
    );
    assert!(character_discovery_workflow_run_input(0, None, &[], &[], &request).is_err());

    let fingerprint = character_discovery_call_fingerprint_json(1, Some(2), &request)
        .expect("call fingerprint should build");
    assert_eq!(fingerprint["workflow"], json!("character_discovery"));
    assert_eq!(fingerprint["project_id"], json!(1));
    assert_eq!(fingerprint["chapter_number"], json!(2));
}

#[test]
fn character_discovery_user_prompt_uses_json_payload() {
    let (chapters, anchors) = discovery_anchor_context("沈砚推门而入");
    let prompt = build_character_discovery_user_prompt(
        &[json!({"name": "沈砚"})],
        &[json!({
            "chapter_number": 1,
            "title": "雨夜",
            "content": "沈砚推门而入"
        })],
        &chapters,
        &anchors,
    )
    .expect("discovery prompt should build");

    assert!(prompt.contains(r#""task": {"#));
    assert!(prompt.contains(r#""name": "character_discovery""#));
    assert!(prompt.contains(r#""existing_profiles""#));
    assert!(prompt.contains(r#""chapters""#));
    let payload_text = prompt
        .split_once("输入：\n")
        .map(|(_, payload)| payload)
        .expect("prompt should include json payload");
    let payload: Value = serde_json::from_str(payload_text).expect("prompt payload should be json");
    assert_eq!(
        payload["output_contract"]["size_limits"]["evidence_anchor_id_count"],
        json!(1)
    );
    assert!(
        !payload["output_contract"]["character_required_fields"]
            .as_array()
            .expect("required fields should be array")
            .contains(&json!("appearance"))
    );
    assert!(
        payload["output_contract"]["system_derived_fields"]
            .as_array()
            .expect("system derived fields should be array")
            .contains(&json!("personality"))
    );
    assert!(payload["output_contract"].get("field_types").is_none());
    assert!(
        payload["output_contract"]["strict_rules"]
            .as_array()
            .expect("strict rules should be array")
            .iter()
            .any(|rule| rule
                .as_str()
                .is_some_and(|text| text.contains("不要输出 first_seen_chapter")))
    );
    assert!(!prompt.contains("已有角色档案列表："));
}

#[test]
fn existing_profile_for_prompt_projects_identity_without_database_metadata() {
    let now = chrono::Utc::now().naive_utc();
    let profile = protagonist_profile::Model {
        id: 42,
        project_id: 9,
        name: "陈恪".to_string(),
        gender: Some("男".to_string()),
        age: Some("二十余岁".to_string()),
        base_attributes: json!({
            "explicit_attributes": {
                "aliases": ["张均"],
                "身份": "东宫书吏"
            },
            "implicit_attributes": {},
            "social_attributes": {}
        }),
        current_state: json!({
            "explicit_attributes": {
                "身份": "太子府书吏",
                "所在地": "长安"
            },
            "implicit_attributes": {},
            "social_attributes": {}
        }),
        character_arc: None,
        voice_fingerprint: None,
        appearance: Some("青衫清瘦".to_string()),
        background: Some("现代史学博士魂穿".to_string()),
        personality: Some("冷静谨慎".to_string()),
        last_synced_chapter: 2,
        created_at: now,
        updated_at: now,
    };

    let value = existing_profile_for_prompt(&profile).expect("profile prompt should build");

    assert_eq!(value["aliases"], json!(["张均"]));
    assert_eq!(value["appearance"], json!("青衫清瘦"));
    assert_eq!(value["base_explicit_attributes"]["身份"], json!("东宫书吏"));
    assert_eq!(
        value["current_explicit_attributes"]["所在地"],
        json!("长安")
    );
    assert!(value.get("id").is_none());
    assert!(value.get("project_id").is_none());
    assert!(value.get("created_at").is_none());
    assert!(value.get("updated_at").is_none());
}

#[test]
fn existing_profile_for_prompt_rejects_missing_explicit_identity_context() {
    let now = chrono::Utc::now().naive_utc();
    let profile = protagonist_profile::Model {
        id: 42,
        project_id: 9,
        name: "陈恪".to_string(),
        gender: None,
        age: None,
        base_attributes: json!({
            "implicit_attributes": {},
            "social_attributes": {}
        }),
        current_state: json!({
            "explicit_attributes": {},
            "implicit_attributes": {},
            "social_attributes": {}
        }),
        character_arc: None,
        voice_fingerprint: None,
        appearance: None,
        background: None,
        personality: None,
        last_synced_chapter: 2,
        created_at: now,
        updated_at: now,
    };

    let error = existing_profile_for_prompt(&profile)
        .expect_err("missing base explicit attributes must fail visibly");
    assert!(error.contains("base_attributes"));
    assert!(error.contains("explicit_attributes"));
}

#[test]
fn chapter_context_projects_analysis_without_database_metadata() {
    let mut chapter = discovery_chapter(2, "generated", "陈恪在东宫见到魏征。");
    chapter.summary = Some("陈恪初见魏征。".to_string());
    chapter.analysis_data = Some(json!({
        "metadata": {
            "characters": ["陈恪", "魏征"],
            "locations": ["东宫"],
            "items": [],
            "tags": ["对话"],
            "tone": "紧张",
            "timeline_marker": "清晨"
        },
        "summaries": {
            "compressed": "陈恪在东宫见到魏征。",
            "one_line": "陈恪初见魏征",
            "keywords": ["东宫", "魏征"]
        },
        "character_states": {
            "陈恪": {
                "location": "东宫",
                "status": "试探应对",
                "changes": ["被魏征注意"],
                "emotional_state": "警惕"
            }
        },
        "foreshadowing": {
            "planted": [],
            "resolved": [],
            "tensions": ["魏征起疑"]
        },
        "key_events": [{
            "type": "relationship",
            "description": "陈恪与魏征初次交锋",
            "importance": "high",
            "involved_characters": ["陈恪", "魏征"]
        }]
    }));

    let value = character_llm_chapter_context(&chapter);

    assert_eq!(value["chapter_number"], json!(2));
    assert_eq!(value["summary"], json!("陈恪初见魏征。"));
    assert_eq!(
        value["analysis"]["character_states"]["陈恪"]["status"],
        json!("试探应对")
    );
    assert_eq!(
        value["analysis"]["key_events"][0]["description"],
        json!("陈恪与魏征初次交锋")
    );
    assert_eq!(value["content"], json!("陈恪在东宫见到魏征。"));
    assert!(value.get("id").is_none());
    assert!(value.get("project_id").is_none());
    assert!(value.get("created_at").is_none());
    assert!(value.get("updated_at").is_none());
    assert!(value["analysis"].get("summaries").is_none());
}

#[test]
fn parse_output_reports_truncated_json() {
    let (chapters, anchors) = discovery_anchor_context("沈砚推门而入。");
    let error = parse_character_discovery_output(
        r#"{"characters":[{"name":"沈砚","aliases":[],"importance":"primary","role":"主角","explicit_attributes":{},"implicit_attributes":{},"social_attributes":{},"evidence_anchor_id":"#,
        &chapters,
        &anchors,
    )
    .expect_err("truncated discovery json should fail");

    assert!(error.contains("JSON 对象"));
    assert!(error.contains("输出被截断") || error.contains("max_tokens"));
}

#[test]
fn parse_output_accepts_canonical_object_and_projects_base_attributes() {
    let (chapters, anchors) = discovery_anchor_context("沈砚推门而入。");
    let evidence_anchor_id = anchor_id_containing(&anchors, "沈砚推门而入");
    let characters = parse_character_discovery_output(
        &canonical_discovery_content(evidence_anchor_id),
        &chapters,
        &anchors,
    )
    .expect("canonical discovery output should parse");

    assert_eq!(characters.len(), 1);
    assert_eq!(characters[0]["name"], json!("沈砚"));
    assert_eq!(characters[0]["aliases"], json!(["沈先生"]));
    assert_eq!(characters[0]["first_seen_chapter"], json!(1));
    assert_eq!(characters[0]["latest_seen_chapter"], json!(1));
    assert_eq!(
        characters[0]["explicit_attributes"]["identity"],
        json!("书院弟子")
    );

    let base = discovered_character_base_attributes(&characters[0])
        .expect("discovered character should map to base attributes");
    assert_eq!(base["explicit_attributes"]["role"], json!("主角"));
    assert_eq!(base["explicit_attributes"]["appearance"], json!("青衫少年"));
    assert_eq!(
        base["implicit_attributes"]["personality"],
        json!("冷静谨慎")
    );
    assert_eq!(base["social_attributes"]["书院"], json!("弟子"));
}

#[test]
fn parse_output_rejects_legacy_evidence_field() {
    let (chapters, anchors) = discovery_anchor_context("沈砚推门而入。");
    let content = json!({
        "characters": [{
            "name": "沈砚",
            "aliases": [],
            "importance": "primary",
            "role": "主角",
            "explicit_attributes": {},
            "implicit_attributes": {},
            "social_attributes": {},
            "evidence": "沈砚推门而入"
        }]
    })
    .to_string();

    let error = parse_character_discovery_output(&content, &chapters, &anchors)
        .expect_err("legacy free-text evidence must fail");
    assert!(error.contains("evidence 已废弃"));
    assert!(error.contains("evidence_anchor_id"));
}

#[test]
fn parse_output_rejects_legacy_evidence_span_id() {
    let (chapters, anchors) = discovery_anchor_context("沈砚推门而入。");
    let content = json!({
        "characters": [{
            "name": "沈砚",
            "aliases": [],
            "importance": "primary",
            "role": "主角",
            "explicit_attributes": {},
            "implicit_attributes": {},
            "social_attributes": {},
            "evidence_span_id": "ch9:s1:f1"
        }]
    })
    .to_string();

    let error = parse_character_discovery_output(&content, &chapters, &anchors)
        .expect_err("legacy evidence span id must fail");
    assert!(error.contains("evidence_span_id 已废弃"));
    assert!(error.contains("evidence_anchor_id"));
}

#[test]
fn parse_output_rejects_model_supplied_derived_fields() {
    let (chapters, anchors) = discovery_anchor_context("沈砚推门而入。");
    let evidence_anchor_id = anchor_id_containing(&anchors, "沈砚推门而入");
    let content = json!({
        "characters": [{
            "name": "沈砚",
            "aliases": [],
            "first_seen_chapter": 1,
            "importance": "primary",
            "role": "主角",
            "explicit_attributes": {},
            "implicit_attributes": {},
            "social_attributes": {},
            "evidence_anchor_id": evidence_anchor_id
        }]
    })
    .to_string();

    let error = parse_character_discovery_output(&content, &chapters, &anchors)
        .expect_err("model supplied derived chapter field must fail");
    assert!(error.contains("未支持字段 `first_seen_chapter`"));
}

#[test]
fn parse_output_rejects_unknown_evidence_anchor_id() {
    let (chapters, anchors) = discovery_anchor_context("沈砚推门而入。");
    let content = canonical_discovery_content(999);

    let error = parse_character_discovery_output(&content, &chapters, &anchors)
        .expect_err("unknown evidence anchor id must fail");
    assert!(error.contains("evidence_anchor_id 无效"));
    assert!(error.contains("text_anchor_id 不在本次输入候选中"));
}

#[test]
fn parse_output_rejects_invalid_importance() {
    let (chapters, anchors) = discovery_anchor_context("沈砚推门而入。");
    let evidence_anchor_id = anchor_id_containing(&anchors, "沈砚推门而入");
    let content = json!({
        "characters": [{
            "name": "沈砚",
            "aliases": [],
            "importance": "lead",
            "role": "主角",
            "explicit_attributes": {},
            "implicit_attributes": {},
            "social_attributes": {},
            "evidence_anchor_id": evidence_anchor_id
        }]
    })
    .to_string();

    let error = parse_character_discovery_output(&content, &chapters, &anchors)
        .expect_err("invalid importance must fail");
    assert!(error.contains("importance"));
}

#[test]
fn parse_output_rejects_non_object_attributes() {
    let (chapters, anchors) = discovery_anchor_context("沈砚推门而入。");
    let evidence_anchor_id = anchor_id_containing(&anchors, "沈砚推门而入");
    let content = json!({
        "characters": [{
            "name": "沈砚",
            "aliases": [],
            "importance": "primary",
            "role": "主角",
            "explicit_attributes": {},
            "implicit_attributes": [],
            "social_attributes": {},
            "evidence_anchor_id": evidence_anchor_id
        }]
    })
    .to_string();

    let error = parse_character_discovery_output(&content, &chapters, &anchors)
        .expect_err("attribute objects must be strict");
    assert!(error.contains("implicit_attributes"));
}

#[test]
fn parse_output_rejects_duplicate_alias_identity() {
    let (chapters, anchors) = discovery_anchor_context("沈砚推门而入。沈先生回头。");
    let first_anchor_id = anchor_id_containing(&anchors, "沈砚推门而入");
    let second_anchor_id = anchor_id_containing(&anchors, "沈先生回头");
    let content = json!({
        "characters": [
            {
                "name": "沈砚",
                "aliases": ["沈先生"],
                "importance": "primary",
                "role": "主角",
                "explicit_attributes": {},
                "implicit_attributes": {},
                "social_attributes": {},
                "evidence_anchor_id": first_anchor_id
            },
            {
                "name": "沈先生",
                "aliases": [],
                "importance": "secondary",
                "role": "先生",
                "explicit_attributes": {},
                "implicit_attributes": {},
                "social_attributes": {},
                "evidence_anchor_id": second_anchor_id
            }
        ]
    })
    .to_string();

    let error = parse_character_discovery_output(&content, &chapters, &anchors)
        .expect_err("duplicate alias identity must fail");
    assert!(error.contains("重复"));
}

#[test]
fn select_chapters_uses_available_content_only() {
    let chapters = vec![
        discovery_chapter(1, "completed", "有效正文"),
        discovery_chapter(3, "completed", "另一章正文"),
    ];

    let selected = select_character_discovery_chapters(chapters, 1, Some(3))
        .expect("selected available chapter should pass");
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].chapter_number, 3);
}

#[test]
fn select_chapters_rejects_missing_or_empty_scope() {
    let missing = select_character_discovery_chapters(
        vec![discovery_chapter(1, "completed", "有效正文")],
        1,
        Some(2),
    )
    .expect_err("missing available chapter should fail");
    assert!(missing.contains("没有可用于角色发现的有效正文"));

    let empty = select_character_discovery_chapters(Vec::new(), 9, None)
        .expect_err("empty available chapter list should fail");
    assert!(empty.contains("项目还没有可用于角色发现的章节"));
    assert!(empty.contains("project_id=9"));
}

#[test]
fn validate_chapter_scope_accepts_input_and_rejects_out_of_input_chapter() {
    let analyzed = [1, 3].into_iter().collect::<BTreeSet<_>>();
    validate_discovered_character_chapter_scope(
        &[json!({
            "name": "沈砚",
            "first_seen_chapter": 1,
            "latest_seen_chapter": 3
        })],
        &analyzed,
    )
    .expect("first/latest seen chapters from input should pass");

    let error = validate_discovered_character_chapter_scope(
        &[json!({
            "name": "沈砚",
            "first_seen_chapter": 1,
            "latest_seen_chapter": 2
        })],
        &analyzed,
    )
    .expect_err("out-of-input discovery chapter should fail");
    assert!(error.contains("latest_seen_chapter"));
    assert!(error.contains("不在本次角色发现输入章节中"));
}
