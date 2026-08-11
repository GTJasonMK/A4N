use serde_json::Value;

use super::progress::progress_stage_completed;

pub fn completed_import_stage(progress: &Value, stage: &str, max: usize) -> Result<usize, String> {
    Ok(progress_stage_completed(progress, stage)?.min(max))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::completed_import_stage;

    #[test]
    fn completed_stage_caps_saved_count_at_current_scope() {
        let progress = json!({
            "stages": {
                "generating_analysis_data": {"completed": 12, "total": 12}
            }
        });

        assert_eq!(
            completed_import_stage(&progress, "generating_analysis_data", 10).unwrap(),
            10
        );
    }
}
