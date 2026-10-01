fn resolve_cached_segment_number(
    index: &SharedSegmentValidationIndex,
    source_shot_id: &str,
    segment_index_in_shot: i32,
) -> Result<i32, String> {
    let source_shot_id = source_shot_id.trim();
    if source_shot_id.is_empty() || segment_index_in_shot <= 0 {
        return Err("稳定片段目标必须提供镜头 ID 和正整数镜头内序号".to_string());
    }
    index
        .segment_numbers_by_shot
        .get(source_shot_id)
        .and_then(|by_index| by_index.get(&segment_index_in_shot))
        .copied()
        .ok_or_else(|| {
            format!("当前片段规划不存在镜头 {source_shot_id} 的第 {segment_index_in_shot} 个片段")
        })
}

fn timeline_items_by_id(
    items: Vec<chapter_video_timeline_item::Model>,
) -> HashMap<i64, chapter_video_timeline_item::Model> {
    let item_count = items.len();
    items
        .into_iter()
        .fold(HashMap::with_capacity(item_count), |mut by_id, item| {
            by_id.entry(item.id).or_insert(item);
            by_id
        })
}

fn inserted_video_execution_identity(
    prompt: &ChapterVideoPromptRecord,
    recipe: &VideoSegmentExecutionRecipeV1,
) -> Result<Value, AppError> {
    let metadata = prompt
        .metadata_json
        .as_ref()
        .and_then(Value::as_object)
        .ok_or_else(|| invalid_argument(format!("插入片段提示词 #{} 缺少 metadata", prompt.id)))?;
    let identity = metadata
        .get("video_execution_identity")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            invalid_argument(format!(
                "插入片段提示词 #{} 缺少 video_execution_identity",
                prompt.id
            ))
        })?;
    if identity.get("contract_epoch").and_then(Value::as_u64)
        != Some(VIDEO_EXECUTION_CONTRACT_EPOCH as u64)
    {
        return Err(invalid_argument(format!(
            "插入片段提示词 #{} 的执行合同已失效，请重新生成插入片段",
            prompt.id
        )));
    }
    let source_content_sha256 = metadata
        .get("source")
        .and_then(|source| source.get("content_sha256"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| invalid_argument(format!("插入片段提示词 #{} 缺少正文指纹", prompt.id)))?;
    let prompt_matches_recipe = prompt.workflow_mode == recipe.workflow_mode.as_str()
        && prompt.style.as_deref().map(str::trim) == Some(recipe.style.trim())
        && prompt.duration_secs == recipe.duration_secs
        && prompt.aspect_ratio.as_deref() == Some(recipe.aspect_ratio.as_str())
        && metadata
            .get("prompt_output_language")
            .and_then(Value::as_str)
            == Some(recipe.prompt_output_language.as_str())
        && reference_image_ids(prompt.reference_image_ids_json.as_ref())
            .map_err(AppError::InvalidArgument)?
            == recipe.provider_reference_image_ids();
    if !prompt_matches_recipe
        || identity
            .get("source_content_sha256")
            .and_then(Value::as_str)
            .map(str::trim)
            != Some(source_content_sha256)
        || identity
            .get("segment_sha256")
            .and_then(Value::as_str)
            .map(str::trim)
            .is_none_or(str::is_empty)
    {
        return Err(invalid_argument(format!(
            "插入片段提示词 #{} 不属于当前执行方案，请重新生成插入片段",
            prompt.id
        )));
    }
    Ok(Value::Object(identity.clone()))
}

fn joined_ids(ids: &[i64]) -> String {
    ids.iter()
        .map(i64::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

fn invalid_argument(message: impl Into<String>) -> AppError {
    AppError::InvalidArgument(message.into())
}
