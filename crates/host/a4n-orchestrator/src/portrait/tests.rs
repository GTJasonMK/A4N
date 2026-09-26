use super::prompt_generation::portrait_prompt_request;
use super::*;
use a4n_api_contract::portrait::PortraitType;
use a4n_prompt::tera_engine::TeraPromptEngine;
use serde_json::json;
use std::path::PathBuf;

fn prompts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../prompts")
}

fn prompt_engine() -> TeraPromptEngine {
    TeraPromptEngine::from_directory(&prompts_dir()).expect("prompts should load")
}

fn prompt_spec(profile_name: &str, portrait_type: PortraitType) -> PortraitPromptSpec<'_> {
    PortraitPromptSpec {
        profile_name,
        appearance: None,
        gender: None,
        age: None,
        personality: None,
        background: None,
        profile_context: None,
        project_context: None,
        portrait_type,
        art_style: None,
        expression: None,
        existing_portrait_count: None,
    }
}

#[tokio::test]
async fn portrait_variants_share_identity_and_send_appearance_once() {
    let engine = prompt_engine();
    let variables = std::collections::HashMap::new();
    let mut first_pack = None;
    for expression in ["calm", "smile"] {
        let mut request = super::prompt_generation::build_generate_prompt_request(
            super::prompt_generation::PortraitPromptRequestInput {
                project_id: 1,
                spec: PortraitPromptSpec {
                    appearance: Some("独特的灰色制服"),
                    expression: Some(expression),
                    ..prompt_spec("林岚", PortraitType::FullBody)
                },
                prompt_renderer: &engine,
                variables: &variables,
            },
        )
        .await
        .unwrap();
        let pack = request.context_manifest_json.as_ref().unwrap()["packs"][0].clone();
        if let Some(first) = &first_pack {
            assert_eq!(&pack, first);
        } else {
            first_pack = Some(pack);
        }
        a4n_llm_context::preview_context_aware_chat_request(&mut request, 1_000_000, 8192).unwrap();
        assert_eq!(
            request.messages[1]
                .content
                .matches("独特的灰色制服")
                .count(),
            1
        );
        assert!(request.messages[1].content.contains(expression));
    }
}

#[test]
fn portrait_prompt_request_requires_chinese_prompt() {
    let engine = prompt_engine();
    let prompt = portrait_prompt_request(
        1,
        PortraitPromptSpec {
            appearance: Some("短发"),
            gender: Some("女"),
            age: Some("二十岁"),
            ..prompt_spec("林岚", PortraitType::FullBody)
        },
        &engine,
    )
    .expect("prompt should serialize");

    assert!(prompt.contains("中文人物三视图与人物拆解设定图 prompt"));
    assert!(prompt.contains(r#""image_goal": "单张人物三视图与人物拆解设定图"#));
    assert!(prompt.contains("正面全身视图"));
    assert!(prompt.contains("侧面全身视图"));
    assert!(prompt.contains("背面全身视图"));
    assert!(prompt.contains(r#""breakdown_sections""#));
    assert!(prompt.contains(r#""layout_spec""#));
    assert!(prompt.contains("服饰层级拆解"));
    assert!(prompt.contains("鞋履拆解"));
    assert!(prompt.contains("材质与色板拆解"));
    assert!(prompt.contains("至少包含服饰层级、发型、鞋履、材质色板四类视觉拆解"));
    assert!(prompt.contains("最终图像不能只有三视图"));
    assert!(prompt.contains("档案明确出现的项链"));
    assert!(prompt.contains("不要硬塞项链、戒指、武器等具体配饰"));
    assert!(!prompt.contains("英文"));
}

#[test]
fn normalize_portrait_prompt_response_rejects_structured_or_fenced_output() {
    assert!(normalize_portrait_prompt_response("  高质量中文人物设定图提示词  ").is_ok());
    for raw in [
        r#"{"prompt":"高质量中文人物设定图提示词"}"#,
        r#"[{"prompt":"高质量中文人物设定图提示词"}]"#,
        "```text\n高质量中文人物设定图提示词\n```",
        "# 人物设定图提示词\n高质量中文人物设定图提示词",
        "<think>先分析</think>\n高质量中文人物设定图提示词",
    ] {
        let err = normalize_portrait_prompt_response(raw).expect_err("structured output must fail");
        assert!(err.to_string().contains("必须是纯 prompt 文本"));
    }
}

#[test]
fn portrait_prompt_request_omits_missing_appearance() {
    let engine = prompt_engine();
    let prompt = portrait_prompt_request(1, prompt_spec("林烬", PortraitType::FullBody), &engine)
        .expect("prompt should serialize");

    assert!(prompt.contains(r#""character_name": "林烬""#));
    assert!(prompt.contains(
        r#""portrait_type": "人物三视图与拆解设定图（正面、侧面、背面，全身，含服饰配饰拆解）""#
    ));
    assert!(!prompt.contains(r#""appearance": "#));
    assert!(!prompt.contains(r#""gender": "#));
    assert!(!prompt.contains(r#""appearance": "外观描述""#));
}

#[test]
fn portrait_prompt_request_omits_blank_appearance() {
    let engine = prompt_engine();
    let prompt = portrait_prompt_request(
        1,
        PortraitPromptSpec {
            appearance: Some(" \n\t "),
            gender: Some(" \n\t "),
            ..prompt_spec("林烬", PortraitType::FullBody)
        },
        &engine,
    )
    .expect("prompt should serialize");

    assert!(!prompt.contains(r#""appearance": "#));
    assert!(!prompt.contains(r#""gender": "#));
    assert!(!prompt.contains(r#""appearance": "外观描述""#));
}

#[test]
fn portrait_task_refers_to_identity_and_keeps_variant_settings() {
    let engine = prompt_engine();
    let profile_context = json!({"current_state":{"explicit_attributes":{"衣着":"青色官袍"}}});
    let project_context = json!({"genre":"历史穿越","tone":"紧张克制"});
    let prompt = portrait_prompt_request(
        1,
        PortraitPromptSpec {
            appearance: Some(" 灰色制服 "),
            gender: Some(" 男 "),
            age: Some(" 二十岁 "),
            personality: Some(" 冷静克制 "),
            background: Some(" 东宫书吏 "),
            profile_context: Some(&profile_context),
            project_context: Some(&project_context),
            art_style: Some(" 水彩风格 "),
            expression: Some(" calm "),
            existing_portrait_count: Some(2),
            ..prompt_spec("林烬", PortraitType::HalfBody)
        },
        &engine,
    )
    .expect("prompt should serialize");

    assert!(prompt.contains(r#""character_name": "林烬""#));
    assert!(prompt.contains(
        r#""portrait_type": "人物三视图与拆解设定图（正面、侧面、背面，重点拆解服饰层级与配饰细节）""#
    ));
    assert!(!prompt.contains("灰色制服"));
    assert!(!prompt.contains("冷静克制"));
    assert!(!prompt.contains("东宫书吏"));
    assert!(prompt.contains(r#""profile_context""#));
    assert!(prompt.contains("portrait_profile_project_context.profile_context"));
    assert!(!prompt.contains("青色官袍"));
    assert!(prompt.contains(r#""project_context""#));
    assert!(prompt.contains("portrait_profile_project_context.project_context"));
    assert!(!prompt.contains("历史穿越"));
    assert!(prompt.contains(r#""art_style": "水彩风格""#));
    assert!(prompt.contains(r#""expression": "calm""#));
    assert!(prompt.contains(r#""existing_portrait_count": 2"#));
}
