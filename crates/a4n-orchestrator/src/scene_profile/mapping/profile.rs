use a4n_core::error::AppError;
use a4n_core::profile_text::{
    normalize_aliases, normalize_required_text_set, required_text, validate_importance,
};
use a4n_db::scene_profile::SceneProfileRecordInput;
use serde_json::json;

use super::super::contracts::SceneProfileDraft;
use super::super::rules::normalize_scene_type_value;

/// 将场景档案草稿规范化为 DB 写入 DTO（runtime 可直接交给 orchestrator）。
pub fn normalize_scene_profile_record_input(
    draft: SceneProfileDraft,
) -> Result<SceneProfileRecordInput, AppError> {
    let normalized = normalize_scene_name(&draft.name)?;
    let draft = normalize_profile_draft(draft)?;
    Ok(scene_profile_record_input(draft, normalized))
}

pub(super) fn normalize_scene_name(name: &str) -> Result<String, AppError> {
    let normalized = name.trim().to_lowercase();
    if normalized.is_empty() {
        return Err(AppError::InvalidArgument("场景名称不能为空".to_string()));
    }
    Ok(normalized)
}

pub(super) fn normalize_profile_draft(
    mut draft: SceneProfileDraft,
) -> Result<SceneProfileDraft, AppError> {
    draft.scene_ref = required_text(&draft.scene_ref, "场景引用")?;
    draft.name = required_text(&draft.name, "场景名称")?;
    draft.aliases = normalize_aliases(draft.aliases);
    draft.scene_type = normalize_scene_type_value(&draft.scene_type)?;
    draft.importance = validate_importance(&draft.importance, "场景重要性")?;
    draft.base_description = required_text(&draft.base_description, "场景基础描述")?;
    draft.visual_description = required_text(&draft.visual_description, "场景视觉描述")?;
    draft.spatial_layout = required_text(&draft.spatial_layout, "场景空间布局")?;
    draft.atmosphere = required_text(&draft.atmosphere, "场景氛围")?;
    draft.image_prompt_seed = required_text(&draft.image_prompt_seed, "场景图片提示词种子")?;
    if !draft.entry_points.is_array() {
        return Err(AppError::InvalidArgument(
            "场景出入口必须是 JSON 数组".to_string(),
        ));
    }
    if !draft.rules.is_array() {
        return Err(AppError::InvalidArgument(
            "场景规则必须是 JSON 数组".to_string(),
        ));
    }
    if draft.first_seen_chapter <= 0 || draft.last_seen_chapter <= 0 {
        return Err(AppError::InvalidArgument(
            "场景首次/最近出现章节必须大于 0".to_string(),
        ));
    }
    if draft.last_seen_chapter < draft.first_seen_chapter {
        return Err(AppError::InvalidArgument(
            "场景最近出现章节不能早于首次出现章节".to_string(),
        ));
    }
    draft.evidence = normalize_required_text_set(draft.evidence, "场景证据")?;
    Ok(draft)
}

pub(super) fn scene_profile_record_input(
    draft: SceneProfileDraft,
    normalized_name: String,
) -> SceneProfileRecordInput {
    SceneProfileRecordInput {
        name: draft.name,
        normalized_name,
        aliases_json: json!(draft.aliases),
        scene_type: draft.scene_type,
        importance: draft.importance,
        parent_scene_id: draft.parent_scene_id,
        base_description: draft.base_description,
        visual_description: draft.visual_description,
        spatial_layout: draft.spatial_layout,
        atmosphere: draft.atmosphere,
        entry_points_json: draft.entry_points,
        rules_json: draft.rules,
        first_seen_chapter: draft.first_seen_chapter,
        last_seen_chapter: draft.last_seen_chapter,
        evidence_json: json!(draft.evidence),
        image_prompt_seed: draft.image_prompt_seed,
    }
}
