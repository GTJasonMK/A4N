use a4n_db_models::models::media::media_asset;

use crate::project::duplicate::ProjectDuplicateError;

pub(super) fn ensure_copyable_derived_media_asset(
    asset: &media_asset::Model,
) -> Result<(), ProjectDuplicateError> {
    if asset.status == "completed" && asset.file_path.is_none() {
        return Err(ProjectDuplicateError::InvalidState {
            current: format!("已完成媒体资产 {} 缺少 file_path", asset.id),
            operation: "复制派生项目媒体资产".to_string(),
        });
    }
    Ok(())
}
