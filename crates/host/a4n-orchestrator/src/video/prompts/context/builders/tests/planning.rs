#[test]
fn video_user_prompt_forbids_markdown_fenced_json() {
    let engine = prompt_renderer();
    let user = render_strict_json_user_prompt(
        &engine,
        PROJECT_ID,
        "测试视频 JSON 输出",
        &json!({
            "task": "test",
            "output_contract": {"format": "json_object"}
        }),
    )
    .expect("video user prompt should render");

    let prefix = video_user_cache_prefix(&user);
    assert!(prefix.contains("第一个字符必须是 `{`"));
    assert!(prefix.contains("最后一个非空字符必须是 `}`"));
    assert!(prefix.contains("禁止以 Markdown 代码围栏包裹 JSON"));
}

#[test]
fn segment_plan_uses_director_duration_and_preserves_performance_requirements() {
    let engine = prompt_renderer();
    let current_shot = crate::video::tests::test_video_execution_shot(6);
    let user =
        build_segment_plan_for_shot_user(planning_context(&engine, "电影感写实"), &current_shot)
            .expect("segment plan user prompt should render");

    let payload = video_user_payload(&user);
    assert_eq!(payload["constraints"]["selected_duration_secs"], json!(6));
    assert!(
        payload["constraints"]["default_duration_secs"].is_null(),
        "segment planning should not expose a separate default duration"
    );
    assert!(
        !contains_json_key(&payload, "duration_secs_options"),
        "segment planning should not expose selectable duration options to the model"
    );
    assert!(
        !contains_json_key(&payload, "model_duration_capability"),
        "segment planning should not expose provider duration capability to the model"
    );
    assert!(payload["constraints"]["duration_selection_rule"].is_null());
    assert!(payload["constraints"]["time_budget_rule"].is_null());
    let required_fields = payload["output_contract"]["required_segment_fields"]
        .as_array()
        .expect("required_segment_fields should be array");
    assert!(
        required_fields
            .iter()
            .any(|field| field.as_str() == Some("performance_plan")),
        "segment plan contract should require performance_plan"
    );
    let action_script_rule = payload["output_contract"]["performance_action_script_rule"]
        .as_str()
        .expect("segment plan contract should include action_script rule");
    for required in ["character", "body", "movement", "hands", "gaze", "timing"] {
        assert!(
            action_script_rule.contains(required),
            "action_script rule should mention required field {required}"
        );
    }
    assert!(
        action_script_rule.contains("非空字符串")
            && action_script_rule.contains("禁止空字符串")
            && action_script_rule.contains("null")
            && action_script_rule.contains("省略字段"),
        "action_script rule should reject empty or missing field values"
    );
    assert!(
        action_script_rule.contains("手未接触道具") && action_script_rule.contains("双手保持稳定"),
        "hands rule should require visible idle hand state when there is no obvious hand action"
    );
}

#[test]
fn locked_director_shot_has_one_count_rule_and_no_split_instructions() {
    let mut shot = crate::video::tests::test_video_execution_shot(10);
    shot.director_constraints.prompt = "人物停在门边".into();
    shot.action_beats.clear();
    let engine = prompt_renderer();
    let user = build_segment_plan_for_shot_user(planning_context(&engine, "电影感写实"), &shot)
        .expect("fixed shot request");
    let payload = video_user_payload(&user);
    assert_eq!(payload["constraints"]["selected_duration_secs"], 10);
    assert!(payload["constraints"]["time_budget_rule"].is_null());
    assert!(payload["constraints"]["duration_selection_rule"].is_null());
    assert_eq!(user.matches("人物停在门边").count(), 1);
    assert!(!user.contains("1 个或多个"));
    assert!(
        payload["output_contract"]["segment_count_rule"]
            .as_str()
            .unwrap()
            .contains("只能输出一个")
    );
}
