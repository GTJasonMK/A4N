//! Pure workflow classification rules for generated video records.

pub fn is_canvas_video_record_fields(chapter_id: Option<i64>, workflow_mode: Option<&str>) -> bool {
    chapter_id.is_none()
        && workflow_mode
            .is_some_and(|mode| a4n_domain::video_workflow::CANVAS_WORKFLOW_MODES.contains(&mode))
}

#[cfg(test)]
mod tests {
    use super::is_canvas_video_record_fields;

    #[test]
    fn canvas_video_record_requires_canvas_workflow_without_chapter() {
        for mode in a4n_domain::video_workflow::CANVAS_WORKFLOW_MODES {
            assert!(is_canvas_video_record_fields(None, Some(mode)));
            assert!(!is_canvas_video_record_fields(Some(1), Some(mode)));
        }
        assert!(!is_canvas_video_record_fields(None, None));
        assert!(!is_canvas_video_record_fields(None, Some("chapter")));
    }
}
