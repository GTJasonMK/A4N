use crate::galgame::GalgameMediaRecordError;

pub(in crate::galgame) fn validate_galgame_media_target_ids(
    project_id: i64,
    target_id: i64,
) -> Result<(), GalgameMediaRecordError> {
    if project_id <= 0 || target_id <= 0 {
        return Err(GalgameMediaRecordError::InvalidArgument(
            "project_id 和 target_id 必须是正整数".to_string(),
        ));
    }
    Ok(())
}

pub(in crate::galgame) fn validate_galgame_media_asset_id(
    media_asset_id: Option<i64>,
) -> Result<(), GalgameMediaRecordError> {
    if media_asset_id.is_some_and(|media_asset_id| media_asset_id <= 0) {
        return Err(GalgameMediaRecordError::InvalidArgument(
            "media_asset_id 必须是正整数".to_string(),
        ));
    }
    Ok(())
}
