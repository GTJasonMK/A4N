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
