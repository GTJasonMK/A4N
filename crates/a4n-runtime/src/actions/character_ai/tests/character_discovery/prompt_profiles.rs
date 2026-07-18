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
fn existing_profile_for_prompt_includes_identity_context_without_database_metadata() {
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

    let value = existing_profile_for_prompt(&profile).expect("profile prompt");

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

    let err = existing_profile_for_prompt(&profile)
        .expect_err("missing base explicit attributes must fail visibly");

    assert!(err.contains("base_attributes"));
    assert!(err.contains("explicit_attributes"));
}

#[test]
fn character_llm_chapter_context_includes_existing_analysis_without_database_metadata() {
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
