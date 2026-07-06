use super::optimization::{
    build_video_prompt_optimization_request, optimized_motion_prompt,
    parse_video_prompt_optimization_output, validate_optimized_prompt_fields,
    video_prompt_optimization_checkpoint_response,
};
use super::*;
use a4n_engine::prompt::tera_engine::TeraPromptEngine;
use a4n_services::video::PromptOutputLanguage;
use chrono::Utc;
use std::path::Path;

fn prompt_engine() -> TeraPromptEngine {
    let prompts_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../prompts");
    TeraPromptEngine::from_directory(&prompts_dir).expect("prompts should load")
}

fn video_prompt_row() -> chapter_video_prompt::Model {
    let now = Utc::now().naive_utc();
    chapter_video_prompt::Model {
        id: 9,
        project_id: 1,
        chapter_id: 2,
        style: Some("电影感写实".to_string()),
        prompt_type: "video_prompt".to_string(),
        workflow_mode: VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES.to_string(),
        shot_number: 3,
        title: Some("穿过走廊".to_string()),
        scene_summary: Some("主角沿走廊前进并停在门前".to_string()),
        visual_prompt: "中景展示主角穿过昏暗走廊，左侧墙面、门框和手电光束作为固定参照物，人物服装和空间关系保持一致。".to_string(),
        motion_prompt: Some("镜头缓慢跟随主角向前，动作从谨慎前行过渡到停在门前侧耳倾听。".to_string()),
        negative_prompt: Some("避免空间跳变".to_string()),
        duration_secs: 5,
        aspect_ratio: Some("16:9".to_string()),
        reference_image_ids_json: None,
        status: "completed".to_string(),
        generated_video_id: None,
        metadata_json: None,
        created_at: now,
        updated_at: now,
    }
}

#[test]
fn update_prompt_input_distinguishes_missing_and_null_nullable_fields() {
    let missing = serde_json::from_value::<UpdateChapterVideoPromptInput>(json!({}))
        .expect("empty update input should deserialize");
    assert_eq!(missing.motion_prompt, None);
    assert_eq!(missing.negative_prompt, None);
    assert_eq!(missing.reference_image_ids_json, None);

    let cleared = serde_json::from_value::<UpdateChapterVideoPromptInput>(json!({
        "motion_prompt": null,
        "negative_prompt": null,
        "reference_image_ids_json": null
    }))
    .expect("null update fields should deserialize as explicit clears");
    assert_eq!(cleared.motion_prompt, Some(None));
    assert_eq!(cleared.negative_prompt, Some(None));
    assert_eq!(cleared.reference_image_ids_json, Some(None));
}

#[test]
fn update_prompt_input_rejects_unknown_fields() {
    let err = serde_json::from_value::<UpdateChapterVideoPromptInput>(json!({
        "visual_prompt": "新提示词",
        "legacy_field": true
    }))
    .expect_err("unknown update input fields should fail");

    assert!(err.to_string().contains("unknown field"));
}

#[test]
fn video_prompt_optimization_output_requires_json_object() {
    let err = parse_video_prompt_optimization_output("优化好了").expect_err("plain text fails");
    assert!(err.contains("JSON"));
}

#[test]
fn video_prompt_optimization_output_rejects_fenced_json() {
    let err = parse_video_prompt_optimization_output(
        "```json\n{\"visual_prompt\":\"新的画面提示词\",\"motion_prompt\":null,\"negative_prompt\":\"避免模糊\"}\n```",
    )
    .expect_err("fenced json must fail instead of being silently unwrapped");
    assert!(err.contains("JSON"));
}

#[test]
fn video_prompt_optimization_output_rejects_unknown_fields() {
    let err = parse_video_prompt_optimization_output(
        r#"{"visual_prompt":"新的画面提示词","motion_prompt":null,"negative_prompt":null,"legacy_notes":"旧字段"}"#,
    )
    .expect_err("unknown optimized fields should fail");

    assert!(err.contains("unknown field"));
}

#[test]
fn video_prompt_optimization_checkpoint_revalidates_response_content() {
    let raw = r#"{
        "visual_prompt": "优化后的画面提示词",
        "motion_prompt": null,
        "negative_prompt": "避免空间跳变"
    }"#;

    let (response_text, usage, finish_reason) = video_prompt_optimization_checkpoint_response(
        &json!({
            "response_content": raw,
            "parsed_output": {
                "visual_prompt": "优化后的画面提示词",
                "motion_prompt": null,
                "negative_prompt": "避免空间跳变"
            },
            "usage": {"prompt_tokens": 10},
            "finish_reason": "stop"
        }),
        "optimize_chapter_video_prompt:prompt:9",
    )
    .expect("valid checkpoint should restore");

    assert_eq!(response_text, raw);
    assert_eq!(
        usage.expect("usage should restore")["prompt_tokens"],
        json!(10)
    );
    assert_eq!(finish_reason.as_deref(), Some("stop"));
}

#[test]
fn video_prompt_optimization_checkpoint_rejects_mismatched_parsed_output() {
    let raw = r#"{
        "visual_prompt": "优化后的画面提示词",
        "motion_prompt": null,
        "negative_prompt": "避免空间跳变"
    }"#;

    let err = video_prompt_optimization_checkpoint_response(
        &json!({
            "response_content": raw,
            "parsed_output": {
                "visual_prompt": "另一个画面提示词",
                "motion_prompt": null,
                "negative_prompt": "避免空间跳变"
            }
        }),
        "optimize_chapter_video_prompt:prompt:9",
    )
    .expect_err("mismatched parsed_output must fail");

    assert!(err.contains("重新解析结果不一致"));
}

const VALID_OPTIMIZED_VIDEO_VISUAL: &str = "[主体定义与素材绑定]\n首帧输入图片定义主角站在昏暗走廊左侧的起始画面，参考图只用于保持人物服装、门框位置、墙面纹理和手电光束方向一致，不作为拼贴内容。\n\n[片段内容以及前因后果]\n主角沿走廊谨慎前进，发现门后传来轻微声响后停在门前侧耳倾听，情绪从警觉转为紧张但保持克制。\n\n[镜头分镜]\n镜头 1：平视中景承接主角站在昏暗走廊左侧的首帧，手电光束扫过左侧墙面和前方门框，脚步缓慢且肩膀收紧。\n镜头 2：镜头克制跟随主角靠近门前，他放慢脚步并侧耳听向门后，手电光源方向和门框位置保持稳定。\n镜头 3：画面稳定在主角停在门前侧耳倾听的状态，表情紧张但克制，走廊方向和墙面纹理不跳变。\n\n[画面风格描述]\n电影感写实，低饱和冷色走廊，手电光束形成明确方向，门框和墙面作为固定空间锚点，镜头稳定克制。\n\n[连续性与结尾状态]\n主角服装、走廊方向、门框位置和手电光源方向保持不变，结尾稳定在门前侧耳倾听，可供下一段承接。\n\n[限制与避免项]\n不要配背景音乐，不要字幕、水印、乱码文字、换脸、空间跳变、光影突变或把参考图画成拼贴。";
const VALID_OPTIMIZED_VIDEO_MOTION: &str = "[运镜与动作]\n镜头以平视中景缓慢跟随主角前进，动作重点是谨慎迈步、手电光束扫过门框、脚步放慢并在门前停住，镜头不跳轴。\n\n[对白与表演]\n无对白，主角通过屏住呼吸、肩膀收紧和侧耳倾听表现紧张，眼神集中在门后方向。";
const VALID_OPTIMIZED_FRAME_PROMPT: &str = "【参考图约束】参考图只用于保持主角服装、发型、门框位置和手电光束方向一致，不作为拼贴或多格画面。\n【画面主体】主角站在昏暗走廊门前，身体略微前倾，侧耳倾听门后的细微声响，表情警觉但克制。\n【空间与道具】左侧墙面、前方门框、手电筒和地面阴影作为固定参照物，人物位于画面中央偏左。\n【构图光线风格】电影感写实，平视中景，低饱和冷色调，手电光束从人物手中斜向门框，阴影边缘清晰。\n【一致性与避免项】保持人物服装、门框位置、走廊方向和光源方向一致，避免字幕、水印、换脸、空间跳变和参考图拼贴。";

#[test]
fn optimized_video_prompt_preserves_existing_motion_when_output_is_null() {
    let prompt = video_prompt_row();

    let motion = optimized_motion_prompt(&prompt, None)
        .expect("video prompt should preserve existing motion prompt");

    assert_eq!(motion, prompt.motion_prompt);
}

#[test]
fn optimized_video_prompt_validation_rejects_missing_video_sections_before_save() {
    let prompt = video_prompt_row();

    let err = validate_optimized_prompt_fields(
        &prompt,
        "主角穿过昏暗走廊，左侧墙面、门框和手电光束作为固定参照物，人物服装和空间关系保持一致；镜头缓慢跟随主角向前，动作从谨慎前行过渡到停在门前侧耳倾听，整体保持电影感写实和低饱和冷色调，但这段文本故意没有任何必需章节标题。",
        Some(VALID_OPTIMIZED_VIDEO_MOTION),
        prompt.negative_prompt.as_deref(),
        PromptOutputLanguage::ZhCn,
    )
    .expect_err("optimization must not save provider-invalid video prompt text");

    assert!(err.contains("visual_prompt 缺少必需章节 [主体定义与素材绑定]"));
}

#[test]
fn optimized_video_prompt_validation_accepts_provider_valid_text_before_save() {
    let prompt = video_prompt_row();

    validate_optimized_prompt_fields(
        &prompt,
        VALID_OPTIMIZED_VIDEO_VISUAL,
        Some(VALID_OPTIMIZED_VIDEO_MOTION),
        prompt.negative_prompt.as_deref(),
        PromptOutputLanguage::ZhCn,
    )
    .expect("provider-valid optimized video prompt text should be saved");
}

#[test]
fn optimized_frame_prompt_validation_uses_frame_sections() {
    let mut prompt = video_prompt_row();
    prompt.prompt_type = "first_frame_prompt".to_string();

    validate_optimized_prompt_fields(
        &prompt,
        VALID_OPTIMIZED_FRAME_PROMPT,
        None,
        prompt.negative_prompt.as_deref(),
        PromptOutputLanguage::ZhCn,
    )
    .expect("valid frame prompt sections should pass before saving optimization");
}

#[test]
fn video_prompt_optimization_request_renders_from_prompt_registry() {
    let engine = prompt_engine();
    let prompt = video_prompt_row();

    let request = build_video_prompt_optimization_request(
        &engine,
        &prompt,
        "让镜头动作更具体，但不要改变空间关系",
        PromptOutputLanguage::ZhCn,
    )
    .expect("optimization prompt templates should render");

    assert_eq!(request.messages.len(), 2);
    assert!(
        request.messages[0]
            .content
            .contains("视频生成提示词优化工程师")
    );
    assert!(request.messages[1].content.contains("让镜头动作更具体"));
    assert!(request.messages[1].content.contains("\"visual_prompt\""));
    assert!(request.messages[1].content.contains("\"output_contract\""));
    assert!(
        request.messages[1]
            .content
            .contains("\"visual_prompt_required_sections\"")
    );
    let manifest = request
        .context_manifest_json
        .as_ref()
        .expect("manifest should be attached");
    assert_eq!(
        manifest["packs"][0]["pack_kind"].as_str(),
        Some("source_artifact")
    );
    assert_eq!(
        manifest["packs"][0]["source_refs_json"]["prompt_id"].as_i64(),
        Some(9)
    );
    assert!(
        manifest["packs"][0]["variants"][0]["canonical_content_json"]
            .to_string()
            .contains("穿过走廊")
    );
    assert!(
        manifest["dynamic_user_content"]
            .as_str()
            .expect("dynamic user content should be string")
            .contains("source_artifact.current_prompt")
    );
    assert_eq!(request.max_tokens, None);
    assert_eq!(request.max_tokens_profile.as_deref(), Some("media_prompt"));
    assert_eq!(
        request
            .trace_context
            .as_ref()
            .map(|context| context.workflow.as_str()),
        Some("video_prompt_optimization")
    );
}

#[test]
fn frame_prompt_delete_group_excludes_analysis_and_segment_plans() {
    for prompt_type in [
        "frame_prompt",
        "first_frame_prompt",
        "chat_video_frame_prompt",
    ] {
        assert!(
            is_generated_frame_prompt_type(prompt_type),
            "{prompt_type} should be deleted by the frame prompt group"
        );
    }

    for prompt_type in [
        "frame_shot_analysis",
        "first_frame_shot_analysis",
        "chat_video_shot_analysis",
        "frame_segment_plan",
        "first_frame_segment_plan",
        "chat_video_segment_plan",
        "video_prompt",
    ] {
        assert!(
            !is_generated_frame_prompt_type(prompt_type),
            "{prompt_type} must not be deleted by the frame prompt group"
        );
    }
}

#[test]
fn all_chain_delete_group_includes_every_video_prompt_row() {
    let rows = [
        "frame_shot_analysis",
        "frame_segment_plan",
        "frame_prompt",
        "first_frame_prompt",
        "chat_video_frame_prompt",
        "video_prompt",
    ]
    .into_iter()
    .enumerate()
    .map(|(index, prompt_type)| {
        let mut row = video_prompt_row();
        row.id = index as i64 + 1;
        row.prompt_type = prompt_type.to_string();
        row
    })
    .collect::<Vec<_>>();

    let selected = video_prompt_delete_group_rows(&rows, "all_chain_prompts")
        .expect("all chain delete group should be supported");
    let selected_types = selected
        .into_iter()
        .map(|row| row.prompt_type)
        .collect::<Vec<_>>();

    assert_eq!(
        selected_types,
        vec![
            "frame_shot_analysis",
            "frame_segment_plan",
            "frame_prompt",
            "first_frame_prompt",
            "chat_video_frame_prompt",
            "video_prompt",
        ]
    );
}
