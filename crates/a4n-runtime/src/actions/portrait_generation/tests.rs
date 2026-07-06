use super::*;
use chrono::NaiveDate;

fn profile_for_test() -> protagonist_profile::Model {
    let now = NaiveDate::from_ymd_opt(2026, 5, 31)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    protagonist_profile::Model {
        id: 42,
        project_id: 7,
        name: "陈恪".to_string(),
        gender: Some("男".to_string()),
        age: Some("二十余岁".to_string()),
        base_attributes: json!({"explicit_attributes":{"身份":"东宫书吏"}}),
        current_state: json!({"explicit_attributes":{"衣着":"青色官袍"}}),
        character_arc: Some(json!({"current":"试图改变玄武门结局"})),
        voice_fingerprint: None,
        appearance: Some("清瘦".to_string()),
        background: Some("隋唐史博士".to_string()),
        personality: Some("冷静".to_string()),
        last_synced_chapter: 3,
        created_at: now,
        updated_at: now,
    }
}

#[test]
fn optional_profile_text_trims_blank_to_none() {
    assert_eq!(
        optional_profile_text(Some("  男  ")),
        Some("男".to_string())
    );
    assert_eq!(optional_profile_text(Some(" \n\t ")), None);
    assert_eq!(optional_profile_text(None), None);
}

#[test]
fn first_expression_or_default_uses_neutral_for_missing_or_blank_values() {
    let empty: Vec<String> = Vec::new();
    let blank = vec!["  ".to_string()];

    for value in [None, Some(empty.as_slice()), Some(blank.as_slice())] {
        assert_eq!(
            first_expression_or_default(value),
            DEFAULT_PORTRAIT_EXPRESSION
        );
    }
}

#[test]
fn first_expression_or_default_uses_first_meaningful_value() {
    let expressions = vec!["  ".to_string(), " happy ".to_string()];

    assert_eq!(
        first_expression_or_default(Some(expressions.as_slice())),
        "happy".to_string()
    );
}

#[test]
fn first_expression_or_default_trims_first_value() {
    let expressions = vec![" happy ".to_string()];

    assert_eq!(
        first_expression_or_default(Some(expressions.as_slice())),
        "happy".to_string()
    );
}

#[test]
fn profile_portrait_seed_prompt_uses_available_context_without_required_descriptors() {
    let mut profile = profile_for_test();
    profile.gender = Some(" ".to_string());
    profile.age = None;
    profile.character_arc = None;
    profile.appearance = Some(" ".to_string());
    profile.background = None;
    profile.personality = None;

    let prompt = profile_portrait_seed_prompt(&profile).expect("prompt should serialize");

    assert!(prompt.contains("单张人物三视图与人物拆解设定图"));
    assert!(prompt.contains("正面全身视图"));
    assert!(prompt.contains("侧面全身视图"));
    assert!(prompt.contains("背面全身视图"));
    assert!(prompt.contains("人物拆解区"));
    assert!(prompt.contains("服饰层级拆解"));
    assert!(prompt.contains("鞋履拆解"));
    assert!(prompt.contains("材质与色板拆解"));
    assert!(prompt.contains("至少包含服饰层级、发型、鞋履、材质色板四类视觉拆解"));
    assert!(prompt.contains("最终图像不能只有三视图"));
    assert!(prompt.contains("档案明确出现的项链"));
    assert!(prompt.contains("不要硬塞项链、戒指、武器等具体配饰"));
    assert!(prompt.contains("角色名：陈恪"));
    assert!(!prompt.contains("性别："));
    assert!(!prompt.contains("外观："));
    assert!(prompt.contains("东宫书吏"));
    assert!(prompt.contains("青色官袍"));
}

#[test]
fn portrait_profile_context_includes_dynamic_state_without_database_metadata() {
    let profile = profile_for_test();

    let context = portrait_profile_context(&profile);
    let text = serde_json::to_string(&context).expect("context should serialize");

    assert!(text.contains("东宫书吏"));
    assert!(text.contains("青色官袍"));
    assert!(text.contains("玄武门"));
    assert!(!text.contains("\"id\""));
    assert!(!text.contains("\"project_id\""));
    assert!(!text.contains("\"created_at\""));
    assert!(!text.contains("\"updated_at\""));
    assert!(!text.contains("\"last_synced_chapter\""));
}
