//! 视频 provider 请求：参考图 ID、CLIProxy 模式/模型、提示词元数据纯规则（无 DB、无 Host）。

use serde_json::{Value, json};

use super::{FRAME_SEGMENT_PLAN_PROMPT_TYPE, boundary_frame_numbers_for_prompt};

/// CLIProxy / Grok 参考图数量上限。
pub const MAX_CLIPROXY_VIDEO_REFERENCE_IMAGES: usize = 7;

/// CLIProxy / Grok 视频 prompt 字符上限。
pub const MAX_CLIPROXY_VIDEO_PROMPT_CHARS: usize = 4096;

pub fn provider_request_snapshot_value(
    key: &str,
    title: &str,
    kind: &str,
    prompt_name: Option<&str>,
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

/// CLIProxy / Grok 参考图输入模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliproxyGrokReferenceMode {
    Reference,
    FirstFrame,
}

/// 应用内视频 provider 支持的工作流。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderGenerationWorkflow {
    BoundaryFrames,
    FirstFrameChain,
    Storyboard,
    Keyframes,
    TextOnly,
}

impl ProviderGenerationWorkflow {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BoundaryFrames => "boundary_frames",
            Self::FirstFrameChain => "first_frame_chain",
            Self::Storyboard => "storyboard",
            Self::Keyframes => "keyframes",
            Self::TextOnly => "text_only",
        }
    }

    pub fn uses_first_frame_chain(self) -> bool {
        matches!(self, Self::FirstFrameChain)
    }

    pub fn uses_storyboard(self) -> bool {
        matches!(self, Self::Storyboard)
    }

    pub fn uses_keyframes(self) -> bool {
        matches!(self, Self::Keyframes)
    }

    pub fn uses_text_only(self) -> bool {
        matches!(self, Self::TextOnly)
    }
}

/// 解析 `provider_options_json.grok_reference_mode`。
pub fn parse_cliproxy_grok_reference_mode(
    provider_options_json: Option<&Value>,
) -> Result<CliproxyGrokReferenceMode, String> {
    let Some(value) = provider_options_json.filter(|value| !value.is_null()) else {
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

/// 模式中文标签（预览文案）。
pub fn cliproxy_grok_reference_mode_label(mode: CliproxyGrokReferenceMode) -> &'static str {
    match mode {
        CliproxyGrokReferenceMode::Reference => "参考模式",
        CliproxyGrokReferenceMode::FirstFrame => "首帧模式",
    }
}

/// 校验提示词 `source_prompt_type` 为当前共享片段规划合同。
pub fn required_prompt_source_prompt_type(
    prompt_id: i64,
    metadata_json: Option<&Value>,
) -> Result<&str, String> {
    let source_prompt_type = metadata_json
        .and_then(|metadata| metadata.get("source_prompt_type"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            format!(
                "视频提示词 #{prompt_id} 缺少 source_prompt_type，请按当前共享片段规划合同重新生成"
            )
        })?;
    if source_prompt_type != FRAME_SEGMENT_PLAN_PROMPT_TYPE {
        return Err(format!(
            "视频提示词 #{prompt_id} source_prompt_type={source_prompt_type}，当前要求 {}",
            FRAME_SEGMENT_PLAN_PROMPT_TYPE
        ));
    }
    Ok(source_prompt_type)
}

pub fn ensure_provider_generation_workflow(
    prompt: &a4n_core::models::chapter::chapter_video_prompt::Model,
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

pub fn prompt_provider_workflow(
    prompt: &a4n_core::models::chapter::chapter_video_prompt::Model,
) -> Result<ProviderGenerationWorkflow, String> {
    ensure_provider_generation_workflow(prompt)?;
    provider_generation_workflow_for_values(
        prompt.id,
        &prompt.workflow_mode,
        prompt.metadata_json.as_ref(),
    )
}

fn provider_generation_workflow_for_values(
    prompt_id: i64,
    workflow_mode: &str,
    metadata_json: Option<&Value>,
) -> Result<ProviderGenerationWorkflow, String> {
    let guidance_mode = metadata_json
        .and_then(|metadata| metadata.get("guidance_mode"))
        .and_then(Value::as_str)
        .ok_or_else(|| format!("视频提示词 #{} 缺少 guidance_mode", prompt_id))?;
    match guidance_mode {
        "none" => return Ok(ProviderGenerationWorkflow::TextOnly),
        "keyframes" => return Ok(ProviderGenerationWorkflow::Keyframes),
        "continuity" if workflow_mode != "first_frame_chain" => {
            return Err(format!(
                "视频提示词 #{} 只有首帧续接方案可以使用 continuity 引导",
                prompt_id
            ));
        }
        "continuity" | "system_frames" => {}
        other => {
            return Err(format!(
                "视频提示词 #{} guidance_mode 无效: {other}",
                prompt_id
            ));
        }
    }
    match workflow_mode {
        "boundary_frames" => Ok(ProviderGenerationWorkflow::BoundaryFrames),
        "first_frame_chain" => Ok(ProviderGenerationWorkflow::FirstFrameChain),
        "storyboard" => Ok(ProviderGenerationWorkflow::Storyboard),
        other => Err(format!(
            "视频提示词 #{} workflow_mode 无效: {}",
            prompt_id, other
        )),
    }
}

pub fn prompt_is_first_frame_chain(
    prompt: &a4n_core::models::chapter::chapter_video_prompt::Model,
) -> Result<bool, String> {
    Ok(prompt_provider_workflow(prompt)?.uses_first_frame_chain())
}

pub fn first_frame_chain_missing_start_image_message(
    prompt: &a4n_core::models::chapter::chapter_video_prompt::Model,
) -> String {
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

pub fn boundary_frame_missing_image_message(
    prompt: &a4n_core::models::chapter::chapter_video_prompt::Model,
    has_first_frame: bool,
    has_last_frame: bool,
) -> Option<String> {
    let (first_frame_number, last_frame_number) = match boundary_frame_numbers_for_prompt(prompt) {
        Ok(numbers) => numbers,
        Err(message) => return Some(message),
    };
    let mut missing = Vec::new();
    if !has_first_frame {
        missing.push(format!("第 {first_frame_number} 张首帧边界图"));
    }
    if !has_last_frame {
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

pub fn storyboard_missing_image_message(
    prompt: &a4n_core::models::chapter::chapter_video_prompt::Model,
) -> String {
    format!(
        "分镜模式第 {} 个视频单元缺少分镜图，请先生成分镜提示词对应的分镜图片",
        prompt.shot_number
    )
}

/// 校验并返回 `video_execution_identity` 对象拷贝。
pub fn required_video_execution_identity(
    prompt_id: i64,
    metadata_json: Option<&Value>,
) -> Result<Value, String> {
    let metadata = metadata_json.ok_or_else(|| {
        format!("视频提示词 #{prompt_id} 缺少 metadata_json，无法验证视频执行身份")
    })?;
    let identity = metadata
        .get("video_execution_identity")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            format!(
                "视频提示词 #{prompt_id} 缺少 metadata_json.video_execution_identity，请重新生成视频提示词"
            )
        })?;
    if identity.get("contract_epoch").and_then(Value::as_u64)
        != Some(a4n_core::video_execution::VIDEO_EXECUTION_CONTRACT_EPOCH as u64)
    {
        return Err(format!(
            "视频提示词 #{prompt_id} 的 video_execution_identity.contract_epoch 不是当前合同 {}",
            a4n_core::video_execution::VIDEO_EXECUTION_CONTRACT_EPOCH
        ));
    }
    for key in [
        "projection_sha256",
        "segment_plan_sha256",
        "unit_plan_sha256",
        "frame_set_sha256",
        "source_content_sha256",
        "style_sha256",
        "aspect_ratio",
        "prompt_output_language",
    ] {
        if identity
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .is_none_or(str::is_empty)
        {
            return Err(format!(
                "视频提示词 #{prompt_id} 的 video_execution_identity.{key} 缺少有效字符串"
            ));
        }
    }
    if identity
        .get("duration_secs")
        .and_then(Value::as_i64)
        .is_none_or(|value| value <= 0)
    {
        return Err(format!(
            "视频提示词 #{prompt_id} 的 video_execution_identity.duration_secs 必须是正整数"
        ));
    }
    for key in ["shooting_script_identity", "generation_asset_fingerprint"] {
        if !identity.get(key).is_some_and(Value::is_object) {
            return Err(format!(
                "视频提示词 #{prompt_id} 的 video_execution_identity.{key} 必须是 JSON 对象"
            ));
        }
    }
    Ok(Value::Object(identity.clone()))
}

/// 关联视频预览 config_id：校验同章节后读取 metadata。
pub fn linked_video_preview_config_id(
    prompt_id: i64,
    prompt_project_id: i64,
    prompt_chapter_id: i64,
    video_id: i64,
    video_project_id: i64,
    video_chapter_id: Option<i64>,
    video_metadata_json: Option<&Value>,
) -> Result<Option<i64>, String> {
    if video_project_id != prompt_project_id || video_chapter_id != Some(prompt_chapter_id) {
        return Err(format!(
            "视频提示词 #{prompt_id} 关联的视频 #{video_id} 不属于同一章节"
        ));
    }
    let Some(config_id) = video_metadata_json.and_then(|metadata| metadata.get("config_id")) else {
        return Ok(None);
    };
    let config_id = config_id.as_i64().ok_or_else(|| {
        format!(
            "视频提示词 #{prompt_id} 关联的视频 #{video_id} metadata_json.config_id 必须是正整数"
        )
    })?;
    if config_id <= 0 {
        return Err(format!(
            "视频提示词 #{prompt_id} 关联的视频 #{video_id} metadata_json.config_id 必须是正整数"
        ));
    }
    Ok(Some(config_id))
}

/// 合并首/尾帧与提示词参考图 ID（去重、忽略 ≤0）。
pub fn reference_ids_with_prompt_frame_images(
    start_frame_id: Option<i64>,
    end_frame_id: Option<i64>,
    prompt_reference_ids: &[i64],
) -> Vec<i64> {
    let mut ids = Vec::new();
    for id in [start_frame_id, end_frame_id]
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

/// 合并分镜图与提示词参考图 ID（去重、忽略 ≤0）。
pub fn reference_ids_with_storyboard_image(
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

/// 解析 data URL 的 MIME；非 data URL 返回 `Ok(None)`。
pub fn data_url_mime(value: &str) -> Result<Option<String>, String> {
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

/// New API 首帧仅允许 PNG/JPEG data URL（非 data URL 放行）。
pub fn ensure_newapi_first_frame_image_supported(image_url: Option<&str>) -> Result<(), String> {
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

/// provider 图片绑定违规文案列表（空表示通过）。
pub fn provider_image_binding_violation_messages(
    backend: &str,
    has_end_frame_input: bool,
    has_storyboard_input: bool,
    grok_mode: CliproxyGrokReferenceMode,
    prompt_reference_image_count: usize,
    newapi_backend: &str,
    cliproxy_backend: &str,
) -> Vec<String> {
    let backend = backend.trim();
    let mut messages = Vec::new();
    if backend == newapi_backend && has_storyboard_input {
        messages.push(
            "New API 视频接口不支持媒体合同要求的分镜 reference_images；请切换到支持参考图输入的视频 provider。"
                .to_string(),
        );
    } else if backend == newapi_backend && has_end_frame_input {
        messages.push(
            "New API 视频接口只有一个起始帧图片输入槽，但当前媒体合同还要求结束帧；继续请求会静默丢弃结束帧。请重新生成不含结束帧的提示词，或切换到支持结束帧输入的视频 provider。"
                .to_string(),
        );
    }
    if backend == cliproxy_backend && grok_mode == CliproxyGrokReferenceMode::FirstFrame {
        if has_storyboard_input {
            messages.push(
                "CLIProxy / Grok 视频接口当前使用单 image 输入，但媒体合同要求分镜 reference_images；请切换到 Grok 参考模式，或切换到支持 reference_images 的视频 provider。"
                    .to_string(),
            );
        } else if has_end_frame_input {
            messages.push(
                "CLIProxy / Grok 视频接口当前使用单 image 输入，但媒体合同还要求结束帧；继续请求会静默丢弃结束帧。请重新生成不含结束帧的提示词、切换到 Grok 参考模式，或切换到支持结束帧输入的视频 provider。"
                    .to_string(),
            );
        } else if prompt_reference_image_count > 0 {
            messages.push(format!(
                "CLIProxy / Grok 首帧模式只有一个 image 输入槽，不能把 {prompt_reference_image_count} 张普通参考图兼作首帧。请移除参考图，或切换到 Grok 参考模式。"
            ));
        }
    }
    if backend == newapi_backend && prompt_reference_image_count > 0 {
        messages.push(format!(
            "New API 视频接口不支持提示词参考图输入；当前视频提示词绑定了 {prompt_reference_image_count} 张参考图。已阻止生成以避免参考图被丢弃，请移除参考图，或切换到支持参考图输入的视频 provider。"
        ));
    }
    messages
}

/// CLIProxy prompt 超长时的错误文案；未超限返回 `None`。
pub fn cliproxy_prompt_length_violation_message(
    backend: &str,
    prompt: &str,
    cliproxy_backend: &str,
) -> Option<String> {
    if backend.trim() != cliproxy_backend {
        return None;
    }
    let prompt_chars = prompt.chars().count();
    let prompt_bytes = prompt.len();
    if prompt_chars <= MAX_CLIPROXY_VIDEO_PROMPT_CHARS {
        return None;
    }
    Some(format!(
        "CLIProxy/Grok 视频 prompt 超过 {MAX_CLIPROXY_VIDEO_PROMPT_CHARS} 字符限制：实际 {prompt_chars} 字符 / {prompt_bytes} UTF-8 字节；请缩短 Visual/Motion Prompt 后再生成"
    ))
}

/// CLIProxy 媒体合同违规条目：错误文案 + Preview note 后缀。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliproxyMediaContractViolation {
    pub message: String,
    /// Preview 追加到 note 时的后缀（Generate 只使用 `message`）。
    pub preview_suffix: &'static str,
}

const CLIPROXY_PREVIEW_STRUCTURE_SUFFIX: &str = " 当前仅展示请求结构。";
const CLIPROXY_PREVIEW_REJECT_SUFFIX: &str = "；真实生成会拒绝该输入。";

/// CLIProxy 参考图媒体合同违规列表（空表示通过）。
///
/// 首帧模式下直接放行；非 cliproxy backend 直接放行。
pub fn cliproxy_media_contract_violations(
    backend: &str,
    grok_mode: CliproxyGrokReferenceMode,
    bound_reference_image_count: usize,
    has_first_frame_image: bool,
    duration_secs: i32,
    cliproxy_backend: &str,
) -> Vec<CliproxyMediaContractViolation> {
    if backend.trim() != cliproxy_backend {
        return Vec::new();
    }
    if grok_mode == CliproxyGrokReferenceMode::FirstFrame {
        return Vec::new();
    }
    let mut violations = Vec::new();
    if has_first_frame_image && bound_reference_image_count > 0 {
        violations.push(CliproxyMediaContractViolation {
            message: format!(
                "CLIProxy / Grok 视频使用首帧 image 时不能同时发送 reference_images；当前请求还绑定了 {bound_reference_image_count} 张参考图。请移除参考图，或切换到支持首帧加参考图的视频 provider。"
            ),
            preview_suffix: CLIPROXY_PREVIEW_STRUCTURE_SUFFIX,
        });
    }
    if bound_reference_image_count > MAX_CLIPROXY_VIDEO_REFERENCE_IMAGES {
        violations.push(CliproxyMediaContractViolation {
            message: format!(
                "CLIProxy / Grok 视频最多支持 {MAX_CLIPROXY_VIDEO_REFERENCE_IMAGES} 张参考图，当前 {bound_reference_image_count} 张"
            ),
            preview_suffix: CLIPROXY_PREVIEW_REJECT_SUFFIX,
        });
    }
    if bound_reference_image_count > 0 && duration_secs > 10 {
        violations.push(CliproxyMediaContractViolation {
            message: "CLIProxy / Grok 视频使用参考图时最大时长为 10 秒，请降低时长或移除参考图"
                .to_string(),
            preview_suffix: CLIPROXY_PREVIEW_REJECT_SUFFIX,
        });
    }
    violations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_ids_dedupe_and_order() {
        assert_eq!(
            reference_ids_with_prompt_frame_images(Some(1), Some(2), &[2, 3, 0]),
            vec![1, 2, 3]
        );
        assert_eq!(
            reference_ids_with_storyboard_image(Some(9), &[9, 8]),
            vec![9, 8]
        );
    }

    #[test]
    fn grok_reference_mode_parses() {
        use serde_json::json;
        assert_eq!(
            parse_cliproxy_grok_reference_mode(None).unwrap(),
            CliproxyGrokReferenceMode::Reference
        );
        assert_eq!(
            parse_cliproxy_grok_reference_mode(Some(
                &json!({"grok_reference_mode": "first_frame"})
            ))
            .unwrap(),
            CliproxyGrokReferenceMode::FirstFrame
        );
        assert!(
            parse_cliproxy_grok_reference_mode(Some(&json!({"grok_reference_mode": "bad"})))
                .is_err()
        );
    }

    #[test]
    fn newapi_first_frame_and_binding_messages() {
        assert!(
            ensure_newapi_first_frame_image_supported(Some("data:image/png;base64,abc")).is_ok()
        );
        assert!(
            ensure_newapi_first_frame_image_supported(Some("data:image/webp;base64,abc")).is_err()
        );
        let msgs = provider_image_binding_violation_messages(
            "newapi-video",
            true,
            false,
            CliproxyGrokReferenceMode::Reference,
            1,
            "newapi-video",
            "cliproxy-openai-video",
        );
        assert!(msgs.iter().any(|m| m.contains("结束帧")));
        assert!(msgs.iter().any(|m| m.contains("提示词参考图")));
        assert!(
            cliproxy_prompt_length_violation_message(
                "cliproxy-openai-video",
                &"x".repeat(MAX_CLIPROXY_VIDEO_PROMPT_CHARS + 1),
                "cliproxy-openai-video",
            )
            .is_some()
        );
        let media = cliproxy_media_contract_violations(
            "cliproxy-openai-video",
            CliproxyGrokReferenceMode::Reference,
            2,
            false,
            12,
            "cliproxy-openai-video",
        );
        assert!(media.iter().any(|v| v.message.contains("最大时长为 10 秒")));
        assert!(
            cliproxy_media_contract_violations(
                "cliproxy-openai-video",
                CliproxyGrokReferenceMode::FirstFrame,
                2,
                true,
                12,
                "cliproxy-openai-video",
            )
            .is_empty()
        );
    }

    #[test]
    fn source_prompt_type_and_identity() {
        use serde_json::json;
        let plan = FRAME_SEGMENT_PLAN_PROMPT_TYPE;
        assert_eq!(
            required_prompt_source_prompt_type(1, Some(&json!({"source_prompt_type": plan})))
                .unwrap(),
            plan
        );
        assert!(required_prompt_source_prompt_type(1, Some(&json!({}))).is_err());
        assert!(required_video_execution_identity(1, None).is_err());

        let identity = json!({
            "contract_epoch": a4n_core::video_execution::VIDEO_EXECUTION_CONTRACT_EPOCH,
            "shooting_script_identity": {
                "record_id": 1,
                "revision": 2,
                "source_revision_key": "source:2",
                "source_content_sha256": "a".repeat(64),
                "script_sha256": "b".repeat(64)
            },
            "projection_sha256": "c".repeat(64),
            "segment_plan_sha256": "d".repeat(64),
            "unit_plan_sha256": "e".repeat(64),
            "frame_set_sha256": "f".repeat(64),
            "source_content_sha256": "a".repeat(64),
            "style_sha256": "1".repeat(64),
            "duration_secs": 4,
            "aspect_ratio": "16:9",
            "prompt_output_language": "zh-CN",
            "generation_asset_fingerprint": {}
        });
        let resolved = required_video_execution_identity(
            7,
            Some(&json!({"video_execution_identity": identity.clone()})),
        )
        .unwrap();
        assert_eq!(resolved, identity);

        assert_eq!(
            linked_video_preview_config_id(1, 1, 2, 9, 1, Some(2), Some(&json!({"config_id": 9})))
                .unwrap(),
            Some(9)
        );
        assert!(
            linked_video_preview_config_id(1, 1, 2, 9, 1, Some(2), Some(&json!({"config_id": 0})))
                .is_err()
        );
    }

    #[test]
    fn guidance_mode_selects_provider_media_contract() {
        assert!(
            provider_generation_workflow_for_values(1, "boundary_frames", Some(&json!({})))
                .is_err()
        );
        assert_eq!(
            provider_generation_workflow_for_values(
                1,
                "boundary_frames",
                Some(&json!({"guidance_mode": "keyframes"})),
            )
            .unwrap(),
            ProviderGenerationWorkflow::Keyframes,
        );
        assert_eq!(
            provider_generation_workflow_for_values(
                1,
                "boundary_frames",
                Some(&json!({"guidance_mode": "none"})),
            )
            .unwrap(),
            ProviderGenerationWorkflow::TextOnly,
        );
        assert!(
            provider_generation_workflow_for_values(
                1,
                "boundary_frames",
                Some(&json!({"guidance_mode": "continuity"})),
            )
            .is_err()
        );
    }
}
