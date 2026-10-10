use a4n_api_contract::theme::{
    DeleteThemeResourceInput, MAX_THEME_RESOURCE_BYTES, PersistThemeResourceInput,
    ThemeResourceRecord, ThemeResourceSummary, ThemeResourceUploadInput,
};
use a4n_error::AppError;
use base64::Engine as _;
use sha2::{Digest, Sha256};

use crate::AppState;
use a4n_storage::{remove_data_file_in_data_dir, write_new_data_file};

pub async fn upload_theme_resource_for_user(
    state: &AppState,
    user_id: i64,
    input: ThemeResourceUploadInput,
) -> Result<ThemeResourceSummary, AppError> {
    let theme_id = input.theme_id;
    let previous_file_path = state
        .theme_read_application()
        .list_resources(user_id, input.theme_id)
        .await?
        .into_iter()
        .find(|resource| resource.resource_key == input.resource_key)
        .map(|resource| resource.file_path);
    let (prepared, created_file) = prepare_theme_resource_file(state, input)?;
    let relative_path = prepared.file_path.clone();
    let resource = match state
        .theme_mutation_application()
        .upsert_resource(user_id, prepared)
        .await
    {
        Ok(resource) => resource,
        Err(error) => {
            if created_file {
                remove_theme_resource_file_if_unused(state, user_id, theme_id, &relative_path)
                    .await
                    .map_err(|cleanup| {
                        AppError::Internal(anyhow::anyhow!(
                            "{error}; 清理未保存主题资源失败: {cleanup}"
                        ))
                    })?;
            }
            return Err(error);
        }
    };
    if let Some(previous_file_path) =
        previous_file_path.filter(|file_path| file_path != &relative_path)
        && let Err(error) =
            remove_theme_resource_file_if_unused(state, user_id, theme_id, &previous_file_path)
                .await
    {
        tracing::warn!(theme_id, file_path = previous_file_path, %error, "清理被替换的主题资源文件失败");
    }
    Ok(resource_to_summary(resource))
}

pub(super) fn prepare_theme_resource_file(
    state: &AppState,
    input: ThemeResourceUploadInput,
) -> Result<(PersistThemeResourceInput, bool), AppError> {
    let extension =
        validate_resource_metadata(&input.resource_key, &input.file_name, &input.mime_type)?;
    if input.data_base64.trim().is_empty() {
        return Err(AppError::InvalidArgument(
            "主题图片内容不能为空".to_string(),
        ));
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(input.data_base64.trim())
        .map_err(|error| AppError::InvalidArgument(format!("主题图片 base64 无效：{error}")))?;
    if bytes.is_empty() || bytes.len() > MAX_THEME_RESOURCE_BYTES {
        return Err(AppError::InvalidArgument(format!(
            "主题图片大小必须在 1 到 {} MiB 之间",
            MAX_THEME_RESOURCE_BYTES / (1024 * 1024)
        )));
    }
    let digest = format!("{:x}", Sha256::digest(&bytes));
    let relative_path =
        theme_resource_relative_path(input.theme_id, &input.resource_key, &digest, extension);
    let target = state.data_dir().join(&relative_path);
    let created_file = !target.exists();
    if created_file {
        write_new_data_file(state.data_dir(), &relative_path, &bytes, "主题背景图片")
            .map_err(AppError::InvalidArgument)?;
    }
    Ok((
        PersistThemeResourceInput {
            theme_id: input.theme_id,
            resource_key: input.resource_key,
            file_name: input.file_name,
            mime_type: input.mime_type,
            file_path: relative_path,
            sha256: digest,
            byte_size: bytes.len() as i64,
        },
        created_file,
    ))
}

pub async fn delete_theme_resource_for_user(
    state: &AppState,
    user_id: i64,
    input: DeleteThemeResourceInput,
) -> Result<(), AppError> {
    let resource = state
        .theme_mutation_application()
        .delete_resource(user_id, input.resource_id)
        .await?;
    remove_theme_resource_file_if_unused(state, user_id, resource.theme_id, &resource.file_path)
        .await
}

pub(super) async fn remove_theme_resource_file_if_unused(
    state: &AppState,
    user_id: i64,
    theme_id: i64,
    file_path: &str,
) -> Result<(), AppError> {
    let still_referenced = state
        .theme_read_application()
        .list_resources(user_id, theme_id)
        .await?
        .into_iter()
        .any(|resource| resource.file_path == file_path);
    if still_referenced {
        return Ok(());
    }
    remove_data_file_in_data_dir(state.data_dir(), file_path, "主题背景图片")
        .map_err(AppError::InvalidArgument)
}

fn resource_to_summary(resource: ThemeResourceRecord) -> ThemeResourceSummary {
    ThemeResourceSummary {
        id: resource.id,
        theme_id: resource.theme_id,
        resource_key: resource.resource_key,
        file_name: resource.file_name,
        mime_type: resource.mime_type,
        file_path: resource.file_path,
        byte_size: resource.byte_size,
    }
}

pub(super) fn validate_resource_metadata(
    resource_key: &str,
    file_name: &str,
    mime_type: &str,
) -> Result<&'static str, AppError> {
    if resource_key.trim() != resource_key
        || !a4n_domain::THEME_ROUTE_FAMILIES.contains(&resource_key)
    {
        return Err(AppError::InvalidArgument("主题资源路由族无效".to_string()));
    }
    if file_name.trim().is_empty()
        || file_name.trim() != file_name
        || file_name.len() > 160
        || file_name.contains('/')
        || file_name.contains('\\')
        || file_name.chars().any(char::is_control)
    {
        return Err(AppError::InvalidArgument("主题图片文件名无效".to_string()));
    }
    a4n_domain::media::image_upload_format_from_mime(mime_type)
        .filter(|format| format.mime_type == mime_type)
        .map(|format| format.extension)
        .or_else(|| (mime_type == "image/gif").then_some("gif"))
        .ok_or_else(|| {
            AppError::InvalidArgument("主题背景只支持 PNG、JPEG、WebP 或 GIF".to_string())
        })
}

pub(super) fn theme_resource_relative_path(
    theme_id: i64,
    resource_key: &str,
    digest: &str,
    extension: &str,
) -> String {
    format!("themes/{theme_id}/{resource_key}-{digest}.{extension}")
}

#[cfg(test)]
mod tests {
    use super::{
        ThemeResourceUploadInput, theme_resource_relative_path, validate_resource_metadata,
    };

    #[test]
    fn resource_metadata_rejects_paths_and_external_mime() {
        let mut input = ThemeResourceUploadInput {
            theme_id: 1,
            resource_key: "home".to_string(),
            file_name: "../x.png".to_string(),
            mime_type: "image/png".to_string(),
            data_base64: "aGVsbG8=".to_string(),
        };
        assert!(
            validate_resource_metadata(&input.resource_key, &input.file_name, &input.mime_type,)
                .is_err()
        );
        input.file_name = "x.png".to_string();
        input.mime_type = "text/html".to_string();
        assert!(
            validate_resource_metadata(&input.resource_key, &input.file_name, &input.mime_type,)
                .is_err()
        );
    }

    #[test]
    fn resource_paths_keep_route_assets_separate() {
        let digest = "a".repeat(64);
        assert_ne!(
            theme_resource_relative_path(1, "home", &digest, "png"),
            theme_resource_relative_path(1, "settings", &digest, "png")
        );
    }
}
