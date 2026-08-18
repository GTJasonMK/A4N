use std::collections::HashMap;

use a4n_api_contract::video::PreviewChapterVideoPromptOverrideInput;
use a4n_core::models::{ai::video_gen_config, chapter::chapter_video_prompt};
use a4n_prompt::rendering::PromptRenderer;
use a4n_video::VideoGenerationRequest;
use serde_json::{Value, json};

use super::{
    CLIPROXY_OPENAI_VIDEO_BACKEND, GEMINI_VEO_VIDEO_BACKEND, MIN_PROVIDER_MOTION_PROMPT_CHARS,
    MIN_PROVIDER_VISUAL_PROMPT_CHARS, NEWAPI_VIDEO_BACKEND, OPENAI_CHAT_VIDEO_BACKEND,
    OPENAI_COMPATIBLE_VIDEO_BACKEND, PromptOutputLanguage, ProviderGenerationWorkflow,
    cliproxy_prompt_length_violation_message, provider_image_binding_violation_messages,
    reference_image_ids, validate_provider_model_text,
};

const CHAPTER_VIDEO_PROVIDER_PROMPT_NAME: &str = "chapter_video_provider_prompt";
const CHAPTER_VIDEO_FIRST_FRAME_PROVIDER_PROMPT_NAME: &str =
    "chapter_video_first_frame_provider_prompt";

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

pub(crate) fn enforce_provider_prompt_length_contract(
    backend: &str,
    prompt: &str,
) -> Result<(), String> {
    match cliproxy_prompt_length_violation_message(backend, prompt, CLIPROXY_OPENAI_VIDEO_BACKEND) {
        Some(message) => Err(message),
        None => Ok(()),
    }
}

pub(crate) fn enforce_provider_image_binding_contract(
    backend: &str,
    workflow: ProviderGenerationWorkflow,
    prompt_reference_image_count: usize,
) -> Result<(), String> {
    if let Some(message) = provider_image_binding_violation_messages(
        backend,
        workflow,
        prompt_reference_image_count,
        NEWAPI_VIDEO_BACKEND,
        CLIPROXY_OPENAI_VIDEO_BACKEND,
    )
    .into_iter()
    .next()
    {
        return Err(message);
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
    provider_workflow: ProviderGenerationWorkflow,
    image_binding: ProviderPromptImageBinding,
) -> Result<String, String> {
    let (prompt_name, variables) =
        build_provider_prompt_plan(prompt, provider_workflow, image_binding)?;
    prompt_renderer
        .render_prompt(prompt_name, &variables, Some(prompt.project_id))
        .await
        .map(|value| value.trim().to_string())
        .map_err(|error| format!("视频 provider 提示词模板 {prompt_name} 渲染失败：{error}"))
}

fn build_provider_prompt_plan(
    prompt: &chapter_video_prompt::Model,
    provider_workflow: ProviderGenerationWorkflow,
    image_binding: ProviderPromptImageBinding,
) -> Result<(&'static str, HashMap<String, Value>), String> {
    let inserted = prompt.timeline_item_id.is_some();
    let first_frame_chain = provider_workflow.uses_first_frame_chain();
    let prompt_name = if first_frame_chain {
        validate_first_frame_chain_metadata(prompt)?;
        CHAPTER_VIDEO_FIRST_FRAME_PROVIDER_PROMPT_NAME
    } else {
        if !inserted {
            validate_standard_metadata(prompt)?;
        }
        CHAPTER_VIDEO_PROVIDER_PROMPT_NAME
    };
    let variables = build_common_provider_prompt_variables(prompt, image_binding)?;
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
    if prompt.timeline_item_id.is_none() && prompt.shot_number <= 1 {
        required_object(metadata, "start_first_frame", prompt.id)?;
    }
    Ok(())
}

fn build_common_provider_prompt_variables(
    prompt: &chapter_video_prompt::Model,
    image_binding: ProviderPromptImageBinding,
) -> Result<HashMap<String, Value>, String> {
    let prompt_output_language = PromptOutputLanguage::from_metadata(prompt.metadata_json.as_ref())
        .map_err(|error| format!("视频提示词 #{} {error}", prompt.id))?;
    let visual_prompt = validate_provider_model_text(
        &prompt.visual_prompt,
        "visual_prompt",
        prompt.id,
        prompt_output_language,
        MIN_PROVIDER_VISUAL_PROMPT_CHARS,
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
            let note = if context.workflow_mode
                == chapter_video_prompt::WORKFLOW_MODE_STORYBOARD
            {
                format!(
                    "{source}New API 的 `image` 字段只作为视频首帧/初始图，不支持分镜模式需要的 reference_images；provider 参考图 ID 数量：{}，参数不发送；预览省略真实媒体输入，真实生成会被阻止，请切换到支持参考图输入的视频 provider。{input_note}；{prompt_length}",
                    context.provider_reference_image_count
                )
            } else {
                let image_source = if context.workflow_mode
                    == chapter_video_prompt::WORKFLOW_MODE_FIRST_FRAME_CHAIN
                {
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
            let role = if context.workflow_mode == chapter_video_prompt::WORKFLOW_MODE_STORYBOARD
                && index == 0
            {
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

#[cfg(test)]
mod tests {
    use super::*;

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
