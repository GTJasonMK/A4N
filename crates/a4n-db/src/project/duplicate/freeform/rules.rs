use crate::project::{SOURCE_MODE_FREEFORM, duplicate::ProjectDuplicateError};

pub(in crate::project::duplicate::freeform) fn ensure_freeform_source_mode(
    source_mode: &str,
) -> Result<(), ProjectDuplicateError> {
    if source_mode != SOURCE_MODE_FREEFORM {
        return Err(ProjectDuplicateError::InvalidArgument(
            "复制自由视频项目只能接收 source_mode=freeform 的项目".to_string(),
        ));
    }
    Ok(())
}
