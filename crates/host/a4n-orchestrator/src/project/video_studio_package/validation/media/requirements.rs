use crate::project::{ImportMediaPayload, media_file_key};

pub(in crate::project::video_studio_package::validation) fn validate_optional_file_size(
    file_size: Option<i64>,
    owner_kind: &str,
    owner_id: i64,
) -> Result<(), String> {
    if let Some(file_size) = file_size
        && file_size < 0
    {
        return Err(format!(
            "媒体记录 file_size 不能为负数: owner_kind={owner_kind}, owner_id={owner_id}, file_size={file_size}"
        ));
    }
    Ok(())
}

pub(in crate::project::video_studio_package::validation) fn require_media_payload(
    media_payload: &ImportMediaPayload,
    owner_kind: &str,
    owner_id: i64,
    field: &str,
) -> Result<(), String> {
    let key = media_file_key(owner_kind, owner_id, field);
    if !media_payload.entries.contains_key(&key) {
        return Err(format!(
            "视频画布项目包缺少媒体文件: owner_kind={owner_kind}, owner_id={owner_id}, field={field}"
        ));
    }
    Ok(())
}
