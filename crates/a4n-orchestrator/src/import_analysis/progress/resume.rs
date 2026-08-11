use super::schema::{
    STAGE_ANALYSIS, STAGE_ORDER, progress_stages_mut, recalc_import_progress,
    required_stage_object_mut,
};
use serde_json::{Value, json};

fn import_stage_index(stage: &str) -> Option<usize> {
    STAGE_ORDER.iter().position(|key| *key == stage)
}

pub fn import_progress_resume_stage(progress: &Value) -> Option<String> {
    let status = progress.get("status").and_then(Value::as_str)?;
    if !matches!(status, "failed" | "cancelled" | "analyzing") {
        return None;
    }
    let stage = progress.get("current_stage").and_then(Value::as_str)?;
    import_stage_index(stage)?;
    progress
        .get("stages")
        .and_then(Value::as_object)
        .and_then(|stages| stages.contains_key(stage).then(|| stage.to_string()))
}

pub fn import_progress_totals_match(progress: &Value, total_chapters: usize) -> bool {
    let Some(stages) = progress.get("stages").and_then(Value::as_object) else {
        return false;
    };
    stages.len() == 1
        && stages
            .get(STAGE_ANALYSIS)
            .and_then(|stage| stage.get("total"))
            .and_then(Value::as_u64)
            .and_then(|total| usize::try_from(total).ok())
            == Some(total_chapters)
}

pub fn resume_import_progress(progress: &mut Value, current_stage: &str) -> Result<(), String> {
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
