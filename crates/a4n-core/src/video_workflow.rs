pub fn is_assembly_video_workflow_mode(mode: &str) -> bool {
    matches!(
        mode,
        "boundary_frames"
            | "first_frame_chain"
            | "reference_media"
            | "chat_video"
            | "storyboard"
            | "mixed"
    )
}

pub fn is_generated_video_workflow_mode(mode: &str) -> bool {
    is_assembly_video_workflow_mode(mode)
        || matches!(
            mode,
            "canvas_video" | "canvas_reference_media" | "canvas_trim" | "canvas_assembly"
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_video_modes_include_assembly_and_canvas_modes() {
        assert!(is_assembly_video_workflow_mode("mixed"));
        assert!(is_generated_video_workflow_mode("mixed"));
        assert!(is_generated_video_workflow_mode("canvas_assembly"));
        assert!(!is_assembly_video_workflow_mode("canvas_assembly"));
        assert!(!is_generated_video_workflow_mode("legacy"));
    }
}
