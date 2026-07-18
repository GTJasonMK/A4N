use serde_json::Value;

use super::progress::{CHAPTERS_PER_PART, progress_stage_completed};

pub fn completed_import_stage(progress: &Value, stage: &str, max: usize) -> Result<usize, String> {
    Ok(progress_stage_completed(progress, stage)?.min(max))
}

pub fn completed_import_part_outlines(
    progress: &Value,
    needs_part_outlines: bool,
    chapter_count: usize,
) -> Result<usize, String> {
    if needs_part_outlines {
        completed_import_stage(
            progress,
            super::progress::STAGE_PARTS,
            chapter_count.div_ceil(CHAPTERS_PER_PART),
        )
    } else {
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{completed_import_part_outlines, completed_import_stage};

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

    #[test]
    fn completed_parts_use_shared_part_size_and_disabled_stage_returns_zero() {
        let progress = json!({
            "stages": {
                "generating_part_outlines": {"completed": 4, "total": 4}
            }
        });

        assert_eq!(
            completed_import_part_outlines(&progress, true, 51).unwrap(),
            3
        );
        assert_eq!(
            completed_import_part_outlines(&progress, false, 51).unwrap(),
            0
        );
    }
}
