use super::schema::{
    progress_stages, progress_stages_mut, recalc_import_progress, required_stage_count,
    required_stage_counts, required_stage_object, required_stage_object_mut,
    validate_stage_objects,
};
use chrono::Utc;
use serde_json::{Value, json};

pub fn progress_stage_completed(progress: &Value, stage: &str) -> Result<usize, String> {
    let stages = progress_stages(progress)?;
    let stage_obj = required_stage_object(stages, stage)?;
    let (completed, _) = required_stage_counts(stage, stage_obj)?;
    usize::try_from(completed)
        .map_err(|_| format!("导入分析进度阶段 {stage}.completed 超出 usize 范围"))
}

pub fn update_import_progress_stage(
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

pub(super) fn mark_import_progress_failed(
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

pub fn finish_import_progress(progress: &mut Value) -> Result<(), String> {
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
