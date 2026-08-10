//! 导入分析分部大纲候选的显式确认。

use a4n_core::{error::AppError, models::chapter::part_outline};
use a4n_db::{
    import_reverse::{ReversePartOutlineApplyRow, apply_reverse_part_outline_records},
    project::{import_analysis_chapter_candidates_for_update, project_for_update},
};
use chrono::Utc;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, TransactionTrait};
use serde_json::{Value, json};

use crate::{OrchestratorContext, import_reverse::import_reverse_db_error};

use super::{
    candidate::{
        locked_import_progress, persist_locked_import_progress, required_candidate_apply_result,
    },
    part_outline::import_part_outline_candidates,
    progress::STAGE_PARTS,
    workflow::{
        ensure_import_analysis_candidate_current_for_chapters,
        ensure_import_analysis_candidate_stage_completed,
    },
};

impl OrchestratorContext {
    pub async fn apply_import_analysis_part_outline_candidate_validated(
        &self,
        project_id: i64,
    ) -> Result<Value, AppError> {
        apply_import_analysis_part_outline_candidate(self, project_id).await
    }
}

pub async fn apply_import_analysis_part_outline_candidate(
    context: &OrchestratorContext,
    project_id: i64,
) -> Result<Value, AppError> {
    let tx = context.database.connection().begin().await?;
    let project = project_for_update(&tx, project_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
    if let Some(operation) = a4n_db::project::blocked_project_write_operation(
        &project.lifecycle_gate,
        "确认导入分析分部大纲候选",
    ) {
        return Err(AppError::InvalidState {
            current: project.lifecycle_gate.clone(),
            operation,
        });
    }
    let chapters = import_analysis_chapter_candidates_for_update(&tx, project_id)
        .await
        .map_err(|error| AppError::InvalidState {
            current: error.to_string(),
            operation: "锁定导入分析章节".to_string(),
        })?
        .into_iter()
        .filter(|chapter| !chapter.content.trim().is_empty())
        .collect::<Vec<_>>();
    let mut progress = locked_import_progress(&project, project_id)?;
    let proposal = part_candidate_from_progress(&progress, project_id)?;

    if proposal.get("state").and_then(Value::as_str) == Some("confirmed") {
        let apply_result =
            required_candidate_apply_result(&proposal, "读取已确认导入分部大纲候选的应用结果")?;
        tx.rollback().await?;
        context
            .sync_project_status_from_artifacts(project_id)
            .await?;
        return Ok(json!({
            "status": "already_confirmed",
            "progress": progress,
            "applied": apply_result,
        }));
    }

    ensure_import_analysis_candidate_current_for_chapters(&progress, &chapters).map_err(
        |error| AppError::InvalidState {
            current: "stale_import_part_outline_candidate".to_string(),
            operation: error,
        },
    )?;
    let rebuilt =
        import_part_outline_candidates(&chapters).map_err(|error| AppError::InvalidState {
            current: "invalid_import_part_outline_candidate_source".to_string(),
            operation: error,
        })?;
    let stored_parts = proposal
        .get("parts")
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::InvalidArgument("分部大纲候选缺少 parts 数组".to_string()))?;
    if stored_parts != &rebuilt {
        return Err(AppError::InvalidState {
            current: "changed_import_part_outline_candidate".to_string(),
            operation: "分部候选与当前章节分析不一致，请重新分析".to_string(),
        });
    }
    let rows = part_candidate_rows(&rebuilt)?;
    let now = Utc::now().naive_utc();
    let applied = apply_reverse_part_outline_records(&tx, project_id, &rows)
        .await
        .map_err(import_reverse_db_error)?;
    let part_numbers = rows.iter().map(|row| row.part_number).collect::<Vec<_>>();
    let applied_parts = part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(project_id))
        .filter(part_outline::Column::PartNumber.is_in(part_numbers))
        .order_by_asc(part_outline::Column::PartNumber)
        .order_by_asc(part_outline::Column::Id)
        .all(&tx)
        .await?;

    let apply_result = json!({
        "part_outlines_applied": true,
        "part_count": applied.part_count,
        "inserted_count": applied.inserted,
        "updated_count": applied.updated,
        "reused_count": applied.reused,
        "part_outline_ids": applied_parts.iter().map(|part| part.id).collect::<Vec<_>>(),
    });
    mark_part_candidate_confirmed(&mut progress, proposal, apply_result.clone(), now)?;
    persist_locked_import_progress(&tx, &project, &progress, now).await?;
    tx.commit().await?;
    context
        .sync_project_status_from_artifacts(project_id)
        .await?;

    Ok(json!({
        "status": "confirmed",
        "progress": progress,
        "applied": apply_result,
    }))
}

fn part_candidate_from_progress(progress: &Value, project_id: i64) -> Result<Value, AppError> {
    let status = progress
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_import_analysis_status".to_string(),
            operation: "确认分部大纲候选".to_string(),
        })?;
    if status != "completed" {
        return Err(AppError::InvalidState {
            current: status.to_string(),
            operation: "导入分析尚未完成，不能确认分部大纲候选".to_string(),
        });
    }
    ensure_import_analysis_candidate_stage_completed(progress, STAGE_PARTS).map_err(|error| {
        AppError::InvalidState {
            current: "invalid_import_analysis_stage_for_part_candidate".to_string(),
            operation: error,
        }
    })?;
    let stage = progress
        .get("stages")
        .and_then(Value::as_object)
        .and_then(|stages| stages.get(STAGE_PARTS))
        .and_then(Value::as_object)
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_part_outline_candidate_stage".to_string(),
            operation: "确认导入分析分部大纲候选".to_string(),
        })?;
    let proposal = stage
        .get("proposal")
        .cloned()
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_part_outline_candidate".to_string(),
            operation: "确认导入分析分部大纲候选".to_string(),
        })?;
    let object = proposal
        .as_object()
        .ok_or_else(|| AppError::InvalidArgument("分部大纲候选必须是 JSON 对象".to_string()))?;
    let state = object
        .get("state")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::InvalidArgument("分部大纲候选缺少 state".to_string()))?;
    if !matches!(state, "proposal" | "confirmed") {
        return Err(AppError::InvalidState {
            current: state.to_string(),
            operation: "确认导入分析分部大纲候选".to_string(),
        });
    }
    if object.get("origin").and_then(Value::as_str) != Some("import_analysis") {
        return Err(AppError::InvalidArgument(
            "只能确认来源为 import_analysis 的分部大纲候选".to_string(),
        ));
    }
    if object.get("project_id").and_then(Value::as_i64) != Some(project_id) {
        return Err(AppError::InvalidArgument(
            "分部大纲候选不属于当前项目".to_string(),
        ));
    }
    let progress_fingerprint =
        progress
            .get("input_fingerprint")
            .ok_or_else(|| AppError::InvalidState {
                current: "missing_import_analysis_input_fingerprint".to_string(),
                operation: "确认导入分析分部大纲候选".to_string(),
            })?;
    if object.get("input_fingerprint") != Some(progress_fingerprint) {
        return Err(AppError::InvalidState {
            current: "part_candidate_input_fingerprint_mismatch".to_string(),
            operation: "确认导入分析分部大纲候选".to_string(),
        });
    }
    let part_count = object
        .get("parts")
        .and_then(Value::as_array)
        .map(Vec::len)
        .ok_or_else(|| AppError::InvalidArgument("分部大纲候选缺少 parts 数组".to_string()))?;
    let expected = stage
        .get("total")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| AppError::InvalidState {
            current: "invalid_part_outline_candidate_total".to_string(),
            operation: "确认导入分析分部大纲候选".to_string(),
        })?;
    if part_count != expected || part_count == 0 {
        return Err(AppError::InvalidState {
            current: "part_outline_candidate_count_mismatch".to_string(),
            operation: format!("候选数量应为 {expected}，实际为 {part_count}"),
        });
    }
    Ok(proposal)
}

fn part_candidate_rows(parts: &[Value]) -> Result<Vec<ReversePartOutlineApplyRow>, AppError> {
    parts
        .iter()
        .map(|part| {
            let object = part.as_object().ok_or_else(|| {
                AppError::InvalidArgument("分部大纲候选条目必须是对象".to_string())
            })?;
            Ok(ReversePartOutlineApplyRow {
                part_number: required_i32(object, "part_number")?,
                start_chapter: required_i32(object, "start_chapter")?,
                end_chapter: required_i32(object, "end_chapter")?,
                title: required_text(object, "title")?,
                summary: required_text(object, "summary")?,
                theme: required_text(object, "theme")?,
                key_events: object.get("key_events").cloned().ok_or_else(|| {
                    AppError::InvalidArgument("分部候选缺少 key_events".to_string())
                })?,
                character_focus: object.get("character_focus").cloned().ok_or_else(|| {
                    AppError::InvalidArgument("分部候选缺少 character_focus".to_string())
                })?,
            })
        })
        .collect()
}

fn required_i32(object: &serde_json::Map<String, Value>, key: &str) -> Result<i32, AppError> {
    object
        .get(key)
        .and_then(Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
        .ok_or_else(|| AppError::InvalidArgument(format!("分部候选缺少有效 {key}")))
}

fn required_text(object: &serde_json::Map<String, Value>, key: &str) -> Result<String, AppError> {
    object
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| AppError::InvalidArgument(format!("分部候选缺少有效 {key}")))
}

fn mark_part_candidate_confirmed(
    progress: &mut Value,
    mut proposal: Value,
    apply_result: Value,
    confirmed_at: chrono::NaiveDateTime,
) -> Result<(), AppError> {
    let stage = progress
        .get_mut("stages")
        .and_then(Value::as_object_mut)
        .and_then(|stages| stages.get_mut(STAGE_PARTS))
        .and_then(Value::as_object_mut)
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_part_outline_candidate_stage".to_string(),
            operation: "记录分部大纲候选确认状态".to_string(),
        })?;
    let object = proposal
        .as_object_mut()
        .ok_or_else(|| AppError::InvalidArgument("分部大纲候选必须是对象".to_string()))?;
    object.insert("state".to_string(), json!("confirmed"));
    object.insert("confirmed_at".to_string(), json!(confirmed_at.to_string()));
    object.insert("apply_result".to_string(), apply_result);
    stage.insert("proposal".to_string(), proposal);
    progress["message"] = json!("分部大纲候选已确认并应用");
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::part_candidate_from_progress;

    #[test]
    fn candidate_requires_matching_project_fingerprint_and_count() {
        let progress = json!({
            "status": "completed",
            "current_stage": "generating_part_outlines",
            "input_fingerprint": {"schema_version": 1},
            "stages": {
                "generating_analysis_data": {"completed": 1, "total": 1, "status": "completed"},
                "generating_outlines": {"completed": 1, "total": 1, "status": "completed"},
                "extracting_blueprint": {"completed": 1, "total": 1, "status": "completed"},
                "generating_part_outlines": {
                    "completed": 1,
                    "total": 1,
                    "status": "completed",
                    "proposal": {
                        "state": "proposal",
                        "origin": "import_analysis",
                        "project_id": 7,
                        "input_fingerprint": {"schema_version": 1},
                        "parts": [{}]
                    }
                }
            }
        });
        assert!(part_candidate_from_progress(&progress, 7).is_ok());
        assert!(part_candidate_from_progress(&progress, 8).is_err());
    }

    #[test]
    fn candidate_cannot_be_confirmed_before_analysis_completes() {
        let progress = json!({
            "status": "analyzing",
            "current_stage": "generating_part_outlines",
            "input_fingerprint": {"schema_version": 1},
            "stages": {
                "generating_analysis_data": {"completed": 1, "total": 1, "status": "completed"},
                "generating_outlines": {"completed": 1, "total": 1, "status": "completed"},
                "extracting_blueprint": {"completed": 1, "total": 1, "status": "completed"},
                "generating_part_outlines": {
                    "completed": 1,
                    "total": 1,
                    "status": "completed",
                    "proposal": {
                        "state": "proposal",
                        "origin": "import_analysis",
                        "project_id": 7,
                        "input_fingerprint": {"schema_version": 1},
                        "parts": [{}]
                    }
                }
            }
        });
        let error = part_candidate_from_progress(&progress, 7)
            .expect_err("an in-progress analysis must not be confirmable");
        assert!(error.to_string().contains("尚未完成"));
    }

    #[test]
    fn part_candidate_requires_explicit_progress_status() {
        let progress = json!({
            "current_stage": "generating_part_outlines",
            "input_fingerprint": {"schema_version": 1},
            "stages": {
                "generating_analysis_data": {"completed": 1, "total": 1, "status": "completed"},
                "generating_outlines": {"completed": 1, "total": 1, "status": "completed"},
                "generating_part_outlines": {
                    "completed": 1,
                    "total": 1,
                    "status": "completed",
                    "proposal": {
                        "state": "proposal",
                        "origin": "import_analysis",
                        "project_id": 7,
                        "input_fingerprint": {"schema_version": 1},
                        "parts": [{}]
                    }
                }
            }
        });

        let error = part_candidate_from_progress(&progress, 7)
            .expect_err("missing progress status must not become a diagnostic placeholder");
        assert!(error.to_string().contains("missing_import_analysis_status"));
    }
}
