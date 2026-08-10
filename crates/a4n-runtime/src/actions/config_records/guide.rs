use std::{collections::HashSet, io::ErrorKind, path::Path};

use crate::{
    AppState,
    actions::admin_records::{AdminPermission, require_admin_permission},
    file_utils::{canonicalize_data_file_path, remove_data_file_in_data_dir, write_new_data_file},
};
use a4n_api_contract::config::{
    USAGE_GUIDE_IMAGE_MAX_BYTES, USAGE_GUIDE_IMAGE_MAX_PIXELS, USAGE_GUIDE_IMAGE_PATH_PREFIX,
    USAGE_GUIDE_NEW_IMAGES_MAX_BYTES,
};
use a4n_core::error::AppError;
use a4n_orchestrator::{
    config::validate_usage_guide_update_document,
    media::{DecodedUploadedImage, decode_uploaded_image_data},
};

pub use a4n_api_contract::config::UsageGuideDocument;

pub async fn get_usage_guide(state: &AppState) -> Result<UsageGuideDocument, AppError> {
    let _update_guard = state.usage_guide_update_lock().lock().await;
    let document = state.settings().usage_guide_document().await?;
    validate_saved_usage_guide_images(state, &document).map_err(AppError::ConfigError)?;
    Ok(document)
}

pub async fn update_usage_guide(
    state: &AppState,
    caller_user_id: i64,
    mut document: UsageGuideDocument,
) -> Result<UsageGuideDocument, AppError> {
    require_admin_permission(state, caller_user_id, AdminPermission::ManageSettings).await?;
    validate_usage_guide_update_document(&document).map_err(AppError::InvalidArgument)?;
    let _update_guard = state.usage_guide_update_lock().lock().await;
    validate_saved_usage_guide_images(state, &document).map_err(AppError::InvalidArgument)?;

    let mut uploads: Vec<(usize, DecodedUploadedImage)> = Vec::new();
    let mut upload_bytes = 0usize;
    for (index, block) in document.blocks.iter().enumerate() {
        let Some(data_base64) = block.data_base64.as_deref() else {
            continue;
        };
        let file_name = block
            .file_name
            .as_deref()
            .ok_or_else(|| AppError::InvalidArgument("使用指南图片缺少文件名".to_string()))?;
        let mime_type = block
            .mime_type
            .as_deref()
            .ok_or_else(|| AppError::InvalidArgument("使用指南图片缺少 MIME".to_string()))?;
        let decoded = decode_uploaded_image_data(
            file_name,
            mime_type,
            data_base64,
            USAGE_GUIDE_IMAGE_MAX_BYTES,
            USAGE_GUIDE_IMAGE_MAX_PIXELS,
        )
        .map_err(AppError::InvalidArgument)?;
        upload_bytes = upload_bytes
            .checked_add(decoded.bytes.len())
            .ok_or_else(|| AppError::InvalidArgument("使用指南图片总大小溢出".to_string()))?;
        if upload_bytes > USAGE_GUIDE_NEW_IMAGES_MAX_BYTES {
            return Err(AppError::InvalidArgument(format!(
                "本次新增图片总大小不能超过 {}MB",
                USAGE_GUIDE_NEW_IMAGES_MAX_BYTES / 1024 / 1024
            )));
        }
        uploads.push((index, decoded));
    }

    let mut created_paths = Vec::with_capacity(uploads.len());
    for (index, decoded) in uploads {
        let path = format!(
            "{USAGE_GUIDE_IMAGE_PATH_PREFIX}{}.{}",
            uuid::Uuid::new_v4().simple(),
            decoded.extension
        );
        if let Err(error) =
            write_new_data_file(state.data_dir(), &path, &decoded.bytes, "使用指南图片")
        {
            return Err(rollback_created_images(
                state,
                &created_paths,
                AppError::ConfigError(error),
            ));
        }
        created_paths.push(path.clone());
        let block = &mut document.blocks[index];
        block.image_path = Some(path);
        block.file_name = Some(decoded.source_file_name);
        block.mime_type = Some(decoded.mime_type.to_string());
        block.data_base64 = None;
    }

    if let Err(error) = state
        .settings()
        .save_usage_guide_document_validated(&document)
        .await
    {
        return Err(rollback_created_images(state, &created_paths, error));
    }

    let retained_paths = document
        .blocks
        .iter()
        .filter_map(|block| block.image_path.as_deref())
        .map(str::to_string)
        .collect::<HashSet<_>>();
    if let Err(error) = cleanup_unretained_usage_guide_images(state.data_dir(), &retained_paths) {
        tracing::warn!(%error, "清理未引用的使用指南图片失败");
    }

    Ok(document)
}

fn validate_saved_usage_guide_images(
    state: &AppState,
    document: &UsageGuideDocument,
) -> Result<(), String> {
    for path in document
        .blocks
        .iter()
        .filter_map(|block| block.image_path.as_deref())
    {
        canonicalize_data_file_path(state.data_dir(), path, "使用指南图片")?;
    }
    Ok(())
}

fn cleanup_unretained_usage_guide_images(
    data_dir: &Path,
    retained_paths: &HashSet<String>,
) -> Result<(), String> {
    let image_dir = data_dir.join(USAGE_GUIDE_IMAGE_PATH_PREFIX);
    let metadata = match std::fs::symlink_metadata(&image_dir) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(format!("无法读取使用指南图片目录：{error}")),
    };
    if metadata.file_type().is_symlink() {
        return Err("使用指南图片目录不能是符号链接".to_string());
    }
    if !metadata.is_dir() {
        return Err("使用指南图片目录不是目录".to_string());
    }

    let mut failures = Vec::new();
    for entry in std::fs::read_dir(&image_dir)
        .map_err(|error| format!("读取使用指南图片目录失败：{error}"))?
    {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                failures.push(format!("读取目录项失败：{error}"));
                continue;
            }
        };
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(error) => {
                failures.push(format!("读取图片类型失败：{error}"));
                continue;
            }
        };
        if file_type.is_symlink() {
            failures.push(format!("拒绝符号链接：{}", entry.path().display()));
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        let Some(file_name) = entry.file_name().to_str().map(str::to_string) else {
            failures.push(format!("图片文件名不是 UTF-8：{}", entry.path().display()));
            continue;
        };
        let relative_path = format!("{USAGE_GUIDE_IMAGE_PATH_PREFIX}{file_name}");
        if retained_paths.contains(&relative_path) {
            continue;
        }
        if let Err(error) =
            remove_data_file_in_data_dir(data_dir, &relative_path, "未引用的使用指南图片")
        {
            failures.push(error);
        }
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("；"))
    }
}

fn rollback_created_images(
    state: &AppState,
    paths: &[String],
    original_error: AppError,
) -> AppError {
    let failures = paths
        .iter()
        .filter_map(|path| {
            remove_data_file_in_data_dir(state.data_dir(), path, "回滚使用指南图片")
                .err()
                .map(|error| format!("{path}: {error}"))
        })
        .collect::<Vec<_>>();
    if failures.is_empty() {
        original_error
    } else {
        AppError::ConfigError(format!(
            "{original_error}；回滚新图片失败：{}",
            failures.join("；")
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::cleanup_unretained_usage_guide_images;

    #[test]
    fn cleanup_keeps_retained_images_and_removes_orphans() {
        let data_dir = tempfile::tempdir().unwrap();
        let image_dir = data_dir.path().join("usage-guide/images");
        std::fs::create_dir_all(&image_dir).unwrap();
        std::fs::write(image_dir.join("keep.png"), b"keep").unwrap();
        std::fs::write(image_dir.join("orphan.png"), b"orphan").unwrap();

        cleanup_unretained_usage_guide_images(
            data_dir.path(),
            &HashSet::from(["usage-guide/images/keep.png".to_string()]),
        )
        .unwrap();

        assert!(image_dir.join("keep.png").is_file());
        assert!(!image_dir.join("orphan.png").exists());
    }
}
