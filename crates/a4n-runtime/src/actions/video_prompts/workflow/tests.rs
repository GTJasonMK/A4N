use super::super::CHAPTER_VIDEO_PROMPT_WORKFLOW_MODES;
use super::*;

#[test]
fn workflow_mode_normalization_accepts_only_supported_explicit_modes() {
    for mode in CHAPTER_VIDEO_PROMPT_WORKFLOW_MODES {
        assert_eq!(
            normalize_video_workflow_mode(Some(mode)).as_deref(),
            Ok(*mode)
        );
    }
    assert_eq!(
        normalize_video_workflow_mode(Some(" unsupported ")),
        Err("视频提示词生成 workflow_mode 无效: unsupported".to_string())
    );
    assert_eq!(
        normalize_video_workflow_mode(Some("  ")),
        Err("视频提示词生成缺少 workflow_mode".to_string())
    );
    assert_eq!(
        normalize_video_workflow_mode(None),
        Err("视频提示词生成缺少 workflow_mode".to_string())
    );
}
