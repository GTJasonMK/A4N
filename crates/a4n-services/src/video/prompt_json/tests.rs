use super::*;
use serde_json::{Value, json};
use std::collections::HashMap;

fn valid_segments() -> Vec<VideoSegmentPlan> {
    vec![VideoSegmentPlan {
        segment_number: 1,
        analysis_shot_id: 1,
        segment_index_in_shot: 1,
        shot_segment_count: 1,
        covered_beat_ids: vec![1, 2],
        title: "开场".to_string(),
        scene_summary: "主角进入档案室".to_string(),
        blocking_summary: "主角从档案室门口走向书桌".to_string(),
        spatial_anchors: json!([{"anchor": "书桌", "usage": "约束主角终点"}]),
        emotion_action_goals: json!([{
            "character": "主角",
            "emotion": "谨慎",
            "visible_action": "放慢脚步并观察"
        }]),
        video_intent: json!({
            "hook": "主角第一次进入异常档案室",
            "audience_effect": "让观众感到安静压迫和不确定",
            "rhythm": "缓慢推进",
            "visual_priority": "主角从门口到书桌的谨慎移动"
        }),
        emotional_arc: json!({
            "start": "谨慎观察",
            "turn": "发现档案室气氛异常",
            "end": "停在书桌旁保持戒备",
            "function": "场景建立"
        }),
        performance_plan: json!({
            "movement_path": "从档案室门口偏左向画面深处书桌慢走，沿左侧书架保持半步距离。",
            "body_performance": "肩膀略收紧，步幅放慢，靠近书桌时重心前移后停住。",
            "gaze_and_hands": "视线先扫过书架阴影再落到书桌，右手轻碰外套边缘。",
            "pause_or_reaction": "靠近书桌前停半拍，确认异常后不急着触碰物件。",
            "camera_relationship": "镜头平视缓慢推进，始终保留门口、书架和书桌的距离关系。",
            "time_blocks": [
                {
                    "start_sec": 0,
                    "end_sec": 3,
                    "camera": "门口宽中景，门框和书桌同框。",
                    "subject_action": "主角停在门口偏左，先观察书架和书桌。",
                    "continuity_target": "左侧窗光和门口位置保持不变。"
                },
                {
                    "start_sec": 3,
                    "end_sec": 6,
                    "camera": "缓慢推进到中景。",
                    "subject_action": "主角沿书架左侧走向书桌，在书桌旁停住。",
                    "continuity_target": "结尾保留主角、书桌和书架的固定关系。"
                }
            ],
            "action_script": [
                {
                    "character": "主角",
                    "body": "肩膀收紧，脚步放慢。",
                    "movement": "从画面左前门口走向画面深处中央书桌。",
                    "hands": "右手轻碰外套边缘，靠近书桌时停住。",
                    "gaze": "从书架阴影转向书桌表面。",
                    "timing": "全段"
                }
            ]
        }),
        material_coverage: json!({
            "required_entities": ["主角", "档案大厅"],
            "missing_or_weak_references": [],
            "risk_note": "关键人物和场景已有参考图"
        }),
        source_span_ids: vec!["ch1:s1:f1".to_string()],
        duration_secs: 6,
        aspect_ratio: "16:9".to_string(),
        reference_image_ids: vec![1, 2],
    }]
}

fn segment_map(segments: &[VideoSegmentPlan]) -> HashMap<i32, &VideoSegmentPlan> {
    segments
        .iter()
        .map(|segment| (segment.segment_number, segment))
        .collect()
}

fn valid_shot_value(_shot_number: i32) -> Value {
    json!({
        "title": "开场",
        "scene_summary": "主角进入档案室",
        "visual_prompt": "[主体定义与素材绑定]\n参考图 1 用于主角脸部、发型、年龄感和深色长外套一致性；参考图 2 用于档案大厅入口、木质书架、铜制吊灯和中央书桌的空间基准。\n\n[片段内容以及前因后果]\n主角从档案室门口偏左的起点进入，目标是靠近深处书桌并确认异常气氛，结尾交给下一段的状态是他停在书桌旁保持戒备。\n\n[镜头分镜]\n镜头 1：门口宽中景保持门框、书架和书桌同框，主角站在画面左侧先停半拍观察，视线从书架阴影扫到深处书桌，空间保持安静压迫。\n镜头 2：平视中景缓慢推进，主角沿左侧书架向书桌慢走，肩膀略收紧，右手轻碰外套边缘，最后在书桌旁停住并保持戒备。\n\n[画面风格描述]\n电影感写实，暖色窗光从画面左侧斜照，阴影落向右后方，构图稳定并突出门、书架和书桌的空间关系。\n\n[连续性与结尾状态]\n人物始终保持画面左侧，门、书桌和书架的位置关系不变，结尾停在书桌旁，表情戒备，左侧窗光方向保持连续。\n\n[限制与避免项]\n不要配背景音乐，不要字幕、水印、换脸、空间跳变、镜头过快或角色服装变化。",
        "motion_prompt": "[运镜与动作]\n镜头从门口宽景缓慢向前推进，只保留一个稳定推进运动，动作节奏严格承接 visual_prompt 中的镜头分镜。\n\n[对白与表演]\n主角低声短句“这里不对劲”，嘴唇微动，脚步放慢。",
        "negative_prompt": "低清晰度、画面模糊、人体结构扭曲、多余肢体、文字不可读、面部不一致、空间跳变、光影不连续",
        "dialogue_cues": [
            {"speaker": "主角", "line": "这里不对劲", "delivery": "低声迟疑", "visible_action": "嘴唇微动并放慢脚步", "listener_reaction": "无人回应，空间保持寂静", "timing": "中段"}
        ]
    })
}

fn response_from_shots(shots: Vec<Value>) -> String {
    json!({ "shots": shots }).to_string()
}

fn parse_with_valid_segments(raw: &str) -> Result<Vec<VideoPromptShot>, AppError> {
    let segments = valid_segments();
    parse_video_prompt_shots(raw, &segment_map(&segments), PromptOutputLanguage::ZhCn)
}

#[test]
fn rejects_video_prompt_response_wrapped_in_single_json_fence() {
    let raw = format!(
        "```json\n{}\n```",
        response_from_shots(vec![valid_shot_value(1)])
    );
    let err = parse_with_valid_segments(&raw)
        .expect_err("markdown-fenced JSON must violate the strict response contract");

    let message = err.to_string();
    assert!(message.contains("严格 JSON 对象"));
    assert!(message.contains("Markdown 代码围栏"));
    assert!(message.contains("不能用 ``` 或 ```json 包裹"));
}

#[test]
fn rejects_video_prompt_motion_prompt_with_duplicate_shot_storyboard() {
    let mut shot = valid_shot_value(1);
    shot["motion_prompt"] = json!(
        "[镜头分镜]\n镜头 1：主角在门口停顿观察。\n镜头 2：主角沿书架走向书桌。\n\n[运镜与动作]\n镜头从门口宽景缓慢推进。\n\n[对白与表演]\n无对白。"
    );
    let raw = response_from_shots(vec![shot]);

    let err = parse_with_valid_segments(&raw)
        .expect_err("motion_prompt must not repeat the visual prompt timeline");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("motion_prompt")
                && message.contains("不应重复镜头分镜")
    ));
}

#[test]
fn rejects_video_prompt_motion_prompt_with_deprecated_continuity_section() {
    let mut shot = valid_shot_value(1);
    shot["motion_prompt"] = json!(
        "[运镜与动作]\n镜头从门口宽景缓慢推进，主角脚步放慢，肩膀收紧，视线从书架阴影移向深处书桌。\n\n[对白与表演]\n主角低声短句“这里不对劲”，嘴唇微动，脚步放慢。\n\n[连续性要求]\n旧版连续性尾态不应再写入 motion_prompt。"
    );
    let raw = response_from_shots(vec![shot]);

    let err = parse_with_valid_segments(&raw)
        .expect_err("motion_prompt must reject deprecated continuity sections");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("motion_prompt")
                && message.contains("旧版视频提示词章节 [连续性要求]")
    ));
}

#[test]
fn rejects_video_prompt_with_legacy_temporal_storyboard_section() {
    let mut shot = valid_shot_value(1);
    shot["visual_prompt"] = json!(
        "[主体定义与素材绑定]\n参考图 1 用于主角一致性，参考图 2 用于档案大厅空间基准。\n\n[片段内容以及前因后果]\n主角从档案室门口进入并靠近书桌，结尾停在书桌旁保持戒备。\n\n[时序分镜]\n总时长：6秒。0-3秒：主角观察。3-6秒：主角走向书桌。\n\n[画面风格描述]\n电影感写实，暖色窗光从画面左侧斜照。\n\n[连续性与结尾状态]\n人物始终保持画面左侧，门、书桌和书架的位置关系不变。\n\n[限制与避免项]\n不要配背景音乐，不要字幕、水印、换脸、空间跳变或角色服装变化。"
    );
    let raw = response_from_shots(vec![shot]);

    let err = parse_with_valid_segments(&raw)
        .expect_err("legacy visual_prompt section must not satisfy the current contract");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("visual_prompt")
                && message.contains("旧版视频提示词章节 [时序分镜]")
    ));
}

#[test]
fn rejects_video_prompt_without_required_visual_sections() {
    let mut shot = valid_shot_value(1);
    shot["visual_prompt"] = json!(
        "[主体定义与素材绑定]\n参考图 1 用于主角一致性，参考图 2 用于档案大厅空间基准。\n\n[片段内容以及前因后果]\n主角从档案室门口进入并靠近书桌，结尾停在书桌旁保持戒备。\n\n[画面风格描述]\n电影感写实，暖色窗光从画面左侧斜照，阴影落向右后方。\n\n[连续性与结尾状态]\n人物始终保持画面左侧，门、书桌和书架的位置关系不变。\n\n[限制与避免项]\n不要配背景音乐，不要字幕、水印、换脸、空间跳变或角色服装变化。"
    );
    let raw = response_from_shots(vec![shot]);

    let err = parse_with_valid_segments(&raw)
        .expect_err("visual_prompt must keep the six-section API contract");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("visual_prompt")
                && message.contains("缺少必需章节 [镜头分镜]")
    ));
}

#[test]
fn rejects_video_prompt_without_required_motion_sections() {
    let mut shot = valid_shot_value(1);
    shot["motion_prompt"] = json!(
        "[运镜与动作]\n镜头从门口宽景缓慢向前推进，只保留一个稳定推进运动，主角脚步放慢，肩膀收紧，视线从书架阴影移向深处书桌。"
    );
    let raw = response_from_shots(vec![shot]);

    let err = parse_with_valid_segments(&raw)
        .expect_err("motion_prompt must keep the two-section API contract");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("motion_prompt")
                && message.contains("缺少必需章节 [对白与表演]")
    ));
}

#[test]
fn rejects_video_prompt_with_upload_status_text() {
    let mut shot = valid_shot_value(1);
    shot["visual_prompt"] = json!(
        "[主体定义与素材绑定]\n无输入首帧。本段仅依赖文字描述保持角色形象一致。\n\n[片段内容以及前因后果]\n主角从档案室门口偏左的起点进入，目标是靠近深处书桌并确认异常气氛，结尾交给下一段的状态是他停在书桌旁保持戒备。\n\n[镜头分镜]\n镜头 1：门口宽中景建立门框、书架和书桌的空间关系，主角在画面左侧停半拍观察，视线扫过书架阴影。\n镜头 2：平视中景缓慢推进，主角沿书架左侧向书桌慢走，最后在书桌旁停住并保持戒备。\n\n[画面风格描述]\n电影感写实，暖色窗光从画面左侧斜照，阴影落向右后方。\n\n[连续性与结尾状态]\n人物始终保持画面左侧，门、书桌和书架的位置关系不变，结尾停在书桌旁。\n\n[限制与避免项]\n不要配背景音乐，不要字幕、水印、换脸、空间跳变、镜头过快或角色服装变化。"
    );
    let raw = response_from_shots(vec![shot]);

    let err = parse_with_valid_segments(&raw)
        .expect_err("API prompt must not write upload-status placeholders");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("visual_prompt")
                && message.contains("上传状态说明")
                && message.contains("无输入首帧")
    ));
}

#[test]
fn rejects_video_prompt_with_tail_frame_upload_status_text() {
    let mut shot = valid_shot_value(1);
    shot["visual_prompt"] = json!(
        "[主体定义与素材绑定]\n参考图 1 用于主角脸部、发型和服装一致性；没有输入尾帧，本段结尾按文字描述生成。\n\n[片段内容以及前因后果]\n主角从档案室门口偏左的起点进入，目标是靠近深处书桌并确认异常气氛，结尾交给下一段的状态是他停在书桌旁保持戒备。\n\n[镜头分镜]\n镜头 1：门口宽中景建立门框、书架和书桌的空间关系，主角在画面左侧停半拍观察，视线扫过书架阴影。\n镜头 2：平视中景缓慢推进，主角沿书架左侧向书桌慢走，最后在书桌旁停住并保持戒备。\n\n[画面风格描述]\n电影感写实，暖色窗光从画面左侧斜照，阴影落向右后方。\n\n[连续性与结尾状态]\n人物始终保持画面左侧，门、书桌和书架的位置关系不变，结尾停在书桌旁。\n\n[限制与避免项]\n不要配背景音乐，不要字幕、水印、换脸、空间跳变、镜头过快或角色服装变化。"
    );
    let raw = response_from_shots(vec![shot]);

    let err = parse_with_valid_segments(&raw)
        .expect_err("API prompt must not write tail-frame upload-status placeholders");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("visual_prompt")
                && message.contains("上传状态说明")
                && message.contains("没有输入尾帧")
    ));
}

#[test]
fn parses_video_prompt_with_gore_detail_in_direct_text() {
    let mut shot = valid_shot_value(1);
    shot["visual_prompt"] = json!(
        "[主体定义与素材绑定]\n参考图 1 用于主角脸部、发型、年龄感和深色长外套一致性；参考图 2 用于档案大厅入口、木质书架、铜制吊灯和中央书桌的空间基准。\n\n[片段内容以及前因后果]\n主角从档案室门口偏左的起点进入，目标是靠近深处书桌并确认异常气氛；画面里血从指缝渗出、血滴落和血迹扩散都应转成可生成的非血腥痕迹与环境压迫感，结尾停在书桌旁保持戒备。\n\n[镜头分镜]\n镜头 1：门口宽中景保持门框、书架和书桌同框，主角站在画面左侧先停半拍观察，视线从书架阴影扫到深处书桌，空间保持安静压迫。\n镜头 2：平视中景缓慢推进，主角沿左侧书架向书桌慢走，肩膀略收紧，看见非血腥痕迹后停顿，最后在书桌旁保持戒备。\n\n[画面风格描述]\n电影感写实，暖色窗光从画面左侧斜照，阴影落向右后方，构图稳定并突出门、书架和书桌的空间关系。\n\n[连续性与结尾状态]\n人物始终保持画面左侧，门、书桌和书架的位置关系不变，结尾停在书桌旁，表情戒备，左侧窗光方向保持连续。\n\n[限制与避免项]\n不要配背景音乐，不要字幕、水印、换脸、空间跳变、镜头过快或角色服装变化。"
    );
    shot["motion_prompt"] = json!(
        "[运镜与动作]\n镜头从首帧的门口宽景缓慢向前推进，主角先在门口停顿观察，再沿书架左侧向书桌移动，脚步放慢并保持戒备。\n\n[对白与表演]\n无对白，表演重点是谨慎观察、呼吸压低和看见非血腥痕迹后的短暂停顿。"
    );
    let raw = response_from_shots(vec![shot]);

    let rows = parse_with_valid_segments(&raw)
        .expect("gore wording is left to prompt optimization instead of hard validation");

    assert_eq!(rows.len(), 1);
}

fn valid_chat_prompt_text() -> String {
    [
        "[主体定义与素材绑定]\n画面从档案室门口开始，主角站在门口偏左，暖色窗光从左侧斜照，远处书桌和书架作为空间锚点。",
        "[片段内容以及前因后果]\n主角进入档案室并保持谨慎。",
        "[镜头分镜]\n镜头 1：门口宽中景建立书架和书桌关系，主角站在门口偏左并停顿观察。\n镜头 2：平视中景缓慢推进，主角沿书架走向书桌并在结尾停住。",
        "[画面风格描述]\n电影感写实，暖色窗光。",
        "[连续性与结尾状态]\n结尾停在书桌旁并保持戒备。",
        "[限制与避免项]\n避免字幕、水印、换脸、空间跳变，不要配背景音乐。",
    ]
    .join("\n")
}

fn valid_chat_video_shot_value(_shot_number: i32) -> Value {
    json!({
        "title": "开场",
        "scene_summary": "主角进入档案室",
        "chat_prompt": valid_chat_prompt_text(),
        "negative_prompt": "避免字幕、水印、换脸、空间跳变"
    })
}

fn parse_chat_with_valid_segments(raw: &str) -> Result<Vec<ChatVideoPromptShot>, AppError> {
    let segments = valid_segments();
    parse_chat_video_prompt_shots(raw, &segment_map(&segments), PromptOutputLanguage::ZhCn)
}

#[test]
fn rejects_chat_prompt_with_internal_image_id_in_direct_text() {
    let mut shot = valid_chat_video_shot_value(1);
    shot["chat_prompt"] = json!(
        "[主体定义与素材绑定]\n参考图 image_id=9 用于场景参考。\n[片段内容以及前因后果]\n主角进入档案室并保持谨慎。\n[镜头分镜]\n镜头 1：门口宽中景建立书架和书桌关系，主角站在门口偏左并停顿观察。\n镜头 2：平视中景缓慢推进，主角沿书架走向书桌并在结尾停住。\n[画面风格描述]\n电影感写实，暖色窗光。\n[连续性与结尾状态]\n结尾停在书桌旁并保持戒备。\n[限制与避免项]\n避免字幕、水印、换脸、空间跳变，不要配背景音乐。"
    );
    let raw = response_from_shots(vec![shot]);

    let err = parse_chat_with_valid_segments(&raw)
        .expect_err("chat prompt text must not expose internal image ids");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("chat_prompt")
                && message.contains("内部图片 ID")
    ));
}

#[test]
fn rejects_chat_prompt_with_upload_status_text() {
    let mut shot = valid_chat_video_shot_value(1);
    shot["chat_prompt"] = json!(
        "[主体定义与素材绑定]\n本段无上传图片，按文字描述生成。\n[片段内容以及前因后果]\n主角进入档案室并保持谨慎。\n[镜头分镜]\n镜头 1：门口宽中景建立书架和书桌关系，主角站在门口偏左并停顿观察。\n镜头 2：平视中景缓慢推进，主角沿书架走向书桌并在结尾停住。\n[画面风格描述]\n电影感写实，暖色窗光。\n[连续性与结尾状态]\n结尾停在书桌旁并保持戒备。\n[限制与避免项]\n避免字幕、水印、换脸、空间跳变，不要配背景音乐。"
    );
    let raw = response_from_shots(vec![shot]);

    let err = parse_chat_with_valid_segments(&raw)
        .expect_err("chat prompt must describe the actual start state instead of upload status");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("chat_prompt")
                && message.contains("上传状态说明")
                && message.contains("无上传图片")
    ));
}

#[test]
fn rejects_chat_prompt_with_reference_image_upload_status_text() {
    let mut shot = valid_chat_video_shot_value(1);
    shot["chat_prompt"] = json!(
        "[主体定义与素材绑定]\n无参考图，按文字描述建立档案室门口、书桌和书架的空间关系。\n[片段内容以及前因后果]\n主角进入档案室并保持谨慎。\n[镜头分镜]\n镜头 1：门口宽中景建立书架和书桌关系，主角站在门口偏左并停顿观察。\n镜头 2：平视中景缓慢推进，主角沿书架走向书桌并在结尾停住。\n[画面风格描述]\n电影感写实，暖色窗光。\n[连续性与结尾状态]\n结尾停在书桌旁并保持戒备。\n[限制与避免项]\n避免字幕、水印、换脸、空间跳变，不要配背景音乐。"
    );
    let raw = response_from_shots(vec![shot]);

    let err = parse_chat_with_valid_segments(&raw)
        .expect_err("chat prompt must describe the actual start state instead of upload status");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("chat_prompt")
                && message.contains("上传状态说明")
                && message.contains("无参考图")
    ));
}

#[test]
fn parses_chat_prompt_with_self_harm_intent_in_direct_text() {
    let mut shot = valid_chat_video_shot_value(1);
    shot["chat_prompt"] = json!(
        "[主体定义与素材绑定]\n画面从档案室门口开始，主角站在门口偏左，暖色窗光从左侧斜照，远处书桌和书架作为空间锚点。\n[片段内容以及前因后果]\n主角进入档案室并为后续自残动作做铺垫。\n[镜头分镜]\n镜头 1：门口宽中景建立书架和书桌关系，主角站在门口偏左并停顿观察。\n镜头 2：平视中景缓慢推进，主角沿书架走向书桌并在结尾停住。\n[画面风格描述]\n电影感写实，暖色窗光。\n[连续性与结尾状态]\n结尾停在书桌旁并保持戒备。\n[限制与避免项]\n避免字幕、水印、换脸、空间跳变，不要配背景音乐。"
    );
    let raw = response_from_shots(vec![shot]);

    let rows = parse_chat_with_valid_segments(&raw)
        .expect("self-harm wording is left to prompt optimization instead of hard validation");

    assert_eq!(rows.len(), 1);
}

#[test]
fn parses_chat_prompt_with_gore_detail_in_direct_text() {
    let mut shot = valid_chat_video_shot_value(1);
    shot["chat_prompt"] = json!(
        "[主体定义与素材绑定]\n画面从档案室门口开始，主角站在门口偏左，暖色窗光从左侧斜照，远处书桌和书架作为空间锚点。\n[片段内容以及前因后果]\n主角进入档案室并保持谨慎，画面用非血腥痕迹和环境压迫感替代血从指缝渗出、血滴落和血迹扩散。\n[镜头分镜]\n镜头 1：门口宽中景建立书架和书桌关系，主角站在门口偏左并停顿观察。\n镜头 2：平视中景缓慢推进，主角沿书架走向书桌，看见非血腥痕迹后停顿并在结尾保持戒备。\n[画面风格描述]\n电影感写实，暖色窗光。\n[连续性与结尾状态]\n结尾停在书桌旁并保持戒备。\n[限制与避免项]\n避免字幕、水印、换脸、空间跳变，不要配背景音乐。"
    );
    let raw = response_from_shots(vec![shot]);

    let rows = parse_chat_with_valid_segments(&raw)
        .expect("gore wording is left to prompt optimization instead of hard validation");

    assert_eq!(rows.len(), 1);
}

#[test]
fn parses_chat_prompt_with_explicit_self_harm_execution_in_direct_text() {
    let mut shot = valid_chat_video_shot_value(1);
    shot["chat_prompt"] = json!(
        "[主体定义与素材绑定]\n画面从档案室门口开始，主角站在门口偏左，暖色窗光从左侧斜照，远处书桌和书架作为空间锚点。\n[片段内容以及前因后果]\n主角进入档案室并保持谨慎。\n[镜头分镜]\n镜头 1：门口宽中景建立书架和书桌关系，主角站在门口偏左并停顿观察。\n镜头 2：平视中景缓慢推进，主角沿书架走向书桌并把铁片刺入眼睛。\n[画面风格描述]\n电影感写实，暖色窗光。\n[连续性与结尾状态]\n结尾停在书桌旁并保持戒备。\n[限制与避免项]\n避免字幕、水印、换脸、空间跳变，不要配背景音乐。"
    );
    let raw = response_from_shots(vec![shot]);

    let rows = parse_chat_with_valid_segments(&raw).expect(
        "self-harm execution wording is left to prompt optimization instead of hard validation",
    );

    assert_eq!(rows.len(), 1);
}

fn valid_source_spans() -> Vec<SourceSpan> {
    vec![SourceSpan {
        id: "ch1:s1:f1".to_string(),
        chapter_id: 1,
        chapter_number: 1,
        paragraph_index: 1,
        sentence_index: 0,
        fragment_index: 0,
        text: "主角进入档案室。".to_string(),
        char_start: 0,
        char_end: 24,
    }]
}

fn valid_segment_plan_value() -> Value {
    json!({
        "segments": [
            {
                "analysis_shot_id": 1,
                "covered_beat_ids": [1, 2],
                "title": "开场",
                "scene_summary": "主角进入档案室",
                "blocking_summary": "主角从档案室门口走向书桌",
                "spatial_anchors": [{"anchor": "书桌", "usage": "约束主角终点"}],
                "emotion_action_goals": [{"character": "主角", "emotion": "谨慎", "visible_action": "放慢脚步并观察"}],
                "video_intent": {
                    "hook": "主角第一次进入异常档案室",
                    "audience_effect": "让观众感到安静压迫和不确定",
                    "rhythm": "缓慢推进",
                    "visual_priority": "主角从门口到书桌的谨慎移动"
                },
                "emotional_arc": {
                    "start": "谨慎观察",
                    "turn": "发现档案室气氛异常",
                    "end": "停在书桌旁保持戒备",
                    "function": "场景建立"
                },
                "performance_plan": {
                    "movement_path": "主角从档案室门口偏左进入，沿书架左侧向画面深处书桌慢走，并在书桌旁停住。",
                    "body_performance": "肩膀收紧，步幅变小，靠近书桌前重心略向前压。",
                    "gaze_and_hands": "视线先扫过书架阴影，再落到书桌表面；右手轻碰外套边缘。",
                    "pause_or_reaction": "靠近书桌前停半拍，确认异常后保持戒备。",
                    "camera_relationship": "平视中景缓慢推进，保留门口、书架和书桌的空间距离。",
                    "time_blocks": [
                        {
                            "start_sec": 0,
                            "end_sec": 3,
                            "camera": "门口宽中景，门框和书桌同框。",
                            "subject_action": "主角停在门口偏左，先观察书架和书桌。",
                            "continuity_target": "门口、书架、书桌和左侧窗光保持稳定。"
                        },
                        {
                            "start_sec": 3,
                            "end_sec": 6,
                            "camera": "镜头缓慢推进到中景。",
                            "subject_action": "主角沿左侧书架走向书桌，在书桌旁停住。",
                            "continuity_target": "结尾停在书桌旁，表情戒备。"
                        }
                    ],
                    "action_script": [
                        {
                            "character": "主角",
                            "body": "肩膀收紧，脚步放慢。",
                            "movement": "从画面左前门口走向画面深处中央书桌。",
                            "hands": "右手轻碰外套边缘。",
                            "gaze": "从书架阴影转向书桌表面。",
                            "timing": "全段"
                        }
                    ]
                },
                "material_coverage": {
                    "required_entities": ["主角", "档案大厅"],
                    "missing_or_weak_references": ["主角", "档案大厅"],
                    "risk_note": "参考一致性由文字约束承担"
                }
            }
        ]
    })
}

fn parse_valid_segment_plan(value: &Value) -> Result<Vec<VideoSegmentPlan>, AppError> {
    parse_video_segment_plan_response(
        &value.to_string(),
        &valid_source_spans(),
        &parse_valid_shot_analysis(&valid_shot_analysis_value())?,
        6,
        "16:9",
    )
}

fn valid_beat_extraction_value() -> Value {
    json!({
        "beats": [
            {
                "natural_seconds": 3.0,
                "required": true,
                "visible_action": "主角推门进入档案室并短暂停顿观察",
                "source_span_ids": ["ch1:s1:f1"]
            },
            {
                "natural_seconds": 3.0,
                "required": true,
                "visible_action": "主角沿书架走向书桌并停步戒备",
                "source_span_ids": ["ch1:s1:f1"]
            }
        ]
    })
}

fn parse_valid_beats(value: &Value) -> Result<Vec<VideoActionBeat>, AppError> {
    parse_video_beat_extraction_response(
        &value.to_string(),
        &valid_source_spans(),
        1,
        "测试动作节拍抽取",
    )
}

fn valid_extracted_beats() -> Vec<VideoActionBeat> {
    parse_valid_beats(&valid_beat_extraction_value()).expect("valid beats parse")
}

fn valid_shot_analysis_value() -> Value {
    json!({
        "cinematic_shots": [
            {
                "beat_ids": [1, 2],
                "title": "进门",
                "scene_intent": "主角谨慎进入档案室",
                "visible_actions": [
                    {
                        "character": "主角",
                        "action": "推门进入并走向书桌",
                        "purpose": "建立角色戒备状态和空间关系"
                    }
                ],
                "dialogue_or_speech_intent": {
                    "has_dialogue": false,
                    "speaker": null,
                    "key_line_or_intent": "无对白，靠动作表达",
                    "visible_performance": "停顿观察、放慢脚步"
                },
                "blocking": {
                    "start": "档案室门口",
                    "movement": "沿书架走向书桌",
                    "end": "书桌旁",
                    "spatial_anchors": ["门", "书桌", "书架"]
                },
                "camera_intent": {
                    "shot_size": "中景",
                    "camera_angle": "平视",
                    "camera_movement": "缓慢推进",
                    "visual_priority": "主角从门口到书桌的移动"
                },
                "transition_in": "承接上一边界帧的门口站位",
                "transition_out": "停在书桌旁作为下一段起点",
                "start_state": {
                    "blocking": "主角位于档案室门口",
                    "emotion": "谨慎",
                    "camera": "平视中景"
                },
                "end_state": {
                    "blocking": "主角停在书桌旁",
                    "emotion": "戒备",
                    "camera": "平视中景保持空间连续"
                },
                "action_load": "medium",
                "continuity_handoff": {
                    "must_preserve": ["主角身份", "门、书桌、书架的空间关系"],
                    "handoff_state": "主角停在书桌旁并保持戒备"
                }
            }
        ]
    })
}

fn parse_valid_shot_analysis(value: &Value) -> Result<VideoShotAnalysis, AppError> {
    parse_video_shot_grouping_response(
        &value.to_string(),
        &valid_source_spans(),
        6,
        &valid_extracted_beats(),
    )
}

#[test]
fn parses_video_shot_grouping_with_extracted_action_beats() {
    let value = valid_shot_analysis_value();

    let analysis = parse_valid_shot_analysis(&value).expect("shot analysis should parse");

    assert_eq!(analysis.cinematic_shots.len(), 1);
    let shot = &analysis.cinematic_shots[0];
    assert_eq!(shot.shot_id, 1);
    assert_eq!(shot.segment_duration_secs, 6);
    assert_eq!(shot.required_segment_count, 1);
    assert_eq!(shot.estimated_natural_seconds, 6.0);
    assert_eq!(shot.action_beats[0].beat_id, 1);
    assert_eq!(shot.action_beats[0].source_span_ids, vec!["ch1:s1:f1"]);
    assert_eq!(analysis.planning_quality.total_required_segments, 1);
}

#[test]
fn rejects_video_shot_analysis_with_model_owned_shot_id() {
    let mut value = valid_shot_analysis_value();
    value["cinematic_shots"][0]["shot_id"] = json!(1);

    let err =
        parse_valid_shot_analysis(&value).expect_err("shot_id must be derived by the backend");

    assert!(err.to_string().contains("unknown field `shot_id`"));
}

#[test]
fn parses_stored_video_shot_analysis_canonical_contract() {
    let analysis =
        parse_valid_shot_analysis(&valid_shot_analysis_value()).expect("raw analysis parses");
    let stored = serde_json::to_value(&analysis).expect("stored analysis serializes");

    let parsed = parse_stored_video_shot_analysis(&stored.to_string(), &valid_source_spans(), 6)
        .expect("stored canonical analysis should parse");

    let shot = &parsed.cinematic_shots[0];
    assert_eq!(shot.shot_id, 1);
    assert_eq!(shot.action_beats[0].beat_id, 1);
    assert_eq!(shot.action_beats[0].beat_type, "object_interaction");
    assert_eq!(shot.estimated_natural_seconds, 6.0);
    assert_eq!(parsed.planning_quality.total_estimated_seconds, 6.0);
}

#[test]
fn reparses_stored_video_shot_analysis_with_current_duration_context() {
    let analysis =
        parse_valid_shot_analysis(&valid_shot_analysis_value()).expect("raw analysis parses");
    let mut stored = serde_json::to_value(&analysis).expect("stored analysis serializes");
    stored["cinematic_shots"][0]["segment_duration_secs"] = json!(6);
    stored["cinematic_shots"][0]["required_segment_count"] = json!(1);
    stored["planning_quality"]["total_estimated_seconds"] = json!(999.0);
    stored["planning_quality"]["total_required_segments"] = json!(99);

    let parsed = parse_stored_video_shot_analysis(&stored.to_string(), &valid_source_spans(), 5)
        .expect("stored duration-derived fields should be rederived");

    let shot = &parsed.cinematic_shots[0];
    assert_eq!(shot.segment_duration_secs, 5);
    assert_eq!(shot.required_segment_count, 2);
    assert_eq!(parsed.planning_quality.total_estimated_seconds, 6.0);
    assert_eq!(parsed.planning_quality.total_required_segments, 2);
}

#[test]
fn rejects_stored_video_shot_analysis_without_planning_quality() {
    let analysis =
        parse_valid_shot_analysis(&valid_shot_analysis_value()).expect("raw analysis parses");
    let mut stored = serde_json::to_value(&analysis).expect("stored analysis serializes");
    stored
        .as_object_mut()
        .expect("stored analysis should be object")
        .remove("planning_quality");

    let err = parse_stored_video_shot_analysis(&stored.to_string(), &valid_source_spans(), 6)
        .expect_err("stored planning_quality must be present");

    assert!(err.to_string().contains("missing field `planning_quality`"));
}

#[test]
fn rejects_legacy_stored_video_shot_analysis_planning_quality_shape() {
    let analysis =
        parse_valid_shot_analysis(&valid_shot_analysis_value()).expect("raw analysis parses");
    let mut stored = serde_json::to_value(&analysis).expect("stored analysis serializes");
    stored["planning_quality"] = json!({
        "legacy": true,
        "model_owned_summary": "旧版本字段不作为恢复来源"
    });

    let err = parse_stored_video_shot_analysis(&stored.to_string(), &valid_source_spans(), 6)
        .expect_err("legacy planning_quality shape must not be accepted silently");

    assert!(err.to_string().contains("unknown field `legacy`"));
}

#[test]
fn reparses_stored_video_shot_analysis_with_stale_derived_fields() {
    let analysis =
        parse_valid_shot_analysis(&valid_shot_analysis_value()).expect("raw analysis parses");
    let mut stored = serde_json::to_value(&analysis).expect("stored analysis serializes");
    stored["cinematic_shots"][0]["estimated_natural_seconds"] = json!(999.0);

    let parsed = parse_stored_video_shot_analysis(&stored.to_string(), &valid_source_spans(), 6)
        .expect("stored shot analysis duration-derived fields should be rederived");

    assert_eq!(parsed.cinematic_shots[0].estimated_natural_seconds, 6.0);
}

#[test]
fn rejects_stored_video_shot_analysis_with_missing_derived_fields() {
    let analysis =
        parse_valid_shot_analysis(&valid_shot_analysis_value()).expect("raw analysis parses");
    let mut stored = serde_json::to_value(&analysis).expect("stored analysis serializes");
    stored["cinematic_shots"][0]["action_beats"][0]
        .as_object_mut()
        .expect("action beat")
        .remove("beat_type");

    let err = parse_stored_video_shot_analysis(&stored.to_string(), &valid_source_spans(), 6)
        .expect_err("stored shot analysis must reject missing canonical derived fields");

    assert!(err.to_string().contains("missing field `beat_type`"));
}

#[test]
fn rejects_stored_video_shot_analysis_with_unknown_field() {
    let analysis =
        parse_valid_shot_analysis(&valid_shot_analysis_value()).expect("raw analysis parses");
    let mut stored = serde_json::to_value(&analysis).expect("stored analysis serializes");
    stored["cinematic_shots"][0]["legacy_notes"] = json!("旧字段");

    let err = parse_stored_video_shot_analysis(&stored.to_string(), &valid_source_spans(), 6)
        .expect_err("stored shot analysis must reject unknown fields");

    assert!(err.to_string().contains("unknown field `legacy_notes`"));
}

#[test]
fn rejects_video_shot_analysis_with_model_owned_source_span_range() {
    let mut value = valid_shot_analysis_value();
    value["cinematic_shots"][0]["source_span_range"] =
        json!({"start": "ch1:s1:f1", "end": "ch1:s1:f1"});

    let err = parse_valid_shot_analysis(&value)
        .expect_err("source_span_range must be derived by the backend");

    assert!(
        err.to_string()
            .contains("unknown field `source_span_range`")
    );
}

#[test]
fn rejects_video_beat_extraction_with_extra_action_beat_description() {
    let mut value = valid_beat_extraction_value();
    value["beats"][0]["description"] = json!("模型额外输出的节拍说明");

    let err = parse_valid_beats(&value).expect_err("extra action beat fields should be rejected");

    assert!(err.to_string().contains("unknown field `description`"));
}

#[test]
fn rejects_video_beat_extraction_with_model_owned_beat_id() {
    let mut value = valid_beat_extraction_value();
    value["beats"][0]["beat_id"] = json!(1);

    let err = parse_valid_beats(&value).expect_err("beat_id must be derived by the backend");

    assert!(err.to_string().contains("unknown field `beat_id`"));
}

#[test]
fn rejects_video_beat_extraction_with_model_owned_beat_type() {
    let mut value = valid_beat_extraction_value();
    value["beats"][0]["beat_type"] = json!("movement");

    let err = parse_valid_beats(&value).expect_err("beat_type must be inferred by the backend");

    assert!(err.to_string().contains("unknown field `beat_type`"));
}

#[test]
fn beat_extraction_eof_json_error_mentions_truncated_response() {
    let err = parse_video_beat_extraction_response(
        r#"{"beats":[{"natural_seconds":2.0,"required":true,"visible_action":"角色停顿","source_span_ids":["ch1:s1:f1"]}"#,
        &valid_source_spans(),
        1,
        "测试动作节拍抽取",
    )
    .expect_err("incomplete JSON must fail explicitly");

    let message = err.to_string();
    assert!(message.contains("响应在 JSON 闭合前结束"));
    assert!(message.contains("输出预览"));
}

#[test]
fn rejects_video_beat_extraction_with_invalid_beat_duration() {
    let mut value = valid_beat_extraction_value();
    value["beats"][0]["natural_seconds"] = json!(0);

    let err = parse_valid_beats(&value).expect_err("beat duration must be positive");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("natural_seconds")
                && message.contains("大于 0")
    ));
}

#[test]
fn validates_segment_plan_against_shot_analysis() {
    let shot_analysis =
        parse_valid_shot_analysis(&valid_shot_analysis_value()).expect("valid analysis");
    let segment_plan = parse_valid_segment_plan(&valid_segment_plan_value()).expect("valid plan");

    validate_segment_plan_against_shot_analysis(&segment_plan, &shot_analysis)
        .expect("segment plan should match shot analysis");
}

#[test]
fn rejects_segment_plan_with_model_owned_segment_index() {
    let mut plan_value = valid_segment_plan_value();
    plan_value["segments"][0]["segment_index_in_shot"] = json!(2);

    let err = parse_valid_segment_plan(&plan_value)
        .expect_err("segment_index_in_shot must be backend-derived");

    assert!(
        err.to_string()
            .contains("unknown field `segment_index_in_shot`")
    );
}

#[test]
fn rejects_segment_plan_with_model_owned_duration_secs() {
    let mut plan_value = valid_segment_plan_value();
    plan_value["segments"][0]["duration_secs"] = json!(8);

    let err =
        parse_valid_segment_plan(&plan_value).expect_err("duration_secs must be backend-derived");

    assert!(err.to_string().contains("unknown field `duration_secs`"));
}

#[test]
fn accepts_segment_plan_when_variable_duration_reduces_default_segment_count() {
    let mut beats = valid_extracted_beats();
    beats[1].natural_seconds = 4.0;
    let shot_analysis = parse_video_shot_grouping_response(
        &valid_shot_analysis_value().to_string(),
        &valid_source_spans(),
        6,
        &beats,
    )
    .expect("valid analysis");
    let segment_plan =
        parse_valid_segment_plan(&valid_segment_plan_value()).expect("plan shape is valid");

    assert_eq!(shot_analysis.cinematic_shots[0].required_segment_count, 2);
    validate_segment_plan_against_shot_analysis(&segment_plan, &shot_analysis)
        .expect("one longer variable-duration segment can cover the required beats");
}

#[test]
fn rejects_segment_plan_missing_required_beat_coverage() {
    let shot_analysis =
        parse_valid_shot_analysis(&valid_shot_analysis_value()).expect("valid analysis");
    let mut plan_value = valid_segment_plan_value();
    plan_value["segments"][0]["covered_beat_ids"] = json!([1]);
    let segment_plan = parse_valid_segment_plan(&plan_value).expect("plan shape is valid");

    let err = validate_segment_plan_against_shot_analysis(&segment_plan, &shot_analysis)
        .expect_err("required beat coverage is required");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("必需动作节拍")
                && message.contains("beat_id 2")
    ));
}

#[test]
fn parses_video_prompt_response_without_model_owned_system_fields() {
    let raw = response_from_shots(vec![valid_shot_value(1)]);
    let shots = parse_with_valid_segments(&raw).expect("valid response");
    assert_eq!(shots.len(), 1);
    assert_eq!(shots[0].shot_number, 1);
    assert!(shots[0].visual_prompt.starts_with("[主体定义与素材绑定]"));
    assert_eq!(shots[0].duration_secs, 6);
    assert_eq!(shots[0].aspect_ratio, "16:9");
    assert_eq!(shots[0].reference_image_ids, vec![1, 2]);
}

#[test]
fn derived_prompt_fields_use_concise_performance_summary() {
    let raw = response_from_shots(vec![valid_shot_value(1)]);
    let shots = parse_with_valid_segments(&raw).expect("valid response");
    let movement = shots[0]
        .blocking_plan
        .get("movement")
        .and_then(Value::as_str)
        .expect("blocking movement should be text");

    assert!(movement.contains("从档案室门口偏左向画面深处书桌慢走"));
    assert!(movement.contains("肩膀略收紧"));
    assert!(
        !movement.contains('{') && !movement.contains("movement_path"),
        "movement should not dump performance_plan JSON: {movement}"
    );
    assert!(
        shots[0].blocking_plan.get("performance_plan").is_none(),
        "blocking_plan should not duplicate the full performance_plan object"
    );
    assert_eq!(shots[0].start_state["character"], "主角");
    for (field, value) in [
        ("start_state.pose", &shots[0].start_state["pose"]),
        (
            "transition_in.first_beat_action",
            &shots[0].transition_in["first_beat_action"],
        ),
        (
            "continuity_check.emotion",
            &shots[0].continuity_check["emotion"],
        ),
        (
            "reference_binding.coverage_check",
            &shots[0].reference_binding["coverage_check"],
        ),
    ] {
        let text = value.as_str().expect("derived field should be text");
        assert!(
            !text.contains('{') && !text.contains("\":"),
            "{field} should be concise natural language, got: {text}"
        );
    }
}

#[test]
fn parses_video_prompt_response_with_empty_shot_specific_negative_prompt() {
    let mut shot = valid_shot_value(1);
    shot["negative_prompt"] = json!("");
    let raw = response_from_shots(vec![shot]);

    let shots = parse_with_valid_segments(&raw).expect("empty shot-specific negative is valid");

    assert_eq!(shots[0].negative_prompt, "");
}

#[test]
fn rejects_video_prompt_with_internal_image_id_in_direct_text() {
    let mut shot = valid_shot_value(1);
    shot["visual_prompt"] = json!(
        "[主体定义与素材绑定]\n参考图（image_id=9）约束档案大厅空间，主角站在门口偏左，木质书架和中央书桌保持稳定关系，暖色窗光从画面左侧斜照，画面从门口宽景开始并在书桌旁稳定收束，整体保持连续的电影感写实风格。\n\n[片段内容以及前因后果]\n主角进入档案室并保持谨慎。\n\n[镜头分镜]\n镜头 1：门口宽中景建立书架和书桌关系，主角站在门口偏左并停顿观察。\n镜头 2：平视中景缓慢推进，主角沿书架走向书桌并在结尾停住。\n\n[画面风格描述]\n电影感写实。\n\n[连续性与结尾状态]\n结尾停在书桌旁。\n\n[限制与避免项]\n不要配背景音乐。"
    );
    let raw = response_from_shots(vec![shot]);

    let err = parse_with_valid_segments(&raw)
        .expect_err("direct prompt text must not expose internal image ids");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("visual_prompt")
                && message.contains("内部图片 ID")
    ));
}

#[test]
fn parses_video_prompt_with_self_harm_intent_in_direct_text() {
    let mut shot = valid_shot_value(1);
    shot["motion_prompt"] = json!(
        "[运镜与动作]\n镜头从门口中景缓慢推近，主角站在门口，手指轻触门框并深呼吸，表情从迟疑转为决绝。\n\n[对白与表演]\n无对白。"
    );
    let raw = response_from_shots(vec![shot]);

    let rows = parse_with_valid_segments(&raw)
        .expect("self-harm wording is left to prompt optimization instead of hard validation");

    assert_eq!(rows.len(), 1);
}

#[test]
fn parses_video_prompt_with_explicit_self_harm_execution_in_direct_text() {
    let mut shot = valid_shot_value(1);
    shot["motion_prompt"] = json!(
        "[运镜与动作]\n镜头从门口中景缓慢推近，主角站在门口，手指轻触门框并深呼吸，然后把铁片刺入眼睛，镜头推近到面部后稳定停住。\n\n[对白与表演]\n无对白。"
    );
    let raw = response_from_shots(vec![shot]);

    let rows = parse_with_valid_segments(&raw).expect(
        "self-harm execution wording is left to prompt optimization instead of hard validation",
    );

    assert_eq!(rows.len(), 1);
}

#[test]
fn parses_video_prompt_response_with_empty_dialogue_beats() {
    let mut shot = valid_shot_value(1);
    shot["dialogue_cues"] = json!([]);
    let raw = response_from_shots(vec![shot]);

    let shots = parse_with_valid_segments(&raw).expect("empty dialogue beats are valid");

    assert!(
        shots[0]
            .dialogue_beats
            .as_array()
            .is_some_and(Vec::is_empty)
    );
}

#[test]
fn rejects_video_prompt_response_missing_dialogue_cues() {
    let mut shot = valid_shot_value(1);
    shot.as_object_mut()
        .expect("shot object")
        .remove("dialogue_cues");
    let raw = response_from_shots(vec![shot]);

    let err = parse_with_valid_segments(&raw)
        .expect_err("dialogue_cues must be explicit, even when empty");

    assert!(err.to_string().contains("missing field `dialogue_cues`"));
}

#[test]
fn parses_video_prompt_response_with_long_dialogue_line() {
    let mut shot = valid_shot_value(1);
    shot["dialogue_cues"][0]["line"] = json!(
        "这是一句被错误完整复制进视频提示词的超长对白，超过四十个中文字符，会让视频模型误以为需要生成连续字幕或复杂口播"
    );
    let raw = response_from_shots(vec![shot]);

    let shots = parse_with_valid_segments(&raw).expect("dialogue length is a prompt quality rule");

    assert!(
        shots[0].dialogue_beats[0]["line"]
            .as_str()
            .is_some_and(|line| line.contains("超长对白"))
    );
}

#[test]
fn derives_dialogue_source_span_ids_from_current_segment() {
    let raw = response_from_shots(vec![valid_shot_value(1)]);

    let shots = parse_with_valid_segments(&raw)
        .expect("dialogue source spans should be derived from current segment");

    assert_eq!(
        shots[0].dialogue_beats[0]["source_span_ids"],
        json!(["ch1:s1:f1"])
    );
}

#[test]
fn revalidates_internal_dialogue_source_span_ids_from_stored_prompt() {
    let raw = response_from_shots(vec![valid_shot_value(1)]);
    let mut shots = parse_with_valid_segments(&raw)
        .expect("model output should be normalized with backend source spans");
    let segments = valid_segments();
    let segment_by_number = segment_map(&segments);
    let segment = segment_by_number.get(&1).expect("segment 1");

    let shot = validate_video_prompt_shot_against_segment(
        shots.remove(0),
        segment,
        PromptOutputLanguage::ZhCn,
    )
    .expect("stored normalized video prompt should be revalidatable");

    assert_eq!(
        shot.dialogue_beats[0]["source_span_ids"],
        json!(["ch1:s1:f1"])
    );
}

#[test]
fn rejects_model_owned_dialogue_source_span_id() {
    let mut shot = valid_shot_value(1);
    shot["dialogue_cues"][0]["source_span_id"] = json!("ch1:s1:f2");
    let raw = response_from_shots(vec![shot]);

    let err = parse_with_valid_segments(&raw)
        .expect_err("dialogue source_span_id is an internal backend-derived field");

    assert!(
        err.to_string()
            .contains("视频提示词 dialogue_cues[1] 不要输出 source_span_id/source_span_ids")
    );
}

#[test]
fn rejects_model_owned_dialogue_source_span_ids() {
    let mut shot = valid_shot_value(1);
    shot["dialogue_cues"][0]["source_span_ids"] = json!(["ch1:s1:f1"]);
    let raw = response_from_shots(vec![shot]);

    let err = parse_with_valid_segments(&raw)
        .expect_err("dialogue source_span_ids is an internal backend-derived field");

    assert!(
        err.to_string()
            .contains("视频提示词 dialogue_cues[1] 不要输出 source_span_id/source_span_ids")
    );
}

#[test]
fn rejects_dialogue_extra_field() {
    let mut shot = valid_shot_value(1);
    shot["dialogue_cues"][0]["dialogue_internal_note"] = json!("模型额外字段");
    let raw = response_from_shots(vec![shot]);

    let err =
        parse_with_valid_segments(&raw).expect_err("dialogue beats must be a strict whitelist");

    assert!(
        err.to_string()
            .contains("视频提示词 dialogue_cues[1].dialogue_internal_note 不是允许字段")
    );
}

#[test]
fn rejects_video_prompt_response_with_model_owned_system_fields() {
    let mut shot = valid_shot_value(1);
    shot["temporal_beats"] = json!([
        {"start_sec": 0, "end_sec": 6, "camera": "宽镜头", "subject_action": "主角移动", "continuity_target": "书桌旁"}
    ]);
    let raw = response_from_shots(vec![shot]);

    assert!(parse_with_valid_segments(&raw).is_err());
}

#[test]
fn rejects_video_prompt_response_with_raw_newline_inside_string() {
    let raw = response_from_shots(vec![valid_shot_value(1)]).replace(
        "[主体定义与素材绑定]\\n参考图 1",
        "[主体定义与素材绑定]\n参考图 1",
    );

    assert!(parse_with_valid_segments(&raw).is_err());
}

#[test]
fn rejects_invalid_json() {
    assert!(parse_with_valid_segments("not json").is_err());
}

#[test]
fn rejects_markdown_fenced_json() {
    let raw = format!(
        "```json\n{}\n```",
        response_from_shots(vec![valid_shot_value(1)])
    );
    assert!(parse_with_valid_segments(&raw).is_err());
}

#[test]
fn rejects_missing_required_fields() {
    let raw = r#"{"shots":[{"title":"Opening"}]}"#;
    assert!(parse_with_valid_segments(raw).is_err());
}

#[test]
fn rejects_video_prompt_with_model_owned_shot_number() {
    let mut shot = valid_shot_value(1);
    shot["shot_number"] = json!(1);
    let raw = response_from_shots(vec![shot]);
    let err = parse_with_valid_segments(&raw).expect_err("shot_number must be backend-derived");
    assert!(err.to_string().contains("unknown field `shot_number`"));
}

#[test]
fn rejects_video_prompt_when_shot_number_cannot_be_derived() {
    let raw = response_from_shots(vec![valid_shot_value(1), valid_shot_value(1)]);
    let err = parse_with_valid_segments(&raw)
        .expect_err("multiple returned shots cannot map to one segment");
    assert!(err.to_string().contains("无法从当前批次和片段数量唯一推导"));
}

#[test]
fn parses_chat_video_prompt_with_source_span_ids() {
    let raw = response_from_shots(vec![valid_chat_video_shot_value(1)]);

    let shots = parse_chat_with_valid_segments(&raw).expect("valid chat prompt");

    assert_eq!(shots[0].shot_number, 1);
    assert_eq!(shots[0].chat_video["source_span_ids"][0], "ch1:s1:f1");
    assert_eq!(shots[0].reference_image_ids, vec![1, 2]);
    assert_eq!(shots[0].chat_video["input_images"][1]["image_id"], 2);
    let action = shots[0].chat_video["timeline"][0]["action"]
        .as_str()
        .expect("chat timeline action should be text");
    assert!(
        !action.contains('{') && !action.contains("movement_path"),
        "chat timeline should not dump performance_plan JSON: {action}"
    );
    assert!(
        shots[0].chat_video.get("performance_plan").is_none(),
        "chat_video should not duplicate the full performance_plan object"
    );
}

#[test]
fn rejects_chat_video_prompt_without_no_background_music_rule() {
    let mut shot = valid_chat_video_shot_value(1);
    let prompt = shot["chat_prompt"]
        .as_str()
        .unwrap()
        .replace("，不要配背景音乐", "");
    shot["chat_prompt"] = json!(prompt);
    let raw = response_from_shots(vec![shot]);

    let err = parse_chat_with_valid_segments(&raw)
        .expect_err("chat prompt should forbid missing background music rule");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("不要配背景音乐")
                || message.contains("背景音乐")
    ));
}

#[test]
fn rejects_chat_video_prompt_when_no_background_music_rule_is_outside_avoidance_section() {
    let mut shot = valid_chat_video_shot_value(1);
    let prompt = shot["chat_prompt"]
        .as_str()
        .unwrap()
        .replace(
            "主角进入档案室并保持谨慎。",
            "主角进入档案室并保持谨慎，不要配背景音乐。",
        )
        .replace("，不要配背景音乐", "");
    shot["chat_prompt"] = json!(prompt);
    let raw = response_from_shots(vec![shot]);

    let err = parse_chat_with_valid_segments(&raw)
        .expect_err("no-background-music rule must appear in the avoidance section");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("限制与避免项")
                && (message.contains("不要配背景音乐") || message.contains("背景音乐"))
    ));
}

#[test]
fn parses_video_segment_plan_with_intent_arc_and_material_coverage() {
    let plan = valid_segment_plan_value();

    let segments = parse_valid_segment_plan(&plan).expect("segment plan should parse");

    assert_eq!(segments.len(), 1);
    assert_eq!(segments[0].video_intent["hook"], "主角第一次进入异常档案室");
    assert_eq!(segments[0].emotional_arc["function"], "场景建立");
    assert_eq!(
        segments[0].performance_plan["movement_path"],
        "主角从档案室门口偏左进入，沿书架左侧向画面深处书桌慢走，并在书桌旁停住。"
    );
    assert!(segments[0].performance_plan["action_script"].is_array());
    assert!(segments[0].material_coverage["risk_note"].is_string());
    assert!(
        segments[0].reference_image_ids.is_empty(),
        "片段规划阶段不绑定具体参考图"
    );
}

#[test]
fn rejects_video_segment_plan_with_duplicate_object_field_hint() {
    let raw = r#"{
      "segments": [
        {
          "analysis_shot_id": 1,
          "covered_beat_ids": [1],
          "title": "开场",
          "scene_summary": "主角进入档案室",
          "blocking_summary": "主角从档案室门口走向书桌",
          "spatial_anchors": [],
          "emotion_action_goals": [],
          "video_intent": {},
          "emotional_arc": {"start": "谨慎"},
          "emotional_arc": {"end": "戒备"},
          "performance_plan": {},
          "material_coverage": {}
        }
      ]
    }"#;

    let err = parse_video_segment_plan_input_values(raw, "视频分段规划 镜头 #2")
        .expect_err("duplicate model-owned fields must be rejected");
    let message = err.to_string();

    assert!(message.contains("duplicate field `emotional_arc`"));
    assert!(message.contains("重复字段"));
    assert!(message.contains("每个字段名只能出现一次"));
}

#[test]
fn rejects_missing_video_segment_plan_performance_plan() {
    let mut plan = valid_segment_plan_value();
    plan["segments"][0]
        .as_object_mut()
        .expect("segment should be object")
        .remove("performance_plan");

    let err = parse_valid_segment_plan(&plan)
        .expect_err("segment plan must fail when performance_plan is missing");

    assert!(err.to_string().contains("missing field `performance_plan`"));
}

#[test]
fn rejects_segment_plan_time_block_empty_camera() {
    let mut plan = valid_segment_plan_value();
    plan["segments"][0]["performance_plan"]["time_blocks"][1]["camera"] = json!("   ");

    let err = parse_valid_segment_plan(&plan)
        .expect_err("segment plan must fail when performance_plan time block fields are empty");
    assert!(
        err.to_string()
            .contains("视频分段规划 performance_plan.time_blocks[2].camera 不能为空")
    );
}

#[test]
fn rejects_segment_plan_time_blocks_exceeding_duration() {
    let mut plan = valid_segment_plan_value();
    plan["segments"][0]["performance_plan"]["time_blocks"][1]["end_sec"] = json!(8);

    let err = parse_valid_segment_plan(&plan)
        .expect_err("segment plan must fail before deriving video prompt temporal_beats");

    assert!(
        err.to_string().contains(
            "视频分段规划 performance_plan.time_blocks[2].end_sec 不能超过 duration_secs"
        )
    );
}

#[test]
fn rejects_segment_plan_action_script_empty_hands() {
    let mut plan = valid_segment_plan_value();
    plan["segments"][0]["performance_plan"]["action_script"][0]["hands"] = json!("   ");

    let err = parse_valid_segment_plan(&plan)
        .expect_err("segment plan must fail when action_script hands is empty");

    assert!(
        err.to_string()
            .contains("视频分段规划 performance_plan.action_script[1].hands 不能为空")
    );
}

#[test]
fn rejects_stored_video_segment_plan_with_legacy_reference_image_ids() {
    let plan = valid_segment_plan_value();
    let segments = parse_valid_segment_plan(&plan).expect("segment plan should parse");
    let mut stored = json!({ "segments": segments });
    stored["segments"][0]["reference_image_ids"] = json!([3, 9]);

    let err = parse_stored_video_segment_plan(&stored.to_string(), &valid_source_spans())
        .expect_err("old segment-level reference bindings must not be reused");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("reference_image_ids")
                && message.contains("片段规划不再绑定具体素材")
    ));
}

#[test]
fn rejects_retargeting_stored_video_segment_plan_for_resume() {
    let plan = valid_segment_plan_value();
    let segments = parse_valid_segment_plan(&plan).expect("segment plan should parse");
    let mut stored = json!({ "segments": segments });
    stored["segments"][0]["duration_secs"] = json!(12);
    stored["segments"][0]["aspect_ratio"] = json!("4:3");
    stored["segments"][0]["reference_image_ids"] = json!([3, 9]);

    let err = parse_stored_video_segment_plan_for_reuse(
        &stored.to_string(),
        &valid_source_spans(),
        "9:16",
    )
    .expect_err("stored segment plan must not be retargeted to current options");

    assert!(
        err.to_string()
            .contains("aspect_ratio 必须等于当前请求 9:16")
    );
}

#[test]
fn reuses_stored_video_segment_plan_with_different_duration() {
    let plan = valid_segment_plan_value();
    let segments = parse_valid_segment_plan(&plan).expect("segment plan should parse");
    let mut stored = json!({ "segments": segments });
    stored["segments"][0]["duration_secs"] = json!(12);
    stored["segments"][0]["performance_plan"]["time_blocks"][0]["end_sec"] = json!(6);
    stored["segments"][0]["performance_plan"]["time_blocks"][1]["start_sec"] = json!(6);
    stored["segments"][0]["performance_plan"]["time_blocks"][1]["end_sec"] = json!(12);

    let parsed = parse_stored_video_segment_plan_for_reuse(
        &stored.to_string(),
        &valid_source_spans(),
        "16:9",
    )
    .expect("duration is planning context and should not block segment plan reuse");

    assert_eq!(parsed[0].duration_secs, 12);
}

#[test]
fn rejects_reusing_stored_video_segment_plan_with_stale_source_span_ids() {
    let plan = valid_segment_plan_value();
    let segments = parse_valid_segment_plan(&plan).expect("segment plan should parse");
    let mut stored = json!({ "segments": segments });
    stored["segments"][0]["source_span_ids"] = json!(["old:span"]);

    let err = parse_stored_video_segment_plan_for_reuse(
        &stored.to_string(),
        &valid_source_spans(),
        "16:9",
    )
    .expect_err("stale source span ids must fail instead of being retargeted");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("source_span_ids 包含不在本次输入候选中的 ID：old:span")
    ));
}

#[test]
fn rejects_video_segment_plan_without_directing_fields() {
    let mut plan = valid_segment_plan_value();
    let segment = plan["segments"][0].as_object_mut().expect("segment object");
    segment.remove("video_intent");
    segment.remove("emotional_arc");
    segment.remove("material_coverage");

    let err = parse_valid_segment_plan(&plan)
        .expect_err("segment plan must include current directing fields");

    assert!(err.to_string().contains("video_intent"));
}

#[test]
fn rejects_video_segment_plan_with_incomplete_directing_fields() {
    let mut plan = valid_segment_plan_value();
    plan["segments"][0]["video_intent"]
        .as_object_mut()
        .expect("video_intent object")
        .remove("visual_priority");
    plan["segments"][0]["emotional_arc"]["end"] = json!("   ");

    let err = parse_valid_segment_plan(&plan)
        .expect_err("segment plan directing fields must match the contract");

    assert!(
        err.to_string()
            .contains("视频分段规划 video_intent.visual_priority 必须是非空字符串")
    );
}

#[test]
fn rejects_video_segment_plan_with_incomplete_spatial_anchor() {
    let mut plan = valid_segment_plan_value();
    plan["segments"][0]["spatial_anchors"][0]
        .as_object_mut()
        .expect("spatial anchor object")
        .remove("usage");

    let err = parse_valid_segment_plan(&plan).expect_err("spatial anchors must match the contract");

    assert!(
        err.to_string()
            .contains("视频分段规划 spatial_anchors[1].usage 必须是非空字符串")
    );
}

#[test]
fn rejects_video_segment_plan_with_incomplete_emotion_action_goal() {
    let mut plan = valid_segment_plan_value();
    plan["segments"][0]["emotion_action_goals"][0]["visible_action"] = json!("");

    let err =
        parse_valid_segment_plan(&plan).expect_err("emotion action goals must match the contract");

    assert!(
        err.to_string()
            .contains("视频分段规划 emotion_action_goals[1].visible_action 不能为空")
    );
}

#[test]
fn rejects_video_segment_plan_with_empty_material_coverage() {
    let mut plan = valid_segment_plan_value();
    plan["segments"][0]["material_coverage"] = json!({});

    let err = parse_valid_segment_plan(&plan).expect_err("material coverage is required");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("material_coverage") && message.contains("不能为空")
    ));
}

#[test]
fn rejects_video_segment_plan_with_incomplete_material_coverage() {
    let mut plan = valid_segment_plan_value();
    plan["segments"][0]["material_coverage"]
        .as_object_mut()
        .expect("material coverage object")
        .remove("risk_note");

    let err =
        parse_valid_segment_plan(&plan).expect_err("material coverage must match the contract");

    assert!(
        err.to_string()
            .contains("视频分段规划 material_coverage.risk_note 必须是非空字符串")
    );
}

#[test]
fn rejects_video_segment_plan_with_material_coverage_reference_ids() {
    let mut plan = valid_segment_plan_value();
    plan["segments"][0]["material_coverage"]["covered_by_reference_image_ids"] =
        json!([{"image_id": 3, "entity": "主角"}]);

    let err = parse_valid_segment_plan(&plan)
        .expect_err("segment material coverage must not bind concrete images");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("covered_by_reference_image_ids")
                && message.contains("不绑定具体图片 ID")
    ));
}

#[test]
fn rejects_video_segment_plan_with_model_owned_reference_image_ids() {
    let mut plan = valid_segment_plan_value();
    plan["segments"][0]["reference_image_ids"] = json!([3, 9]);

    let err = parse_valid_segment_plan(&plan)
        .expect_err("segment-level references must not be output by the model");

    assert!(
        err.to_string()
            .contains("unknown field `reference_image_ids`")
    );
}

#[test]
fn parses_video_step_json_object() {
    let value = parse_video_step_object(r#"{"mood":"紧张"}"#, "chapter_video_segment_plan")
        .expect("object should parse");

    assert_eq!(value["mood"], "紧张");
}

#[test]
fn rejects_video_step_json_object_with_raw_newline_inside_string() {
    assert!(
        parse_video_step_object(
            r#"{"mood":"紧张
压抑"}"#,
            "chapter_video_first_frame_video_prompt",
        )
        .is_err()
    );
}

#[test]
fn rejects_video_step_plain_text() {
    assert!(parse_video_step_object("not json", "chapter_video_segment_plan").is_err());
}

#[test]
fn rejects_video_step_array() {
    assert!(parse_video_step_object(r#"[]"#, "chapter_video_segment_plan").is_err());
}
