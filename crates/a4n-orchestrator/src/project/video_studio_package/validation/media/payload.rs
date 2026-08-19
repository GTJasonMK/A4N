use super::owner::{MediaEntryOwner, resolve_media_entry_owner};
use crate::project::sha256_bytes;
use crate::project::video_studio_package::{VideoStudioMediaFileEntry, VideoStudioProjectPackage};
use a4n_core::models::media::media_asset::FILE_PATH_FIELD;

pub(crate) fn validate_media_entry_bytes(
    package: &VideoStudioProjectPackage,
    entry: &VideoStudioMediaFileEntry,
    bytes: &[u8],
) -> Result<(), String> {
    match resolve_media_entry_owner(package, entry)? {
        MediaEntryOwner::GeneratedImage(image) => validate_payload_size(
            image.file_size,
            bytes,
            &entry.owner_kind,
            entry.owner_id,
            &entry.field,
        ),
        MediaEntryOwner::GeneratedVideo(video) if entry.field == FILE_PATH_FIELD => {
            validate_payload_size(
                video.file_size,
                bytes,
                &entry.owner_kind,
                entry.owner_id,
                &entry.field,
            )
        }
        MediaEntryOwner::MediaAsset(asset) if entry.field == FILE_PATH_FIELD => {
            validate_payload_size(
                asset.file_size,
                bytes,
                &entry.owner_kind,
                entry.owner_id,
                &entry.field,
            )?;
            if let Some(expected_checksum) = asset.checksum.as_deref() {
                let actual_checksum = sha256_bytes(bytes);
                if actual_checksum != expected_checksum {
                    return Err(format!(
                        "媒体文件 checksum 与清单不一致: owner_kind={}, owner_id={}, field={}, expected={}, actual={}",
                        entry.owner_kind,
                        entry.owner_id,
                        entry.field,
                        expected_checksum,
                        actual_checksum
                    ));
                }
            }
            Ok(())
        }
        MediaEntryOwner::GeneratedVideo(_) | MediaEntryOwner::MediaAsset(_) => Ok(()),
    }
}

fn validate_payload_size(
    expected_size: Option<i64>,
    bytes: &[u8],
    owner_kind: &str,
    owner_id: i64,
    field: &str,
) -> Result<(), String> {
    let Some(expected_size) = expected_size else {
        return Ok(());
    };
    if expected_size != bytes.len() as i64 {
        return Err(format!(
            "媒体文件大小与清单不一致: owner_kind={owner_kind}, owner_id={owner_id}, field={field}, expected={expected_size}, actual={}",
            bytes.len()
        ));
    }
    Ok(())
}
