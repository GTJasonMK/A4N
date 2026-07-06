use super::*;
use crate::actions::video_generation::provider_request::{
    ProviderGenerationWorkflow, prompt_provider_workflow, reference_ids_with_storyboard_image,
};
use a4n_engine::prompt::tera_engine::TeraPromptEngine;
use chrono::Utc;
use std::path::Path;

const LONG_VISUAL: &str = "[主体定义与素材绑定]\n首帧输入图片定义小宦官站在昏暗殿内中央偏左的开头画面，尾帧输入图片定义他靠近暗廊入口但不越过的结尾画面；参考图只用于保持内侍服饰、殿内柱影、暗廊入口和烛光空间关系一致。\n\n[片段内容以及前因后果]\n小宦官从殿内中央偏左位置承接上一状态，听见右后方暗廊方向传来细微异响后压住恐惧回望，结尾停在暗廊入口前并保持警觉。\n\n[镜头分镜]\n镜头 1：平视中景从殿内中央偏左的首帧承接，小宦官停住听向右后方暗角，肩膀轻微收紧，柱影和地面烛光反射保持清晰。\n镜头 2：镜头维持同一轴线轻微手持漂移，他原地缓慢后缩半步，靠近暗廊入口但不越过，最后稳定为回望黑暗走廊的戒备姿态。\n\n[画面风格描述]\n电影感写实，右侧暖烛光与暗廊冷阴影对比，平视中景，柱影和地面烛光反射作为固定参照物，整体保持压抑悬疑的低饱和色调。\n\n[连续性与结尾状态]\n小宦官脸部、身形和内侍服饰不变，暗廊始终位于画面右后方，殿门余光、柱影和地面烛光反射位置不变；结尾人物靠近暗廊入口但仍留在入口前，回望黑暗走廊。\n\n[限制与避免项]\n不要配背景音乐，不要字幕、水印、乱码文字、换脸、换装、跳轴、空间跳变、光影方向突变或把参考图画成拼贴。";
const LONG_MOTION: &str = "[运镜与动作]\n镜头保持平视中景和轻微手持漂移但不跳轴，小宦官先停在殿内中央偏左看向暗角，然后原地轻微后缩，缓慢回头望向右后方暗廊，肩膀收紧并压住恐惧，动作节奏缓慢且稳定。\n\n[对白与表演]\n小宦官压低声音说“谁在那儿”，口型短促，眼神避开又重新看向暗廊，表情保持害怕但努力镇定，停顿后屏住呼吸。";
const LONG_EN_VISUAL: &str = "[Subject And Asset Binding]\nThe first input frame defines the young palace attendant standing near the center-left of a dim hall, while the ending frame defines him close to the dark corridor entrance without crossing it; reference images only preserve costume, face, pillar shadows, corridor placement, and candlelight direction.\n\n[Segment Context]\nThe young palace attendant continues from the previous tense pause, hears a faint disturbance from the right rear corridor, suppresses fear, turns back, and ends near the corridor entrance while staying alert and physically restrained.\n\n[Shot Storyboard]\nShot 1: an eye-level medium shot carries over from the center-left hall position; the character holds still, shoulders tightening as his gaze shifts toward the right rear darkness while the pillars and floor reflections stay readable.\nShot 2: the camera keeps the same axis with a restrained drift; he subtly steps back in place, approaches the corridor entrance without crossing it, and stabilizes in a guarded look toward the dark passage.\n\n[Visual Style]\nCinematic realism with warm candlelight from screen right contrasting against the cool darkness of the corridor, eye-level medium framing, readable pillar shadows, floor reflections, muted color, and a grounded suspenseful atmosphere.\n\n[Continuity And End State]\nThe character face, body shape, attendant costume, hall layout, corridor direction, candlelight direction, and relative positions of pillars and floor reflections remain unchanged; the final frame holds him before the corridor entrance looking back into darkness.\n\n[Constraints And Avoidances]\nNo background music, subtitles, watermarks, garbled text, face swap, costume change, axis jump, spatial jump, lighting direction jump, collage layout, extra limbs, or unreadable text.";
const LONG_EN_MOTION: &str = "[Camera And Action]\nUse an eye-level medium shot with a restrained slow drift that never crosses the axis. The young palace attendant first pauses at the center-left of the hall, then makes a small backward shift, slowly turns toward the right rear corridor, tightens his shoulders, and controls his fear while the movement remains slow and readable.\n\n[Dialogue And Performance]\nThe character quietly says \"谁在那儿\" in the original Chinese line, with short lip movement, shallow breathing, guarded eyes, and a frightened but controlled expression; the spoken line must not become subtitles.";

fn prompt_engine() -> TeraPromptEngine {
    let prompts_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../prompts");
    TeraPromptEngine::from_directory(&prompts_dir).expect("prompts should load")
}

fn provider_prompt_image_binding(
    prompt: &chapter_video_prompt::Model,
    boundary_frame_ids: &BoundaryFrameImageIds,
) -> ProviderPromptImageBinding {
    ProviderPromptImageBinding {
        has_first_frame_image: boundary_frame_ids.first.is_some(),
        has_last_frame_image: boundary_frame_ids.last.is_some(),
        reference_image_count: media::reference_image_ids(prompt.reference_image_ids_json.as_ref())
            .expect("test prompt reference ids should be valid")
            .len(),
        has_storyboard_reference_image: false,
    }
}

fn build_video_generation_provider_prompt(
    prompt: &chapter_video_prompt::Model,
    boundary_frame_ids: &BoundaryFrameImageIds,
) -> Result<String, String> {
    let engine = prompt_engine();
    super::build_video_generation_provider_prompt_with_binding(
        &engine,
        prompt,
        provider_prompt_image_binding(prompt, boundary_frame_ids),
    )
}

fn build_video_generation_provider_preview_prompt(
    prompt: &chapter_video_prompt::Model,
    boundary_frame_ids: &BoundaryFrameImageIds,
) -> Result<String, String> {
    let engine = prompt_engine();
    super::build_video_generation_provider_preview_prompt_with_binding(
        &engine,
        prompt,
        provider_prompt_image_binding(prompt, boundary_frame_ids),
    )
}

#[test]
fn newapi_first_frame_image_accepts_png_and_jpeg_data_urls() {
    ensure_newapi_first_frame_image_supported(Some("data:image/png;base64,abc"))
        .expect("png should be accepted");
    ensure_newapi_first_frame_image_supported(Some("data:image/jpeg;base64,abc"))
        .expect("jpeg should be accepted");
    ensure_newapi_first_frame_image_supported(Some("data:image/jpg;base64,abc"))
        .expect("jpg mime should be accepted");
}

#[test]
fn newapi_first_frame_image_rejects_unsupported_data_url_mime() {
    let err = ensure_newapi_first_frame_image_supported(Some("data:image/webp;base64,abc"))
        .expect_err("webp should be rejected for New API video image");

    assert!(err.contains("New API 视频首帧 image 仅支持 PNG/JPEG"));
    assert!(err.contains("image/webp"));
    assert!(err.contains("不能放进 New API 的首帧 image 字段"));
}

#[test]
fn newapi_first_frame_image_rejects_malformed_data_url() {
    let err = ensure_newapi_first_frame_image_supported(Some("data:image/png,abc"))
        .expect_err("malformed data url should be rejected");

    assert!(err.contains("data URL 格式无效"));
}

#[test]
fn provider_image_transport_matches_backend_media_contract() {
    assert_eq!(
        provider_image_input_transport_with_override(GEMINI_VEO_VIDEO_BACKEND, None).unwrap(),
        ProviderImageInputTransport::InlineDataUrl
    );
    assert_eq!(
        provider_image_input_transport_with_override(NEWAPI_VIDEO_BACKEND, None).unwrap(),
        ProviderImageInputTransport::InlineDataUrl
    );
    assert_eq!(
        provider_image_input_transport_with_override(OPENAI_COMPATIBLE_VIDEO_BACKEND, None)
            .unwrap(),
        ProviderImageInputTransport::InlineDataUrl
    );
    assert_eq!(
        provider_default_image_input_transport_with_remote(CLIPROXY_OPENAI_VIDEO_BACKEND, false),
        ProviderImageInputTransport::PublicUrl
    );
    assert_eq!(
        provider_default_image_input_transport_with_remote(CLIPROXY_OPENAI_VIDEO_BACKEND, true),
        ProviderImageInputTransport::RemoteTempUpload
    );
    assert_eq!(
        provider_image_input_transport_with_override(
            OPENAI_COMPATIBLE_VIDEO_BACKEND,
            Some("public_url")
        )
        .unwrap(),
        ProviderImageInputTransport::PublicUrl
    );
    assert_eq!(
        provider_image_input_transport_with_override(
            OPENAI_COMPATIBLE_VIDEO_BACKEND,
            Some("remote_temp_upload")
        )
        .unwrap(),
        ProviderImageInputTransport::RemoteTempUpload
    );
    let err = provider_image_input_transport_with_override(
        OPENAI_COMPATIBLE_VIDEO_BACKEND,
        Some("bad-mode"),
    )
    .expect_err("invalid override should be rejected");
    assert!(err.contains("A4N_VIDEO_IMAGE_INPUT_TRANSPORT 无效"));
    assert!(err.contains("inline_data_url"));
    assert!(err.contains("public_url"));
    assert!(err.contains("remote_temp_upload"));
    assert!(err.contains("auto"));

    let err = provider_image_input_transport_with_override(
        CLIPROXY_OPENAI_VIDEO_BACKEND,
        Some("inline_data_url"),
    )
    .expect_err("cliproxy video cannot accept inline image payloads");
    assert!(err.contains("cliproxy-openai-video 不支持 base64/data URL"));
}

#[test]
fn provider_generation_workflow_rejects_chat_video_as_unsupported_not_invalid() {
    let mut prompt = prompt_with_metadata();
    prompt.workflow_mode = "chat_video".to_string();

    let err = ensure_provider_generation_workflow(&prompt)
        .expect_err("chat_video is valid but cannot use app provider generation");

    assert!(err.contains("Chat 生视频模式"));
    assert!(err.contains("不使用应用内视频 provider 生成"));
    assert!(!err.contains("workflow_mode 无效"));
}

#[test]
fn prompt_is_first_frame_chain_accepts_only_provider_generation_workflows() {
    let mut prompt = prompt_with_metadata();
    assert!(
        !prompt_is_first_frame_chain(&prompt)
            .expect("boundary frame prompts are provider-generatable")
    );

    prompt.workflow_mode = "first_frame_chain".to_string();
    assert!(
        prompt_is_first_frame_chain(&prompt)
            .expect("first-frame-chain prompts are provider-generatable")
    );

    prompt.workflow_mode = "chat_video".to_string();
    let err = prompt_is_first_frame_chain(&prompt)
        .expect_err("chat_video cannot be mapped to provider frame mode");
    assert!(err.contains("Chat 生视频模式"));
}

#[test]
fn provider_generation_workflow_accepts_storyboard_as_reference_mode_workflow() {
    let mut prompt = prompt_with_metadata();
    prompt.workflow_mode = "storyboard".to_string();

    ensure_provider_generation_workflow(&prompt).expect("storyboard can use provider generation");
    assert_eq!(
        prompt_provider_workflow(&prompt).expect("storyboard should map to provider workflow"),
        ProviderGenerationWorkflow::Storyboard
    );
    assert!(!prompt_is_first_frame_chain(&prompt).expect("storyboard is provider-generatable"));
}

#[test]
fn newapi_rejects_boundary_frame_mode_instead_of_dropping_tail_frame() {
    let mut note = None;
    let err = enforce_provider_image_binding_contract(
        NEWAPI_VIDEO_BACKEND,
        ProviderGenerationWorkflow::BoundaryFrames,
        CliproxyGrokReferenceMode::Reference,
        0,
        VideoProviderRequestPurpose::Generate,
        &mut note,
    )
    .expect_err("newapi must not silently drop boundary tail frame input");

    assert!(err.contains("只有一个首帧图片输入槽"));
    assert!(err.contains("不能用于首尾帧模式"));
    assert!(err.contains("尾帧图片会被静默丢弃"));
}

#[test]
fn newapi_rejects_storyboard_mode_because_reference_images_are_required() {
    let mut note = None;
    let err = enforce_provider_image_binding_contract(
        NEWAPI_VIDEO_BACKEND,
        ProviderGenerationWorkflow::Storyboard,
        CliproxyGrokReferenceMode::Reference,
        0,
        VideoProviderRequestPurpose::Generate,
        &mut note,
    )
    .expect_err("newapi must not silently drop the storyboard reference image");

    assert!(err.contains("New API 视频接口不支持 reference_images"));
    assert!(err.contains("不能用于分镜模式"));
    assert!(note.is_none());
}

#[test]
fn cliproxy_reference_mode_allows_boundary_frame_mode_as_reference_images() {
    let mut note = None;

    enforce_provider_image_binding_contract(
        CLIPROXY_OPENAI_VIDEO_BACKEND,
        ProviderGenerationWorkflow::BoundaryFrames,
        CliproxyGrokReferenceMode::Reference,
        0,
        VideoProviderRequestPurpose::Generate,
        &mut note,
    )
    .expect("cliproxy reference mode should send boundary images as ordinary references");

    assert!(note.is_none());
}

#[test]
fn cliproxy_reference_mode_allows_storyboard_mode_as_reference_images() {
    let mut note = None;

    enforce_provider_image_binding_contract(
        CLIPROXY_OPENAI_VIDEO_BACKEND,
        ProviderGenerationWorkflow::Storyboard,
        CliproxyGrokReferenceMode::Reference,
        1,
        VideoProviderRequestPurpose::Generate,
        &mut note,
    )
    .expect("cliproxy reference mode should send storyboard image as an ordinary reference");

    assert!(note.is_none());
}

#[test]
fn cliproxy_first_frame_mode_rejects_boundary_frame_mode_instead_of_dropping_tail_frame() {
    let mut note = None;
    let err = enforce_provider_image_binding_contract(
        CLIPROXY_OPENAI_VIDEO_BACKEND,
        ProviderGenerationWorkflow::BoundaryFrames,
        CliproxyGrokReferenceMode::FirstFrame,
        0,
        VideoProviderRequestPurpose::Generate,
        &mut note,
    )
    .expect_err("cliproxy first-frame mode must not silently drop boundary tail frame input");

    assert!(err.contains("CLIProxy / Grok 视频接口"));
    assert!(err.contains("首帧模式"));
    assert!(err.contains("不能用于首尾帧模式"));
    assert!(err.contains("尾帧图片会被静默丢弃"));
    assert!(note.is_none());
}

#[test]
fn cliproxy_first_frame_mode_rejects_storyboard_mode_instead_of_treating_it_as_first_frame() {
    let mut note = None;
    let err = enforce_provider_image_binding_contract(
        CLIPROXY_OPENAI_VIDEO_BACKEND,
        ProviderGenerationWorkflow::Storyboard,
        CliproxyGrokReferenceMode::FirstFrame,
        1,
        VideoProviderRequestPurpose::Generate,
        &mut note,
    )
    .expect_err("cliproxy first-frame mode must not treat storyboard image as input_reference");

    assert!(err.contains("CLIProxy / Grok 视频接口"));
    assert!(err.contains("首帧模式"));
    assert!(err.contains("不能用于分镜模式"));
    assert!(note.is_none());
}

#[test]
fn newapi_rejects_material_reference_images_instead_of_dropping_them() {
    let mut note = None;
    let err = enforce_provider_image_binding_contract(
        NEWAPI_VIDEO_BACKEND,
        ProviderGenerationWorkflow::FirstFrameChain,
        CliproxyGrokReferenceMode::Reference,
        2,
        VideoProviderRequestPurpose::Generate,
        &mut note,
    )
    .expect_err("newapi must not silently drop material reference images");

    assert!(err.contains("不支持提示词参考图输入"));
    assert!(err.contains("绑定了 2 张参考图"));
    assert!(err.contains("已阻止生成以避免参考图被丢弃"));
}

#[test]
fn reference_ids_with_storyboard_image_keeps_storyboard_first_and_deduplicates() {
    let ids = reference_ids_with_storyboard_image(Some(10), &[10, 11, 0, 12, 11]);

    assert_eq!(ids, vec![10, 11, 12]);
}

#[test]
fn newapi_preview_reports_unsupported_image_bindings_without_blocking() {
    let mut note = None;

    enforce_provider_image_binding_contract(
        NEWAPI_VIDEO_BACKEND,
        ProviderGenerationWorkflow::BoundaryFrames,
        CliproxyGrokReferenceMode::Reference,
        1,
        VideoProviderRequestPurpose::Preview,
        &mut note,
    )
    .expect("preview should report but not block unsupported newapi bindings");

    let note = note.expect("preview note should describe unsupported bindings");
    assert!(note.contains("不能用于首尾帧模式"));
    assert!(note.contains("不支持提示词参考图输入"));
    assert!(note.contains("当前仅展示请求结构"));
}

#[test]
fn cliproxy_rejects_first_frame_plus_material_refs_instead_of_dropping_refs() {
    let mut note = None;
    let err = enforce_cliproxy_video_media_contract(
        CLIPROXY_OPENAI_VIDEO_BACKEND,
        "grok-imagine-video",
        CliproxyGrokReferenceMode::Reference,
        2,
        true,
        6,
        VideoProviderRequestPurpose::Generate,
        &mut note,
    )
    .expect_err("cliproxy must not silently drop refs when input_reference is used");

    assert!(err.contains("不能同时发送 reference_images"));
    assert!(err.contains("绑定了 2 张参考图"));
    assert!(note.is_none());
}

#[test]
fn cliproxy_preview_warns_first_frame_plus_material_refs_without_blocking() {
    let mut note = None;
    enforce_cliproxy_video_media_contract(
        CLIPROXY_OPENAI_VIDEO_BACKEND,
        "grok-imagine-video",
        CliproxyGrokReferenceMode::Reference,
        2,
        true,
        6,
        VideoProviderRequestPurpose::Preview,
        &mut note,
    )
    .expect("preview should report unsupported cliproxy media combination");

    let note = note.expect("preview note should describe dropped refs");
    assert!(note.contains("不能同时发送 reference_images"));
    assert!(note.contains("当前仅展示请求结构"));
}

#[test]
fn cliproxy_image_to_video_only_models_reject_reference_images_before_provider_call() {
    for model in [
        "grok-imagine-video-1.5",
        "grok-imagine-video-1.5-preview",
        "grok-imagine-video-1.5-2026-05-30",
    ] {
        let mut note = None;
        let err = enforce_cliproxy_video_media_contract(
            CLIPROXY_OPENAI_VIDEO_BACKEND,
            model,
            CliproxyGrokReferenceMode::Reference,
            1,
            false,
            6,
            VideoProviderRequestPurpose::Generate,
            &mut note,
        )
        .expect_err("grok 1.5 models must not send unsupported reference_images");

        assert!(err.contains(model));
        assert!(err.contains("不支持 reference_images"));
        assert!(err.contains("Grok 首帧模式"));
        assert!(note.is_none());
    }
}

#[test]
fn reference_image_ids_rejects_invalid_reference_image_ids_instead_of_dropping_them() {
    let err = media::reference_image_ids(Some(&json!([11, "12"])))
        .expect_err("string ids must not be silently ignored");

    assert!(err.contains("reference_image_ids[1] 必须是正整数"));
}

#[test]
fn reference_image_ids_rejects_duplicate_reference_image_ids() {
    let err = media::reference_image_ids(Some(&json!([11, 11])))
        .expect_err("duplicate ids must not be silently collapsed");

    assert!(err.contains("reference_image_ids 包含重复 ID：11"));
}

#[test]
fn boundary_frame_missing_image_message_requires_both_boundary_images() {
    let mut row = prompt_with_metadata();
    row.shot_number = 3;

    let message = boundary_frame_missing_image_message(
        &row,
        &BoundaryFrameImageIds {
            first: Some(31),
            last: None,
        },
    )
    .expect("missing tail frame should be reported");

    assert!(message.contains("首尾帧模式第 3 段"));
    assert!(message.contains("第 4 张尾帧边界图"));
    assert!(message.contains("请先在首尾帧生成里生成对应边界帧图片"));
    assert!(
        boundary_frame_missing_image_message(
            &row,
            &BoundaryFrameImageIds {
                first: Some(31),
                last: Some(32),
            },
        )
        .is_none()
    );
}

#[test]
fn cliproxy_reference_mode_reference_ids_include_boundary_frames_first() {
    let ids = reference_ids_with_boundary_frame_images(
        &BoundaryFrameImageIds {
            first: Some(88),
            last: Some(90),
        },
        &[59, 88, 80],
    );

    assert_eq!(ids, vec![88, 90, 59, 80]);
}

#[test]
fn chapter_video_provider_snapshot_uses_image_placeholders() {
    let prompt = prompt_with_metadata();
    let cfg = video_config(GEMINI_VEO_VIDEO_BACKEND);
    let context = ChapterVideoProviderRequestContext {
        prompt,
        cfg,
        request: VideoGenerationRequest {
            prompt: "完整 provider 文本提示词".to_string(),
            negative_prompt: Some("低清晰度".to_string()),
            width: 1280,
            height: 720,
            duration_secs: 4,
            fps: 24,
            model: "test-video-model".to_string(),
            first_frame_image_url: Some("data:image/png;base64,REAL_FIRST_FRAME".to_string()),
            last_frame_image_url: Some("data:image/png;base64,REAL_LAST_FRAME".to_string()),
            reference_image_urls: vec!["data:image/png;base64,REAL_REFERENCE_IMAGE".to_string()],
            reference_media: None,
        },
        prompt_reference_image_ids: Some(json!([11])),
        provider_reference_image_ids: Some(json!([11])),
        prompt_source: None,
        workflow_mode: "boundary_frames".to_string(),
        source_prompt_type: Some("frame_segment_plan".to_string()),
        boundary_frame_ids: BoundaryFrameImageIds {
            first: Some(31),
            last: Some(32),
        },
        prompt_reference_image_count: 1,
        provider_reference_image_count: 1,
        first_frame_image_url_available: true,
        last_frame_image_url_available: true,
        provider_input_note: None,
    };

    let snapshot = chapter_video_provider_request_snapshot(&context)
        .expect("chapter video snapshot should render");
    let request = snapshot
        .get("request")
        .expect("snapshot should include provider request");
    let serialized = request.to_string();

    assert!(serialized.contains("完整 provider 文本提示词"));
    assert!(serialized.contains("generated_image #31"));
    assert!(serialized.contains("generated_image #32"));
    assert!(serialized.contains("generated_image #11"));
    assert!(!serialized.contains("REAL_FIRST_FRAME"));
    assert!(!serialized.contains("REAL_LAST_FRAME"));
    assert!(!serialized.contains("REAL_REFERENCE_IMAGE"));
}

fn prompt_with_metadata() -> chapter_video_prompt::Model {
    let now = Utc::now().naive_utc();
    chapter_video_prompt::Model {
        id: 7,
        project_id: 1,
        chapter_id: 2,
        style: Some("电影感写实".to_string()),
        prompt_type: "video_prompt".to_string(),
        workflow_mode: "boundary_frames".to_string(),
        shot_number: 1,
        title: Some("回望暗廊".to_string()),
        scene_summary: Some("小宦官回望暗处，压住恐惧".to_string()),
        visual_prompt: LONG_VISUAL.to_string(),
        motion_prompt: Some(LONG_MOTION.to_string()),
        negative_prompt: Some(
            "低清晰度、画面模糊、人体结构扭曲、多余肢体、面部不一致、空间跳变、光影不连续"
                .to_string(),
        ),
        duration_secs: 4,
        aspect_ratio: Some("16:9".to_string()),
        reference_image_ids_json: Some(json!([11])),
        status: "pending".to_string(),
        generated_video_id: None,
        metadata_json: Some(json!({
            "shot": {
                "title": "回望暗廊",
                "scene_summary": "小宦官回望暗处，压住恐惧",
                "blocking_plan": {"start": "殿内中央偏左", "movement": "原地后缩回望", "end": "靠近暗廊入口但不越过"},
                "lighting_plan": {"key_light": "烛光", "direction": "画面右侧", "continuity": "继承首尾帧"},
                "emotion_action_beats": [{"character": "小宦官", "emotion": "恐惧", "visible_action": "肩膀收紧并回头"}],
                "cinematography_plan": {"shot_size": "中景", "camera_angle": "平视", "camera_movement": "轻微手持漂移", "axis_rule": "暗廊保持右后方", "lens_hint": "标准镜头"},
                "temporal_beats": [{"start_sec": 0, "end_sec": 4, "camera": "中景轻微漂移", "subject_action": "回头并靠近暗廊入口", "continuity_target": "抵达尾帧"}],
                "reference_binding": {"bindings": [{"image_id": 11, "entity": "小宦官", "role": "character", "usage": "保持脸和服装一致", "must_keep": "同一人物"}], "coverage_check": "参考图 11 已绑定"},
                "continuity_check": {"blocking": "连续", "lighting": "连续", "emotion": "已动作化", "references": "已覆盖", "revision": "无需修订"}
            },
            "dialogue_beats": [
                {"speaker": "小宦官", "line": "谁在那儿", "delivery": "压低声音试探", "visible_action": "嘴唇微动并回头", "listener_reaction": "暗廊无人回应", "timing": "中段"}
            ],
            "start_boundary_frame": {
                "title": "殿内首帧",
                "scene_summary": "小宦官站在殿内中央偏左",
                "frame_prompt": "小宦官站在昏暗殿内中央偏左，看向右后方暗角，烛光从画面右侧斜照。",
                "blocking_state": {"characters": [{"name": "小宦官", "position": "中央偏左"}]},
                "lighting_state": {"key_light": "烛光", "direction": "画面右侧"}
            },
            "end_boundary_frame": {
                "title": "暗廊尾帧",
                "scene_summary": "小宦官靠近暗廊入口并回望",
                "frame_prompt": "小宦官靠近暗廊入口但没有越过，回望右后方黑暗走廊。",
                "blocking_state": {"characters": [{"name": "小宦官", "position": "暗廊入口前"}]},
                "lighting_state": {"key_light": "烛光", "direction": "画面右侧"}
            }
        })),
        created_at: now,
        updated_at: now,
    }
}

fn reference_media_video_with_metadata(metadata_json: Option<Value>) -> generated_video::Model {
    let now = Utc::now().naive_utc();
    generated_video::Model {
        id: 42,
        project_id: 1,
        chapter_id: Some(2),
        video_type: "reference_media_video".to_string(),
        workflow_mode: Some("reference_media".to_string()),
        file_path: None,
        thumbnail_path: None,
        prompt: "这是一段足够长的参考媒体视频提示词，用于描述人物外观、场景结构、镜头调度、动作节奏、光影方向、情绪变化和画面风格；其中明确使用 @image_file_1 作为主角造型参考，并使用 @video_file_1 作为动作节奏参考。".to_string(),
        negative_prompt: None,
        model: "seedance-2-0-pro".to_string(),
        width: 1280,
        height: 720,
        duration_secs: 8.0,
        fps: 24,
        file_size: None,
        source_video_id: None,
        source_image_id: None,
        status: "completed".to_string(),
        provider_job_id: None,
        error_message: None,
        metadata_json,
        is_favorite: false,
        created_at: now,
        updated_at: now,
    }
}

#[test]
fn linked_video_preview_config_id_uses_positive_config_id() {
    let prompt = prompt_with_metadata();
    let mut video = reference_media_video_with_metadata(Some(json!({ "config_id": 9 })));
    video.video_type = "chapter_video".to_string();
    video.workflow_mode = Some("boundary_frames".to_string());

    assert_eq!(
        linked_video_preview_config_id(&prompt, &video)
            .expect("positive config_id should be reused"),
        Some(9)
    );
}

#[test]
fn linked_video_preview_config_id_allows_missing_config_id() {
    let prompt = prompt_with_metadata();
    let mut video = reference_media_video_with_metadata(Some(json!({
        "provider_backend": "external-import"
    })));
    video.video_type = "chapter_video".to_string();
    video.workflow_mode = Some("boundary_frames".to_string());

    assert_eq!(
        linked_video_preview_config_id(&prompt, &video)
            .expect("external imports may not have provider config_id"),
        None
    );
}

#[test]
fn linked_video_preview_config_id_rejects_malformed_config_id() {
    let prompt = prompt_with_metadata();
    let mut video = reference_media_video_with_metadata(Some(json!({ "config_id": "9" })));
    video.video_type = "chapter_video".to_string();
    video.workflow_mode = Some("boundary_frames".to_string());

    let err = linked_video_preview_config_id(&prompt, &video)
        .expect_err("string config_id must not fall back to default config");

    assert!(err.contains("metadata_json.config_id 必须是正整数"));

    video.metadata_json = Some(json!({ "config_id": 0 }));
    let err = linked_video_preview_config_id(&prompt, &video)
        .expect_err("non-positive config_id must not fall back to default config");

    assert!(err.contains("metadata_json.config_id 必须是正整数"));
}

fn video_config(backend: &str) -> video_gen_config::Model {
    let now = Utc::now().naive_utc();
    video_gen_config::Model {
        id: 9,
        name: "test video config".to_string(),
        backend: backend.to_string(),
        base_url: Some("https://example.test".to_string()),
        api_key_encrypted: Some("test-key".to_string()),
        model: "test-video-model".to_string(),
        max_concurrent: 1,
        default_width: 1280,
        default_height: 720,
        default_duration_secs: 4,
        min_duration_secs: 1,
        max_duration_secs: 60,
        duration_mode: "range".to_string(),
        supported_durations_json: None,
        default_fps: 24,
        provider_options_json: None,
        is_default: true,
        created_at: now,
        updated_at: now,
    }
}

fn provider_preview_context(backend: &str) -> ChapterVideoProviderRequestContext {
    ChapterVideoProviderRequestContext {
        prompt: prompt_with_metadata(),
        cfg: video_config(backend),
        request: VideoGenerationRequest {
            prompt: "导演提示词正文".to_string(),
            negative_prompt: Some("低清晰度".to_string()),
            width: 1280,
            height: 720,
            duration_secs: 4,
            fps: 24,
            model: "test-video-model".to_string(),
            first_frame_image_url: Some("https://example.invalid/a4n-preview/first".into()),
            last_frame_image_url: Some("https://example.invalid/a4n-preview/last".into()),
            reference_image_urls: vec!["https://example.invalid/a4n-preview/ref".into()],
            reference_media: None,
        },
        prompt_reference_image_ids: Some(json!([11])),
        provider_reference_image_ids: Some(json!([11])),
        prompt_source: None,
        workflow_mode: "boundary_frames".to_string(),
        source_prompt_type: Some("frame_segment_plan".to_string()),
        boundary_frame_ids: BoundaryFrameImageIds {
            first: Some(21),
            last: Some(22),
        },
        prompt_reference_image_count: 1,
        provider_reference_image_count: 1,
        first_frame_image_url_available: true,
        last_frame_image_url_available: true,
        provider_input_note: None,
    }
}

#[test]
fn cliproxy_preview_rejects_invalid_grok_reference_mode() {
    let mut context = provider_preview_context(CLIPROXY_OPENAI_VIDEO_BACKEND);
    context.cfg.provider_options_json = Some(json!({
        "grok_reference_mode": "invalid-mode"
    }));

    let err = video_provider_preview_payload(&context)
        .expect_err("invalid grok_reference_mode must not silently fall back");

    assert!(err.contains("grok_reference_mode 无效"));
}

#[test]
fn cliproxy_reference_preview_sends_reference_images_without_input_reference() {
    let mut context = provider_preview_context(CLIPROXY_OPENAI_VIDEO_BACKEND);
    context.request.first_frame_image_url = None;
    context.request.last_frame_image_url = None;
    context.request.reference_image_urls = vec![
        "https://example.invalid/a4n-preview/ref-1".into(),
        "https://example.invalid/a4n-preview/ref-2".into(),
    ];
    context.prompt_reference_image_ids = Some(json!([11, 12]));
    context.provider_reference_image_ids = Some(json!([11, 12]));
    context.prompt_reference_image_count = 2;
    context.provider_reference_image_count = 2;
    context.first_frame_image_url_available = false;
    context.last_frame_image_url_available = false;

    let (_kind, _title, note, request) =
        video_provider_preview_payload(&context).expect("cliproxy reference preview should render");

    assert!(note.contains("所有输入图片均作为 reference_images 发送"));
    assert!(note.contains("不会发送 input_reference"));
    assert_eq!(
        request["reference_images"][0]["url"],
        "https://example.invalid/a4n-preview/ref-1"
    );
    assert_eq!(
        request["reference_images"][1]["url"],
        "https://example.invalid/a4n-preview/ref-2"
    );
    assert!(request.get("input_reference").is_none());
}

#[test]
fn canvas_cliproxy_reference_mode_maps_frame_inputs_to_reference_ids() {
    let mut cfg = video_config(CLIPROXY_OPENAI_VIDEO_BACKEND);
    cfg.provider_options_json = Some(json!({
        "grok_reference_mode": "reference",
    }));
    let input = GenerateCanvasVideoInput {
        user_id: 1,
        project_id: 2,
        graph_id: 3,
        node_id: 4,
        workflow_run_id: None,
        prompt: "画布提示词".to_string(),
        negative_prompt: None,
        config_id: Some(cfg.id),
        width: None,
        height: None,
        duration_secs: None,
        fps: None,
        first_frame_image_id: Some(21),
        last_frame_image_id: Some(22),
        reference_image_ids: vec![31, 21, 32],
        source_node_ids: vec![9],
    };
    let mut note = None;

    let binding = canvas_video_provider_image_binding(&cfg, &input, &mut note)
        .expect("cliproxy reference mode should normalize canvas image inputs");

    assert_eq!(binding.first_frame_image_id, None);
    assert_eq!(binding.last_frame_image_id, None);
    assert_eq!(binding.reference_image_ids, vec![21, 22, 31, 32]);
    let note = note.expect("normalization should explain frame-to-reference mapping");
    assert!(note.contains("参考模式"));
    assert!(note.contains("21, 22, 31, 32"));
}

#[test]
fn canvas_cliproxy_first_frame_mode_keeps_first_frame_slot() {
    let mut cfg = video_config(CLIPROXY_OPENAI_VIDEO_BACKEND);
    cfg.provider_options_json = Some(json!({
        "grok_reference_mode": "first_frame",
    }));
    let input = GenerateCanvasVideoInput {
        user_id: 1,
        project_id: 2,
        graph_id: 3,
        node_id: 4,
        workflow_run_id: None,
        prompt: "画布提示词".to_string(),
        negative_prompt: None,
        config_id: Some(cfg.id),
        width: None,
        height: None,
        duration_secs: None,
        fps: None,
        first_frame_image_id: Some(21),
        last_frame_image_id: None,
        reference_image_ids: vec![],
        source_node_ids: vec![],
    };
    let mut note = None;

    let binding = canvas_video_provider_image_binding(&cfg, &input, &mut note)
        .expect("cliproxy first-frame mode should preserve the first-frame input slot");

    assert_eq!(binding.first_frame_image_id, Some(21));
    assert_eq!(binding.last_frame_image_id, None);
    assert!(binding.reference_image_ids.is_empty());
    assert!(note.is_none());
}

#[test]
fn provider_prompt_keeps_video_request_text_compact() {
    let prompt = build_video_generation_provider_prompt(
        &prompt_with_metadata(),
        &BoundaryFrameImageIds {
            first: Some(21),
            last: Some(22),
        },
    )
    .expect("complete prompt should compile");

    assert!(prompt.contains("【输入图片顺序】"));
    assert!(prompt.contains("【视频生成主提示词】"));
    assert!(prompt.contains("【动作与运镜补充】"));
    assert!(prompt.contains("[主体定义与素材绑定]"));
    assert!(prompt.contains("[限制与避免项]"));
    assert!(prompt.contains("首帧输入图片是视频第 0 秒画面"));
    assert!(prompt.contains("尾帧输入图片是视频结尾目标"));
    assert!(prompt.contains("本请求按顺序输入 1 张参考图"));
    assert!(prompt.contains("[对白与表演]"));
    assert!(prompt.contains("小宦官压低声音说“谁在那儿”"));
    assert!(!prompt.contains("[连续性要求]"));
    assert!(!prompt.contains("[Continuity Requirements]"));
    assert!(!prompt.contains("片段标题："));
    assert!(!prompt.contains("剧情摘要："));
    assert!(!prompt.contains("时长与画幅："));
    assert!(!prompt.contains("【全局限制】"));
    assert!(!prompt.contains("负面约束："));
    assert!(!prompt.contains("主体/素材："));
    assert!(!prompt.contains("素材参考图只用于"));
    assert!(!prompt.contains("低清晰度"));
    assert!(!prompt.contains("不是首帧输入"));
    assert!(!prompt.contains("参考图 #11 不能作为视频首帧"));
    assert!(!prompt.contains("不能把尾帧塞进首帧"));
    assert!(!prompt.contains("单个 `image` 字段不是通用参考图容器"));
    assert!(!prompt.contains("generated_image #"));
    assert!(!prompt.contains("当前 provider"));
    assert!(!prompt.contains("【片段独立生成上下文】"));
    assert!(!prompt.contains("【首尾状态与转场】"));
    assert!(!prompt.contains("【首尾帧状态】"));
    assert!(!prompt.contains("【时间段节拍】"));
    assert!(!prompt.contains("temporal_beats"));
    assert!(!prompt.contains("start_sec"));
    assert!(!prompt.contains("end_sec"));
    assert!(!prompt.contains("参考图 11 已绑定"));
    assert!(!prompt.contains("{"));
    assert!(!prompt.contains("}"));
    assert!(!prompt.contains("\"start\""));
    assert!(!prompt.contains("\"blocking_plan\""));
}

#[test]
fn provider_prompt_rejects_deprecated_motion_continuity_section() {
    let mut row = prompt_with_metadata();
    row.motion_prompt = Some("[运镜与动作]\n镜头保持平视中景轻微手持漂移，小宦官先停住听向暗角，再缓慢回头并轻微后缩，动作节奏压低且不跳轴。\n\n[对白与表演]\n小宦官压低声音说“谁在那儿”，嘴唇短促微动，随后屏住呼吸，眼神仍锁向黑暗处。\n\n[连续性要求]\n这是旧版重复章节，应该由 visual_prompt 的连续性与结尾状态承担。".to_string());

    let err = build_video_generation_provider_prompt(
        &row,
        &BoundaryFrameImageIds {
            first: Some(21),
            last: Some(22),
        },
    )
    .expect_err("true provider prompt must reject deprecated motion continuity sections");

    assert!(err.contains("motion_prompt 使用旧版视频提示词章节 [连续性要求]"));
    assert!(err.contains("重新生成或优化为当前格式"));
}

#[test]
fn cliproxy_prompt_length_preflight_rejects_generate_over_char_limit() {
    let prompt = "镜".repeat(MAX_CLIPROXY_VIDEO_PROMPT_CHARS + 1);
    let mut note = None;
    let err = enforce_provider_prompt_length_contract(
        CLIPROXY_OPENAI_VIDEO_BACKEND,
        &prompt,
        VideoProviderRequestPurpose::Generate,
        &mut note,
    )
    .expect_err("generate should reject oversized CLIProxy prompts before provider call");

    assert!(err.contains("CLIProxy/Grok 视频 prompt 超过"));
    assert!(err.contains("字符限制"));
    assert!(note.is_none());
}

#[test]
fn cliproxy_prompt_length_preflight_warns_preview_over_char_limit() {
    let prompt = "镜".repeat(MAX_CLIPROXY_VIDEO_PROMPT_CHARS + 1);
    let mut note = None;
    enforce_provider_prompt_length_contract(
        CLIPROXY_OPENAI_VIDEO_BACKEND,
        &prompt,
        VideoProviderRequestPurpose::Preview,
        &mut note,
    )
    .expect("preview should report oversized CLIProxy prompts without blocking");

    let note = note.expect("preview note should include prompt length warning");
    assert!(note.contains("CLIProxy/Grok 视频 prompt 超过"));
    assert!(note.contains("字符限制"));
    assert!(note.contains("当前仅展示请求结构"));
}

#[test]
fn provider_prompt_rejects_duplicate_motion_temporal_storyboard_section() {
    let mut row = prompt_with_metadata();
    row.visual_prompt = "[主体定义与素材绑定]\n首帧定义开头画面。\n\n[片段内容以及前因后果]\n小宦官从殿内中央偏左承接上一状态，听见暗廊方向的异响后压住恐惧回望，结尾停在暗廊入口前。\n\n[镜头分镜]\n镜头 1：平视中景承接殿内中央偏左站位，小宦官停顿并看向右后方暗角，肩膀收紧。\n镜头 2：镜头保持同一轴线轻微漂移，他缓慢后缩并停在暗廊入口前，最后仍不越过入口。\n\n[画面风格描述]\n电影感写实，右侧暖烛光和暗廊冷阴影对比。\n\n[连续性与结尾状态]\n人物靠近暗廊入口但不越过，光源方向不变。\n\n[限制与避免项]\n不要配背景音乐，不要字幕、水印、换脸或空间跳变。".to_string();
    row.motion_prompt = Some("[镜头分镜]\n镜头 1：小宦官停顿观察暗角。\n镜头 2：小宦官后缩并停在暗廊入口前。\n\n[运镜与动作]\n镜头保持中景轻微手持漂移，不跳轴，暗廊始终位于画面右后方。\n\n[对白与表演]\n小宦官压低声音说“谁在那儿”，嘴唇微动。".to_string());

    let err = build_video_generation_provider_prompt(
        &row,
        &BoundaryFrameImageIds {
            first: Some(21),
            last: Some(22),
        },
    )
    .expect_err("provider prompt must reject duplicate motion timeline");

    assert!(err.contains("motion_prompt 不应重复镜头分镜"));
}

#[test]
fn provider_prompt_accepts_english_sections_when_prompt_language_is_english() {
    let mut row = prompt_with_metadata();
    row.visual_prompt = LONG_EN_VISUAL.to_string();
    row.motion_prompt = Some(LONG_EN_MOTION.to_string());
    row.metadata_json
        .as_mut()
        .and_then(Value::as_object_mut)
        .expect("test metadata should be an object")
        .insert("prompt_output_language".to_string(), json!("en"));

    let prompt = build_video_generation_provider_prompt(
        &row,
        &BoundaryFrameImageIds {
            first: Some(21),
            last: Some(22),
        },
    )
    .expect("English provider prompt should be validated with English sections");

    assert!(prompt.contains("[Subject And Asset Binding]"));
    assert!(prompt.contains("[Camera And Action]"));
    assert!(!prompt.contains("缺少必需章节 [主体定义与素材绑定]"));
}

#[test]
fn provider_prompt_rejects_english_sections_when_prompt_language_defaults_to_chinese() {
    let mut row = prompt_with_metadata();
    row.visual_prompt = LONG_EN_VISUAL.to_string();
    row.motion_prompt = Some(LONG_EN_MOTION.to_string());

    let err = build_video_generation_provider_prompt(
        &row,
        &BoundaryFrameImageIds {
            first: Some(21),
            last: Some(22),
        },
    )
    .expect_err("default Chinese prompt language should require Chinese sections");

    assert!(err.contains("visual_prompt 缺少必需章节 [主体定义与素材绑定]"));
}

#[test]
fn provider_prompt_rejects_stale_missing_input_image_status_with_bound_media() {
    let mut row = prompt_with_metadata();
    row.workflow_mode = "first_frame_chain".to_string();
    row.shot_number = 2;
    row.visual_prompt = "[主体定义与素材绑定]\n无输入首帧。本段仅依赖文字描述保持角色形象一致。\n\n[片段内容以及前因后果]\n角色承接上一段尾态继续阅读，读到关键句后抬头看向窗外。\n\n[镜头分镜]\n镜头 1：窗边中近景保持角色双手持信纸低头默读，下午暖光从窗侧落在脸部和桌面。\n镜头 2：镜头轻微推近，角色读到关键句后抬头看向窗外，手中信纸不离开画面。\n镜头 3：画面稳定在角色震惊与好奇交织的表情，视线锁定窗外并保留窗边光源方向。\n\n[画面风格描述]\n京阿尼动画风格，下午暖光，窗边中近景。\n\n[连续性与结尾状态]\n角色双手仍持信纸，视线锁定窗外，光源方向保持一致。\n\n[限制与避免项]\n不要配背景音乐，不要字幕、水印、换脸或空间跳变。".to_string();

    let err = build_video_generation_provider_prompt(
        &row,
        &BoundaryFrameImageIds {
            first: Some(31),
            last: None,
        },
    )
    .expect_err("provider prompt must reject stale upload status text");

    assert!(err.contains("visual_prompt 包含上传状态说明"));
    assert!(err.contains("无输入首帧"));
}

#[test]
fn provider_prompt_rejects_stale_missing_input_image_status_without_bound_media() {
    let mut row = prompt_with_metadata();
    row.reference_image_ids_json = None;
    row.visual_prompt = "[主体定义与素材绑定]\n无输入首帧。本段仅依赖文字描述保持角色形象一致。\n\n[片段内容以及前因后果]\n角色承接上一段尾态继续阅读，读到关键句后抬头看向窗外。\n\n[镜头分镜]\n镜头 1：窗边中近景保持角色双手持信纸低头默读，下午暖光从窗侧落在脸部和桌面。\n镜头 2：镜头轻微推近，角色读到关键句后抬头看向窗外，手中信纸不离开画面。\n镜头 3：画面稳定在角色震惊与好奇交织的表情，视线锁定窗外并保留窗边光源方向。\n\n[画面风格描述]\n京阿尼动画风格，下午暖光，窗边中近景。\n\n[连续性与结尾状态]\n角色双手仍持信纸，视线锁定窗外，光源方向保持一致。\n\n[限制与避免项]\n不要配背景音乐，不要字幕、水印、换脸或空间跳变。".to_string();

    let err = build_video_generation_provider_prompt(
        &row,
        &BoundaryFrameImageIds {
            first: None,
            last: None,
        },
    )
    .expect_err("provider prompt must reject stale upload status text");

    assert!(err.contains("visual_prompt 包含上传状态说明"));
    assert!(err.contains("无输入首帧"));
}

#[test]
fn provider_prompt_rejects_stale_tail_frame_upload_status() {
    let mut row = prompt_with_metadata();
    row.visual_prompt = "[主体定义与素材绑定]\n首帧输入图片定义角色站在窗边的开头画面；没有输入尾帧，本段结尾按文字描述生成。\n\n[片段内容以及前因后果]\n角色承接上一段尾态继续阅读，读到关键句后抬头看向窗外，表情从专注转为震惊与好奇。\n\n[镜头分镜]\n镜头 1：窗边中近景承接首帧站位，角色双手持信纸低头默读，柔和自然光从左侧窗户照入。\n镜头 2：镜头轻微推近，角色读到关键句后抬头看向窗外，发丝和校服细节保持稳定。\n镜头 3：画面稳定在角色震惊与好奇交织的表情，视线锁定窗外，桌椅和窗光方向不变。\n\n[画面风格描述]\n京阿尼动画风格，下午暖光，窗边中近景，柔和自然光从左侧窗户照入。\n\n[连续性与结尾状态]\n角色双手仍持信纸，视线锁定窗外，发型、校服、桌椅和光源方向保持一致。\n\n[限制与避免项]\n不要配背景音乐，不要字幕、水印、换脸或空间跳变。".to_string();

    let err = build_video_generation_provider_prompt(
        &row,
        &BoundaryFrameImageIds {
            first: Some(21),
            last: None,
        },
    )
    .expect_err("provider prompt must reject stale tail-frame upload status text");

    assert!(err.contains("visual_prompt 包含上传状态说明"));
    assert!(err.contains("没有输入尾帧"));
}

#[test]
fn chapter_video_negative_prompt_keeps_only_explicit_specific_terms() {
    let specific = provider_specific_negative_prompt(Some("手机屏幕反光过强"))
        .expect("specific negative prompt should be preserved");

    assert_eq!(specific, "手机屏幕反光过强");
    assert_eq!(provider_specific_negative_prompt(None), None);
    assert_eq!(
        provider_specific_negative_prompt(Some("  无特殊要求  ")),
        None
    );
}

#[test]
fn provider_prompt_rejects_short_visual_prompt() {
    let mut row = prompt_with_metadata();
    row.visual_prompt = "宽镜头，电影感".to_string();

    let err = build_video_generation_provider_prompt(
        &row,
        &BoundaryFrameImageIds {
            first: Some(21),
            last: Some(22),
        },
    )
    .expect_err("short provider prompt must be rejected");

    assert!(err.contains("visual_prompt 过短"));
}

#[test]
fn provider_prompt_rejects_missing_motion_prompt_without_empty_default() {
    let mut row = prompt_with_metadata();
    row.motion_prompt = None;

    let err = build_video_generation_provider_prompt(
        &row,
        &BoundaryFrameImageIds {
            first: Some(21),
            last: Some(22),
        },
    )
    .expect_err("true provider prompt must require motion prompt");

    assert!(err.contains("缺少 motion_prompt"));
}

#[test]
fn provider_prompt_rejects_dictionary_like_prompt_text() {
    let mut row = prompt_with_metadata();
    row.visual_prompt = format!(
        "{} {{\"blocking_plan\":\"站位数据不应该作为画面提示词发送\"}}",
        LONG_VISUAL
    );

    let err = build_video_generation_provider_prompt(
        &row,
        &BoundaryFrameImageIds {
            first: Some(21),
            last: Some(22),
        },
    )
    .expect_err("dictionary-like prompt text should be rejected");

    assert!(err.contains("visual_prompt 包含 JSON/字典结构片段"));
}

#[test]
fn provider_prompt_rejects_internal_image_id_text_after_manual_edit() {
    let mut row = prompt_with_metadata();
    row.visual_prompt = format!("{LONG_VISUAL}\n参考图 image_id=11 用于人物一致性。");

    let err = build_video_generation_provider_prompt(
        &row,
        &BoundaryFrameImageIds {
            first: Some(21),
            last: Some(22),
        },
    )
    .expect_err("provider prompt must reject internal image ids in editable text");

    assert!(err.contains("visual_prompt"));
    assert!(err.contains("内部图片 ID"));
}

#[test]
fn provider_prompt_rejects_internal_workflow_prompt_text() {
    let mut row = prompt_with_metadata();
    row.visual_prompt = format!(
        "{} 这是首帧串联模式的单段视频，起始帧来自上一段尾帧自动采集。",
        LONG_VISUAL
    );

    let err = build_video_generation_provider_prompt(
        &row,
        &BoundaryFrameImageIds {
            first: Some(21),
            last: Some(22),
        },
    )
    .expect_err("internal workflow prompt text should be rejected");

    assert!(err.contains("visual_prompt 包含视频生成内部流程说明"));
    assert!(err.contains("首帧串联"));
}

#[test]
fn provider_prompt_rejects_missing_aspect_ratio_without_unspecified_default() {
    let mut row = prompt_with_metadata();
    row.aspect_ratio = None;

    let err = build_video_generation_provider_prompt(
        &row,
        &BoundaryFrameImageIds {
            first: Some(21),
            last: Some(22),
        },
    )
    .expect_err("true provider prompt must require aspect ratio");

    assert!(err.contains("缺少 aspect_ratio"));
}

#[test]
fn provider_preview_text_placeholders_keep_blank_draft_request_renderable() {
    let mut row = prompt_with_metadata();
    row.visual_prompt = String::new();
    row.motion_prompt = None;
    let mut note = None;

    ensure_chapter_video_prompt_preview_texts(&mut row, &mut note);
    let prompt = build_video_generation_provider_preview_prompt(
        &row,
        &BoundaryFrameImageIds {
            first: Some(21),
            last: Some(22),
        },
    )
    .expect("preview placeholders should make provider prompt renderable");

    let note = note.expect("blank preview fields should be noted");
    assert!(note.contains("visual_prompt 过短"));
    assert!(note.contains("motion_prompt 过短"));
    assert!(prompt.contains(VIDEO_PROVIDER_VISUAL_PREVIEW_PLACEHOLDER));
    assert!(prompt.contains(VIDEO_PROVIDER_MOTION_PREVIEW_PLACEHOLDER));
    assert!(prompt.contains("【视频生成主提示词】"));
    assert!(prompt.contains("【动作与运镜补充】"));
}

#[test]
fn provider_preview_replaces_deprecated_motion_continuity_section_with_placeholder() {
    let mut row = prompt_with_metadata();
    row.motion_prompt = Some("[运镜与动作]\n镜头保持平视中景轻微手持漂移，小宦官先停住听向暗角，再缓慢回头并轻微后缩，动作节奏压低且不跳轴。\n\n[对白与表演]\n小宦官压低声音说“谁在那儿”，嘴唇短促微动，随后屏住呼吸，眼神仍锁向黑暗处。\n\n[连续性要求]\n这是旧版重复章节，应该由 visual_prompt 的连续性与结尾状态承担。".to_string());
    let mut note = None;

    ensure_chapter_video_prompt_preview_texts(&mut row, &mut note);
    let prompt = build_video_generation_provider_preview_prompt(
        &row,
        &BoundaryFrameImageIds {
            first: Some(21),
            last: Some(22),
        },
    )
    .expect("preview should replace deprecated motion sections with a placeholder");

    let note = note.expect("deprecated preview fields should be noted");
    assert!(note.contains("motion_prompt 包含已废弃章节 [连续性要求]"));
    assert!(prompt.contains(VIDEO_PROVIDER_MOTION_PREVIEW_PLACEHOLDER));
    assert!(!prompt.contains("[连续性要求]"));
    assert!(!prompt.contains("旧版重复章节"));
}

#[test]
fn provider_preview_replaces_dictionary_like_prompt_text_with_placeholder() {
    let mut row = prompt_with_metadata();
    row.visual_prompt = format!(
        "{} {{\"blocking_plan\":\"站位数据不应该作为画面提示词发送\"}}",
        LONG_VISUAL
    );
    let mut note = None;

    ensure_chapter_video_prompt_preview_texts(&mut row, &mut note);
    let prompt = build_video_generation_provider_preview_prompt(
        &row,
        &BoundaryFrameImageIds {
            first: Some(21),
            last: Some(22),
        },
    )
    .expect("preview should replace dictionary-like visual prompt");

    let note = note.expect("dictionary-like preview fields should be noted");
    assert!(note.contains("visual_prompt 包含 JSON/字典结构片段"));
    assert!(prompt.contains(VIDEO_PROVIDER_VISUAL_PREVIEW_PLACEHOLDER));
    assert!(!prompt.contains("blocking_plan"));
    assert!(!prompt.contains("{"));
}

#[test]
fn provider_preview_replaces_internal_workflow_prompt_text_with_placeholder() {
    let mut row = prompt_with_metadata();
    row.visual_prompt = format!(
        "{} 首帧串联规则：本段起始帧来自上一段尾帧自动采集。",
        LONG_VISUAL
    );
    let mut note = None;

    ensure_chapter_video_prompt_preview_texts(&mut row, &mut note);
    let prompt = build_video_generation_provider_preview_prompt(
        &row,
        &BoundaryFrameImageIds {
            first: Some(21),
            last: Some(22),
        },
    )
    .expect("preview should replace internal workflow visual prompt");

    let note = note.expect("internal workflow preview fields should be noted");
    assert!(note.contains("visual_prompt 包含视频生成内部流程说明"));
    assert!(prompt.contains(VIDEO_PROVIDER_VISUAL_PREVIEW_PLACEHOLDER));
    assert!(!prompt.contains("首帧串联规则"));
    assert!(!prompt.contains("上一段尾帧"));
}

#[test]
fn provider_preview_replaces_internal_image_id_prompt_text_with_placeholder() {
    let mut row = prompt_with_metadata();
    row.visual_prompt = format!("{LONG_VISUAL}\n参考图 image_id=11 用于人物一致性。");
    let mut note = None;

    ensure_chapter_video_prompt_preview_texts(&mut row, &mut note);
    let prompt = build_video_generation_provider_preview_prompt(
        &row,
        &BoundaryFrameImageIds {
            first: Some(21),
            last: Some(22),
        },
    )
    .expect("preview should replace internal image id visual prompt");

    let note = note.expect("internal image id preview fields should be noted");
    assert!(note.contains("visual_prompt 不符合自然语言提示词约束"));
    assert!(note.contains("内部图片 ID"));
    assert!(prompt.contains(VIDEO_PROVIDER_VISUAL_PREVIEW_PLACEHOLDER));
    assert!(!prompt.contains("image_id=11"));
}

#[test]
fn provider_preview_metadata_placeholders_keep_request_renderable() {
    let mut row = prompt_with_metadata();
    row.metadata_json = None;
    let mut note = None;

    ensure_chapter_video_prompt_preview_metadata(&mut row, &mut note)
        .expect("preview metadata placeholders should be built");
    let prompt = build_video_generation_provider_preview_prompt(
        &row,
        &BoundaryFrameImageIds {
            first: None,
            last: None,
        },
    )
    .expect("preview metadata placeholders should make provider prompt renderable");

    let note = note.expect("missing metadata should be noted");
    assert!(note.contains("缺少结构化 metadata_json"));
    assert!(note.contains("metadata.shot"));
    assert!(prompt.contains("【视频生成主提示词】"));
    assert!(prompt.contains("【动作与运镜补充】"));
    assert!(!prompt.contains("请求预览占位"));
    assert!(!prompt.contains("【首尾帧状态】"));
    assert!(!prompt.contains("【片段独立生成上下文】"));
    assert!(!prompt.contains("【连续性自检】"));
    assert!(!prompt.contains("\"blocking\""));
}

#[test]
fn provider_prompt_still_rejects_missing_metadata_without_preview_placeholders() {
    let mut row = prompt_with_metadata();
    row.metadata_json = None;

    let err = build_video_generation_provider_prompt(
        &row,
        &BoundaryFrameImageIds {
            first: None,
            last: None,
        },
    )
    .expect_err("true provider prompt should still require structured metadata");

    assert!(err.contains("缺少结构化元数据"));
}

#[test]
fn provider_preview_override_applies_editor_draft_fields() {
    let mut row = prompt_with_metadata();
    let mut note = None;

    apply_chapter_video_prompt_preview_override(
        &mut row,
        Some(PreviewChapterVideoPromptOverrideInput {
            visual_prompt: Some("草稿视觉提示".to_string()),
            motion_prompt: Some(Some("草稿动作提示".to_string())),
            negative_prompt: Some(None),
            duration_secs: Some(0),
            reference_image_ids_json: Some(Some(json!([31, 32]))),
        }),
        &mut note,
    );

    assert_eq!(row.visual_prompt, "草稿视觉提示");
    assert_eq!(row.motion_prompt.as_deref(), Some("草稿动作提示"));
    assert_eq!(row.negative_prompt, None);
    assert_eq!(row.duration_secs, 1);
    assert_eq!(row.reference_image_ids_json, Some(json!([31, 32])));
    let note = note.expect("draft override should be noted");
    assert!(note.contains("草稿覆盖"));
    assert!(note.contains("duration_secs=0 无效"));
}

#[test]
fn preview_prompt_override_distinguishes_missing_and_null_nullable_fields() {
    let missing = serde_json::from_value::<PreviewChapterVideoPromptOverrideInput>(json!({}))
        .expect("empty override should deserialize");
    assert_eq!(missing.motion_prompt, None);
    assert_eq!(missing.negative_prompt, None);
    assert_eq!(missing.reference_image_ids_json, None);

    let cleared = serde_json::from_value::<PreviewChapterVideoPromptOverrideInput>(json!({
        "motion_prompt": null,
        "negative_prompt": null,
        "reference_image_ids_json": null
    }))
    .expect("null override fields should deserialize as explicit clears");
    assert_eq!(cleared.motion_prompt, Some(None));
    assert_eq!(cleared.negative_prompt, Some(None));
    assert_eq!(cleared.reference_image_ids_json, Some(None));
}

#[test]
fn first_frame_chain_provider_prompt_keeps_material_refs_out_of_image() {
    let mut row = prompt_with_metadata();
    row.workflow_mode = "first_frame_chain".to_string();
    row.shot_number = 2;
    row.metadata_json = Some(json!({
        "source_prompt_type": "frame_segment_plan",
        "shot": {
            "title": "回望暗廊",
            "scene_summary": "小宦官从上一段尾帧继续回望暗处",
            "blocking_plan": {"start": "上一段尾帧的暗廊入口前", "movement": "原地后缩回望", "end": "靠近暗廊入口但不越过"},
            "lighting_plan": {"key_light": "烛光", "direction": "画面右侧", "continuity": "继承上一段尾帧"},
            "emotion_action_beats": [{"character": "小宦官", "emotion": "恐惧", "visible_action": "肩膀收紧并回头"}],
            "cinematography_plan": {"shot_size": "中景", "camera_angle": "平视", "camera_movement": "轻微手持漂移", "axis_rule": "暗廊保持右后方", "lens_hint": "标准镜头"},
            "temporal_beats": [{"start_sec": 0, "end_sec": 4, "camera": "中景轻微漂移", "subject_action": "回头并靠近暗廊入口", "continuity_target": "形成可采集尾帧"}],
            "reference_binding": {"bindings": [{"image_id": 11, "entity": "小宦官", "role": "character", "usage": "保持脸和服装一致", "must_keep": "同一人物"}], "coverage_check": "参考图 11 已绑定"},
            "continuity_check": {"blocking": "连续", "lighting": "连续", "emotion": "已动作化", "references": "已覆盖", "revision": "无需修订"}
        },
        "start_first_frame": {
            "title": "上一段尾帧",
            "scene_summary": "小宦官停在暗廊入口前",
            "frame_prompt": "小宦官靠近暗廊入口但没有越过，回望右后方黑暗走廊。",
            "blocking_state": {"characters": [{"name": "小宦官", "position": "暗廊入口前"}]},
            "lighting_state": {"key_light": "烛光", "direction": "画面右侧"}
        }
    }));

    let prompt = build_video_generation_provider_prompt(
        &row,
        &BoundaryFrameImageIds {
            first: Some(31),
            last: None,
        },
    )
    .expect("first-frame-chain provider prompt should compile");

    assert!(prompt.contains("本次起步输入图片是视频第 0 秒画面"));
    assert!(prompt.contains("【视频生成主提示词】"));
    assert!(prompt.contains("【动作与运镜补充】"));
    assert!(prompt.contains("本请求按顺序输入 1 张参考图"));
    assert!(!prompt.contains("素材参考图只用于"));
    assert!(!prompt.contains("【首帧与结尾状态】"));
    assert!(!prompt.contains("【片段独立生成上下文】"));
    assert!(!prompt.contains("0-4秒：中景轻微漂移，回头并靠近暗廊入口，形成可采集尾帧。"));
    assert!(!prompt.contains("【首帧串联规则】"));
    assert!(!prompt.contains("首帧串联模式"));
    assert!(!prompt.contains("上一段视频自动采集"));
    assert!(!prompt.contains("上一段尾帧"));
    assert!(!prompt.contains("系统采集"));
    assert!(!prompt.contains("供系统"));
    assert!(!prompt.contains("generated_image #"));
    assert!(!prompt.contains("专用 `image` / `first_frame_image_url`"));
    assert!(!prompt.contains("参考图 #11 不能作为视频首帧"));
    assert!(!prompt.contains("不是首帧输入"));
    assert!(!prompt.contains("不要把参考图 #11 的原始画面当作视频开头画面"));
    assert!(!prompt.contains("【首尾帧状态】"));
}

#[test]
fn provider_prompt_uses_actual_bound_media_for_input_image_section() {
    let mut row = prompt_with_metadata();
    row.workflow_mode = "first_frame_chain".to_string();
    row.shot_number = 2;
    let engine = prompt_engine();
    let prompt = super::build_video_generation_provider_prompt_with_binding(
        &engine,
        &row,
        ProviderPromptImageBinding {
            has_first_frame_image: true,
            has_last_frame_image: false,
            reference_image_count: 0,
            has_storyboard_reference_image: false,
        },
    )
    .expect("provider prompt should compile");

    assert!(prompt.contains("- 首帧输入图片是视频第 0 秒画面"));
    assert!(!prompt.contains("本请求按顺序输入"));
    assert!(!prompt.contains("尾帧输入图片是视频结尾目标"));
}

#[test]
fn first_frame_chain_provider_prompt_allows_tail_frame_start_without_frame_metadata() {
    let mut row = prompt_with_metadata();
    row.workflow_mode = "first_frame_chain".to_string();
    row.shot_number = 2;
    row.metadata_json = Some(json!({
        "source_prompt_type": "frame_segment_plan",
        "shot": {
            "title": "回望暗廊",
            "scene_summary": "小宦官从上一段尾帧继续回望暗处",
            "blocking_plan": {"start": "上一段尾帧的暗廊入口前", "movement": "原地后缩回望", "end": "靠近暗廊入口但不越过"},
            "lighting_plan": {"key_light": "烛光", "direction": "画面右侧", "continuity": "继承上一段尾帧"},
            "emotion_action_beats": [{"character": "小宦官", "emotion": "恐惧", "visible_action": "肩膀收紧并回头"}],
            "cinematography_plan": {"shot_size": "中景", "camera_angle": "平视", "camera_movement": "轻微手持漂移", "axis_rule": "暗廊保持右后方", "lens_hint": "标准镜头"},
            "temporal_beats": [{"start_sec": 0, "end_sec": 4, "camera": "中景轻微漂移", "subject_action": "回头并靠近暗廊入口", "continuity_target": "形成可采集尾帧"}],
            "reference_binding": {"bindings": [], "coverage_check": "无参考图"},
            "continuity_check": {"blocking": "连续", "lighting": "连续", "emotion": "已动作化", "references": "无素材引用", "revision": "无需修订"}
        },
        "start_first_frame": null
    }));

    let prompt = build_video_generation_provider_prompt(
        &row,
        &BoundaryFrameImageIds {
            first: Some(31),
            last: None,
        },
    )
    .expect("tail-frame chained provider prompt should compile without start_first_frame");

    assert!(prompt.contains("本次起步输入图片是视频第 0 秒画面"));
    assert!(prompt.contains("必须从这张图自然推进"));
    assert!(prompt.contains("【视频生成主提示词】"));
    assert!(!prompt.contains("上一段视频自动采集"));
    assert!(!prompt.contains("上一段尾帧"));
    assert!(!prompt.contains("起始帧由系统"));
    assert!(!prompt.contains("首帧串联规则"));
    assert!(!prompt.contains("供系统"));
    assert!(!prompt.contains("不得改用素材参考图或重新设计首帧"));
}

#[test]
fn provider_preview_reports_missing_first_frame_chain_start_without_hiding_request() {
    let mut row = prompt_with_metadata();
    row.workflow_mode = "first_frame_chain".to_string();
    row.shot_number = 2;
    row.metadata_json = Some(json!({
        "source_prompt_type": "frame_segment_plan",
        "shot": {
            "title": "回望暗廊",
            "scene_summary": "小宦官从上一段尾帧继续回望暗处",
            "blocking_plan": {"start": "上一段尾帧的暗廊入口前", "movement": "原地后缩回望", "end": "靠近暗廊入口但不越过"},
            "lighting_plan": {"key_light": "烛光", "direction": "画面右侧", "continuity": "继承上一段尾帧"},
            "emotion_action_beats": [{"character": "小宦官", "emotion": "恐惧", "visible_action": "肩膀收紧并回头"}],
            "cinematography_plan": {"shot_size": "中景", "camera_angle": "平视", "camera_movement": "轻微手持漂移", "axis_rule": "暗廊保持右后方", "lens_hint": "标准镜头"},
            "temporal_beats": [{"start_sec": 0, "end_sec": 4, "camera": "中景轻微漂移", "subject_action": "回头并靠近暗廊入口", "continuity_target": "形成可采集尾帧"}],
            "reference_binding": {"bindings": [], "coverage_check": "无参考图"},
            "continuity_check": {"blocking": "连续", "lighting": "连续", "emotion": "已动作化", "references": "无素材引用", "revision": "无需修订"}
        },
        "start_first_frame": null
    }));
    let frame_ids = BoundaryFrameImageIds {
        first: None,
        last: None,
    };
    let provider_prompt = build_video_generation_provider_prompt(&row, &frame_ids)
        .expect("preview should still build the text prompt");
    let context = ChapterVideoProviderRequestContext {
        prompt: row,
        cfg: video_config(NEWAPI_VIDEO_BACKEND),
        request: VideoGenerationRequest {
            prompt: provider_prompt,
            negative_prompt: None,
            width: 1280,
            height: 720,
            duration_secs: 4,
            fps: 24,
            model: "test-video-model".to_string(),
            first_frame_image_url: None,
            last_frame_image_url: None,
            reference_image_urls: Vec::new(),
            reference_media: None,
        },
        prompt_reference_image_ids: Some(json!([11])),
        provider_reference_image_ids: Some(json!([11])),
        prompt_source: None,
        workflow_mode: "first_frame_chain".to_string(),
        source_prompt_type: Some("frame_segment_plan".to_string()),
        boundary_frame_ids: frame_ids,
        prompt_reference_image_count: 1,
        provider_reference_image_count: 1,
        first_frame_image_url_available: false,
        last_frame_image_url_available: false,
        provider_input_note: Some(
            "当前首帧串联起始帧缺失：首帧串联第 2 个视频单元缺少上一视频单元尾帧图片；此处仅展示 provider 请求结构，真实生成仍需补齐该输入"
                .to_string(),
        ),
    };

    let (_, _, note, request) =
        video_provider_preview_payload(&context).expect("preview payload should render");

    assert!(note.contains("当前首帧串联起始帧缺失"));
    assert!(note.contains("真实生成仍需补齐该输入"));
    assert!(request.get("image").is_none());
    assert!(request.get("prompt").is_some());
}

#[test]
fn provider_preview_uses_openai_compatible_request_body() {
    let context = provider_preview_context(OPENAI_COMPATIBLE_VIDEO_BACKEND);

    let (kind, title, note, request) =
        video_provider_preview_payload(&context).expect("preview payload");

    assert_eq!(kind, "video_generation_openai_compatible");
    assert!(title.contains("/videos/generations"));
    assert!(note.contains("OpenAI 兼容视频"));
    assert!(note.contains("实际发送 prompt："));
    assert!(note.contains("UTF-8 字节"));
    assert_eq!(request["model"], "test-video-model");
    assert_eq!(
        request["image"]["url"],
        "https://example.invalid/a4n-preview/first"
    );
    assert_eq!(
        request["last_frame"]["url"],
        "https://example.invalid/a4n-preview/last"
    );
    assert_eq!(request["response_format"], "b64_json");
    assert!(request.get("reference_media").is_none());
}

#[test]
fn provider_preview_uses_gemini_veo_request_body_shape() {
    let context = provider_preview_context(GEMINI_VEO_VIDEO_BACKEND);

    let (kind, title, note, request) =
        video_provider_preview_payload(&context).expect("preview payload");

    assert_eq!(kind, "video_generation_gemini_veo");
    assert!(title.contains("predictLongRunning"));
    assert!(note.contains("inlineData 占位"));
    let instance = &request["instances"][0];
    assert_eq!(instance["prompt"], "导演提示词正文");
    assert_eq!(instance["image"]["inlineData"]["mimeType"], "image/png");
    assert!(
        instance["image"]["inlineData"]["data"]
            .as_str()
            .unwrap()
            .contains("预览媒体占位")
    );
    assert_eq!(instance["lastFrame"]["inlineData"]["mimeType"], "image/png");
    assert_eq!(instance["referenceImages"][0]["referenceType"], "asset");
    assert_eq!(request["parameters"]["durationSeconds"], 4);
    assert!(request.get("first_frame_image_url").is_none());
    assert!(request.get("reference_image_urls").is_none());
}

#[test]
fn provider_preview_rejects_unknown_backend_without_internal_request_fallback() {
    let context = provider_preview_context("unknown-video");

    let err = video_provider_preview_payload(&context)
        .expect_err("unknown backend must not expose internal request fallback");

    assert!(err.contains("未知视频 backend=unknown-video"));
    assert!(err.contains("无法展示真实 provider 请求体"));
}

#[test]
fn provider_preview_duration_note_reports_unsupported_duration_without_blocking() {
    let mut cfg = video_config(NEWAPI_VIDEO_BACKEND);
    cfg.duration_mode = "fixed".to_string();
    cfg.min_duration_secs = 4;
    cfg.max_duration_secs = 8;
    cfg.supported_durations_json = Some(json!([4, 8]));
    let mut note = None;

    append_preview_duration_note(&mut note, &cfg, 7);

    let note = note.expect("unsupported preview duration should be reported");
    assert!(note.contains("不支持 7 秒"));
    assert!(note.contains("预览仍展示请求结构"));
    assert!(note.contains("真实生成会失败"));
}

#[test]
fn reference_media_urls_must_use_lowercase_http_scheme() {
    let mut note = None;
    let err = reference_media_urls(
        vec!["HTTPS://example.test/ref.png".to_string()],
        "image_files",
        MAX_SEEDANCE2_IMAGE_FILES,
        VideoProviderRequestPurpose::Generate,
        &mut note,
    )
    .expect_err("uppercase scheme must match frontend validation and be rejected");

    assert!(err.contains("小写 http:// 或 https://"));
}

#[test]
fn reference_media_preview_note_includes_provider_input_note() {
    let context = ReferenceMediaProviderRequestPreviewContext {
        request: VideoGenerationRequest {
            prompt: "使用 @image_file_1 作为人物服装与脸部一致性参考，镜头从中景缓慢推进，保持场景光线、动作节奏、人物站位和画面质感一致，生成单段参考媒体视频。".to_string(),
            negative_prompt: None,
            width: 1280,
            height: 720,
            duration_secs: 7,
            fps: 24,
            model: "seedance-2-0-pro".to_string(),
            first_frame_image_url: None,
            last_frame_image_url: None,
            reference_image_urls: Vec::new(),
            reference_media: Some(VideoReferenceMediaRequest {
                image_files: vec!["https://example.test/ref.png".to_string()],
                video_files: Vec::new(),
                audio_files: Vec::new(),
            }),
        },
        config_id: Some(9),
        provider_backend: Some("newapi-video".to_string()),
        provider_input_note: Some(
            "当前配置不是 seedance2-video；预览仍展示请求结构，真实生成会失败".to_string(),
        ),
    };

    let preview = reference_media_provider_request_preview(
        "reference_media_video_generation",
        "参考媒体视频模型请求",
        "当前表单",
        &context,
    )
    .expect("reference media preview should render");

    let note = preview.steps[0]
        .note
        .as_deref()
        .expect("preview note should be present");
    assert!(note.contains("backend=newapi-video"));
    assert!(note.contains("当前配置不是 seedance2-video"));
    assert!(note.contains("真实生成会失败"));
}

#[test]
fn reference_media_preview_helpers_use_placeholders_for_incomplete_inputs() {
    let mut note = None;
    let mut prompt = reference_media_prompt("", VideoProviderRequestPurpose::Preview, &mut note)
        .expect("preview prompt should use placeholder");
    let urls = reference_media_urls(
        vec!["ftp://example.test/ref.png".to_string()],
        "image_files",
        MAX_SEEDANCE2_IMAGE_FILES,
        VideoProviderRequestPurpose::Preview,
        &mut note,
    )
    .expect("preview urls should use placeholders");
    ensure_reference_media_tokens(
        &mut prompt,
        "image_file",
        urls.len(),
        VideoProviderRequestPurpose::Preview,
        &mut note,
    )
    .expect("preview should append missing token placeholders");

    assert_eq!(
        urls,
        vec!["https://example.invalid/a4n-preview/image_files/1".to_string()]
    );
    assert!(prompt.contains(REFERENCE_MEDIA_PREVIEW_PROMPT_PLACEHOLDER));
    assert!(prompt.contains("@image_file_1"));
    let note = note.expect("preview helper should report incomplete inputs");
    assert!(note.contains("prompt 过短"));
    assert!(note.contains("未以小写 http:// 或 https:// 开头"));
    assert!(note.contains("缺少 @image_file_1"));
}

#[test]
fn reference_media_prompt_must_reference_every_bound_token() {
    let mut prompt = "这是一段足够长的参考媒体视频提示词，用于描述人物外观、场景结构、镜头调度、动作节奏、光影方向、情绪变化和画面风格；其中明确使用 @image_file_1 作为主角造型参考，但故意遗漏第二张图片引用以验证绑定校验。".to_string();
    let mut note = None;

    let err = ensure_reference_media_tokens(
        &mut prompt,
        "image_file",
        2,
        VideoProviderRequestPurpose::Generate,
        &mut note,
    )
    .expect_err("missing token must be rejected");

    assert!(err.contains("@image_file_2"));
}

#[test]
fn reference_media_prompt_requires_substantial_direct_prompt() {
    let mut note = None;
    let err = reference_media_prompt(
        "@image_file_1 做参考",
        VideoProviderRequestPurpose::Generate,
        &mut note,
    )
    .expect_err("short prompt must be rejected");

    assert!(err.contains("prompt 过短"));
}

#[test]
fn reference_media_aspect_ratio_maps_to_seedance_dimensions() {
    assert_eq!(
        reference_media_dimensions_for_aspect_ratio("21:9").expect("valid ratio"),
        (1680, 720)
    );
    assert_eq!(
        reference_media_dimensions_for_aspect_ratio("3:4").expect("valid ratio"),
        (768, 1024)
    );
    assert!(reference_media_dimensions_for_aspect_ratio("2:1").is_err());
}

#[test]
fn video_mime_from_path_rejects_unknown_video_extensions() {
    assert_eq!(
        media::video_mime_from_path(Path::new("clip.mp4")).expect("mp4 MIME"),
        "video/mp4"
    );
    assert_eq!(
        media::video_mime_from_path(Path::new("clip.M4V")).expect("m4v MIME"),
        "video/mp4"
    );
    assert_eq!(
        media::video_mime_from_path(Path::new("clip.webm")).expect("webm MIME"),
        "video/webm"
    );
    assert_eq!(
        media::video_mime_from_path(Path::new("clip.mov")).expect("mov MIME"),
        "video/quicktime"
    );

    let err = media::video_mime_from_path(Path::new("clip.avi"))
        .expect_err("unsupported extension should fail");
    assert!(err.contains("视频文件扩展名不支持播放：avi"));

    let err =
        media::video_mime_from_path(Path::new("clip")).expect_err("missing extension should fail");
    assert_eq!(err, "视频文件缺少扩展名，无法确定 MIME 类型");
}

#[test]
fn reference_media_record_reads_provider_request_snapshot() {
    let request = json!({
        "model_id": "seedance-2-0-pro",
        "parameters": {
            "prompt": "历史快照里的完整参考媒体视频提示词",
            "functionMode": "omni_reference",
            "image_files": ["https://example.test/character.png"],
            "video_files": ["https://example.test/action.mp4"],
            "audio_files": []
        }
    });
    let video = reference_media_video_with_metadata(Some(json!({
        "provider_request_snapshot": provider_request_snapshot_value(
            "reference_media_video",
            "Seedance 2.0 /task/submit 请求",
            "video_generation_seedance2_omni_reference",
            Some("reference_media_video"),
            Some("生成时请求快照".to_string()),
            request
        )
    })));

    let preview = model_request_preview_from_provider_snapshot(
        &video,
        "video_provider_request_record",
        "历史视频模型请求",
    )
    .expect("saved snapshot should preview directly");
    let body = preview.steps[0]
        .request
        .as_ref()
        .expect("snapshot preview should expose request");

    assert_eq!(
        body.pointer("/model_id").and_then(Value::as_str),
        Some("seedance-2-0-pro")
    );
    assert_eq!(
        body.pointer("/parameters/image_files/0")
            .and_then(Value::as_str),
        Some("https://example.test/character.png")
    );
    assert_eq!(
        body.pointer("/parameters/video_files/0")
            .and_then(Value::as_str),
        Some("https://example.test/action.mp4")
    );
}

#[test]
fn provider_request_record_rejects_mismatched_workflow_mode() {
    let mut video = reference_media_video_with_metadata(Some(json!({
        "provider_request_snapshot": provider_request_snapshot_value(
            "reference_media_video",
            "Seedance 2.0 /task/submit 请求",
            "video_generation_seedance2_omni_reference",
            Some("reference_media_video"),
            None,
            json!({
                "model_id": "seedance-2-0-pro",
                "parameters": {"prompt": "历史快照 prompt"}
            })
        )
    })));
    video.workflow_mode = Some("boundary_frames".to_string());

    let err = validate_video_provider_request_snapshot_owner(&video)
        .expect_err("mismatched explicit workflow mode must be rejected");

    assert!(err.contains("workflow_mode=boundary_frames"));
    assert!(err.contains("reference_media_video 类型不匹配"));
}

#[test]
fn provider_request_record_note_reports_unconfirmed_failed_delivery() {
    let mut video = reference_media_video_with_metadata(Some(json!({
        "provider_request_snapshot": provider_request_snapshot_value(
            "reference_media_video",
            "Seedance 2.0 /task/submit 请求",
            "video_generation_seedance2_omni_reference",
            Some("reference_media_video"),
            Some("生成时请求快照".to_string()),
            json!({
                "model_id": "seedance-2-0-pro",
                "parameters": {"prompt": "历史快照 prompt"}
            })
        )
    })));
    video.status = "failed".to_string();

    let preview = model_request_preview_from_provider_snapshot(
        &video,
        "video_provider_request_record",
        "历史视频模型请求",
    )
    .expect("failed video with snapshot should still preview constructed request");
    let note = preview.steps[0]
        .note
        .as_deref()
        .expect("failed record should include delivery note");

    assert!(note.contains("生成时请求快照"));
    assert!(note.contains("无法仅凭该状态确认 provider 是否实际接收请求"));
    assert!(note.contains("构造出的 provider 请求快照"));
}

#[test]
fn provider_request_record_requires_saved_snapshot() {
    let video = reference_media_video_with_metadata(Some(json!({
        "reference_media": {
            "functionMode": "omni_reference",
            "image_files": ["https://example.test/character.png"]
        }
    })));

    let err = model_request_preview_from_provider_snapshot(
        &video,
        "video_provider_request_record",
        "历史视频模型请求",
    )
    .expect_err("missing snapshot cannot be previewed");

    assert!(err.contains("缺少 provider 请求快照"));
    assert!(err.contains("无法直接预览"));
}

#[test]
fn provider_request_record_rejects_blank_required_snapshot_fields() {
    let video = reference_media_video_with_metadata(Some(json!({
        "provider_request_snapshot": {
            "schema_version": 1,
            "captured_at": "2026-06-29T20:29:00Z",
            "key": "reference_media_video",
            "title": "   ",
            "kind": "video_generation_seedance2_omni_reference",
            "prompt_name": "reference_media_video",
            "state": "ready",
            "request": {
                "model_id": "seedance-2-0-pro",
                "parameters": {"prompt": "历史快照 prompt"}
            }
        }
    })));

    let err = model_request_preview_from_provider_snapshot(
        &video,
        "video_provider_request_record",
        "历史视频模型请求",
    )
    .expect_err("blank snapshot title should fail explicitly");

    assert!(err.contains("provider 请求快照字段 title 不能为空"));
}

#[test]
fn provider_generation_run_update_requires_at_least_one_running_run() {
    ensure_provider_generation_run_updated(42, 1, "完成 provider 视频生成")
        .expect("updated generation run should satisfy provider lifecycle");

    let err = ensure_provider_generation_run_updated(42, 0, "完成 provider 视频生成")
        .expect_err("missing generation run target must be visible");
    assert_eq!(
        err,
        "完成 provider 视频生成失败：视频 #42 缺少可更新的 generation_run"
    );
}
