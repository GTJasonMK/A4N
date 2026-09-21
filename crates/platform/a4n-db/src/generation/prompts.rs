use super::*;

pub(crate) async fn prompt_artifact_by_id<C>(
    db: &C,
    prompt_artifact_id: i64,
) -> Result<prompt_artifact::Model, GenerationDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(prompt_artifact_id, "prompt_artifact_id")?;
    prompt_artifact::Entity::find_by_id(prompt_artifact_id)
        .one(db)
        .await?
        .ok_or_else(|| {
            GenerationDbError::NotFound(format!("提示词产物不存在: {prompt_artifact_id}"))
        })
}

pub fn is_valid_generation_prompt_kind(value: &str) -> bool {
    GENERATION_PROMPT_KINDS.contains(&value)
}

pub(in crate::generation) fn validate_prompt_artifact_text(
    prompt_kind: &str,
    prompt_text: &str,
) -> Result<(), GenerationDbError> {
    validate_final_prompt_text(prompt_text)?;
    if matches!(
        normalize_prompt_kind(prompt_kind)?,
        GENERATION_PROMPT_KIND_VIDEO_PROMPT | "chat_video_prompt" | "provider_video_prompt"
    ) {
        for section in VIDEO_PROMPT_REQUIRED_SECTIONS {
            if !prompt_text.contains(section) {
                return Err(GenerationDbError::InvalidArgument(format!(
                    "视频提示词缺少必需章节 {section}"
                )));
            }
        }
    }
    Ok(())
}

pub fn render_prompt_text(
    spec: &generation_spec::Model,
    prompt_kind: &str,
) -> Result<String, GenerationDbError> {
    match prompt_kind {
        GENERATION_PROMPT_KIND_IMAGE_PROMPT => {
            Ok(required_json_string(&spec.params_json, "image_prompt")?.to_string())
        }
        "negative_prompt" => {
            Ok(required_json_string(&spec.params_json, "negative_prompt")?.to_string())
        }
        GENERATION_PROMPT_KIND_VIDEO_PROMPT | "chat_video_prompt" | "provider_video_prompt" => {
            let input_images = required_json_string(&spec.params_json, "input_image_introduction")?;
            let segment_context = required_json_string(&spec.params_json, "segment_context")?;
            let time_distribution = required_time_distribution(&spec.params_json)?;
            let visual_style = required_json_string(&spec.params_json, "visual_style")?;
            let continuity_and_end_state =
                required_json_string(&spec.params_json, "continuity_and_end_state")?;
            let restrictions_and_avoidance =
                required_json_string(&spec.params_json, "restrictions_and_avoidance")?;
            Ok(format!(
                "[主体定义与素材绑定]\n{}\n\n[片段内容以及前因后果]\n{}\n\n[镜头分镜]\n{}\n\n[画面风格描述]\n{}\n\n[连续性与结尾状态]\n{}\n\n[限制与避免项]\n{}",
                input_images,
                segment_context,
                time_distribution,
                visual_style,
                continuity_and_end_state,
                restrictions_and_avoidance
            ))
        }
        _ => Err(GenerationDbError::InvalidArgument(format!(
            "提示词类型无效: {prompt_kind}"
        ))),
    }
}

pub(crate) async fn insert_prompt_artifact<C>(
    db: &C,
    input: PromptArtifactInsertInput,
) -> Result<prompt_artifact::Model, GenerationDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(input.generation_spec_id, "generation_spec_id")?;
    let prompt_kind = normalize_prompt_kind(&input.prompt_kind)?.to_string();
    let prompt_text = required_owned_text(input.prompt_text, "prompt_text")?;
    validate_prompt_artifact_text(&prompt_kind, &prompt_text)?;
    let renderer_version = required_owned_text(input.renderer_version, "renderer_version")?;
    let now = Utc::now().naive_utc();
    prompt_artifact::ActiveModel {
        generation_spec_id: Set(input.generation_spec_id),
        prompt_kind: Set(prompt_kind),
        prompt_text: Set(prompt_text),
        renderer_version: Set(renderer_version),
        edited_by_user: Set(input.edited_by_user),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(GenerationDbError::Database)
}

fn required_json_string<'a>(params: &'a Value, field: &str) -> Result<&'a str, GenerationDbError> {
    let text = params.get(field).and_then(Value::as_str).ok_or_else(|| {
        GenerationDbError::InvalidArgument(format!("params_json.{field} 必须是非空字符串"))
    })?;
    required_text(text, &format!("params_json.{field}"))
}

fn required_time_distribution(params: &Value) -> Result<String, GenerationDbError> {
    let items = params
        .get("time_distribution")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            GenerationDbError::InvalidArgument(
                "params_json.time_distribution 必须是非空字符串数组".to_string(),
            )
        })?;
    if items.is_empty() {
        return Err(GenerationDbError::InvalidArgument(
            "params_json.time_distribution 必须是非空字符串数组".to_string(),
        ));
    }
    let mut lines = Vec::with_capacity(items.len());
    for item in items {
        let line = item.as_str().ok_or_else(|| {
            GenerationDbError::InvalidArgument(
                "params_json.time_distribution 必须是非空字符串数组".to_string(),
            )
        })?;
        lines.push(required_text(line, "params_json.time_distribution[]")?.to_string());
    }
    Ok(lines.join("\n"))
}

pub(crate) fn normalize_video_prompt_kind(value: &str) -> Result<&str, GenerationDbError> {
    let value = normalize_prompt_kind(value)?;
    if matches!(
        value,
        GENERATION_PROMPT_KIND_VIDEO_PROMPT | "chat_video_prompt" | "provider_video_prompt"
    ) {
        Ok(value)
    } else {
        Err(GenerationDbError::InvalidArgument(format!(
            "prompt_kind 无效: {value}"
        )))
    }
}

fn normalize_prompt_kind(value: &str) -> Result<&str, GenerationDbError> {
    normalize_one_of(value, "prompt_kind", GENERATION_PROMPT_KINDS)
}

fn validate_final_prompt_text(prompt_text: &str) -> Result<(), GenerationDbError> {
    required_text(prompt_text, "prompt_text")?;
    for token in INTERNAL_PROMPT_TOKENS {
        if prompt_text.contains(token) {
            return Err(GenerationDbError::InvalidArgument(format!(
                "最终提示词不能包含内部字段: {token}"
            )));
        }
    }
    Ok(())
}
