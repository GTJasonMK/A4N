use super::schema::{
    CHAPTERS_PER_PART, STAGE_ORDER, progress_stages_mut, recalc_import_progress,
    required_stage_object_mut,
};
use serde_json::{Value, json};

fn import_stage_index(stage: &str) -> Option<usize> {
    STAGE_ORDER.iter().position(|key| *key == stage)
}

pub fn import_progress_resume_stage(progress: &Value, needs_part_outlines: bool) -> Option<String> {
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

pub fn import_progress_totals_match(
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
