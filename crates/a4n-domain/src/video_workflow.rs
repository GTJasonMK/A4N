pub const VIDEO_TYPE_CHAPTER: &str = "chapter_video";
pub const VIDEO_TYPE_REFERENCE_MEDIA: &str = "reference_media_video";
pub const VIDEO_TYPE_TRIMMED_CLIP: &str = "trimmed_clip";
pub const VIDEO_TYPE_ASSEMBLY_INSERT: &str = "assembly_insert";
pub const VIDEO_TYPE_ASSEMBLY_PUSH: &str = "assembly_push";
pub const VIDEO_TYPE_ASSEMBLY_POSTPROCESS_EDIT: &str = "assembly_postprocess_edit";
pub const VIDEO_TYPE_CANVAS: &str = "canvas_video";
pub const VIDEO_TYPE_CANVAS_ASSEMBLY: &str = "canvas_assembly";

pub const VIDEO_STATUS_PENDING: &str = "pending";
pub const VIDEO_STATUS_RUNNING: &str = "running";
pub const VIDEO_STATUS_FAILED: &str = "failed";
pub const VIDEO_STATUS_INTERRUPTED: &str = "interrupted";
pub const VIDEO_STATUS_COMPLETED: &str = "completed";
pub const VIDEO_STATUS_CANCELLED: &str = "cancelled";
pub const VIDEO_GENERATION_PROGRESS_TOTAL: u32 = 7;

pub const ASSEMBLY_PUSH_STATUS_PENDING: &str = "pending";
pub const ASSEMBLY_PUSH_STATUS_ACCEPTED: &str = "accepted";
pub const ASSEMBLY_PUSH_STATUS_REJECTED: &str = "rejected";

pub const WORKFLOW_MODE_BOUNDARY_FRAMES: &str = "boundary_frames";
pub const WORKFLOW_MODE_FIRST_FRAME_CHAIN: &str = "first_frame_chain";
pub const WORKFLOW_MODE_CHAT_VIDEO: &str = "chat_video";
pub const WORKFLOW_MODE_STORYBOARD: &str = "storyboard";

pub fn is_terminal_video_status(status: &str) -> bool {
    matches!(
        status,
        VIDEO_STATUS_COMPLETED | VIDEO_STATUS_FAILED | VIDEO_STATUS_CANCELLED
    )
}

pub const WORKFLOW_MODE_MIXED: &str = "mixed";
pub const WORKFLOW_MODE_CANVAS_VIDEO: &str = "canvas_video";
pub const WORKFLOW_MODE_CANVAS_TRIM: &str = "canvas_trim";
pub const WORKFLOW_MODE_CANVAS_ASSEMBLY: &str = "canvas_assembly";
pub const CANVAS_WORKFLOW_MODES: [&str; 3] = [
    WORKFLOW_MODE_CANVAS_VIDEO,
    WORKFLOW_MODE_CANVAS_TRIM,
    WORKFLOW_MODE_CANVAS_ASSEMBLY,
];

pub const WORKFLOW_MODE_REFERENCE_MEDIA: &str = "reference_media";
const ASSEMBLY_WORKFLOW_MODES: &[&str] = &[
    WORKFLOW_MODE_REFERENCE_MEDIA,
    WORKFLOW_MODE_BOUNDARY_FRAMES,
    WORKFLOW_MODE_FIRST_FRAME_CHAIN,
    WORKFLOW_MODE_CHAT_VIDEO,
    WORKFLOW_MODE_STORYBOARD,
    WORKFLOW_MODE_MIXED,
];

pub fn is_assembly_video_workflow_mode(mode: &str) -> bool {
    ASSEMBLY_WORKFLOW_MODES.contains(&mode)
}

pub fn is_generated_video_workflow_mode(mode: &str) -> bool {
    is_assembly_video_workflow_mode(mode) || CANVAS_WORKFLOW_MODES.contains(&mode)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_video_modes_include_assembly_and_canvas_modes() {
        assert!(is_assembly_video_workflow_mode(WORKFLOW_MODE_MIXED));
        assert!(is_generated_video_workflow_mode(WORKFLOW_MODE_MIXED));
        assert!(is_generated_video_workflow_mode(
            WORKFLOW_MODE_CANVAS_ASSEMBLY
        ));
        assert!(!is_assembly_video_workflow_mode(
            WORKFLOW_MODE_CANVAS_ASSEMBLY
        ));
        assert!(!is_generated_video_workflow_mode("legacy"));
    }
}
