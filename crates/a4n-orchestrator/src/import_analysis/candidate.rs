//! 导入分析蓝图候选的显式审阅与应用。
//!
//! 候选是分析阶段的事实快照，不是另一条模型生成路径。确认时只读取项目进度中
//! 已保存的 proposal，并将其中明确覆盖的蓝图主体字段写入正式蓝图；角色和关系
//! 需要独立的识别/同步结果，不能因为候选没有携带它们而被清空。

use a4n_core::{
    blueprint::{increment_blueprint_version, project_status_after_blueprint_generation},
    error::AppError,
    event::bridge::InternalEvent,
    models::project::{blueprint_version, novel_blueprint},
    state::validator::parse_project_status,
};
use a4n_db::{
    blueprint::{
        InsertBlueprintVersionInput, NovelBlueprintUpsertInput, SyncProjectAfterBlueprintInput,
        blueprint_changed_fields, blueprint_version_snapshot, insert_blueprint_version_record,
        sync_project_after_blueprint_record, upsert_novel_blueprint_record,
    },
    import_reverse::resolve_reverse_blueprint_apply_record,
    project::{
        ProjectStatusHistoryInput, blueprint_characters_for_project,
        blueprint_relationships_for_project, import_analysis_chapter_candidates_for_update,
        insert_project_status_history, novel_blueprint_for_project, project_for_update,
    },
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set, TransactionTrait,
};
use serde_json::{Value, json};

use crate::{
    OrchestratorContext,
    blueprint::blueprint_db_error,
    import_reverse::{import_reverse_db_error, reverse_blueprint_apply_metadata},
    project::project_status_write_error_to_app_error,
};

use super::{
    blueprint::import_blueprint_genre_for_candidate,
    progress::STAGE_BLUEPRINT,
    workflow::{
        ensure_import_analysis_candidate_current_for_chapters,
        ensure_import_analysis_candidate_stage_completed,
    },
};

impl OrchestratorContext {
    pub async fn apply_import_analysis_blueprint_candidate_validated(
        &self,
        project_id: i64,
        created_by: Option<String>,
    ) -> Result<Value, AppError> {
        apply_import_analysis_blueprint_candidate(self, project_id, created_by).await
    }
}

/// 应用已保存的导入蓝图候选。
///
/// `created_by` 只用于版本审计，不参与候选内容选择；候选内容始终从项目进度读取，
/// 不接受调用方传入的任意 JSON。
pub async fn apply_import_analysis_blueprint_candidate(
    context: &OrchestratorContext,
    project_id: i64,
    created_by: Option<String>,
) -> Result<Value, AppError> {
    let tx = context.database.connection().begin().await?;
    let current_project = project_for_update(&tx, project_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
    if let Some(operation) = a4n_db::project::blocked_project_write_operation(
        &current_project.lifecycle_gate,
        "确认导入分析蓝图候选",
    ) {
        return Err(AppError::InvalidState {
            current: current_project.lifecycle_gate.clone(),
            operation,
        });
    }
    let chapters = import_analysis_chapter_candidates_for_update(&tx, project_id)
        .await
        .map_err(|error| AppError::InvalidState {
            current: error.to_string(),
            operation: "锁定导入分析章节".to_string(),
        })?;
    let minimum_total_chapters = chapters
        .iter()
        .map(|chapter| chapter.chapter_number)
        .max()
        .unwrap_or(0);
    let chapters = chapters
        .into_iter()
        .filter(|chapter| !chapter.content.trim().is_empty())
        .collect::<Vec<_>>();
    // 项目行锁内读取候选，避免确认时把并发分析写入的最新进度覆盖掉。
    let mut progress = locked_import_progress(&current_project, project_id)?;
    let proposal = candidate_from_progress(&progress, project_id)?;

    // 网络重试或用户重复点击不应再次生成版本。
    if proposal.get("state").and_then(Value::as_str) == Some("confirmed") {
        let apply_result =
            required_candidate_apply_result(&proposal, "读取已确认导入蓝图候选的应用结果")?;
        tx.rollback().await?;
        return Ok(json!({
            "status": "already_confirmed",
            "progress": progress,
            "applied": apply_result,
        }));
    }

    // 锁住正文行后再做指纹检查：已提交的正文修改会先被读到，正在进行的修改则
    // 排在本次确认之后，避免检查通过后正文立刻被另一事务替换。
    ensure_import_analysis_candidate_current_for_chapters(&progress, &chapters).map_err(
        |error| AppError::InvalidState {
            current: "stale_import_blueprint_candidate".to_string(),
            operation: error,
        },
    )?;

    let now = Utc::now().naive_utc();

    let existing_blueprint = novel_blueprint_for_project(&tx, project_id).await?;
    let existing_characters = blueprint_characters_for_project(&tx, project_id).await?;
    let existing_relationships = blueprint_relationships_for_project(&tx, project_id).await?;
    let normalized = normalize_candidate(&proposal, &current_project, existing_blueprint.as_ref())?;

    // 候选没有角色/关系识别结果，只校验蓝图主体字段；不能伪造实体来套用完整反推合同。
    let draft = reverse_blueprint_apply_metadata(&normalized).map_err(AppError::InvalidArgument)?;
    let apply_record = resolve_reverse_blueprint_apply_record(&draft, minimum_total_chapters, now)
        .map_err(import_reverse_db_error)?;
    let total_chapters = apply_record.total_chapters;
    let needs_part_outlines = apply_record.needs_part_outlines;
    let chapters_per_part = apply_record.chapters_per_part;
    let next_version = next_blueprint_version_for_connection(
        &tx,
        project_id,
        existing_blueprint
            .as_ref()
            .map(|blueprint| blueprint.current_version.as_str()),
    )
    .await?;
    let previous_status = parse_project_status(&current_project.status)?;
    let next_status = project_status_after_blueprint_generation(previous_status);
    let created_by = created_by.ok_or_else(|| {
        AppError::InvalidArgument("确认导入蓝图候选必须提供审计操作者".to_string())
    })?;
    let updated_blueprint = upsert_novel_blueprint_record(
        &tx,
        NovelBlueprintUpsertInput {
            project_id,
            title: draft.title.clone(),
            genre: draft.genre.clone(),
            style: Some(draft.style.clone()),
            tone: Some(draft.tone.clone()),
            target_audience: Some(draft.target_audience.clone()),
            one_sentence_summary: Some(draft.one_sentence_summary.clone()),
            full_synopsis: Some(draft.full_synopsis.clone()),
            world_setting: Some(draft.world_setting.clone()),
            needs_part_outlines,
            total_chapters,
            chapters_per_part,
            current_version: next_version.clone(),
            now,
        },
        existing_blueprint.clone(),
    )
    .await
    .map_err(blueprint_db_error)?;
    let updated_fields = blueprint_changed_fields(existing_blueprint.as_ref(), &updated_blueprint);
    let version_data = blueprint_version_snapshot(
        &updated_blueprint,
        &existing_characters,
        &existing_relationships,
    );
    let mut raw_blueprint = normalized;
    raw_blueprint
        .as_object_mut()
        .ok_or_else(|| AppError::InvalidState {
            current: "invalid_normalized_import_blueprint".to_string(),
            operation: "保存导入分析蓝图来源".to_string(),
        })?
        .insert("import_analysis_proposal".to_string(), proposal.clone());
    insert_blueprint_version_record(
        &tx,
        InsertBlueprintVersionInput {
            project_id,
            version: next_version.clone(),
            change_description: Some("确认导入分析蓝图候选".to_string()),
            blueprint_data: version_data.clone(),
            created_at: now,
            created_by: Some(created_by.clone()),
        },
    )
    .await
    .map_err(blueprint_db_error)?;
    sync_project_after_blueprint_record(
        &tx,
        SyncProjectAfterBlueprintInput {
            project_id,
            title: updated_blueprint.title.clone(),
            status: next_status.as_str().to_string(),
            target_chapters: total_chapters,
            total_chapters: updated_blueprint.total_chapters,
            chapters_per_part: updated_blueprint.chapters_per_part,
            raw_blueprint: serde_json::to_string_pretty(&raw_blueprint)?,
            now,
        },
    )
    .await
    .map_err(blueprint_db_error)?;
    if previous_status != next_status {
        insert_project_status_history(
            &tx,
            ProjectStatusHistoryInput {
                project_id,
                from_status: previous_status,
                to_status: next_status,
                reason: "确认导入分析蓝图候选",
                changed_by: created_by,
                changed_at: now,
            },
        )
        .await
        .map_err(project_status_write_error_to_app_error)?;
    }
    let apply_result = json!({
        "blueprint_applied": true,
        "version": next_version,
        "character_count": existing_characters.len(),
        "relationship_count": existing_relationships.len(),
        "updated_fields": updated_fields,
    });
    mark_candidate_progress_confirmed(&mut progress, proposal, apply_result.clone(), now)?;
    // 正式蓝图和候选确认状态同事务提交，避免蓝图已落库后再用旧进度覆盖并发分析。
    persist_locked_import_progress(&tx, &current_project, &progress, now).await?;
    tx.commit().await?;

    if previous_status != next_status {
        context.publish_internal_event(InternalEvent::ProjectStatusChanged {
            project_id,
            from: previous_status,
            to: next_status,
            timestamp: now,
        })?;
    }
    context.publish_internal_event(InternalEvent::BlueprintUpdated {
        project_id,
        version: next_version.clone(),
    })?;

    Ok(json!({
        "status": "confirmed",
        "progress": progress,
        "applied": apply_result,
    }))
}

async fn next_blueprint_version_for_connection<C>(
    db: &C,
    project_id: i64,
    current_version: Option<&str>,
) -> Result<String, AppError>
where
    C: ConnectionTrait,
{
    let mut candidate = match current_version {
        Some(current) => increment_blueprint_version(current)?,
        None => "1.0".to_string(),
    };
    loop {
        let exists = blueprint_version::Entity::find()
            .filter(blueprint_version::Column::ProjectId.eq(project_id))
            .filter(blueprint_version::Column::Version.eq(candidate.as_str()))
            .one(db)
            .await?
            .is_some();
        if !exists {
            return Ok(candidate);
        }
        candidate = increment_blueprint_version(&candidate)?;
    }
}

pub(super) fn locked_import_progress(
    project: &a4n_core::models::project::novel_project::Model,
    project_id: i64,
) -> Result<Value, AppError> {
    let raw =
        project
            .import_analysis_progress
            .as_deref()
            .ok_or_else(|| AppError::InvalidState {
                current: "missing_import_analysis_progress".to_string(),
                operation: "读取导入蓝图候选".to_string(),
            })?;
    serde_json::from_str(raw).map_err(|error| AppError::InvalidState {
        current: format!("invalid_import_analysis_progress: {error}"),
        operation: format!("读取导入蓝图候选: project_id={project_id}"),
    })
}

pub(super) async fn persist_locked_import_progress<C>(
    db: &C,
    project: &a4n_core::models::project::novel_project::Model,
    progress: &Value,
    now: chrono::NaiveDateTime,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    let mut active: a4n_core::models::project::novel_project::ActiveModel = project.clone().into();
    active.import_analysis_progress = Set(Some(progress.to_string()));
    active.updated_at = Set(now);
    active.update(db).await?;
    Ok(())
}

fn mark_candidate_progress_confirmed(
    progress: &mut Value,
    mut proposal: Value,
    apply_result: Value,
    confirmed_at: chrono::NaiveDateTime,
) -> Result<(), AppError> {
    let stage = progress
        .get_mut("stages")
        .and_then(Value::as_object_mut)
        .and_then(|stages| stages.get_mut(STAGE_BLUEPRINT))
        .and_then(Value::as_object_mut)
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_blueprint_candidate_stage".to_string(),
            operation: "记录导入蓝图候选确认状态".to_string(),
        })?;
    let object = proposal
        .as_object_mut()
        .ok_or_else(|| AppError::InvalidState {
            current: "invalid_blueprint_candidate".to_string(),
            operation: "记录导入蓝图候选确认状态".to_string(),
        })?;
    object.insert("state".to_string(), json!("confirmed"));
    object.insert("confirmed_at".to_string(), json!(confirmed_at.to_string()));
    object.insert("apply_result".to_string(), apply_result);
    stage.insert("proposal".to_string(), proposal);
    progress["message"] = json!("导入蓝图候选已确认并应用");
    Ok(())
}

pub(super) fn required_candidate_apply_result(
    proposal: &Value,
    operation: &str,
) -> Result<Value, AppError> {
    proposal
        .get("apply_result")
        .filter(|value| value.is_object())
        .cloned()
        .ok_or_else(|| AppError::InvalidState {
            current: "confirmed_import_candidate_missing_apply_result".to_string(),
            operation: operation.to_string(),
        })
}

fn candidate_from_progress(progress: &Value, project_id: i64) -> Result<Value, AppError> {
    let status = progress
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_import_analysis_status".to_string(),
            operation: "确认导入蓝图候选".to_string(),
        })?;
    if status != "completed" {
        return Err(AppError::InvalidState {
            current: status.to_string(),
            operation: "导入分析尚未完成，不能确认蓝图候选".to_string(),
        });
    }
    ensure_import_analysis_candidate_stage_completed(progress, STAGE_BLUEPRINT).map_err(
        |error| AppError::InvalidState {
            current: "invalid_import_analysis_stage_for_candidate".to_string(),
            operation: error,
        },
    )?;
    let proposal = progress
        .get("stages")
        .and_then(Value::as_object)
        .and_then(|stages| stages.get(STAGE_BLUEPRINT))
        .and_then(Value::as_object)
        .and_then(|stage| stage.get("proposal"))
        .cloned()
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_blueprint_candidate".to_string(),
            operation: "确认导入蓝图候选".to_string(),
        })?;
    let object = proposal
        .as_object()
        .ok_or_else(|| AppError::InvalidArgument("导入蓝图候选必须是 JSON 对象".to_string()))?;
    let state = object
        .get("state")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::InvalidArgument("导入蓝图候选缺少 state".to_string()))?;
    if !matches!(state, "proposal" | "confirmed") {
        return Err(AppError::InvalidState {
            current: state.to_string(),
            operation: "确认导入蓝图候选".to_string(),
        });
    }
    let origin = object
        .get("origin")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::InvalidArgument("导入蓝图候选缺少 origin".to_string()))?;
    if origin != "import_analysis" {
        return Err(AppError::InvalidArgument(
            "只能确认来源为 import_analysis 的蓝图候选".to_string(),
        ));
    }
    let candidate_project_id = object
        .get("project_id")
        .and_then(Value::as_i64)
        .ok_or_else(|| AppError::InvalidArgument("导入蓝图候选缺少有效 project_id".to_string()))?;
    if candidate_project_id != project_id {
        return Err(AppError::InvalidArgument(
            "导入蓝图候选不属于当前项目".to_string(),
        ));
    }
    let progress_fingerprint =
        progress
            .get("input_fingerprint")
            .ok_or_else(|| AppError::InvalidState {
                current: "missing_import_analysis_input_fingerprint".to_string(),
                operation: "确认导入蓝图候选".to_string(),
            })?;
    let candidate_fingerprint =
        object
            .get("input_fingerprint")
            .ok_or_else(|| AppError::InvalidState {
                current: "missing_candidate_input_fingerprint".to_string(),
                operation: "确认导入蓝图候选".to_string(),
            })?;
    if candidate_fingerprint != progress_fingerprint {
        return Err(AppError::InvalidState {
            current: "candidate_input_fingerprint_mismatch".to_string(),
            operation: "确认导入蓝图候选".to_string(),
        });
    }
    Ok(proposal)
}

fn normalize_candidate(
    proposal: &Value,
    project: &a4n_core::models::project::novel_project::Model,
    existing: Option<&novel_blueprint::Model>,
) -> Result<Value, AppError> {
    let value = proposal
        .as_object()
        .ok_or_else(|| AppError::InvalidArgument("导入蓝图候选必须是 JSON 对象".to_string()))?;
    let title = candidate_or_existing_text(
        value,
        "title",
        existing.map(|row| row.title.as_str()),
        non_empty_text(&project.title),
    )?;
    let project_genre = import_blueprint_genre_for_candidate(&project.project_type)
        .map(str::to_string)
        .map_err(AppError::InvalidArgument)?;
    let genre = candidate_or_existing_text(
        value,
        "genre",
        existing.map(|row| row.genre.as_str()),
        Some(project_genre),
    )?;
    let one_sentence_summary = candidate_or_existing_text(
        value,
        "one_sentence_summary",
        existing.and_then(|row| row.one_sentence_summary.as_deref()),
        None,
    )?;
    let full_synopsis = candidate_or_existing_text(
        value,
        "full_synopsis",
        existing.and_then(|row| row.full_synopsis.as_deref()),
        None,
    )?;
    let total_chapters = candidate_i32(value, "total_chapters")?
        .or(project.total_chapters)
        .or(Some(project.target_chapters))
        .filter(|value| *value > 0)
        .ok_or_else(|| {
            AppError::InvalidArgument("导入蓝图候选缺少有效 total_chapters".to_string())
        })?;
    Ok(json!({
        "title": title,
        "genre": genre,
        "style": candidate_or_existing_text(
            value,
            "style",
            existing.and_then(|row| row.style.as_deref()),
            None,
        )?,
        "tone": candidate_or_existing_text(
            value,
            "tone",
            existing.and_then(|row| row.tone.as_deref()),
            None,
        )?,
        "target_audience": candidate_or_existing_text(
            value,
            "target_audience",
            existing.and_then(|row| row.target_audience.as_deref()),
            None,
        )?,
        "one_sentence_summary": one_sentence_summary,
        "full_synopsis": full_synopsis,
        "world_setting": canonical_world_setting(
            value.get("world_setting"),
            existing.and_then(|row| row.world_setting.as_ref()),
        )?,
        "total_chapters": total_chapters,
    }))
}

fn non_empty_text(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

fn candidate_or_existing_text(
    value: &serde_json::Map<String, Value>,
    key: &str,
    existing: Option<&str>,
    fallback: Option<String>,
) -> Result<String, AppError> {
    let candidate = value
        .get(key)
        .map(|value| {
            value.as_str().and_then(non_empty_text).ok_or_else(|| {
                AppError::InvalidArgument(format!("导入蓝图候选字段 {key} 必须是非空字符串"))
            })
        })
        .transpose()?;
    candidate
        .or_else(|| existing.and_then(non_empty_text))
        .or(fallback)
        .ok_or_else(|| AppError::InvalidArgument(format!("导入蓝图候选缺少有效 {key}")))
}

fn candidate_i32(
    value: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<Option<i32>, AppError> {
    value
        .get(key)
        .map(|value| {
            value
                .as_i64()
                .and_then(|value| i32::try_from(value).ok())
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!("导入蓝图候选字段 {key} 必须是有效 i32"))
                })
        })
        .transpose()
}

fn canonical_world_setting(
    source: Option<&Value>,
    existing: Option<&Value>,
) -> Result<Value, AppError> {
    let source = source
        .map(|value| {
            value.as_object().ok_or_else(|| {
                AppError::InvalidArgument("导入蓝图候选 world_setting 必须是 JSON 对象".to_string())
            })
        })
        .transpose()?;
    let existing = existing
        .map(|value| {
            value.as_object().ok_or_else(|| AppError::InvalidState {
                current: "invalid_existing_blueprint_world_setting".to_string(),
                operation: "读取已有正式蓝图世界观".to_string(),
            })
        })
        .transpose()?;
    let required = |key: &str| {
        source
            .and_then(|object| object.get(key))
            .or_else(|| existing.and_then(|object| object.get(key)))
            .cloned()
            .ok_or_else(|| {
                AppError::InvalidArgument(format!("导入蓝图候选 world_setting 缺少有效字段 {key}"))
            })
    };

    Ok(json!({
        "premise": required("premise")?,
        "core_rules": required("core_rules")?,
        "technology_or_magic": required("technology_or_magic")?,
        "key_locations": required("key_locations")?,
        "factions": required("factions")?,
        "history_timeline": required("history_timeline")?,
        "culture_and_society": required("culture_and_society")?,
        "conflict_sources": required("conflict_sources")?,
        "mysteries_and_foreshadowing": required("mysteries_and_foreshadowing")?,
        "genre_promises": required("genre_promises")?,
    }))
}

#[cfg(test)]
mod tests {
    use super::{
        STAGE_BLUEPRINT, candidate_from_progress, canonical_world_setting,
        mark_candidate_progress_confirmed, required_candidate_apply_result,
    };
    use chrono::{DateTime, Utc};
    use serde_json::json;

    fn progress_with(candidate_fingerprint: Option<serde_json::Value>) -> serde_json::Value {
        let mut proposal = json!({
            "state": "proposal",
            "origin": "import_analysis",
            "project_id": 7,
        });
        if let Some(fingerprint) = candidate_fingerprint {
            proposal["input_fingerprint"] = fingerprint;
        }
        json!({
            "status": "completed",
            "current_stage": "extracting_blueprint",
            "input_fingerprint": {"schema_version": 1},
            "stages": {
                "generating_analysis_data": {"completed": 1, "total": 1, "status": "completed"},
                "generating_outlines": {"completed": 1, "total": 1, "status": "completed"},
                "extracting_blueprint": {
                    "completed": 1,
                    "total": 1,
                    "status": "completed",
                    "proposal": proposal
                }
            }
        })
    }

    #[test]
    fn candidate_requires_matching_input_fingerprint() {
        let progress = progress_with(Some(json!({"schema_version": 1})));
        assert!(candidate_from_progress(&progress, 7).is_ok());

        let progress = progress_with(Some(json!({"schema_version": 2})));
        assert!(candidate_from_progress(&progress, 7).is_err());

        let progress = progress_with(None);
        assert!(candidate_from_progress(&progress, 7).is_err());
    }

    #[test]
    fn candidate_cannot_be_confirmed_before_analysis_completes() {
        let mut progress = progress_with(Some(json!({"schema_version": 1})));
        progress["status"] = json!("analyzing");
        let error = candidate_from_progress(&progress, 7)
            .expect_err("an in-progress analysis must not be confirmable");
        assert!(error.to_string().contains("尚未完成"));
    }

    #[test]
    fn candidate_requires_explicit_progress_status() {
        let mut progress = progress_with(Some(json!({"schema_version": 1})));
        progress
            .as_object_mut()
            .expect("progress fixture should be an object")
            .remove("status");

        let error = candidate_from_progress(&progress, 7)
            .expect_err("missing progress status must not become a diagnostic placeholder");
        assert!(error.to_string().contains("missing_import_analysis_status"));
    }

    #[test]
    fn sparse_candidate_world_setting_is_rejected_without_fabrication() {
        let candidate = json!({"locations": ["新城"], "tags": ["悬疑"]});
        let error = canonical_world_setting(Some(&candidate), None)
            .expect_err("sparse world data must not become a formal blueprint");

        assert!(error.to_string().contains("premise"));
        assert!(!error.to_string().contains("未识别"));
        assert!(!error.to_string().contains("待补充"));
    }

    #[test]
    fn complete_existing_world_setting_is_preserved() {
        let existing = json!({
            "premise": "用户确认的前提",
            "core_rules": "用户确认的规则",
            "technology_or_magic": {
                "source": "无",
                "rules": ["规则"],
                "limits": ["限制"],
                "costs": ["代价"]
            },
            "key_locations": [{
                "name": "旧城",
                "description": "已确认",
                "story_function": "调查据点"
            }],
            "factions": [{
                "name": "旧势力",
                "description": "已确认",
                "agenda": "守城",
                "representative_characters": ["甲"]
            }],
            "history_timeline": [{"period": "旧纪元", "event": "建城", "impact": "延续至今"}],
            "culture_and_society": {
                "values": "秩序",
                "class_structure": "分层",
                "daily_life": "守城"
            },
            "conflict_sources": [{"name": "旧冲突", "description": "已确认", "stakes": "存亡"}],
            "mysteries_and_foreshadowing": [{"mystery": "旧谜团", "early_clues": ["线索"], "payoff_direction": "终章"}],
            "genre_promises": ["悬疑"]
        });

        let normalized = canonical_world_setting(None, Some(&existing))
            .expect("a complete existing world setting should remain applicable");

        assert_eq!(normalized, existing);
    }

    #[test]
    fn confirmation_updates_candidate_state_without_replacing_progress_snapshot() {
        let mut progress = progress_with(Some(json!({"schema_version": 1})));
        let proposal = progress["stages"][STAGE_BLUEPRINT]["proposal"].clone();
        mark_candidate_progress_confirmed(
            &mut progress,
            proposal,
            json!({"version": "1.0"}),
            DateTime::<Utc>::from_timestamp(1, 0)
                .expect("valid timestamp")
                .naive_utc(),
        )
        .expect("candidate confirmation should be representable");

        assert_eq!(
            progress["stages"][STAGE_BLUEPRINT]["proposal"]["state"],
            "confirmed"
        );
        assert_eq!(
            progress["stages"][STAGE_BLUEPRINT]["proposal"]["apply_result"]["version"],
            "1.0"
        );
        assert_eq!(progress["input_fingerprint"]["schema_version"], 1);
    }

    #[test]
    fn confirmed_candidate_requires_structured_apply_result() {
        let error = required_candidate_apply_result(&json!({"state": "confirmed"}), "读取确认结果")
            .expect_err("a confirmed candidate without an apply result is invalid");
        assert!(error.to_string().contains("missing_apply_result"));

        let result = required_candidate_apply_result(
            &json!({"state": "confirmed", "apply_result": {"version": "1.0"}}),
            "读取确认结果",
        )
        .expect("structured apply result should be returned");
        assert_eq!(result["version"], "1.0");
    }
}
