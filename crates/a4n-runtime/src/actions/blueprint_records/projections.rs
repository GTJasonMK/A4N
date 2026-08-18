use super::{BlueprintSummary, BlueprintVersionSummary};
use a4n_core::{
    error::AppError,
    models::project::{blueprint_version, novel_blueprint},
};

pub(super) fn blueprint_summary(
    bp: &novel_blueprint::Model,
    sources_need_review: bool,
) -> BlueprintSummary {
    BlueprintSummary {
        id: bp.id,
        project_id: bp.project_id,
        title: bp.title.clone(),
        genre: bp.genre.clone(),
        style: bp.style.clone(),
        tone: bp.tone.clone(),
        target_audience: bp.target_audience.clone(),
        one_sentence_summary: bp.one_sentence_summary.clone(),
        full_synopsis: bp.full_synopsis.clone(),
        world_setting: bp.world_setting.clone(),
        current_version: bp.current_version.clone(),
        total_chapters: bp.total_chapters,
        chapters_per_part: bp.chapters_per_part,
        source_review_status: if sources_need_review {
            "needs_review"
        } else {
            "not_required"
        }
        .to_string(),
    }
}

pub(super) fn blueprint_version_summary(
    v: &blueprint_version::Model,
) -> Result<BlueprintVersionSummary, AppError> {
    Ok(BlueprintVersionSummary {
        id: v.id,
        version: v.version.clone(),
        change_description: persisted_version_text(
            v.change_description.as_deref(),
            v.id,
            "change_description",
        )?,
        created_at: v.created_at.to_string(),
        created_by: persisted_version_text(v.created_by.as_deref(), v.id, "created_by")?,
    })
}

fn persisted_version_text(
    value: Option<&str>,
    version_id: i64,
    field: &str,
) -> Result<String, AppError> {
    let value = value.ok_or_else(|| AppError::InvalidState {
        current: format!("blueprint_version {version_id} 缺少 {field}"),
        operation: "读取蓝图版本".to_string(),
    })?;
    if value.is_empty() || value.trim() != value {
        return Err(AppError::InvalidState {
            current: format!("blueprint_version {version_id} 的 {field} 非规范"),
            operation: "读取蓝图版本".to_string(),
        });
    }
    Ok(value.to_string())
}
