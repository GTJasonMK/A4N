use super::*;
use crate::event::sse::{ProgressPhaseKind, ProgressTaskKind};

#[test]
fn galgame_task_type_uses_galgame_generation_progress_meta() {
    let meta = TaskType::GalgameGeneration.progress_meta("正在执行 Galgame 管线");

    assert_eq!(TaskType::GalgameGeneration.as_str(), "galgame_generation");
    assert!(matches!(
        &meta.task_kind,
        ProgressTaskKind::GalgameGeneration
    ));
    assert_eq!(meta.stage_code, "galgame_pipeline");
    assert_eq!(meta.stage_label, "Galgame 管线");
    assert!(matches!(&meta.phase_kind, ProgressPhaseKind::ModelWait));
    assert_eq!(meta.detail.as_deref(), Some("正在执行 Galgame 管线"));
}
