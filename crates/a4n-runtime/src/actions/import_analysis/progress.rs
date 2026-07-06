use a4n_core::models::project::novel_project;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, Set};
use serde_json::{Map, Value, json};

const CHAPTERS_PER_PART: usize = 25;
const STAGE_ORDER: [&str; 4] = [
    "generating_analysis_data",
    "generating_outlines",
    "extracting_blueprint",
    "generating_part_outlines",
];

pub(super) fn initial_import_progress(total_chapters: usize, needs_part_outlines: bool) -> Value {
    let part_count = if needs_part_outlines {
        total_chapters.div_ceil(CHAPTERS_PER_PART)
    } else {
        0
    };
    let mut stages = serde_json::Map::new();
    for (key, name, total) in [
        ("generating_analysis_data", "生成分析数据", total_chapters),
        ("generating_outlines", "更新章节大纲", total_chapters),
        ("extracting_blueprint", "反推蓝图信息", 1usize),
        ("generating_part_outlines", "生成分部大纲", part_count),
    ] {
        if key == "generating_part_outlines" && !needs_part_outlines {
            continue;
        }
        stages.insert(
            key.to_string(),
            json!({
                "name": name,
                "completed": 0,
                "total": total,
                "status": "pending",
            }),
        );
    }

    json!({
        "status": "analyzing",
        "current_stage": "generating_analysis_data",
        "stages": stages,
        "message": "正在初始化导入分析...",
        "started_at": Utc::now().naive_utc().to_string(),
        "error": null,
        "needs_part_outlines": needs_part_outlines,
        "overall_progress": 0,
    })
}

fn parse_import_progress(project: &novel_project::Model) -> Result<Option<Value>, String> {
    let Some(raw) = project.import_analysis_progress.as_deref() else {
        return Ok(None);
    };

    serde_json::from_str(raw).map(Some).map_err(|err| {
        format!(
            "导入分析进度 JSON 无效: project_id={}, error={err}",
            project.id
        )
    })
}

pub(super) async fn load_import_progress_optional(
    db: &impl ConnectionTrait,
    project_id: i64,
) -> Result<Option<Value>, String> {
    let project = novel_project::Entity::find_by_id(project_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("项目不存在: project_id={project_id}"))?;

    let Some(mut progress) = parse_import_progress(&project)? else {
        return Ok(None);
    };
    recalc_import_progress(&mut progress)?;
    Ok(Some(progress))
}

fn progress_stages(progress: &Value) -> Result<&Map<String, Value>, String> {
    progress
        .get("stages")
        .and_then(Value::as_object)
        .ok_or_else(|| "导入分析进度缺少 stages 对象".to_string())
}

fn progress_stages_mut(progress: &mut Value) -> Result<&mut Map<String, Value>, String> {
    progress
        .get_mut("stages")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "导入分析进度缺少 stages 对象".to_string())
}

fn required_stage_object<'a>(
    stages: &'a Map<String, Value>,
    stage: &str,
) -> Result<&'a Map<String, Value>, String> {
    stages
        .get(stage)
        .ok_or_else(|| format!("导入分析进度缺少阶段: {stage}"))?
        .as_object()
        .ok_or_else(|| format!("导入分析进度阶段 {stage} 必须是对象"))
}

fn required_stage_object_mut<'a>(
    stages: &'a mut Map<String, Value>,
    stage: &str,
) -> Result<&'a mut Map<String, Value>, String> {
    stages
        .get_mut(stage)
        .ok_or_else(|| format!("导入分析进度缺少阶段: {stage}"))?
        .as_object_mut()
        .ok_or_else(|| format!("导入分析进度阶段 {stage} 必须是对象"))
}

fn required_stage_count(
    stage_key: &str,
    stage: &Map<String, Value>,
    field: &str,
) -> Result<u64, String> {
    stage
        .get(field)
        .ok_or_else(|| format!("导入分析进度阶段 {stage_key} 缺少 {field}"))?
        .as_u64()
        .ok_or_else(|| format!("导入分析进度阶段 {stage_key}.{field} 必须是非负整数"))
}

fn required_stage_counts(
    stage_key: &str,
    stage: &Map<String, Value>,
) -> Result<(u64, u64), String> {
    let total = required_stage_count(stage_key, stage, "total")?;
    let completed = required_stage_count(stage_key, stage, "completed")?;
    if completed > total {
        return Err(format!(
            "导入分析进度阶段 {stage_key}.completed 不能大于 total: completed={completed}, total={total}"
        ));
    }
    Ok((completed, total))
}

fn validate_stage_objects(stages: &Map<String, Value>) -> Result<(), String> {
    for (stage_key, stage) in stages {
        if !stage.is_object() {
            return Err(format!("导入分析进度阶段 {stage_key} 必须是对象"));
        }
    }
    Ok(())
}

fn recalc_import_progress(progress: &mut Value) -> Result<(), String> {
    let (completed, total) = {
        let stages = progress_stages(progress)?;
        let mut total = 0u128;
        let mut completed = 0u128;
        for (stage_key, stage) in stages {
            let stage = stage
                .as_object()
                .ok_or_else(|| format!("导入分析进度阶段 {stage_key} 必须是对象"))?;
            let (stage_completed, stage_total) = required_stage_counts(stage_key, stage)?;
            total += u128::from(stage_total);
            completed += u128::from(stage_completed);
        }
        (completed, total)
    };

    if total == 0 {
        return Err("导入分析进度总数必须大于 0".to_string());
    }

    let percent = ((completed * 100) / total).clamp(0, 100);
    progress["overall_progress"] = json!(percent as u64);
    Ok(())
}

fn import_stage_index(stage: &str) -> Option<usize> {
    STAGE_ORDER.iter().position(|key| *key == stage)
}

pub(super) fn import_progress_resume_stage(
    progress: &Value,
    needs_part_outlines: bool,
) -> Option<String> {
    let status = progress.get("status").and_then(Value::as_str)?;
    if !matches!(status, "failed" | "cancelled" | "analyzing") {
        return None;
    }
    let stage = progress.get("current_stage").and_then(Value::as_str)?;
    if stage == "generating_part_outlines" && !needs_part_outlines {
        return None;
    }
    import_stage_index(stage)?;
    progress
        .get("stages")
        .and_then(Value::as_object)
        .and_then(|stages| stages.contains_key(stage).then(|| stage.to_string()))
}

pub(super) fn import_progress_totals_match(
    progress: &Value,
    total_chapters: usize,
    needs_part_outlines: bool,
) -> bool {
    let Some(stages) = progress.get("stages").and_then(Value::as_object) else {
        return false;
    };
    let part_count = if needs_part_outlines {
        total_chapters.div_ceil(CHAPTERS_PER_PART)
    } else {
        0
    };
    for (stage, expected) in [
        ("generating_analysis_data", total_chapters),
        ("generating_outlines", total_chapters),
        ("extracting_blueprint", 1usize),
        ("generating_part_outlines", part_count),
    ] {
        if stage == "generating_part_outlines" && !needs_part_outlines {
            continue;
        }
        let Some(actual) = stages
            .get(stage)
            .and_then(|v| v.get("total"))
            .and_then(Value::as_u64)
        else {
            return false;
        };
        if actual as usize != expected {
            return false;
        }
    }
    true
}

pub(super) fn resume_import_progress(
    progress: &mut Value,
    current_stage: &str,
) -> Result<(), String> {
    {
        let stages = progress_stages_mut(progress)?;
        let stage = required_stage_object_mut(stages, current_stage)?;
        stage.insert("status".to_string(), json!("in_progress"));
    }
    progress["status"] = json!("analyzing");
    progress["current_stage"] = json!(current_stage);
    progress["message"] = json!("正在恢复导入分析...");
    progress["error"] = Value::Null;
    recalc_import_progress(progress)?;
    Ok(())
}

pub(super) fn progress_stage_completed(progress: &Value, stage: &str) -> Result<usize, String> {
    let stages = progress_stages(progress)?;
    let stage_obj = required_stage_object(stages, stage)?;
    let (completed, _) = required_stage_counts(stage, stage_obj)?;
    usize::try_from(completed)
        .map_err(|_| format!("导入分析进度阶段 {stage}.completed 超出 usize 范围"))
}

pub(super) fn update_import_progress_stage(
    progress: &mut Value,
    stage: &str,
    completed: usize,
    total: usize,
    message: impl Into<String>,
) -> Result<(), String> {
    {
        let stages = progress_stages(progress)?;
        required_stage_object(stages, stage)?;
        validate_stage_objects(stages)?;
    }
    if completed > total {
        return Err(format!(
            "导入分析进度阶段 {stage}.completed 不能大于 total: completed={completed}, total={total}"
        ));
    }
    progress["status"] = json!("analyzing");
    progress["current_stage"] = json!(stage);
    progress["message"] = json!(message.into());

    {
        let stages = progress_stages_mut(progress)?;
        for (key, value) in stages.iter_mut() {
            let stage_obj = value
                .as_object_mut()
                .ok_or_else(|| format!("导入分析进度阶段 {key} 必须是对象"))?;
            if key == stage {
                stage_obj.insert("completed".to_string(), json!(completed));
                stage_obj.insert("total".to_string(), json!(total));
                stage_obj.insert(
                    "status".to_string(),
                    json!(if total > 0 && completed >= total {
                        "completed"
                    } else {
                        "in_progress"
                    }),
                );
            } else if stage_obj.get("status").and_then(Value::as_str) == Some("in_progress") {
                stage_obj.insert("status".to_string(), json!("completed"));
            }
        }
    }
    recalc_import_progress(progress)?;
    Ok(())
}

fn mark_import_progress_failed(
    progress: &mut Value,
    error: impl Into<String>,
) -> Result<(), String> {
    let error = error.into();
    let message = format!("导入分析失败: {error}");
    progress["status"] = json!("failed");
    progress["error"] = json!(error);
    progress["message"] = json!(message);
    progress["failed_at"] = json!(Utc::now().naive_utc().to_string());
    if let Some(current_stage) = progress
        .get("current_stage")
        .and_then(Value::as_str)
        .map(str::to_string)
    {
        let stages = progress_stages_mut(progress)?;
        let stage = required_stage_object_mut(stages, &current_stage)?;
        stage.insert("status".to_string(), json!("failed"));
    }
    recalc_import_progress(progress)?;
    Ok(())
}

pub(super) fn finish_import_progress(progress: &mut Value) -> Result<(), String> {
    let totals = {
        let stages = progress_stages(progress)?;
        validate_stage_objects(stages)?;
        stages
            .iter()
            .map(|(stage_key, stage)| {
                let stage = stage
                    .as_object()
                    .ok_or_else(|| format!("导入分析进度阶段 {stage_key} 必须是对象"))?;
                required_stage_count(stage_key, stage, "total")
                    .map(|total| (stage_key.clone(), total))
            })
            .collect::<Result<Vec<_>, _>>()?
    };

    {
        let stages = progress_stages_mut(progress)?;
        for (stage_key, total) in totals {
            let stage_obj = required_stage_object_mut(stages, &stage_key)?;
            stage_obj.insert("completed".to_string(), json!(total));
            stage_obj.insert("status".to_string(), json!("completed"));
        }
    }
    progress["status"] = json!("completed");
    progress["message"] = json!("导入分析完成");
    progress["completed_at"] = json!(Utc::now().naive_utc().to_string());
    recalc_import_progress(progress)?;
    Ok(())
}

pub(super) async fn save_import_progress(
    db: &impl ConnectionTrait,
    project_id: i64,
    progress: &Value,
) -> Result<Value, String> {
    let project = novel_project::Entity::find_by_id(project_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("项目不存在: project_id={project_id}"))?;

    let mut active: novel_project::ActiveModel = project.into();
    active.import_analysis_progress = Set(Some(progress.to_string()));
    active.updated_at = Set(Utc::now().naive_utc());
    active.update(db).await.map_err(|e| e.to_string())?;
    Ok(progress.clone())
}

pub(super) async fn load_import_progress(
    db: &impl ConnectionTrait,
    project_id: i64,
) -> Result<Value, String> {
    load_import_progress_optional(db, project_id)
        .await?
        .ok_or_else(|| format!("项目尚未创建导入分析进度: project_id={project_id}"))
}

pub(super) async fn save_failed_import_progress(
    db: &impl ConnectionTrait,
    project_id: i64,
    error: &str,
) -> Result<(), String> {
    let mut progress = load_import_progress(db, project_id).await?;
    mark_import_progress_failed(&mut progress, error)?;
    save_import_progress(db, project_id, &progress).await?;
    Ok(())
}

pub(super) async fn cancel_import_progress(
    db: &impl ConnectionTrait,
    project_id: i64,
) -> Result<(), String> {
    let mut progress = load_import_progress(db, project_id).await?;
    progress["status"] = json!("cancelled");
    progress["message"] = json!("导入分析已取消");
    progress["cancelled_at"] = json!(Utc::now().naive_utc().to_string());
    save_import_progress(db, project_id, &progress).await?;
    Ok(())
}

pub(super) async fn import_analysis_cancelled(
    db: &impl ConnectionTrait,
    project_id: i64,
) -> Result<bool, String> {
    let progress = load_import_progress(db, project_id).await?;
    Ok(progress.get("status").and_then(Value::as_str) == Some("cancelled"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_progress(status: &str, current_stage: &str, part_total: usize) -> Value {
        json!({
            "status": status,
            "current_stage": current_stage,
            "stages": {
                "generating_analysis_data": {
                    "name": "生成分析数据",
                    "completed": 12,
                    "total": 20,
                    "status": "completed",
                },
                "generating_outlines": {
                    "name": "更新章节大纲",
                    "completed": 8,
                    "total": 20,
                    "status": "in_progress",
                },
                "extracting_blueprint": {
                    "name": "反推蓝图信息",
                    "completed": 1,
                    "total": 1,
                    "status": "completed",
                },
                "generating_part_outlines": {
                    "name": "生成分部大纲",
                    "completed": 0,
                    "total": part_total,
                    "status": "pending",
                }
            }
        })
    }

    #[test]
    fn totals_match_detects_stage_shape() {
        let progress = sample_progress("failed", "generating_outlines", 1);
        assert!(import_progress_totals_match(&progress, 20, true));
        assert!(!import_progress_totals_match(&progress, 21, true));
        assert!(import_progress_totals_match(&progress, 20, false));
    }

    #[test]
    fn resume_stage_rejects_disabled_part_outline_stage() {
        let progress = sample_progress("failed", "generating_part_outlines", 2);
        assert_eq!(import_progress_resume_stage(&progress, false), None);
        assert_eq!(
            import_progress_resume_stage(&progress, true),
            Some("generating_part_outlines".to_string())
        );
    }

    #[test]
    fn resume_progress_marks_current_stage_in_progress() {
        let mut progress = sample_progress("failed", "generating_outlines", 2);
        resume_import_progress(&mut progress, "generating_outlines").expect("resume succeeds");
        assert_eq!(progress["status"], "analyzing");
        assert_eq!(progress["error"], Value::Null);
        assert_eq!(progress["current_stage"], "generating_outlines");
        assert_eq!(
            progress["stages"]["generating_outlines"]["status"],
            "in_progress"
        );
    }

    #[test]
    fn failure_progress_records_error_and_timestamp() {
        let mut progress = sample_progress("analyzing", "generating_outlines", 2);
        mark_import_progress_failed(&mut progress, "boom").expect("failure progress is valid");
        assert_eq!(progress["status"], "failed");
        assert_eq!(progress["error"], "boom");
        assert_eq!(progress["message"], "导入分析失败: boom");
        assert!(progress["failed_at"].as_str().is_some());
        assert_eq!(
            progress["stages"]["generating_outlines"]["status"],
            "failed"
        );
    }

    #[test]
    fn invalid_progress_json_is_reported() {
        let project = novel_project::Model {
            id: 42,
            user_id: 1,
            title: "测试".to_string(),
            description: String::new(),
            project_type: "novel_long".to_string(),
            project_kind: "text".to_string(),
            source_mode: "self".to_string(),
            root_project_id: None,
            parent_project_id: None,
            source_snapshot_id: None,
            source_sync_status: "current".to_string(),
            derived_project_status: None,
            status: "draft".to_string(),
            target_chapters: 20,
            current_chapter: 0,
            total_chapters: None,
            chapters_per_part: None,
            avatar_svg: None,
            avatar_animal: None,
            raw_blueprint: None,
            import_analysis_progress: Some("{broken".to_string()),
            export_config: None,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        };

        let err = parse_import_progress(&project).expect_err("invalid JSON should fail");
        assert!(err.contains("project_id=42"));
    }

    #[test]
    fn stage_completed_reads_completed_count() {
        let progress = sample_progress("analyzing", "generating_outlines", 2);
        assert_eq!(
            progress_stage_completed(&progress, "generating_analysis_data")
                .expect("completed count is present"),
            12
        );
        let err = progress_stage_completed(&progress, "missing_stage")
            .expect_err("missing stage should be explicit");
        assert!(err.contains("缺少阶段: missing_stage"));
    }

    #[test]
    fn recalc_rejects_malformed_stage_counts() {
        let mut progress = sample_progress("analyzing", "generating_outlines", 2);
        progress["stages"]["generating_outlines"]
            .as_object_mut()
            .expect("stage object")
            .remove("total");

        let err = recalc_import_progress(&mut progress).expect_err("missing total should fail");
        assert!(err.contains("generating_outlines 缺少 total"));
    }

    #[test]
    fn recalc_rejects_completed_count_above_total() {
        let mut progress = sample_progress("analyzing", "generating_outlines", 2);
        progress["stages"]["generating_outlines"]["completed"] = json!(21);

        let err =
            recalc_import_progress(&mut progress).expect_err("completed above total should fail");

        assert!(err.contains("generating_outlines.completed 不能大于 total"));
        assert!(err.contains("completed=21"));
        assert!(err.contains("total=20"));
    }

    #[test]
    fn recalc_rejects_zero_total_progress() {
        let mut progress = json!({
            "status": "analyzing",
            "current_stage": "empty",
            "stages": {
                "empty": {
                    "name": "空进度",
                    "completed": 0,
                    "total": 0,
                    "status": "pending",
                }
            }
        });

        let err = recalc_import_progress(&mut progress).expect_err("zero total should fail");

        assert!(err.contains("导入分析进度总数必须大于 0"));
    }

    #[test]
    fn update_progress_rejects_completed_count_above_total_without_mutating() {
        let mut progress = sample_progress("analyzing", "generating_outlines", 2);
        let before = progress.clone();

        let err = update_import_progress_stage(
            &mut progress,
            "generating_outlines",
            21,
            20,
            "bad progress",
        )
        .expect_err("completed above total should fail");

        assert!(err.contains("generating_outlines.completed 不能大于 total"));
        assert_eq!(progress, before);
    }

    #[test]
    fn finish_rejects_noninteger_stage_total() {
        let mut progress = sample_progress("analyzing", "generating_outlines", 2);
        progress["stages"]["generating_outlines"]["total"] = json!("20");

        let err = finish_import_progress(&mut progress).expect_err("noninteger total should fail");
        assert!(err.contains("generating_outlines.total 必须是非负整数"));
    }
}
