use std::collections::HashMap;

use a4n_core::models::{ai::video_gen_config, chapter::chapter_video_prompt};
use a4n_prompt::rendering::PromptRenderer;
use a4n_video::VideoGenerationRequest;
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Value, json};

use super::{
    CLIPROXY_OPENAI_VIDEO_BACKEND, GEMINI_VEO_VIDEO_BACKEND, MIN_PROVIDER_MOTION_PROMPT_CHARS,
    MIN_PROVIDER_VISUAL_PROMPT_CHARS, NEWAPI_VIDEO_BACKEND, OPENAI_CHAT_VIDEO_BACKEND,
    OPENAI_COMPATIBLE_VIDEO_BACKEND, PromptOutputLanguage, ProviderGenerationWorkflow,
    ProviderPromptTextViolation, VideoProviderValidationMode,
    cliproxy_prompt_length_violation_message, prompt_is_first_frame_chain,
    provider_image_binding_violation_messages, provider_prompt_text_violation, reference_image_ids,
    validate_provider_model_text, validation_error_or_preview_note,
};

const CHAPTER_VIDEO_PROVIDER_PROMPT_NAME: &str = "chapter_video_provider_prompt";
const CHAPTER_VIDEO_FIRST_FRAME_PROVIDER_PROMPT_NAME: &str =
    "chapter_video_first_frame_provider_prompt";

pub const VIDEO_PROVIDER_VISUAL_PREVIEW_PLACEHOLDER: &str = "[主体定义与素材绑定]\n【请求预览占位】当前视频 visual_prompt 未完整填写。真实生成前需要补全主体、输入帧或参考图用途、人物站位、场景锚点、构图和光影约束；此占位仅用于展示将发送给视频模型的请求结构。\n\n[片段内容以及前因后果]\n【请求预览占位】真实生成前需要补全片段承接状态、剧情动作、情绪转折和结尾交接状态。\n\n[镜头分镜]\n镜头 1：【请求预览占位】补全起始镜头，交代主体、景别、空间锚点和起始动作。\n镜头 2：【请求预览占位】补全收束镜头，交代动作桥、反应和结尾状态。\n\n[画面风格描述]\n【请求预览占位】真实生成前需要补全画面风格、光线、色彩、镜头质感和材质细节。\n\n[连续性与结尾状态]\n【请求预览占位】真实生成前需要补全人物身份、服装、道具、空间关系、光源方向和可承接尾态。\n\n[限制与避免项]\n【请求预览占位】不要配背景音乐，不要字幕、水印、角色变形、空间跳变或光影不连续。";
pub const VIDEO_PROVIDER_MOTION_PREVIEW_PLACEHOLDER: &str = "[运镜与动作]\n【请求预览占位】当前视频 motion_prompt 未完整填写。真实生成前需要补全机位、镜头运动、速度节奏、人物动作和转场方式；此占位仅用于展示将发送给视频模型的请求结构。\n\n[对白与表演]\n【请求预览占位】真实生成前需要补全对白意图、语气、口型、手势和听者反应；无对白时明确写无对白。";

#[derive(Debug, Clone, Copy, Default)]
pub struct BoundaryFrameImageIds {
    pub first: Option<i64>,
    pub last: Option<i64>,
}

#[derive(Debug, Clone, Copy)]
pub struct ProviderPromptImageBinding {
    pub has_first_frame_image: bool,
    pub has_last_frame_image: bool,
    pub reference_image_count: usize,
    pub has_storyboard_reference_image: bool,
}

#[derive(Clone)]
pub struct ChapterVideoProviderRequestContext {
    pub prompt: chapter_video_prompt::Model,
    pub config: video_gen_config::Model,
    pub request: VideoGenerationRequest,
    pub prompt_reference_image_ids: Option<Value>,
    pub provider_reference_image_ids: Option<Value>,
    pub prompt_source: Option<Value>,
    pub workflow_mode: String,
    pub source_prompt_type: Option<String>,
    pub video_execution_identity: Value,
    pub boundary_frame_ids: BoundaryFrameImageIds,
    pub prompt_reference_image_count: usize,
    pub provider_reference_image_count: usize,
    pub first_frame_image_url_available: bool,
    pub last_frame_image_url_available: bool,
    pub provider_input_note: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PreviewChapterVideoPromptOverrideInput {
    #[serde(default)]
    pub visual_prompt: Option<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_update_field")]
    pub motion_prompt: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_nullable_update_field")]
    pub negative_prompt: Option<Option<String>>,
    #[serde(default)]
    pub duration_secs: Option<i32>,
    #[serde(default, deserialize_with = "deserialize_nullable_update_field")]
    pub reference_image_ids_json: Option<Option<Value>>,
}

fn deserialize_nullable_update_field<'de, D, T>(
    deserializer: D,
) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Ok(Some(Option::<T>::deserialize(deserializer)?))
}

pub fn append_provider_input_note(note: &mut Option<String>, addition: impl AsRef<str>) {
    let addition = addition.as_ref().trim();
    if addition.is_empty() {
        return;
    }
    match note {
        Some(note) if !note.trim().is_empty() => {
            note.push('；');
            note.push_str(addition);
        }
        _ => *note = Some(addition.to_string()),
    }
}

pub fn append_provider_input_notes(note: &mut Option<String>, additions: Vec<String>) {
    for addition in additions {
        append_provider_input_note(note, addition);
    }
}

pub(crate) fn enforce_provider_prompt_length_contract(
    backend: &str,
    prompt: &str,
    mode: VideoProviderValidationMode,
    note: &mut Option<String>,
) -> Result<(), String> {
    let Some(message) =
        cliproxy_prompt_length_violation_message(backend, prompt, CLIPROXY_OPENAI_VIDEO_BACKEND)
    else {
        return Ok(());
    };
    reject_generation_or_note_preview(mode, note, message, " 当前仅展示请求结构。")
}

pub(crate) fn enforce_provider_image_binding_contract(
    backend: &str,
    workflow: ProviderGenerationWorkflow,
    prompt_reference_image_count: usize,
    mode: VideoProviderValidationMode,
    note: &mut Option<String>,
) -> Result<(), String> {
    for message in provider_image_binding_violation_messages(
        backend,
        workflow,
        prompt_reference_image_count,
        NEWAPI_VIDEO_BACKEND,
        CLIPROXY_OPENAI_VIDEO_BACKEND,
    ) {
        reject_generation_or_note_preview(mode, note, message, " 当前仅展示请求结构。")?;
    }
    Ok(())
}

fn reject_generation_or_note_preview(
    mode: VideoProviderValidationMode,
    note: &mut Option<String>,
    message: String,
    preview_suffix: &str,
) -> Result<(), String> {
    if let Some(extra) = validation_error_or_preview_note(mode, message, preview_suffix)? {
        append_provider_input_note(note, extra);
    }
    Ok(())
}

pub fn provider_specific_negative_prompt(specific: Option<&str>) -> Option<String> {
    specific
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .filter(|value| !matches!(*value, "无" | "无特殊负面词" | "无特殊要求"))
        .map(str::to_string)
}

pub async fn build_video_generation_provider_prompt_with_renderer(
    prompt_renderer: &dyn PromptRenderer,
    prompt: &chapter_video_prompt::Model,
    image_binding: ProviderPromptImageBinding,
) -> Result<String, String> {
    build_video_generation_provider_prompt_inner(prompt_renderer, prompt, image_binding, false)
        .await
}

pub async fn build_video_generation_provider_preview_prompt_with_renderer(
    prompt_renderer: &dyn PromptRenderer,
    prompt: &chapter_video_prompt::Model,
    image_binding: ProviderPromptImageBinding,
) -> Result<String, String> {
    build_video_generation_provider_prompt_inner(prompt_renderer, prompt, image_binding, true).await
}

async fn build_video_generation_provider_prompt_inner(
    prompt_renderer: &dyn PromptRenderer,
    prompt: &chapter_video_prompt::Model,
    image_binding: ProviderPromptImageBinding,
    allow_preview_placeholders: bool,
) -> Result<String, String> {
    let (prompt_name, variables) =
        build_provider_prompt_plan(prompt, image_binding, allow_preview_placeholders)?;
    prompt_renderer
        .render_prompt(prompt_name, &variables, Some(prompt.project_id))
        .await
        .map(|value| value.trim().to_string())
        .map_err(|error| format!("视频 provider 提示词模板 {prompt_name} 渲染失败：{error}"))
}

fn build_provider_prompt_plan(
    prompt: &chapter_video_prompt::Model,
    image_binding: ProviderPromptImageBinding,
    allow_preview_placeholders: bool,
) -> Result<(&'static str, HashMap<String, Value>), String> {
    let prompt_name = if prompt_is_first_frame_chain(prompt)? {
        validate_first_frame_chain_metadata(prompt)?;
        CHAPTER_VIDEO_FIRST_FRAME_PROVIDER_PROMPT_NAME
    } else {
        validate_standard_metadata(prompt)?;
        CHAPTER_VIDEO_PROVIDER_PROMPT_NAME
    };
    let variables =
        build_common_provider_prompt_variables(prompt, image_binding, allow_preview_placeholders)?;
    Ok((prompt_name, variables))
}

fn validate_standard_metadata(prompt: &chapter_video_prompt::Model) -> Result<(), String> {
    if prompt.metadata_json.is_none() {
        return Err(format!(
            "视频提示词 #{} 缺少结构化元数据，请重新生成视频提示词后再生成视频",
            prompt.id
        ));
    }
    Ok(())
}

fn validate_first_frame_chain_metadata(prompt: &chapter_video_prompt::Model) -> Result<(), String> {
    let metadata = prompt.metadata_json.as_ref().ok_or_else(|| {
        format!(
            "视频提示词 #{} 缺少结构化元数据，请重新生成首帧串联视频提示词后再生成视频",
            prompt.id
        )
    })?;
    if prompt.shot_number <= 1 {
        required_object(metadata, "start_first_frame", prompt.id)?;
    }
    Ok(())
}

fn build_common_provider_prompt_variables(
    prompt: &chapter_video_prompt::Model,
    image_binding: ProviderPromptImageBinding,
    allow_preview_placeholders: bool,
) -> Result<HashMap<String, Value>, String> {
    let prompt_output_language = PromptOutputLanguage::from_metadata(prompt.metadata_json.as_ref());
    let visual_prompt = validate_provider_model_text(
        &prompt.visual_prompt,
        "visual_prompt",
        prompt.id,
        prompt_output_language,
        MIN_PROVIDER_VISUAL_PROMPT_CHARS,
        allow_preview_placeholders,
    )?;
    let raw_motion_prompt = prompt
        .motion_prompt
        .as_deref()
        .ok_or_else(|| format!("视频提示词 #{} 缺少 motion_prompt", prompt.id))?;
    let motion_prompt = validate_provider_model_text(
        raw_motion_prompt,
        "motion_prompt",
        prompt.id,
        prompt_output_language,
        MIN_PROVIDER_MOTION_PROMPT_CHARS,
        allow_preview_placeholders,
    )?;
    required_prompt_aspect_ratio(prompt)?;
    let mut variables = HashMap::new();
    variables.insert(
        "has_first_frame_image".to_string(),
        json!(image_binding.has_first_frame_image),
    );
    variables.insert("visual_prompt".to_string(), json!(visual_prompt));
    variables.insert("motion_prompt".to_string(), json!(motion_prompt));
    variables.insert(
        "subject_asset_bindings".to_string(),
        json!(provider_subject_asset_bindings(image_binding)),
    );
    Ok(variables)
}

fn provider_subject_asset_bindings(image_binding: ProviderPromptImageBinding) -> Option<String> {
    let mut lines = Vec::new();
    if image_binding.has_first_frame_image {
        lines.push(
            "- 首帧输入图片是视频第 0 秒画面；从该画面的构图、人物站位、姿态、道具和光影自然推进。"
                .to_string(),
        );
    }
    if image_binding.has_last_frame_image {
        lines.push(
            "- 尾帧输入图片是视频结尾目标；收束到该画面的构图、人物站位、姿态、道具和光影。"
                .to_string(),
        );
    }
    if image_binding.reference_image_count > 0 {
        if image_binding.has_storyboard_reference_image {
            lines.push(format!(
                "- 本请求按顺序输入 {} 张参考图；第 1 张参考图是当前片段分镜图，只用于约束构图、主体姿态、空间关系和光影，不是视频首帧定格；其余参考图继续按输入顺序用于人物、场景、道具、服装和风格一致性。",
                image_binding.reference_image_count
            ));
        } else {
            lines.push(format!(
                "- 本请求按顺序输入 {} 张参考图；提示词中的“参考图 N”对应第 N 张输入图片，只用于人物、场景、道具、服装、边界状态和风格一致性。",
                image_binding.reference_image_count
            ));
        }
    }
    (!lines.is_empty()).then(|| lines.join("\n"))
}

fn required_prompt_aspect_ratio(prompt: &chapter_video_prompt::Model) -> Result<&str, String> {
    prompt
        .aspect_ratio
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("视频提示词 #{} 缺少 aspect_ratio", prompt.id))
}

fn required_object<'a>(
    metadata: &'a Value,
    key: &str,
    prompt_id: i64,
) -> Result<&'a Value, String> {
    let value = metadata.get(key).ok_or_else(|| {
        format!(
            "视频提示词 #{} 缺少 {} 元数据，请重新生成视频提示词后再生成视频",
            prompt_id, key
        )
    })?;
    if !value.is_object() {
        return Err(format!("视频提示词 #{} 的 {} 元数据无效", prompt_id, key));
    }
    Ok(value)
}

pub fn apply_chapter_video_prompt_preview_override(
    prompt: &mut chapter_video_prompt::Model,
    input: Option<PreviewChapterVideoPromptOverrideInput>,
    note: &mut Option<String>,
) -> Result<(), String> {
    let Some(input) = input else {
        return Ok(());
    };
    if input.duration_secs.is_some_and(|value| value <= 0) {
        return Err("视频提示词 duration_secs 必须是正整数".to_string());
    }
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
        prompt.duration_secs = duration_secs;
    }
    if let Some(reference_image_ids_json) = input.reference_image_ids_json {
        prompt.reference_image_ids_json = reference_image_ids_json;
    }
    Ok(())
}

pub fn ensure_chapter_video_prompt_preview_texts(
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
    prompt.motion_prompt = Some(preview_model_text_or_placeholder(
        prompt.motion_prompt.as_deref().unwrap_or_default(),
        "motion_prompt",
        prompt_output_language,
        MIN_PROVIDER_MOTION_PROMPT_CHARS,
        VIDEO_PROVIDER_MOTION_PREVIEW_PLACEHOLDER,
        note,
    ));
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
    let Some(violation) =
        provider_prompt_text_violation(trimmed, field, prompt_output_language, min_chars, false)
    else {
        return trimmed.to_string();
    };
    match violation {
        ProviderPromptTextViolation::DictionaryLike => replace_with_preview_placeholder(
            note,
            field,
            "包含 JSON/字典结构片段",
            "真实生成前需要改写为自然语言",
            placeholder,
        ),
        ProviderPromptTextViolation::InternalWorkflowTerm(term) => {
            replace_with_preview_placeholder(
                note,
                field,
                &format!("包含视频生成内部流程说明“{term}”"),
                "真实生成前需要改写为只描述画面和动作的自然语言",
                placeholder,
            )
        }
        ProviderPromptTextViolation::UploadStatusTerm(term) => replace_with_preview_placeholder(
            note,
            field,
            &format!("包含上传状态说明“{term}”"),
            "真实生成前需要改写为实际起点、输入帧画面或上一段尾态",
            placeholder,
        ),
        ProviderPromptTextViolation::DuplicateMotionTimeline => replace_with_preview_placeholder(
            note,
            field,
            "重复镜头分镜",
            "真实生成前需要把时间轴只保留在 visual_prompt",
            placeholder,
        ),
        ProviderPromptTextViolation::MissingRequiredSection(section) => {
            replace_with_preview_placeholder(
                note,
                field,
                &format!("缺少必需章节 {section}"),
                "真实生成前需要补全完整提示词",
                placeholder,
            )
        }
        ProviderPromptTextViolation::StaleSection(section) => replace_with_preview_placeholder(
            note,
            field,
            &format!("包含已废弃章节 {section}"),
            "真实生成前需要重新生成当前格式提示词",
            placeholder,
        ),
        ProviderPromptTextViolation::DirectPrompt(message) => replace_with_preview_placeholder(
            note,
            field,
            &format!("不符合自然语言提示词约束：{message}"),
            "真实生成前需要改写",
            placeholder,
        ),
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
        ProviderPromptTextViolation::PreviewPlaceholder => replace_with_preview_placeholder(
            note,
            field,
            "包含请求预览占位文本",
            "真实生成前需要保存完整提示词",
            placeholder,
        ),
    }
}

fn replace_with_preview_placeholder(
    note: &mut Option<String>,
    field: &str,
    violation: &str,
    requirement: &str,
    placeholder: &str,
) -> String {
    append_provider_input_note(
        note,
        format!("{field} {violation}；预览使用占位文本展示请求结构，{requirement}。"),
    );
    placeholder.to_string()
}

pub fn ensure_chapter_video_prompt_preview_metadata(
    prompt: &mut chapter_video_prompt::Model,
    note: &mut Option<String>,
) -> Result<(), String> {
    let provider_workflow = super::prompt_provider_workflow(prompt)?;
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
    if provider_workflow.uses_text_only() || provider_workflow.uses_keyframes() {
        // These modes intentionally have no system-frame metadata.
    } else if provider_workflow.uses_first_frame_chain() {
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
    for (key, value) in preview_shot_sections() {
        if shot.contains_key(key) {
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
        "title": prompt.title.as_deref().unwrap_or("请求预览占位镜头"),
        "scene_summary": prompt.scene_summary.as_deref().unwrap_or("请求预览占位剧情摘要；真实生成前需要重新生成完整视频提示词。"),
        "blocking_plan": {"preview_placeholder": true, "requirement": "真实生成前需要补全人物站位、走位、空间轴线和首尾状态。"},
        "lighting_plan": {"preview_placeholder": true, "requirement": "真实生成前需要补全主光方向、色温、阴影和首尾帧光影连续性。"},
        "emotion_action_beats": [{"preview_placeholder": true, "requirement": "真实生成前需要补全情绪变化、可见动作和节拍。"}],
        "dialogue_beats": [],
        "cinematography_plan": {"preview_placeholder": true, "requirement": "真实生成前需要补全景别、机位、镜头运动、轴线和焦段提示。"},
        "temporal_beats": [{"preview_placeholder": true, "requirement": "真实生成前需要补全分秒段动作、镜头和连续性目标。"}],
        "reference_binding": {"bindings": [], "coverage_check": "请求预览占位；真实生成前需要覆盖所有参考图。"},
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
        "scene_summary": prompt.scene_summary.as_deref().unwrap_or("请求预览占位帧摘要；真实生成前需要重新生成完整帧提示词。"),
        "frame_prompt": format!(
            "【请求预览占位】{label}结构化帧提示缺失。真实生成前需要补全静态画面、人物站位、构图、光影、服装道具和可承接的结尾状态。"
        ),
        "blocking_state": {"preview_placeholder": true, "requirement": "真实生成前需要补全人物、位置、姿态、道具状态和画面轴线。"},
        "lighting_state": {"preview_placeholder": true, "requirement": "真实生成前需要补全主光方向、阴影、色温和环境光。"}
    })
}

fn preview_shot_sections() -> [(&'static str, Value); 12] {
    [
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
            json!({"preview_placeholder": true, "requirement": "真实生成前需要补全文字化起始状态，包括人物站位、姿态、朝向、场景锚点、机位和光影。"}),
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
            json!({"preview_placeholder": true, "requirement": "真实生成前需要补全文字化结尾状态，作为下一段 start_state 的承接来源。"}),
        ),
        (
            "blocking_plan",
            json!({"preview_placeholder": true, "requirement": "真实生成前需要补全人物站位、走位、空间轴线和首尾状态。"}),
        ),
        (
            "lighting_plan",
            json!({"preview_placeholder": true, "requirement": "真实生成前需要补全主光方向、色温、阴影和首尾帧光影连续性。"}),
        ),
        (
            "emotion_action_beats",
            json!([{"preview_placeholder": true, "requirement": "真实生成前需要补全情绪变化、可见动作和节拍。"}]),
        ),
        ("dialogue_beats", json!([])),
        (
            "cinematography_plan",
            json!({"preview_placeholder": true, "requirement": "真实生成前需要补全景别、机位、镜头运动、轴线和焦段提示。"}),
        ),
        (
            "temporal_beats",
            json!([{"preview_placeholder": true, "requirement": "真实生成前需要补全分秒段动作、镜头和连续性目标。"}]),
        ),
        (
            "reference_binding",
            json!({"bindings": [], "coverage_check": "请求预览占位；真实生成前需要覆盖所有参考图。"}),
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
    ]
}

pub(crate) struct ChapterVideoProviderDescriptor {
    pub(crate) request_kind: &'static str,
    pub(crate) request_title: &'static str,
    pub(crate) note: String,
}

pub(crate) fn chapter_video_provider_descriptor(
    context: &ChapterVideoProviderRequestContext,
    source_label: &str,
) -> Result<ChapterVideoProviderDescriptor, String> {
    let prompt_length = video_request_prompt_length_context(&context.request);
    let input_note = context
        .provider_input_note
        .as_deref()
        .map(|note| format!("；{note}"))
        .unwrap_or_default();
    let source = format!("{source_label}；");
    let descriptor = match context.config.backend.as_str() {
        NEWAPI_VIDEO_BACKEND => {
            let note = if context.workflow_mode == "storyboard" {
                format!(
                    "{source}New API 的 `image` 字段只作为视频首帧/初始图，不支持分镜模式需要的 reference_images；provider 参考图 ID 数量：{}，参数不发送；预览省略真实媒体输入，真实生成会被阻止，请切换到支持参考图输入的视频 provider。{input_note}；{prompt_length}",
                    context.provider_reference_image_count
                )
            } else {
                let image_source = if context.workflow_mode == "first_frame_chain" {
                    "首帧串联起始帧"
                } else {
                    "首尾帧模式首帧"
                };
                format!(
                    "{source}New API 的 `image` 字段只作为视频首帧/初始图；提示词参考图不会放入 `image`，也不会作为参考图参数发送。{} image 绑定 generated_image：{}；尾帧 generated_image：{}，参数不发送；提示词参考图 ID 数量：{}，参数不发送；预览省略真实媒体输入，生成时仅首帧会作为 PNG/JPEG data URL 发送。{input_note}；{prompt_length}",
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
                )
            };
            ChapterVideoProviderDescriptor {
                request_kind: "video_generation_newapi",
                request_title: "New API /video/generations 请求",
                note,
            }
        }
        OPENAI_COMPATIBLE_VIDEO_BACKEND => ChapterVideoProviderDescriptor {
            request_kind: "video_generation_openai_compatible",
            request_title: "OpenAI 兼容 /videos/generations 请求",
            note: format!(
                "{source}OpenAI 兼容视频 `/videos/generations` 请求；provider 参考图：{}；提示词参考图：{}；首尾帧媒体输入：{} / {}；预览省略真实媒体输入，生成时默认发送图片 data URL；如设置 A4N_VIDEO_IMAGE_INPUT_TRANSPORT=public_url，则发送公网 URL；如设置为 remote_temp_upload，则先上传到远端临时媒体接口再发送返回 URL。{input_note}；{prompt_length}",
                context.provider_reference_image_count,
                context.prompt_reference_image_count,
                context.first_frame_image_url_available,
                context.last_frame_image_url_available
            ),
        },
        OPENAI_CHAT_VIDEO_BACKEND => ChapterVideoProviderDescriptor {
            request_kind: "video_generation_chat_completions",
            request_title: "OpenAI Chat-Completions 视频请求",
            note: format!(
                "{source}OpenAI Chat-Completions 视频请求；provider 参考图：{}；提示词参考图：{}；首尾帧媒体输入：{} / {}；预览省略真实媒体输入，生成时默认发送图片 data URL；如设置 A4N_VIDEO_IMAGE_INPUT_TRANSPORT=public_url，则发送公网 URL；如设置为 remote_temp_upload，则先上传到远端临时媒体接口再发送返回 URL。{input_note}；{prompt_length}",
                context.provider_reference_image_count,
                context.prompt_reference_image_count,
                context.first_frame_image_url_available,
                context.last_frame_image_url_available
            ),
        },
        GEMINI_VEO_VIDEO_BACKEND => ChapterVideoProviderDescriptor {
            request_kind: "video_generation_gemini_veo",
            request_title: "Gemini Veo predictLongRunning 请求",
            note: format!(
                "{source}Gemini Veo 原生 `predictLongRunning` 请求；首尾帧媒体输入：{} / {}；provider 参考图：{}；提示词参考图：{}；预览用 inlineData 占位展示实际请求形状，生成时发送真实图片 data URL。{input_note}；{prompt_length}",
                context.first_frame_image_url_available,
                context.last_frame_image_url_available,
                context.provider_reference_image_count,
                context.prompt_reference_image_count
            ),
        },
        CLIPROXY_OPENAI_VIDEO_BACKEND => ChapterVideoProviderDescriptor {
            request_kind: "video_generation_cliproxy_openai",
            request_title: "CLIProxy / Grok 视频请求",
            note: format!(
                "{source}CLIProxy / Grok 请求形态由当前片段素材决定：无图片时纯文本生成，首帧通过 image 发送，参考素材通过 reference_images 发送；首帧与参考图不可混用，端点不支持尾帧。Base URL 为 `/v1` 时创建路由是 `/videos/generations`，为 `/openai/v1` 时是包装路由 `/videos`。当前首帧/尾帧：{} / {}；provider 参考图：{}；提示词参考图：{}；默认直接发送图片 data URL。{input_note}；{prompt_length}",
                context.first_frame_image_url_available,
                context.last_frame_image_url_available,
                context.request.media.reference_images().len(),
                context.prompt_reference_image_count,
            ),
        },
        backend => ChapterVideoProviderDescriptor {
            request_kind: "video_provider_request",
            request_title: "章节视频 provider 请求",
            note: format!(
                "{source}视频 provider 请求；backend={backend}；provider 参考图：{}；提示词参考图：{}；首尾帧媒体输入：{} / {}。{input_note}；{prompt_length}",
                context.provider_reference_image_count,
                context.prompt_reference_image_count,
                context.first_frame_image_url_available,
                context.last_frame_image_url_available
            ),
        },
    };
    Ok(descriptor)
}

pub fn video_request_prompt_length_context(request: &VideoGenerationRequest) -> String {
    format!(
        "实际发送 prompt：{} 字符 / {} UTF-8 字节",
        request.prompt.chars().count(),
        request.prompt.len()
    )
}

pub fn append_video_request_prompt_length_context(error: String, context: &str) -> String {
    if error.contains(context) {
        error
    } else {
        format!("{error}；{context}")
    }
}

pub(crate) fn sanitized_chapter_video_request_for_snapshot(
    context: &ChapterVideoProviderRequestContext,
) -> Result<VideoGenerationRequest, String> {
    let reference_ids = reference_image_ids(context.provider_reference_image_ids.as_ref())?;
    let first_frame_image_url = context
        .boundary_frame_ids
        .first
        .filter(|_| context.request.media.first_frame().is_some())
        .map(|id| historical_image_placeholder_media_url(id, "首帧"));
    let last_frame_image_url = context
        .boundary_frame_ids
        .last
        .filter(|_| context.request.media.last_frame().is_some())
        .map(|id| historical_image_placeholder_media_url(id, "尾帧"));
    let reference_image_urls = reference_ids
        .into_iter()
        .enumerate()
        .take(context.request.media.reference_images().len())
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
        media: a4n_video::VideoMediaInput::from_images(
            first_frame_image_url,
            last_frame_image_url,
            reference_image_urls,
        )
        .map_err(|error| error.to_string())?,
    })
}

fn historical_image_placeholder_media_url(id: i64, role: &str) -> String {
    format!(
        "https://example.invalid/a4n-media/<omitted for historical provider request snapshot: {role} generated_image #{id}; media input was omitted from snapshot>"
    )
}

pub(crate) fn attach_chapter_video_snapshot_identity(
    mut snapshot: Value,
    context: &ChapterVideoProviderRequestContext,
) -> Value {
    let Some(object) = snapshot.as_object_mut() else {
        return snapshot;
    };
    object.insert(
        "video_execution_identity".to_string(),
        context.video_execution_identity.clone(),
    );
    object.insert(
        "source_prompt_type".to_string(),
        context
            .source_prompt_type
            .clone()
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    object.insert(
        "workflow_mode".to_string(),
        Value::String(context.workflow_mode.clone()),
    );
    snapshot
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_override_preserves_missing_and_explicit_null() {
        let missing = serde_json::from_value::<PreviewChapterVideoPromptOverrideInput>(json!({}))
            .expect("missing fields should deserialize");
        assert_eq!(missing.motion_prompt, None);
        assert_eq!(missing.reference_image_ids_json, None);

        let cleared = serde_json::from_value::<PreviewChapterVideoPromptOverrideInput>(json!({
            "motion_prompt": null,
            "reference_image_ids_json": null
        }))
        .expect("explicit null should deserialize as a clear operation");
        assert_eq!(cleared.motion_prompt, Some(None));
        assert_eq!(cleared.reference_image_ids_json, Some(None));
    }

    #[test]
    fn negative_prompt_and_length_context_are_stable() {
        assert_eq!(
            provider_specific_negative_prompt(Some("  无特殊要求  ")),
            None
        );
        assert_eq!(
            provider_specific_negative_prompt(Some("  手机屏幕反光过强  ")),
            Some("手机屏幕反光过强".to_string())
        );
        let context = "实际发送 prompt：10 字符 / 20 UTF-8 字节";
        assert_eq!(
            append_video_request_prompt_length_context("provider failed".to_string(), context),
            format!("provider failed；{context}")
        );
        assert_eq!(
            append_video_request_prompt_length_context(
                format!("provider failed；{context}"),
                context,
            ),
            format!("provider failed；{context}")
        );
    }
}
