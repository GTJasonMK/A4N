use chrono::Utc;
use serde_json::{Map, Value, json};

pub const STAGE_ANALYSIS: &str = "generating_analysis_data";
pub(super) const STAGE_ORDER: [&str; 1] = [STAGE_ANALYSIS];

pub fn initial_import_progress(total_chapters: usize) -> Value {
    let mut stages = Map::new();
    stages.insert(
        STAGE_ANALYSIS.to_string(),
        json!({
            "name": "逐章分析与资料同步",
            "completed": 0,
            "total": total_chapters,
            "status": "pending",
        }),
    );

    json!({
        "status": "analyzing",
        "current_stage": STAGE_ANALYSIS,
        "stages": stages,
        "message": "正在初始化导入分析...",
        "started_at": Utc::now().naive_utc().to_string(),
        "error": null,
        "overall_progress": 0,
    })
}

pub(super) fn parse_import_progress(
    project_id: i64,
    raw: Option<&str>,
) -> Result<Option<Value>, String> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    serde_json::from_str(raw)
        .map(Some)
        .map_err(|err| format!("导入分析进度 JSON 无效: project_id={project_id}, error={err}"))
}

pub(super) fn progress_stages(progress: &Value) -> Result<&Map<String, Value>, String> {
    progress
        .get("stages")
        .and_then(Value::as_object)
        .ok_or_else(|| "导入分析进度缺少 stages 对象".to_string())
}

pub(super) fn progress_stages_mut(progress: &mut Value) -> Result<&mut Map<String, Value>, String> {
    progress
        .get_mut("stages")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "导入分析进度缺少 stages 对象".to_string())
}

pub(super) fn required_stage_object<'a>(
    stages: &'a Map<String, Value>,
    stage: &str,
) -> Result<&'a Map<String, Value>, String> {
    stages
        .get(stage)
        .ok_or_else(|| format!("导入分析进度缺少阶段: {stage}"))?
        .as_object()
        .ok_or_else(|| format!("导入分析进度阶段 {stage} 必须是对象"))
}

pub(super) fn required_stage_object_mut<'a>(
    stages: &'a mut Map<String, Value>,
    stage: &str,
) -> Result<&'a mut Map<String, Value>, String> {
    stages
        .get_mut(stage)
        .ok_or_else(|| format!("导入分析进度缺少阶段: {stage}"))?
        .as_object_mut()
        .ok_or_else(|| format!("导入分析进度阶段 {stage} 必须是对象"))
}

pub(super) fn required_stage_count(
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

pub(super) fn required_stage_counts(
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

pub(super) fn validate_stage_objects(stages: &Map<String, Value>) -> Result<(), String> {
    for (stage_key, stage) in stages {
        if !stage.is_object() {
            return Err(format!("导入分析进度阶段 {stage_key} 必须是对象"));
        }
    }
    Ok(())
}

pub(super) fn recalc_import_progress(progress: &mut Value) -> Result<(), String> {
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
