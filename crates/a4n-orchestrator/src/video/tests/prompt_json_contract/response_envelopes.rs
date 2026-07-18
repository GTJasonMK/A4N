#[test]
fn rejects_non_json_video_frame_prompt_response_with_preview() {
    let defaults = [VideoFramePromptDefaults {
        frame_number: 1,
        duration_secs: 6,
        aspect_ratio: "16:9",
        segment_numbers: vec![1],
        reference_image_ids: vec![],
    }];
    let err = parse_video_frame_prompts_with_defaults(
        "我无法生成这个首尾帧提示词。",
        &defaults,
        PromptOutputLanguage::ZhCn,
    )
    .expect_err("non-json LLM output must fail with a diagnostic preview");

    let message = err.to_string();
    assert!(message.contains("视频首尾帧提示词 响应必须是严格 JSON 对象"));
    assert!(message.contains("输出预览"));
    assert!(message.contains("我无法生成这个首尾帧提示词"));
}

#[test]
fn rejects_video_frame_prompt_response_with_visible_thinking_tag() {
    let defaults = [VideoFramePromptDefaults {
        frame_number: 1,
        duration_secs: 6,
        aspect_ratio: "16:9",
        segment_numbers: vec![1],
        reference_image_ids: vec![],
    }];
    let err = parse_video_frame_prompts_with_defaults(
        "<antml thinking>\n让我逐步分析这个任务。\n</antml thinking>\n{\"frames\": []}",
        &defaults,
        PromptOutputLanguage::ZhCn,
    )
    .expect_err("visible thinking tags must not be stripped before strict JSON parsing");

    let message = err.to_string();
    assert!(message.contains("thinking/reasoning 标签"));
    assert!(message.contains("没有遵守严格 JSON response_format"));
    assert!(message.contains("输出预览：<antml thinking>"));
}

#[test]
fn rejects_complete_markdown_fenced_video_frame_prompt_response() {
    let defaults = [VideoFramePromptDefaults {
        frame_number: 1,
        duration_secs: 6,
        aspect_ratio: "16:9",
        segment_numbers: vec![1],
        reference_image_ids: vec![3],
    }];
    let err = parse_video_frame_prompts_with_defaults(
        r#"```json
{
  "frames": [{
    "title": "门前",
    "scene_summary": "角色推门进入",
    "frame_prompt": "夜晚室内外交界处的紧闭木门占据画面中央，角色站在门前偏左，半侧身面向木门并伸出右手准备推门，袖口和手部清晰可见，门框、铜环和地面阴影作为固定参照物，月光从画面左侧落下，整体保持悬疑写实风格和稳定构图。",
    "blocking_state": {"characters": [{"name": "角色", "position": "门前", "facing": "面向门", "pose": "伸手"}], "spatial_anchors": ["门"], "do_not_change": ["门在人物前方"]},
    "lighting_state": {"time_of_day": "夜晚", "key_light": "月光", "direction": "画面左侧", "shadow": "柔和阴影", "intent": "紧张"},
    "negative_prompt": "低清晰度、画面模糊、人体结构扭曲、多余手指、面部不一致、空间关系错误"
  }]
}
```"#,
        &defaults,
        PromptOutputLanguage::ZhCn,
    )
    .expect_err("markdown-fenced JSON must violate the strict response contract");

    let message = err.to_string();
    assert!(message.contains("视频首尾帧提示词 响应必须是严格 JSON 对象"));
    assert!(message.contains("Markdown 代码围栏"));
    assert!(message.contains("不能用 ``` 或 ```json 包裹"));
    assert!(message.contains("输出预览：```json"));
}

#[test]
fn rejects_invalid_markdown_fenced_video_frame_prompt_response_with_hint() {
    let defaults = [VideoFramePromptDefaults {
        frame_number: 1,
        duration_secs: 6,
        aspect_ratio: "16:9",
        segment_numbers: vec![1],
        reference_image_ids: vec![],
    }];
    let err = parse_video_frame_prompts_with_defaults(
        "```json\n不是 JSON\n```",
        &defaults,
        PromptOutputLanguage::ZhCn,
    )
    .expect_err("invalid markdown-fenced LLM output must fail with a direct hint");

    let message = err.to_string();
    assert!(message.contains("Markdown 代码围栏"));
    assert!(message.contains("不能用 ``` 或 ```json 包裹"));
    assert!(message.contains("输出预览：```json"));
}

#[test]
fn rejects_markdown_fenced_video_frame_prompt_response_with_explanatory_text() {
    let defaults = [VideoFramePromptDefaults {
        frame_number: 1,
        duration_secs: 6,
        aspect_ratio: "16:9",
        segment_numbers: vec![1],
        reference_image_ids: vec![],
    }];
    let err = parse_video_frame_prompts_with_defaults(
        "```json\n{\"frames\": []}\n```\n这是结果",
        &defaults,
        PromptOutputLanguage::ZhCn,
    )
    .expect_err("markdown-fenced output with external prose must stay invalid");

    let message = err.to_string();
    assert!(message.contains("Markdown 代码围栏"));
    assert!(message.contains("不能用 ``` 或 ```json 包裹"));
    assert!(message.contains("输出预览：```json"));
}

#[test]
fn static_frame_contract_rejects_storyboard_panels() {
    let defaults = [VideoFramePromptDefaults {
        frame_number: 1,
        duration_secs: 6,
        aspect_ratio: "16:9",
        segment_numbers: vec![1],
        reference_image_ids: vec![],
    }];
    let err = parse_video_frame_prompts_with_defaults(
        r#"{
          "frames": [{
            "title": "门前",
            "scene_summary": "角色停在门前",
            "storyboard_panels": [],
            "frame_prompt": "单张静态画面",
            "blocking_state": {},
            "lighting_state": {},
            "negative_prompt": "文字、水印"
          }]
        }"#,
        &defaults,
        PromptOutputLanguage::ZhCn,
    )
    .expect_err("static frame output must not accept storyboard-only fields");

    assert!(
        err.to_string()
            .contains("unknown field `storyboard_panels`")
    );
}

#[test]
fn storyboard_contract_requires_storyboard_panels() {
    let defaults = [VideoFramePromptDefaults {
        frame_number: 1,
        duration_secs: 6,
        aspect_ratio: "16:9",
        segment_numbers: vec![1],
        reference_image_ids: vec![],
    }];
    let err = parse_storyboard_frame_prompts_with_defaults(
        r#"{
          "frames": [{
            "title": "门前分镜",
            "scene_summary": "角色推门",
            "frame_prompt": "两格分镜板",
            "blocking_state": {},
            "lighting_state": {},
            "negative_prompt": "文字、水印"
          }]
        }"#,
        &defaults,
        PromptOutputLanguage::ZhCn,
    )
    .expect_err("storyboard output must include storyboard_panels");

    assert!(
        err.to_string()
            .contains("missing field `storyboard_panels`")
    );
}
