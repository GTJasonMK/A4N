use super::ensure_freeform_source_mode;
use crate::project::{SOURCE_MODE_FREEFORM, duplicate::ProjectDuplicateError};

#[test]
fn freeform_source_mode_gate_preserves_error_contract() {
    assert!(ensure_freeform_source_mode(SOURCE_MODE_FREEFORM).is_ok());
    assert!(matches!(
        ensure_freeform_source_mode("self_text"),
        Err(ProjectDuplicateError::InvalidArgument(message))
            if message == "复制自由视频项目只能接收 source_mode=freeform 的项目"
    ));
}
