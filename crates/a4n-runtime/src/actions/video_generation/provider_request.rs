use super::*;

mod prompt_rendering;

pub(super) use self::prompt_rendering::{
    build_video_generation_provider_preview_prompt_with_binding,
    build_video_generation_provider_prompt_with_binding, provider_specific_negative_prompt,
};

pub(super) fn chapter_video_provider_request_snapshot(
    context: &ChapterVideoProviderRequestContext,
) -> Result<Value, String> {
    let mut snapshot_context = context.clone();
    snapshot_context.request = sanitized_chapter_video_request_for_snapshot(context)?;
    let (request_kind, request_title, request_note, request) =
        video_provider_preview_payload(&snapshot_context)?;
    Ok(provider_request_snapshot_value(
        "chapter_video",
        request_title,
        request_kind,
        Some("chapter_video_prompt"),
        Some(request_note),
        request,
    ))
}

fn sanitized_chapter_video_request_for_snapshot(
    context: &ChapterVideoProviderRequestContext,
) -> Result<VideoGenerationRequest, String> {
    let reference_ids = media::reference_image_ids(context.provider_reference_image_ids.as_ref())?;
    let first_frame_image_url = context
        .boundary_frame_ids
        .first
        .filter(|_| context.request.first_frame_image_url.is_some())
        .map(|id| historical_image_placeholder_media_url(id, "首帧"));
    let last_frame_image_url = context
        .boundary_frame_ids
        .last
        .filter(|_| context.request.last_frame_image_url.is_some())
        .map(|id| historical_image_placeholder_media_url(id, "尾帧"));
    let reference_image_urls = reference_ids
        .into_iter()
        .enumerate()
        .take(context.request.reference_image_urls.len())
        .map(|(index, id)| {
            let role = if context.workflow_mode == "storyboard" && index == 0 {
                "分镜图"
            } else {
                "参考图"
            };
            historical_image_placeholder_media_url(id, role)
        })
        .collect();

    Ok(VideoGenerationRequest {
        prompt: context.request.prompt.clone(),
        negative_prompt: context.request.negative_prompt.clone(),
        width: context.request.width,
        height: context.request.height,
        duration_secs: context.request.duration_secs,
        fps: context.request.fps,
        model: context.request.model.clone(),
        first_frame_image_url,
        last_frame_image_url,
        reference_image_urls,
        reference_media: None,
    })
}

fn historical_image_placeholder_media_url(id: i64, role: &str) -> String {
    format!(
        "https://example.invalid/a4n-media/<omitted for historical provider request snapshot: {role} generated_image #{id}; media input was omitted from snapshot>"
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CliproxyGrokReferenceMode {
    Reference,
    FirstFrame,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ProviderGenerationWorkflow {
    BoundaryFrames,
    FirstFrameChain,
    Storyboard,
}

impl ProviderGenerationWorkflow {
    fn as_str(self) -> &'static str {
        match self {
            Self::BoundaryFrames => "boundary_frames",
            Self::FirstFrameChain => "first_frame_chain",
            Self::Storyboard => "storyboard",
        }
    }

    fn uses_first_frame_chain(self) -> bool {
        matches!(self, Self::FirstFrameChain)
    }

    fn uses_storyboard(self) -> bool {
        matches!(self, Self::Storyboard)
    }
}

pub(super) fn cliproxy_grok_reference_mode(
    cfg: &video_gen_config::Model,
) -> Result<CliproxyGrokReferenceMode, String> {
    let Some(value) = cfg
        .provider_options_json
        .as_ref()
        .filter(|value| !value.is_null())
    else {
        return Ok(CliproxyGrokReferenceMode::Reference);
    };
    let Value::Object(map) = value else {
        return Err("视频 provider_options_json 必须是 JSON 对象".to_string());
    };
    let Some(mode) = map
        .get("grok_reference_mode")
        .filter(|value| !value.is_null())
    else {
        return Ok(CliproxyGrokReferenceMode::Reference);
    };
    let mode = mode
        .as_str()
        .map(str::trim)
        .ok_or_else(|| "视频 provider_options_json.grok_reference_mode 必须是字符串".to_string())?;
    match mode {
        "reference" => Ok(CliproxyGrokReferenceMode::Reference),
        "first_frame" => Ok(CliproxyGrokReferenceMode::FirstFrame),
        other => Err(format!(
            "视频 provider_options_json.grok_reference_mode 无效：{other}"
        )),
    }
}

fn cliproxy_grok_reference_mode_label(mode: CliproxyGrokReferenceMode) -> &'static str {
    match mode {
        CliproxyGrokReferenceMode::Reference => "参考模式",
        CliproxyGrokReferenceMode::FirstFrame => "首帧模式",
    }
}

pub(super) fn provider_request_snapshot_value(
    key: &'static str,
    title: &'static str,
    kind: &'static str,
    prompt_name: Option<&'static str>,
    note: Option<String>,
    request: Value,
) -> Value {
    json!({
        "schema_version": 1,
        "captured_at": chrono::Utc::now().to_rfc3339(),
        "key": key,
        "title": title,
        "kind": kind,
        "prompt_name": prompt_name,
        "state": "ready",
        "note": note,
        "request": request,
    })
}

pub(super) fn model_request_preview_from_provider_snapshot(
    video: &generated_video::Model,
    workflow: &str,
    title: &str,
) -> Result<ModelRequestPreview, String> {
    let metadata = video
        .metadata_json
        .as_ref()
        .and_then(Value::as_object)
        .ok_or_else(|| {
            format!(
                "视频 #{} 缺少 metadata_json，无法直接预览生成时构造的 provider 请求",
                video.id
            )
        })?;
    let snapshot = metadata
        .get(PROVIDER_REQUEST_SNAPSHOT_METADATA_KEY)
        .and_then(Value::as_object)
        .ok_or_else(|| {
            format!(
                "视频 #{} 缺少 provider 请求快照，无法直接预览生成时构造的请求；请重新生成该视频后再查看历史请求",
                video.id
            )
        })?;
    if snapshot.get("schema_version").and_then(Value::as_i64) != Some(1) {
        return Err(format!(
            "视频 #{} provider 请求快照版本无效，无法直接预览",
            video.id
        ));
    }

    let key = required_snapshot_string(snapshot, "key", video.id)?;
    let step_title = required_snapshot_string(snapshot, "title", video.id)?;
    let kind = required_snapshot_string(snapshot, "kind", video.id)?;
    let state = required_snapshot_string(snapshot, "state", video.id)?;
    let prompt_name = optional_snapshot_string(snapshot, "prompt_name", video.id)?;
    let note = snapshot_note_for_video_record(
        optional_snapshot_string(snapshot, "note", video.id)?,
        video,
    );
    let request = snapshot
        .get("request")
        .filter(|value| value.is_object())
        .cloned()
        .ok_or_else(|| {
            format!(
                "视频 #{} provider 请求快照缺少 request 对象，无法直接预览",
                video.id
            )
        })?;

    Ok(model_request_preview(
        workflow,
        title,
        vec![provider_request_step(
            key,
            step_title,
            kind,
            prompt_name,
            state,
            note,
            &request,
        )?],
    ))
}

fn snapshot_note_for_video_record(
    note: Option<String>,
    video: &generated_video::Model,
) -> Option<String> {
    let status = video.status.trim();
    let status_note = match status {
        "completed" => None,
        "running" | "pending" => Some(format!(
            "视频当前状态为 {status}；下方快照是生成流程已经构造出的 provider 请求，任务可能仍在处理中"
        )),
        "failed" | "cancelled" => Some(format!(
            "视频当前状态为 {status}；无法仅凭该状态确认 provider 是否实际接收请求，下方展示的是生成流程构造出的 provider 请求快照"
        )),
        other if !other.is_empty() => Some(format!(
            "视频当前状态为 {other}；下方展示的是生成流程构造出的 provider 请求快照"
        )),
        _ => Some("视频状态为空；下方展示的是生成流程构造出的 provider 请求快照".to_string()),
    };
    match (note, status_note) {
        (Some(note), Some(status_note)) if !note.trim().is_empty() => {
            Some(format!("{}；{}", note.trim(), status_note))
        }
        (Some(note), _) if !note.trim().is_empty() => Some(note),
        (_, Some(status_note)) => Some(status_note),
        _ => None,
    }
}

pub(super) fn validate_video_provider_request_snapshot_owner(
    video: &generated_video::Model,
) -> Result<(), String> {
    match (video.video_type.as_str(), video.workflow_mode.as_deref()) {
        ("chapter_video", Some("boundary_frames" | "first_frame_chain" | "storyboard"))
        | ("reference_media_video", Some("reference_media")) => Ok(()),
        ("chapter_video", Some(mode)) => Err(format!(
            "视频 #{} 的 workflow_mode={} 与 chapter_video 类型不匹配，无法直接预览 provider 请求",
            video.id, mode
        )),
        ("reference_media_video", Some(mode)) => Err(format!(
            "视频 #{} 的 workflow_mode={} 与 reference_media_video 类型不匹配，无法直接预览 provider 请求",
            video.id, mode
        )),
        ("chapter_video" | "reference_media_video", None) => Err(format!(
            "视频 #{} 缺少 workflow_mode，无法直接预览 provider 请求",
            video.id
        )),
        _ => Err("裁剪片段没有独立的视频模型 provider 请求".to_string()),
    }
}

fn required_snapshot_string(
    snapshot: &Map<String, Value>,
    field: &str,
    video_id: i64,
) -> Result<String, String> {
    let value = optional_snapshot_string(snapshot, field, video_id)?
        .ok_or_else(|| format!("视频 #{video_id} provider 请求快照缺少 {field}，无法直接预览"))?;
    if value.trim().is_empty() {
        return Err(format!(
            "视频 #{video_id} provider 请求快照字段 {field} 不能为空"
        ));
    }
    Ok(value)
}

fn optional_snapshot_string(
    snapshot: &Map<String, Value>,
    field: &str,
    video_id: i64,
) -> Result<Option<String>, String> {
    let Some(value) = snapshot.get(field) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    value
        .as_str()
        .map(|value| Some(value.to_string()))
        .ok_or_else(|| format!("视频 #{video_id} provider 请求快照字段 {field} 必须是字符串"))
}

pub(super) fn video_provider_preview_payload(
    context: &ChapterVideoProviderRequestContext,
) -> Result<(&'static str, &'static str, String, Value), String> {
    if context.cfg.backend == NEWAPI_VIDEO_BACKEND {
        let request = NewApiVideoProvider::request_body_preview(&context.request)
            .map_err(|e| e.to_string())?;
        if context.workflow_mode == "storyboard" {
            let mut note = format!(
                "New API 的 `image` 字段只作为视频首帧/初始图，不支持分镜模式需要的 reference_images；provider 参考图 ID 数量：{}，参数不发送；预览省略真实媒体输入，真实生成会被阻止，请切换到支持参考图输入的视频 provider。",
                context.provider_reference_image_count
            );
            if let Some(input_note) = context.provider_input_note.as_deref() {
                note.push('；');
                note.push_str(input_note);
            }
            append_video_request_prompt_length_note(&mut note, &context.request);
            return Ok((
                "video_generation_newapi",
                "New API /video/generations 请求",
                note,
                request,
            ));
        }
        let image_source = if context.workflow_mode == "first_frame_chain" {
            "首帧串联起始帧"
        } else {
            "首尾帧模式首帧"
        };
        let mut note = format!(
            "New API 的 `image` 字段只作为视频首帧/初始图；提示词参考图不会放入 `image`，也不会作为参考图参数发送。{} image 绑定 generated_image：{}；尾帧 generated_image：{}，参数不发送；提示词参考图 ID 数量：{}，参数不发送；预览省略真实媒体输入，生成时仅首帧会作为 PNG/JPEG data URL 发送。",
            image_source,
            context
                .boundary_frame_ids
                .first
                .map(|id| id.to_string())
                .unwrap_or_else(|| "无".to_string()),
            context
                .boundary_frame_ids
                .last
                .map(|id| id.to_string())
                .unwrap_or_else(|| "无".to_string()),
            context.prompt_reference_image_count
        );
        if let Some(input_note) = context.provider_input_note.as_deref() {
            note.push('；');
            note.push_str(input_note);
        }
        append_video_request_prompt_length_note(&mut note, &context.request);
        return Ok((
            "video_generation_newapi",
            "New API /video/generations 请求",
            note,
            request,
        ));
    }

    if context.cfg.backend == OPENAI_COMPATIBLE_VIDEO_BACKEND {
        let mut note = format!(
            "OpenAI 兼容视频 `/videos/generations` 请求；provider 参考图：{}；提示词参考图：{}；首尾帧媒体输入：{} / {}；预览省略真实媒体输入，生成时默认发送图片 data URL；如设置 A4N_VIDEO_IMAGE_INPUT_TRANSPORT=public_url，则发送公网 URL；如设置为 remote_temp_upload，则先上传到远端临时媒体接口再发送返回 URL。",
            context.provider_reference_image_count,
            context.prompt_reference_image_count,
            context.first_frame_image_url_available,
            context.last_frame_image_url_available
        );
        if let Some(input_note) = context.provider_input_note.as_deref() {
            note.push('；');
            note.push_str(input_note);
        }
        append_video_request_prompt_length_note(&mut note, &context.request);
        return Ok((
            "video_generation_openai_compatible",
            "OpenAI 兼容 /videos/generations 请求",
            note,
            HttpVideoProvider::request_body_preview(&context.request),
        ));
    }

    if context.cfg.backend == CLIPROXY_OPENAI_VIDEO_BACKEND {
        let request =
            a4n_engine::video::CliProxyOpenAiVideoProvider::request_body_preview_with_options(
                &context.request,
                context.cfg.provider_options_json.as_ref(),
            )
            .map_err(|e| e.to_string())?;
        let grok_mode = cliproxy_grok_reference_mode(&context.cfg)?;
        let mut note = match grok_mode {
            CliproxyGrokReferenceMode::Reference => format!(
                "CLIProxy `/openai/v1/videos` 请求；Grok 输入模式：{}；所有输入图片均作为 reference_images 发送，最多 {} 张，当前绑定参考图：{}，实际发送参考图参数：{}；不会发送 input_reference 或尾帧图片槽；真实生成必须使用公网媒体 URL，auto 模式会优先使用已配置的 remote_temp_upload。",
                cliproxy_grok_reference_mode_label(grok_mode),
                MAX_CLIPROXY_VIDEO_REFERENCE_IMAGES,
                context.provider_reference_image_count,
                context.request.reference_image_urls.len()
            ),
            CliproxyGrokReferenceMode::FirstFrame => format!(
                "CLIProxy `/openai/v1/videos` 请求；Grok 输入模式：{}；首帧图片会作为 input_reference.image_url 发送：{}；尾帧图片不会作为 provider 图片参数发送：{}；当前绑定提示词参考图：{}，实际发送参考图参数：{}；真实生成必须使用公网媒体 URL，auto 模式会优先使用已配置的 remote_temp_upload。",
                cliproxy_grok_reference_mode_label(grok_mode),
                context.first_frame_image_url_available,
                context.last_frame_image_url_available,
                context.prompt_reference_image_count,
                context.request.reference_image_urls.len()
            ),
        };
        if let Some(input_note) = context.provider_input_note.as_deref() {
            note.push('；');
            note.push_str(input_note);
        }
        append_video_request_prompt_length_note(&mut note, &context.request);
        return Ok((
            "video_generation_cliproxy_openai",
            "CLIProxy OpenAI /videos 请求",
            note,
            request,
        ));
    }

    if context.cfg.backend == GEMINI_VEO_VIDEO_BACKEND {
        let request = GeminiVeoVideoProvider::request_body_preview(&context.request)
            .map_err(|e| e.to_string())?;
        let mut note = format!(
            "Gemini Veo 原生 `predictLongRunning` 请求；首尾帧媒体输入：{} / {}；provider 参考图：{}；提示词参考图：{}；预览用 inlineData 占位展示实际请求形状，生成时发送真实图片 data URL。",
            context.first_frame_image_url_available,
            context.last_frame_image_url_available,
            context.provider_reference_image_count,
            context.prompt_reference_image_count
        );
        if let Some(input_note) = context.provider_input_note.as_deref() {
            note.push('；');
            note.push_str(input_note);
        }
        append_video_request_prompt_length_note(&mut note, &context.request);
        return Ok((
            "video_generation_gemini_veo",
            "Gemini Veo predictLongRunning 请求",
            note,
            request,
        ));
    }

    if context.cfg.backend == OPENAI_CHAT_VIDEO_BACKEND {
        let mut note = format!(
            "OpenAI Chat-Completions 视频请求；provider 参考图：{}；提示词参考图：{}；首尾帧媒体输入：{} / {}；预览省略真实媒体输入，生成时默认发送图片 data URL；如设置 A4N_VIDEO_IMAGE_INPUT_TRANSPORT=public_url，则发送公网 URL；如设置为 remote_temp_upload，则先上传到远端临时媒体接口再发送返回 URL。",
            context.provider_reference_image_count,
            context.prompt_reference_image_count,
            context.first_frame_image_url_available,
            context.last_frame_image_url_available
        );
        if let Some(input_note) = context.provider_input_note.as_deref() {
            note.push('；');
            note.push_str(input_note);
        }
        append_video_request_prompt_length_note(&mut note, &context.request);
        return Ok((
            "video_generation_chat_completions",
            "OpenAI Chat-Completions 视频请求",
            note,
            ChatCompletionsVideoProvider::request_body_preview(&context.request),
        ));
    }

    Err(format!(
        "未知视频 backend={}，无法展示真实 provider 请求体",
        context.cfg.backend
    ))
}

fn append_video_request_prompt_length_note(note: &mut String, request: &VideoGenerationRequest) {
    note.push('；');
    note.push_str(&video_request_prompt_length_context(request));
}

pub(super) fn append_provider_input_note(note: &mut Option<String>, addition: impl Into<String>) {
    let addition = addition.into();
    if addition.trim().is_empty() {
        return;
    }
    match note {
        Some(note) if !note.trim().is_empty() => {
            note.push('；');
            note.push_str(addition.trim());
        }
        _ => *note = Some(addition.trim().to_string()),
    }
}

fn append_provider_input_notes(note: &mut Option<String>, additions: Vec<String>) {
    for addition in additions {
        append_provider_input_note(note, addition);
    }
}

pub(super) fn apply_chapter_video_prompt_preview_override(
    prompt: &mut chapter_video_prompt::Model,
    input: Option<PreviewChapterVideoPromptOverrideInput>,
    note: &mut Option<String>,
) {
    let Some(input) = input else {
        return;
    };
    append_provider_input_note(
        note,
        "预览使用当前编辑区草稿覆盖已保存提示词；真实生成仍会读取已保存提示词，请保存后再生成。",
    );
    if let Some(visual_prompt) = input.visual_prompt {
        prompt.visual_prompt = visual_prompt.trim().to_string();
    }
    if let Some(motion_prompt) = input.motion_prompt {
        prompt.motion_prompt = motion_prompt
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
    }
    if let Some(negative_prompt) = input.negative_prompt {
        prompt.negative_prompt = negative_prompt
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
    }
    if let Some(duration_secs) = input.duration_secs {
        if duration_secs > 0 {
            prompt.duration_secs = duration_secs;
        } else {
            append_provider_input_note(
                note,
                format!(
                    "草稿 duration_secs={} 无效；预览使用 1 秒占位，真实生成仍需保存有效时长。",
                    duration_secs
                ),
            );
            prompt.duration_secs = 1;
        }
    }
    if let Some(reference_image_ids_json) = input.reference_image_ids_json {
        prompt.reference_image_ids_json = reference_image_ids_json;
    }
}

pub(super) fn ensure_chapter_video_prompt_preview_texts(
    prompt: &mut chapter_video_prompt::Model,
    note: &mut Option<String>,
) {
    let prompt_output_language = PromptOutputLanguage::from_metadata(prompt.metadata_json.as_ref());
    prompt.visual_prompt = preview_model_text_or_placeholder(
        &prompt.visual_prompt,
        "visual_prompt",
        prompt_output_language,
        MIN_PROVIDER_VISUAL_PROMPT_CHARS,
        VIDEO_PROVIDER_VISUAL_PREVIEW_PLACEHOLDER,
        note,
    );
    let motion_prompt = prompt.motion_prompt.as_deref().unwrap_or_default();
    prompt.motion_prompt = Some(preview_model_text_or_placeholder(
        motion_prompt,
        "motion_prompt",
        prompt_output_language,
        MIN_PROVIDER_MOTION_PROMPT_CHARS,
        VIDEO_PROVIDER_MOTION_PREVIEW_PLACEHOLDER,
        note,
    ));
}

pub(super) fn ensure_chapter_video_prompt_preview_metadata(
    prompt: &mut chapter_video_prompt::Model,
    note: &mut Option<String>,
) -> Result<(), String> {
    let provider_workflow = prompt_provider_workflow(prompt)?;
    let mut metadata = match prompt.metadata_json.take() {
        Some(Value::Object(map)) => Value::Object(map),
        Some(_) => {
            append_provider_input_note(
                note,
                "视频提示词 metadata_json 不是对象；预览使用结构化占位元数据展示 provider 请求，真实生成仍需重新生成完整视频提示词。",
            );
            json!({})
        }
        None => {
            append_provider_input_note(
                note,
                "视频提示词缺少结构化 metadata_json；预览使用结构化占位元数据展示 provider 请求，真实生成仍需重新生成完整视频提示词。",
            );
            json!({})
        }
    };
    let root = metadata
        .as_object_mut()
        .ok_or_else(|| "预览 metadata 占位构造失败".to_string())?;
    ensure_preview_object_field(root, "shot", preview_placeholder_shot(prompt), note, "shot");
    if let Some(shot) = root.get_mut("shot").and_then(Value::as_object_mut) {
        ensure_preview_shot_sections(shot, note);
    }
    if provider_workflow.uses_first_frame_chain() {
        if prompt.shot_number <= 1 {
            ensure_preview_object_field(
                root,
                "start_first_frame",
                preview_placeholder_frame(prompt, "首帧串联起始帧"),
                note,
                "start_first_frame",
            );
        }
    } else if provider_workflow.uses_storyboard() {
        ensure_preview_object_field(
            root,
            "storyboard_frame",
            preview_placeholder_frame(prompt, "分镜图"),
            note,
            "storyboard_frame",
        );
    } else {
        ensure_preview_object_field(
            root,
            "start_boundary_frame",
            preview_placeholder_frame(prompt, "首帧边界"),
            note,
            "start_boundary_frame",
        );
        ensure_preview_object_field(
            root,
            "end_boundary_frame",
            preview_placeholder_frame(prompt, "尾帧边界"),
            note,
            "end_boundary_frame",
        );
    }
    prompt.metadata_json = Some(metadata);
    Ok(())
}

fn ensure_preview_object_field(
    root: &mut Map<String, Value>,
    key: &str,
    replacement: Value,
    note: &mut Option<String>,
    label: &str,
) {
    if root.get(key).and_then(Value::as_object).is_some() {
        return;
    }
    let reason = if root.contains_key(key) {
        "不是对象"
    } else {
        "缺失"
    };
    root.insert(key.to_string(), replacement);
    append_provider_input_note(
        note,
        format!(
            "视频提示词 metadata.{label} {reason}；预览使用占位结构展示 provider 请求，真实生成仍需完整结构化元数据。"
        ),
    );
}

fn ensure_preview_shot_sections(shot: &mut Map<String, Value>, note: &mut Option<String>) {
    for (key, value) in [
        (
            "self_contained_context",
            json!({
                "must_repeat_from_previous": ["预览占位：上一段必须带入的人物、空间、道具、光影和镜头信息"],
                "must_establish_in_segment": ["预览占位：本段独立生成时必须建立的剧情动作、场景锚点和情绪目标"],
                "must_preserve_throughout": ["预览占位：本段全程必须保持的人物身份、空间关系、光影方向和参考图约束"],
                "handoff_to_next": "预览占位：结尾要交给下一段继续的文字化状态",
                "omission_risks": ["预览占位：遗漏上下文会导致片段像独立短片、人物/空间/镜头断裂"]
            }),
        ),
        (
            "start_state",
            json!({
                "preview_placeholder": true,
                "requirement": "真实生成前需要补全文字化起始状态，包括人物站位、姿态、朝向、场景锚点、机位和光影。"
            }),
        ),
        (
            "transition_in",
            json!({
                "from_previous_end": "预览占位：上一段结尾状态",
                "to_current_start": "预览占位：当前段起始状态",
                "cut_type": "预览占位：转场方式",
                "camera_transition": "预览占位：机位或视角变化",
                "first_beat_action": "预览占位：首个动作节拍"
            }),
        ),
        (
            "end_state",
            json!({
                "preview_placeholder": true,
                "requirement": "真实生成前需要补全文字化结尾状态，作为下一段 start_state 的承接来源。"
            }),
        ),
        (
            "blocking_plan",
            json!({
                "preview_placeholder": true,
                "requirement": "真实生成前需要补全人物站位、走位、空间轴线和首尾状态。"
            }),
        ),
        (
            "lighting_plan",
            json!({
                "preview_placeholder": true,
                "requirement": "真实生成前需要补全主光方向、色温、阴影和首尾帧光影连续性。"
            }),
        ),
        (
            "emotion_action_beats",
            json!([
                {
                    "preview_placeholder": true,
                    "requirement": "真实生成前需要补全情绪变化、可见动作和节拍。"
                }
            ]),
        ),
        ("dialogue_beats", json!([])),
        (
            "cinematography_plan",
            json!({
                "preview_placeholder": true,
                "requirement": "真实生成前需要补全景别、机位、镜头运动、轴线和焦段提示。"
            }),
        ),
        (
            "temporal_beats",
            json!([
                {
                    "preview_placeholder": true,
                    "requirement": "真实生成前需要补全分秒段动作、镜头和连续性目标。"
                }
            ]),
        ),
        (
            "reference_binding",
            json!({
                "bindings": [],
                "coverage_check": "请求预览占位；真实生成前需要覆盖所有参考图。"
            }),
        ),
        (
            "continuity_check",
            json!({
                "preview_placeholder": true,
                "blocking": "待补全",
                "lighting": "待补全",
                "emotion": "待补全",
                "references": "待补全",
                "revision": "真实生成前需要重新生成完整视频提示词。"
            }),
        ),
    ] {
        if shot.get(key).is_some() {
            continue;
        }
        shot.insert(key.to_string(), value);
        append_provider_input_note(
            note,
            format!(
                "视频提示词 metadata.shot.{key} 缺失；预览使用占位结构展示 provider 请求，真实生成仍需补全。"
            ),
        );
    }
}

fn preview_placeholder_shot(prompt: &chapter_video_prompt::Model) -> Value {
    json!({
        "title": prompt
            .title
            .as_deref()
            .unwrap_or("请求预览占位镜头"),
        "scene_summary": prompt
            .scene_summary
            .as_deref()
            .unwrap_or("请求预览占位剧情摘要；真实生成前需要重新生成完整视频提示词。"),
        "blocking_plan": {
            "preview_placeholder": true,
            "requirement": "真实生成前需要补全人物站位、走位、空间轴线和首尾状态。"
        },
        "lighting_plan": {
            "preview_placeholder": true,
            "requirement": "真实生成前需要补全主光方向、色温、阴影和首尾帧光影连续性。"
        },
        "emotion_action_beats": [
            {
                "preview_placeholder": true,
                "requirement": "真实生成前需要补全情绪变化、可见动作和节拍。"
            }
        ],
        "dialogue_beats": [],
        "cinematography_plan": {
            "preview_placeholder": true,
            "requirement": "真实生成前需要补全景别、机位、镜头运动、轴线和焦段提示。"
        },
        "temporal_beats": [
            {
                "preview_placeholder": true,
                "requirement": "真实生成前需要补全分秒段动作、镜头和连续性目标。"
            }
        ],
        "reference_binding": {
            "bindings": [],
            "coverage_check": "请求预览占位；真实生成前需要覆盖所有参考图。"
        },
        "continuity_check": {
            "preview_placeholder": true,
            "blocking": "待补全",
            "lighting": "待补全",
            "emotion": "待补全",
            "references": "待补全",
            "revision": "真实生成前需要重新生成完整视频提示词。"
        }
    })
}

fn preview_placeholder_frame(prompt: &chapter_video_prompt::Model, label: &str) -> Value {
    json!({
        "title": format!("{label}（请求预览占位）"),
        "scene_summary": prompt
            .scene_summary
            .as_deref()
            .unwrap_or("请求预览占位帧摘要；真实生成前需要重新生成完整帧提示词。"),
        "frame_prompt": format!(
            "【请求预览占位】{label}结构化帧提示缺失。真实生成前需要补全静态画面、人物站位、构图、光影、服装道具和可承接的结尾状态。"
        ),
        "blocking_state": {
            "preview_placeholder": true,
            "requirement": "真实生成前需要补全人物、位置、姿态、道具状态和画面轴线。"
        },
        "lighting_state": {
            "preview_placeholder": true,
            "requirement": "真实生成前需要补全主光方向、阴影、色温和环境光。"
        }
    })
}

fn preview_model_text_or_placeholder(
    value: &str,
    field: &str,
    prompt_output_language: PromptOutputLanguage,
    min_chars: usize,
    placeholder: &str,
    note: &mut Option<String>,
) -> String {
    let trimmed = value.trim();
    let violation =
        provider_prompt_text_violation(trimmed, field, prompt_output_language, min_chars, false);
    if violation.is_none() {
        return trimmed.to_string();
    }
    match violation.expect("checked above") {
        ProviderPromptTextViolation::DictionaryLike => {
            append_provider_input_note(
                note,
                format!(
                    "{field} 包含 JSON/字典结构片段；预览使用占位文本展示请求结构，真实生成前需要改写为自然语言。"
                ),
            );
            placeholder.to_string()
        }
        ProviderPromptTextViolation::InternalWorkflowTerm(term) => {
            append_provider_input_note(
                note,
                format!(
                    "{field} 包含视频生成内部流程说明“{term}”；预览使用占位文本展示请求结构，真实生成前需要改写为只描述画面和动作的自然语言。"
                ),
            );
            placeholder.to_string()
        }
        ProviderPromptTextViolation::UploadStatusTerm(term) => {
            append_provider_input_note(
                note,
                format!(
                    "{field} 包含上传状态说明“{term}”；预览使用占位文本展示请求结构，真实生成前需要改写为实际起点、输入帧画面或上一段尾态。"
                ),
            );
            placeholder.to_string()
        }
        ProviderPromptTextViolation::DuplicateMotionTimeline => {
            append_provider_input_note(
                note,
                format!(
                    "{field} 重复镜头分镜；预览使用占位文本展示请求结构，真实生成前需要把时间轴只保留在 visual_prompt。"
                ),
            );
            placeholder.to_string()
        }
        ProviderPromptTextViolation::MissingRequiredSection(section) => {
            append_provider_input_note(
                note,
                format!(
                    "{field} 缺少必需章节 {section}；预览使用占位文本展示请求结构，真实生成前需要补全完整提示词。"
                ),
            );
            placeholder.to_string()
        }
        ProviderPromptTextViolation::StaleSection(section) => {
            append_provider_input_note(
                note,
                format!(
                    "{field} 包含已废弃章节 {section}；预览使用占位文本展示请求结构，真实生成前需要重新生成当前格式提示词。"
                ),
            );
            placeholder.to_string()
        }
        ProviderPromptTextViolation::DirectPrompt(message) => {
            append_provider_input_note(
                note,
                format!(
                    "{field} 不符合自然语言提示词约束：{message}；预览使用占位文本展示请求结构，真实生成前需要改写。"
                ),
            );
            placeholder.to_string()
        }
        ProviderPromptTextViolation::Empty => {
            append_provider_input_note(
                note,
                format!(
                    "{field} 过短，至少需要 {min_chars} 个字符；预览使用占位文本展示请求结构，真实生成仍需补全。"
                ),
            );
            placeholder.to_string()
        }
        ProviderPromptTextViolation::TooShort { .. } => {
            append_provider_input_note(
                note,
                format!(
                    "{field} 过短，至少需要 {min_chars} 个字符；预览使用占位文本展示请求结构，真实生成仍需补全。"
                ),
            );
            format!("{trimmed}\n\n{placeholder}")
        }
        ProviderPromptTextViolation::PreviewPlaceholder => {
            append_provider_input_note(
                note,
                format!(
                    "{field} 包含请求预览占位文本；预览使用占位文本展示请求结构，真实生成前需要保存完整提示词。"
                ),
            );
            placeholder.to_string()
        }
    }
}

pub(super) fn append_preview_duration_note(
    note: &mut Option<String>,
    cfg: &video_gen_config::Model,
    duration_secs: i32,
) {
    if let Err(err) = validate_video_duration_for_config_secs(cfg, duration_secs) {
        append_provider_input_note(
            note,
            format!(
                "当前视频配置不支持 {duration_secs} 秒；预览仍展示请求结构，真实生成会失败：{err}"
            ),
        );
    }
}

pub(super) async fn build_chapter_video_provider_request(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
    prompt_id: i64,
    user_id: i64,
    config_id: Option<i64>,
    purpose: VideoProviderRequestPurpose,
    image_mode: VideoRequestImageMode,
    prompt_override: Option<PreviewChapterVideoPromptOverrideInput>,
) -> Result<ChapterVideoProviderRequestContext, String> {
    let mut prompt = chapter_video_prompt::Entity::find_by_id(prompt_id)
        .one(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "视频提示词不存在".to_string())?;
    if prompt.project_id != project_id || prompt.chapter_id != chapter_id {
        return Err("视频提示词不属于该章节".into());
    }
    match purpose {
        VideoProviderRequestPurpose::Generate => {
            resolve_project_chapter_video_source(state, project_id, chapter_id, "生成视频", true)
                .await?;
        }
        VideoProviderRequestPurpose::Preview => {
            resolve_project_chapter_video_source(
                state,
                project_id,
                chapter_id,
                "预览视频生成请求",
                false,
            )
            .await?;
        }
    }
    if prompt.prompt_type != "video_prompt" {
        return Err("请选择 video_prompt 类型的提示词生成视频".into());
    }
    let mut provider_input_note = None;
    if purpose == VideoProviderRequestPurpose::Preview {
        apply_chapter_video_prompt_preview_override(
            &mut prompt,
            prompt_override,
            &mut provider_input_note,
        );
        ensure_chapter_video_prompt_preview_texts(&mut prompt, &mut provider_input_note);
        ensure_chapter_video_prompt_preview_metadata(&mut prompt, &mut provider_input_note)?;
    }
    if purpose == VideoProviderRequestPurpose::Generate {
        if prompt.status == "running" {
            return Err("当前视频提示词正在生成视频，请先等待完成后再重新生成".into());
        }
        if prompt.generated_video_id.is_some() {
            return Err("该视频提示词已生成视频，请先删除对应视频后再重新生成".into());
        }
        ensure_prompt_matches_current_chapter(state, &prompt).await?;
    }
    let config_id = match purpose {
        VideoProviderRequestPurpose::Generate => config_id,
        VideoProviderRequestPurpose::Preview => {
            provider_request_preview_config_id(state, &prompt, config_id).await?
        }
    };
    let cfg = resolve_video_config(state, config_id).await?;
    match purpose {
        VideoProviderRequestPurpose::Generate => {
            validate_video_duration_for_config_secs(&cfg, prompt.duration_secs)?;
        }
        VideoProviderRequestPurpose::Preview => {
            append_preview_duration_note(&mut provider_input_note, &cfg, prompt.duration_secs);
        }
    }

    let prompt_reference_image_ids_json = prompt.reference_image_ids_json.clone();
    let prompt_reference_image_ids =
        media::reference_image_ids(prompt_reference_image_ids_json.as_ref())?;
    let prompt_reference_image_count = prompt_reference_image_ids.len();
    let source_prompt_type = prompt_source_prompt_type(&prompt).map(str::to_string);
    let provider_workflow = prompt_provider_workflow(&prompt)?;
    let first_frame_chain = provider_workflow.uses_first_frame_chain();
    let storyboard_workflow = provider_workflow.uses_storyboard();
    let cliproxy_grok_mode = cliproxy_grok_reference_mode(&cfg)?;
    let cliproxy_first_frame_mode = cfg.backend == CLIPROXY_OPENAI_VIDEO_BACKEND
        && cliproxy_grok_mode == CliproxyGrokReferenceMode::FirstFrame;
    let cliproxy_reference_mode = cfg.backend == CLIPROXY_OPENAI_VIDEO_BACKEND
        && cliproxy_grok_mode == CliproxyGrokReferenceMode::Reference;
    enforce_provider_image_binding_contract(
        &cfg.backend,
        provider_workflow,
        cliproxy_grok_mode,
        prompt_reference_image_count,
        purpose,
        &mut provider_input_note,
    )?;
    let image_transport = provider_image_input_transport(&cfg.backend)?;
    if image_transport == ProviderImageInputTransport::RemoteTempUpload {
        append_provider_input_note(
            &mut provider_input_note,
            "远端临时媒体上传：生成时会 POST 图片到 A4N_VIDEO_REMOTE_TEMP_UPLOAD_URL，并使用返回的公网 URL；预览省略真实图片数据。"
                .to_string(),
        );
    }
    let (boundary_frame_ids, storyboard_image_id) = if first_frame_chain {
        let start =
            first_frame_chain_start_image_id_for_prompt(state, project_id, chapter_id, &prompt)
                .await?;
        if start.is_none() {
            let message = first_frame_chain_missing_start_image_message(&prompt);
            if purpose == VideoProviderRequestPurpose::Generate {
                return Err(message);
            }
            append_provider_input_note(
                &mut provider_input_note,
                format!(
                    "当前首帧串联起始帧缺失：{message}；此处仅展示 provider 请求结构，真实生成仍需补齐该输入"
                ),
            );
        }
        (
            BoundaryFrameImageIds {
                first: start,
                last: None,
            },
            None,
        )
    } else if storyboard_workflow {
        let storyboard_image_id =
            storyboard_image_id_for_prompt(state, project_id, chapter_id, &prompt).await?;
        if storyboard_image_id.is_none() {
            let message = storyboard_missing_image_message(&prompt);
            if purpose == VideoProviderRequestPurpose::Generate {
                return Err(message);
            }
            append_provider_input_note(
                &mut provider_input_note,
                format!(
                    "当前分镜图缺失：{message}；此处仅展示 provider 请求结构，真实生成仍需补齐该输入"
                ),
            );
        }
        (BoundaryFrameImageIds::default(), storyboard_image_id)
    } else {
        let frame_ids =
            boundary_frame_image_ids_for_prompt(state, project_id, chapter_id, &prompt).await?;
        if let Some(message) = boundary_frame_missing_image_message(&prompt, &frame_ids) {
            if purpose == VideoProviderRequestPurpose::Generate {
                return Err(message);
            }
            append_provider_input_note(
                &mut provider_input_note,
                format!(
                    "当前首尾帧图片不完整：{message}；此处仅展示 provider 请求结构，真实生成仍需补齐该输入"
                ),
            );
        }
        (frame_ids, None)
    };
    let provider_reference_image_ids = if storyboard_workflow {
        reference_ids_with_storyboard_image(storyboard_image_id, &prompt_reference_image_ids)
    } else if cliproxy_reference_mode {
        reference_ids_with_boundary_frame_images(&boundary_frame_ids, &prompt_reference_image_ids)
    } else {
        prompt_reference_image_ids.clone()
    };
    let provider_reference_ids_are_authoritative = storyboard_workflow || cliproxy_reference_mode;
    let bound_reference_image_count = if provider_reference_ids_are_authoritative {
        provider_reference_image_ids.len()
    } else {
        prompt_reference_image_count
    };
    if cliproxy_reference_mode
        && (boundary_frame_ids.first.is_some() || boundary_frame_ids.last.is_some())
    {
        append_provider_input_note(
            &mut provider_input_note,
            format!(
                "CLIProxy/Grok 参考模式已将边界帧图片作为普通 reference_images 发送；实际参考图 ID 顺序：{}",
                provider_reference_image_ids
                    .iter()
                    .map(i64::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        );
    }
    if storyboard_workflow && storyboard_image_id.is_some() {
        append_provider_input_note(
            &mut provider_input_note,
            format!(
                "分镜模式已将分镜图作为第 1 张 reference_images 发送，后续为素材参考图；实际参考图 ID 顺序：{}",
                provider_reference_image_ids
                    .iter()
                    .map(i64::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        );
    }
    let provider_reference_image_ids_json = if provider_reference_ids_are_authoritative {
        Some(json!(provider_reference_image_ids.clone()))
    } else {
        prompt_reference_image_ids_json.clone()
    };
    let (reference_image_urls, first_frame_image_url, last_frame_image_url) = match image_mode {
        VideoRequestImageMode::ProviderImageInputs => {
            let cliproxy_has_first_frame =
                cfg.backend == CLIPROXY_OPENAI_VIDEO_BACKEND && boundary_frame_ids.first.is_some();
            let should_send_reference_images = cfg.backend != NEWAPI_VIDEO_BACKEND
                && !(cliproxy_first_frame_mode
                    && (cliproxy_has_first_frame || storyboard_workflow));
            let mut reference_image_urls = if should_send_reference_images {
                if provider_reference_ids_are_authoritative {
                    media::reference_image_provider_inputs_by_ids(
                        state,
                        user_id,
                        project_id,
                        chapter_id,
                        provider_reference_image_ids.clone(),
                        image_transport,
                    )
                    .await?
                } else {
                    reference_image_provider_inputs(
                        state,
                        user_id,
                        project_id,
                        chapter_id,
                        prompt_reference_image_ids_json.as_ref(),
                        image_transport,
                    )
                    .await?
                }
            } else if cliproxy_first_frame_mode && !cliproxy_has_first_frame && !storyboard_workflow
            {
                let mut urls = reference_image_provider_inputs(
                    state,
                    user_id,
                    project_id,
                    chapter_id,
                    prompt_reference_image_ids_json.as_ref(),
                    image_transport,
                )
                .await?;
                urls.truncate(1);
                urls
            } else {
                Vec::new()
            };
            if cliproxy_first_frame_mode && !storyboard_workflow && !reference_image_urls.is_empty()
            {
                reference_image_urls.truncate(1);
            }
            let first_frame_image_url = if cliproxy_reference_mode || storyboard_workflow {
                None
            } else {
                reference_image_provider_input_by_id(
                    state,
                    user_id,
                    project_id,
                    chapter_id,
                    boundary_frame_ids.first,
                    image_transport,
                )
                .await?
            };
            let last_frame_image_url = if storyboard_workflow
                || cfg.backend == NEWAPI_VIDEO_BACKEND
                || cfg.backend == CLIPROXY_OPENAI_VIDEO_BACKEND
            {
                None
            } else {
                reference_image_provider_input_by_id(
                    state,
                    user_id,
                    project_id,
                    chapter_id,
                    boundary_frame_ids.last,
                    image_transport,
                )
                .await?
            };
            (
                reference_image_urls,
                first_frame_image_url,
                last_frame_image_url,
            )
        }
        VideoRequestImageMode::PreviewPlaceholders => {
            let cliproxy_has_first_frame =
                cfg.backend == CLIPROXY_OPENAI_VIDEO_BACKEND && boundary_frame_ids.first.is_some();
            let should_send_reference_images = cfg.backend != NEWAPI_VIDEO_BACKEND
                && !(cliproxy_first_frame_mode
                    && (cliproxy_has_first_frame || storyboard_workflow));
            let reference_image_urls = if should_send_reference_images
                || (cliproxy_first_frame_mode && !cliproxy_has_first_frame && !storyboard_workflow)
            {
                let placeholders = if provider_reference_ids_are_authoritative {
                    media::reference_image_preview_placeholders_by_ids(
                        state,
                        user_id,
                        project_id,
                        chapter_id,
                        provider_reference_image_ids.clone(),
                    )
                    .await?
                } else {
                    reference_image_preview_placeholders(
                        state,
                        user_id,
                        project_id,
                        chapter_id,
                        prompt_reference_image_ids_json.as_ref(),
                    )
                    .await?
                };
                append_provider_input_notes(&mut provider_input_note, placeholders.notes);
                if cliproxy_first_frame_mode && !storyboard_workflow {
                    placeholders.urls.into_iter().take(1).collect()
                } else {
                    placeholders.urls
                }
            } else {
                Vec::new()
            };
            let first_frame_image_url = if cliproxy_reference_mode || storyboard_workflow {
                None
            } else {
                let (url, notes) = reference_image_preview_placeholder_by_id(
                    state,
                    user_id,
                    project_id,
                    chapter_id,
                    boundary_frame_ids.first,
                )
                .await?;
                append_provider_input_notes(&mut provider_input_note, notes);
                url
            };
            let last_frame_image_url = if storyboard_workflow
                || cfg.backend == NEWAPI_VIDEO_BACKEND
                || cfg.backend == CLIPROXY_OPENAI_VIDEO_BACKEND
            {
                None
            } else {
                let (url, notes) = reference_image_preview_placeholder_by_id(
                    state,
                    user_id,
                    project_id,
                    chapter_id,
                    boundary_frame_ids.last,
                )
                .await?;
                append_provider_input_notes(&mut provider_input_note, notes);
                url
            };
            (
                reference_image_urls,
                first_frame_image_url,
                last_frame_image_url,
            )
        }
    };
    if cfg.backend == CLIPROXY_OPENAI_VIDEO_BACKEND
        && cliproxy_first_frame_mode
        && !storyboard_workflow
        && prompt_reference_image_count > 0
    {
        let note = if boundary_frame_ids.first.is_some() {
            format!(
                "CLIProxy/Grok 首帧模式已使用生成首帧作为 input_reference，{} 张提示词参考图不会作为 reference_images 发送。",
                prompt_reference_image_count
            )
        } else {
            format!(
                "CLIProxy/Grok 首帧模式会把第一张提示词参考图作为 input_reference 使用；其余 {} 张提示词参考图不会发送。",
                prompt_reference_image_count.saturating_sub(1)
            )
        };
        append_provider_input_note(&mut provider_input_note, note);
    }
    if cfg.backend == NEWAPI_VIDEO_BACKEND && purpose == VideoProviderRequestPurpose::Generate {
        ensure_newapi_first_frame_image_supported(first_frame_image_url.as_deref())?;
    }
    let first_frame_image_url_available = first_frame_image_url.is_some()
        || (cliproxy_first_frame_mode && !storyboard_workflow && !reference_image_urls.is_empty());
    let last_frame_image_url_available = last_frame_image_url.is_some();
    enforce_cliproxy_video_media_contract(
        &cfg.backend,
        &cfg.model,
        cliproxy_grok_mode,
        bound_reference_image_count,
        first_frame_image_url_available,
        prompt.duration_secs,
        purpose,
        &mut provider_input_note,
    )?;
    let provider_prompt_image_binding = ProviderPromptImageBinding {
        has_first_frame_image: first_frame_image_url_available,
        has_last_frame_image: last_frame_image_url_available,
        reference_image_count: reference_image_urls.len(),
        has_storyboard_reference_image: storyboard_workflow && storyboard_image_id.is_some(),
    };
    let prompt_source = prompt
        .metadata_json
        .as_ref()
        .and_then(|m| m.get("source"))
        .cloned();
    let full_prompt = match purpose {
        VideoProviderRequestPurpose::Generate => {
            build_video_generation_provider_prompt_with_binding(
                state.engine.as_ref(),
                &prompt,
                provider_prompt_image_binding,
            )?
        }
        VideoProviderRequestPurpose::Preview => {
            build_video_generation_provider_preview_prompt_with_binding(
                state.engine.as_ref(),
                &prompt,
                provider_prompt_image_binding,
            )?
        }
    };
    let negative_prompt = provider_specific_negative_prompt(prompt.negative_prompt.as_deref());
    let request = VideoGenerationRequest {
        prompt: full_prompt,
        negative_prompt,
        width: cfg.default_width,
        height: cfg.default_height,
        duration_secs: prompt.duration_secs,
        fps: cfg.default_fps,
        model: cfg.model.clone(),
        first_frame_image_url,
        last_frame_image_url,
        reference_image_urls,
        reference_media: None,
    };
    enforce_provider_prompt_length_contract(
        &cfg.backend,
        &request.prompt,
        purpose,
        &mut provider_input_note,
    )?;
    Ok(ChapterVideoProviderRequestContext {
        prompt_reference_image_count,
        provider_reference_image_count: bound_reference_image_count,
        prompt,
        cfg,
        request,
        prompt_reference_image_ids: prompt_reference_image_ids_json,
        provider_reference_image_ids: provider_reference_image_ids_json,
        prompt_source,
        workflow_mode: provider_workflow.as_str().to_string(),
        source_prompt_type,
        boundary_frame_ids,
        first_frame_image_url_available,
        last_frame_image_url_available,
        provider_input_note,
    })
}

fn provider_image_input_transport(backend: &str) -> Result<ProviderImageInputTransport, String> {
    let override_value = std::env::var("A4N_VIDEO_IMAGE_INPUT_TRANSPORT").ok();
    provider_image_input_transport_with_override(backend, override_value.as_deref())
}

pub(super) fn enforce_provider_prompt_length_contract(
    backend: &str,
    prompt: &str,
    purpose: VideoProviderRequestPurpose,
    note: &mut Option<String>,
) -> Result<(), String> {
    if backend.trim() != CLIPROXY_OPENAI_VIDEO_BACKEND {
        return Ok(());
    }
    let prompt_chars = prompt.chars().count();
    let prompt_bytes = prompt.len();
    if prompt_chars <= MAX_CLIPROXY_VIDEO_PROMPT_CHARS {
        return Ok(());
    }
    let message = format!(
        "CLIProxy/Grok 视频 prompt 超过 {MAX_CLIPROXY_VIDEO_PROMPT_CHARS} 字符限制：实际 {prompt_chars} 字符 / {prompt_bytes} UTF-8 字节；请缩短 Visual/Motion Prompt 后再生成"
    );
    if purpose == VideoProviderRequestPurpose::Generate {
        return Err(message);
    }
    append_provider_input_note(note, format!("{message} 当前仅展示请求结构。"));
    Ok(())
}

pub(super) fn provider_image_input_transport_with_override(
    backend: &str,
    override_value: Option<&str>,
) -> Result<ProviderImageInputTransport, String> {
    let backend = backend.trim();
    let override_value = override_value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase);
    match override_value.as_deref() {
        Some("public_url" | "public-url" | "url") => Ok(ProviderImageInputTransport::PublicUrl),
        Some("remote_temp_upload" | "remote-temp-upload" | "remote_temp" | "remote-temp") => {
            Ok(ProviderImageInputTransport::RemoteTempUpload)
        }
        Some("inline_data_url" | "inline-data-url" | "inline" | "data_url" | "data-url") => {
            if backend == CLIPROXY_OPENAI_VIDEO_BACKEND {
                return Err(
                    "cliproxy-openai-video 不支持 base64/data URL 视频图片输入，请设置 A4N_VIDEO_IMAGE_INPUT_TRANSPORT=remote_temp_upload 并配置 A4N_VIDEO_REMOTE_TEMP_UPLOAD_URL/A4N_VIDEO_REMOTE_TEMP_UPLOAD_TOKEN，或设置 A4N_PUBLIC_APP_URL/A4N_PUBLIC_MEDIA_BASE_URL"
                        .to_string(),
                );
            }
            Ok(ProviderImageInputTransport::InlineDataUrl)
        }
        Some("auto") | None => Ok(provider_default_image_input_transport(backend)),
        Some(other) => Err(format!(
            "A4N_VIDEO_IMAGE_INPUT_TRANSPORT 无效：{other}；允许值为 auto、inline_data_url、public_url 或 remote_temp_upload"
        )),
    }
}

fn provider_default_image_input_transport(backend: &str) -> ProviderImageInputTransport {
    provider_default_image_input_transport_with_remote(backend, remote_temp_upload_configured())
}

pub(super) fn provider_default_image_input_transport_with_remote(
    backend: &str,
    remote_temp_configured: bool,
) -> ProviderImageInputTransport {
    if backend.trim() == CLIPROXY_OPENAI_VIDEO_BACKEND {
        if remote_temp_configured {
            return ProviderImageInputTransport::RemoteTempUpload;
        }
        return ProviderImageInputTransport::PublicUrl;
    }
    ProviderImageInputTransport::InlineDataUrl
}

pub(super) fn enforce_cliproxy_video_media_contract(
    backend: &str,
    model: &str,
    grok_mode: CliproxyGrokReferenceMode,
    bound_reference_image_count: usize,
    has_first_frame_image: bool,
    duration_secs: i32,
    purpose: VideoProviderRequestPurpose,
    note: &mut Option<String>,
) -> Result<(), String> {
    if backend.trim() != CLIPROXY_OPENAI_VIDEO_BACKEND {
        return Ok(());
    }
    if grok_mode == CliproxyGrokReferenceMode::FirstFrame {
        return Ok(());
    }
    if bound_reference_image_count > 0 && !cliproxy_grok_model_supports_reference_images(model) {
        let message = format!(
            "CLIProxy / Grok 视频模型 {model} 不支持 reference_images；当前请求需要发送 {bound_reference_image_count} 张参考图。请改用 grok-imagine-video，或切换为 Grok 首帧模式并使用首帧串联流程。"
        );
        if purpose == VideoProviderRequestPurpose::Generate {
            return Err(message);
        }
        append_provider_input_note(note, format!("{message} 当前仅展示请求结构。"));
    }
    if has_first_frame_image && bound_reference_image_count > 0 {
        let message = format!(
            "CLIProxy / Grok 视频使用首帧 input_reference 时不能同时发送 reference_images；当前请求还绑定了 {bound_reference_image_count} 张参考图。请移除参考图，或切换到支持首帧加参考图的视频 provider。"
        );
        if purpose == VideoProviderRequestPurpose::Generate {
            return Err(message);
        }
        append_provider_input_note(note, format!("{message} 当前仅展示请求结构。"));
    }
    if bound_reference_image_count > MAX_CLIPROXY_VIDEO_REFERENCE_IMAGES {
        let message = format!(
            "CLIProxy / Grok 视频最多支持 {MAX_CLIPROXY_VIDEO_REFERENCE_IMAGES} 张参考图，当前 {bound_reference_image_count} 张"
        );
        if purpose == VideoProviderRequestPurpose::Generate {
            return Err(message);
        }
        append_provider_input_note(note, format!("{message}；真实生成会拒绝该输入。"));
    }
    if bound_reference_image_count > 0 && duration_secs > 10 {
        let message =
            "CLIProxy / Grok 视频使用参考图时最大时长为 10 秒，请降低时长或移除参考图".to_string();
        if purpose == VideoProviderRequestPurpose::Generate {
            return Err(message);
        }
        append_provider_input_note(note, format!("{message}；真实生成会拒绝该输入。"));
    }
    Ok(())
}

fn cliproxy_grok_model_supports_reference_images(model: &str) -> bool {
    !cliproxy_grok_model_is_image_to_video_only(model)
}

fn cliproxy_grok_model_is_image_to_video_only(model: &str) -> bool {
    matches!(
        cliproxy_grok_model_base(model).as_deref(),
        Some("grok-imagine-video-1.5")
            | Some("grok-imagine-video-1.5-preview")
            | Some("grok-imagine-video-1.5-2026-05-30")
    )
}

fn cliproxy_grok_model_base(model: &str) -> Option<String> {
    let model = model.trim().to_ascii_lowercase();
    if model.is_empty() {
        return None;
    }
    Some(
        model
            .rsplit('/')
            .next()
            .unwrap_or(model.as_str())
            .trim()
            .to_string(),
    )
}

pub(super) fn enforce_provider_image_binding_contract(
    backend: &str,
    workflow: ProviderGenerationWorkflow,
    grok_mode: CliproxyGrokReferenceMode,
    prompt_reference_image_count: usize,
    purpose: VideoProviderRequestPurpose,
    note: &mut Option<String>,
) -> Result<(), String> {
    let backend = backend.trim();
    if backend == NEWAPI_VIDEO_BACKEND && workflow == ProviderGenerationWorkflow::Storyboard {
        let message =
            "New API 视频接口不支持 reference_images，不能用于分镜模式；请切换到支持参考图输入的视频 provider。"
                .to_string();
        if purpose == VideoProviderRequestPurpose::Generate {
            return Err(message);
        }
        append_provider_input_note(note, format!("{message} 当前仅展示请求结构。"));
    } else if backend == NEWAPI_VIDEO_BACKEND && !workflow.uses_first_frame_chain() {
        let message =
            "New API 视频接口只有一个首帧图片输入槽，只能作为视频起始帧使用；不能用于首尾帧模式，否则尾帧图片会被静默丢弃。请改用首帧串联模式，或切换到支持尾帧/参考图输入的视频 provider。"
                .to_string();
        if purpose == VideoProviderRequestPurpose::Generate {
            return Err(message);
        }
        append_provider_input_note(note, format!("{message} 当前仅展示请求结构。"));
    }
    if backend == CLIPROXY_OPENAI_VIDEO_BACKEND
        && grok_mode == CliproxyGrokReferenceMode::FirstFrame
        && workflow == ProviderGenerationWorkflow::Storyboard
    {
        let message =
            "CLIProxy / Grok 视频接口当前是首帧模式，只有一个 input_reference 输入槽，不能用于分镜模式；请切换到 Grok 参考模式，或切换到支持 reference_images 的视频 provider。"
                .to_string();
        if purpose == VideoProviderRequestPurpose::Generate {
            return Err(message);
        }
        append_provider_input_note(note, format!("{message} 当前仅展示请求结构。"));
    } else if backend == CLIPROXY_OPENAI_VIDEO_BACKEND
        && grok_mode == CliproxyGrokReferenceMode::FirstFrame
        && !workflow.uses_first_frame_chain()
    {
        let message =
            "CLIProxy / Grok 视频接口当前是首帧模式，只有一个 input_reference 输入槽，只能作为视频起始帧使用；不能用于首尾帧模式，否则尾帧图片会被静默丢弃。请改用首帧串联模式，切换到 Grok 参考模式，或切换到支持尾帧输入的视频 provider。"
                .to_string();
        if purpose == VideoProviderRequestPurpose::Generate {
            return Err(message);
        }
        append_provider_input_note(note, format!("{message} 当前仅展示请求结构。"));
    }
    if backend != NEWAPI_VIDEO_BACKEND {
        return Ok(());
    }
    if prompt_reference_image_count > 0 {
        let message = format!(
            "New API 视频接口不支持提示词参考图输入；当前视频提示词绑定了 {prompt_reference_image_count} 张参考图。已阻止生成以避免参考图被丢弃，请移除参考图，或切换到支持参考图输入的视频 provider。"
        );
        if purpose == VideoProviderRequestPurpose::Generate {
            return Err(message);
        }
        append_provider_input_note(note, format!("{message} 当前仅展示请求结构。"));
    }
    Ok(())
}

fn prompt_source_prompt_type(prompt: &chapter_video_prompt::Model) -> Option<&str> {
    prompt
        .metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("source_prompt_type"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

async fn provider_request_preview_config_id(
    state: &AppState,
    prompt: &chapter_video_prompt::Model,
    requested_config_id: Option<i64>,
) -> Result<Option<i64>, String> {
    if requested_config_id.is_some() {
        return Ok(requested_config_id);
    }
    let Some(video_id) = prompt.generated_video_id else {
        return Ok(None);
    };
    let video = generated_video::Entity::find_by_id(video_id)
        .one(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("视频提示词 #{} 关联的视频 #{video_id} 不存在", prompt.id))?;
    linked_video_preview_config_id(prompt, &video)
}

pub(super) fn linked_video_preview_config_id(
    prompt: &chapter_video_prompt::Model,
    video: &generated_video::Model,
) -> Result<Option<i64>, String> {
    if video.project_id != prompt.project_id || video.chapter_id != Some(prompt.chapter_id) {
        return Err(format!(
            "视频提示词 #{} 关联的视频 #{} 不属于同一章节",
            prompt.id, video.id
        ));
    }
    let Some(config_id) = video
        .metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("config_id"))
    else {
        return Ok(None);
    };
    let config_id = config_id.as_i64().ok_or_else(|| {
        format!(
            "视频提示词 #{} 关联的视频 #{} metadata_json.config_id 必须是正整数",
            prompt.id, video.id
        )
    })?;
    if config_id <= 0 {
        return Err(format!(
            "视频提示词 #{} 关联的视频 #{} metadata_json.config_id 必须是正整数",
            prompt.id, video.id
        ));
    }
    Ok(Some(config_id))
}

pub(super) fn ensure_provider_generation_workflow(
    prompt: &chapter_video_prompt::Model,
) -> Result<(), String> {
    match prompt.workflow_mode.as_str() {
        "boundary_frames" | "first_frame_chain" | "storyboard" => Ok(()),
        "chat_video" => Err(format!(
            "视频提示词 #{} 属于 Chat 生视频模式，不使用应用内视频 provider 生成；请复制提示词到外部 Chat 工具生成后导入视频",
            prompt.id
        )),
        other => Err(format!(
            "视频提示词 #{} workflow_mode 无效: {}",
            prompt.id, other
        )),
    }
}

pub(super) fn prompt_provider_workflow(
    prompt: &chapter_video_prompt::Model,
) -> Result<ProviderGenerationWorkflow, String> {
    ensure_provider_generation_workflow(prompt)?;
    match prompt.workflow_mode.as_str() {
        "boundary_frames" => Ok(ProviderGenerationWorkflow::BoundaryFrames),
        "first_frame_chain" => Ok(ProviderGenerationWorkflow::FirstFrameChain),
        "storyboard" => Ok(ProviderGenerationWorkflow::Storyboard),
        other => Err(format!(
            "视频提示词 #{} workflow_mode 无效: {}",
            prompt.id, other
        )),
    }
}

pub(super) fn prompt_is_first_frame_chain(
    prompt: &chapter_video_prompt::Model,
) -> Result<bool, String> {
    Ok(prompt_provider_workflow(prompt)?.uses_first_frame_chain())
}

fn first_frame_chain_missing_start_image_message(prompt: &chapter_video_prompt::Model) -> String {
    if prompt.shot_number <= 1 {
        "首帧串联第 1 个视频单元缺少首帧图片，请先生成首帧提示词对应的首帧图片".to_string()
    } else {
        format!(
            "首帧串联第 {} 个视频单元缺少上一视频单元尾帧图片，请先生成并完成第 {} 个视频单元，让系统采集尾帧后再继续",
            prompt.shot_number,
            prompt.shot_number - 1
        )
    }
}

pub(super) fn boundary_frame_missing_image_message(
    prompt: &chapter_video_prompt::Model,
    frame_ids: &BoundaryFrameImageIds,
) -> Option<String> {
    let (first_frame_number, last_frame_number) = match boundary_frame_numbers_for_prompt(prompt) {
        Ok(numbers) => numbers,
        Err(message) => return Some(message),
    };
    let mut missing = Vec::new();
    if frame_ids.first.is_none() {
        missing.push(format!("第 {first_frame_number} 张首帧边界图"));
    }
    if frame_ids.last.is_none() {
        missing.push(format!("第 {last_frame_number} 张尾帧边界图"));
    }
    if missing.is_empty() {
        return None;
    }
    Some(format!(
        "首尾帧模式第 {} 个视频单元缺少{}，请先在首尾帧生成里生成对应边界帧图片",
        prompt.shot_number,
        missing.join("、")
    ))
}

fn storyboard_missing_image_message(prompt: &chapter_video_prompt::Model) -> String {
    format!(
        "分镜模式第 {} 个视频单元缺少分镜图，请先生成分镜提示词对应的分镜图片",
        prompt.shot_number
    )
}

pub(super) fn reference_ids_with_boundary_frame_images(
    frame_ids: &BoundaryFrameImageIds,
    prompt_reference_ids: &[i64],
) -> Vec<i64> {
    let mut ids = Vec::new();
    for id in [frame_ids.first, frame_ids.last]
        .into_iter()
        .flatten()
        .chain(prompt_reference_ids.iter().copied())
    {
        if id > 0 && !ids.contains(&id) {
            ids.push(id);
        }
    }
    ids
}

pub(super) fn reference_ids_with_storyboard_image(
    storyboard_image_id: Option<i64>,
    prompt_reference_ids: &[i64],
) -> Vec<i64> {
    let mut ids = Vec::new();
    for id in storyboard_image_id
        .into_iter()
        .chain(prompt_reference_ids.iter().copied())
    {
        if id > 0 && !ids.contains(&id) {
            ids.push(id);
        }
    }
    ids
}

pub(super) fn ensure_newapi_first_frame_image_supported(
    image_url: Option<&str>,
) -> Result<(), String> {
    let Some(image_url) = image_url.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(());
    };
    let Some(mime) = data_url_mime(image_url)? else {
        return Ok(());
    };
    match mime.as_str() {
        "image/png" | "image/jpeg" | "image/jpg" => Ok(()),
        other => Err(format!(
            "New API 视频首帧 image 仅支持 PNG/JPEG data URL，当前首帧是 {other}；请先使用 PNG/JPEG 图片作为首帧。素材参考图、WebP/GIF 或参考板不能放进 New API 的首帧 image 字段"
        )),
    }
}

fn data_url_mime(value: &str) -> Result<Option<String>, String> {
    let value = value.trim();
    let Some(rest) = value.strip_prefix("data:") else {
        return Ok(None);
    };
    let Some((mime, _)) = rest.split_once(";base64,") else {
        return Err(
            "New API 视频首帧 image data URL 格式无效，必须是 data:image/png;base64,... 或 data:image/jpeg;base64,..."
                .to_string(),
        );
    };
    let mime = mime.trim().to_ascii_lowercase();
    if mime.is_empty() {
        return Err("New API 视频首帧 image data URL 缺少 MIME 类型".to_string());
    }
    Ok(Some(mime))
}
