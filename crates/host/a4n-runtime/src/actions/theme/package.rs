use std::{
    collections::HashSet,
    io::{Cursor, Read},
};

use a4n_api_contract::theme::{
    CreateThemeInput, ThemePackageExport, ThemePackageUploadInput, ThemeResourceUploadInput,
};
use a4n_archive_core::{ArchiveEntry, write_zip};
use a4n_error::AppError;
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zip::ZipArchive;

use super::resources::{
    MAX_THEME_RESOURCE_BYTES, theme_resource_relative_path, upload_theme_resource_for_user,
    validate_resource_metadata,
};
use crate::AppState;
use a4n_storage::{canonicalize_data_file_path, remove_data_file_in_data_dir};

const PACKAGE_VERSION: u16 = 1;
const MAX_PACKAGE_BYTES: usize = 24 * 1024 * 1024;
const MAX_UNCOMPRESSED_BYTES: u64 = MAX_PACKAGE_BYTES as u64;
const MAX_RESOURCE_COUNT: usize = a4n_domain::THEME_ROUTE_FAMILIES.len();

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ThemePackageManifest {
    version: u16,
    name: String,
    mode: a4n_domain::ThemeMode,
    document: a4n_domain::ThemeDocument,
    resources: Vec<ThemePackageResource>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ThemePackageResource {
    resource_key: String,
    file_name: String,
    mime_type: String,
    path: String,
    sha256: String,
}

pub async fn export_theme_package_for_user(
    state: &AppState,
    user_id: i64,
    theme_id: i64,
) -> Result<ThemePackageExport, AppError> {
    let theme = state
        .theme_read_application()
        .export_theme(user_id, theme_id)
        .await?;
    let document = theme.document.clone();
    a4n_domain::validate_theme_document(&document).map_err(AppError::InvalidArgument)?;

    let resources = state
        .theme_read_application()
        .list_resources(user_id, theme_id)
        .await?;
    let mut entries = Vec::with_capacity(resources.len());
    let mut manifest_resources = Vec::with_capacity(resources.len());
    for resource in resources {
        validate_resource_metadata(
            &resource.resource_key,
            &resource.file_name,
            &resource.mime_type,
        )
        .map_err(|error| AppError::ConfigError(format!("主题资源元数据无效：{error}")))?;
        let path =
            canonicalize_data_file_path(state.data_dir(), &resource.file_path, "主题包背景图片")
                .map_err(AppError::InvalidArgument)?;
        let bytes = std::fs::read(&path).map_err(|error| {
            AppError::Internal(anyhow::anyhow!(
                "读取主题资源失败：{}：{error}",
                path.display()
            ))
        })?;
        let digest = format!("{:x}", Sha256::digest(&bytes));
        if digest != resource.sha256 {
            return Err(AppError::ConfigError(format!(
                "主题资源校验失败：{}",
                resource.resource_key
            )));
        }
        let package_path = format!("assets/{}", resource.resource_key);
        manifest_resources.push(ThemePackageResource {
            resource_key: resource.resource_key.clone(),
            file_name: resource.file_name,
            mime_type: resource.mime_type,
            path: package_path.clone(),
            sha256: digest,
        });
        entries.push(ArchiveEntry {
            path: package_path,
            bytes,
            label: format!("主题资源 {}", resource.resource_key),
        });
    }
    let manifest = ThemePackageManifest {
        version: PACKAGE_VERSION,
        name: theme.name.clone(),
        mode: theme.mode,
        document,
        resources: manifest_resources,
    };
    let manifest_bytes = serde_json::to_vec_pretty(&manifest).map_err(AppError::Serialization)?;
    let bytes = write_zip(
        std::iter::once(ArchiveEntry {
            path: "theme.json".to_string(),
            bytes: manifest_bytes,
            label: "主题清单".to_string(),
        })
        .chain(entries),
        "主题包",
    )
    .map_err(|error| AppError::Internal(anyhow::anyhow!("生成主题包失败：{error}")))?;
    Ok(ThemePackageExport {
        file_name: format!("{}.a4n-theme.zip", safe_file_name(&theme.name)),
        mime_type: "application/zip".to_string(),
        data_base64: base64::engine::general_purpose::STANDARD.encode(bytes),
    })
}

pub async fn import_theme_package_for_user(
    state: &AppState,
    user_id: i64,
    input: ThemePackageUploadInput,
) -> Result<crate::actions::theme::ThemeSummary, AppError> {
    let bytes = decode_package(&input)?;
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|error| AppError::InvalidArgument(format!("主题包不是有效 ZIP：{error}")))?;
    let mut uncompressed_bytes = 0_u64;
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|error| AppError::InvalidArgument(format!("读取主题包目录失败：{error}")))?;
        uncompressed_bytes = uncompressed_bytes.saturating_add(entry.size());
        if uncompressed_bytes > MAX_UNCOMPRESSED_BYTES {
            return Err(AppError::InvalidArgument(
                "主题包解压后大小超出限制".to_string(),
            ));
        }
    }
    let mut manifest_bytes = Vec::new();
    {
        let mut entry = archive
            .by_name("theme.json")
            .map_err(|_| AppError::InvalidArgument("主题包缺少 theme.json".to_string()))?;
        entry
            .read_to_end(&mut manifest_bytes)
            .map_err(|error| AppError::InvalidArgument(format!("读取主题清单失败：{error}")))?;
    }
    let manifest: ThemePackageManifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|error| AppError::InvalidArgument(format!("主题清单无效：{error}")))?;
    if manifest.version != PACKAGE_VERSION || manifest.resources.len() > MAX_RESOURCE_COUNT {
        return Err(AppError::InvalidArgument(
            "主题包版本或资源数量不受支持".to_string(),
        ));
    }
    a4n_domain::validate_theme_document(&manifest.document).map_err(AppError::InvalidArgument)?;
    let resource_keys = validate_manifest_resources(&manifest)?;
    for (route_family, background) in &manifest.document.backgrounds {
        if let Some(resource_key) = &background.resource_key
            && !resource_keys.contains(resource_key)
        {
            return Err(AppError::InvalidArgument(format!(
                "主题背景 {route_family} 引用了缺失资源：{resource_key}"
            )));
        }
    }
    let theme = state
        .theme_mutation_application()
        .import_theme(
            user_id,
            CreateThemeInput {
                name: manifest.name,
                mode: manifest.mode,
                document: manifest.document,
            },
        )
        .await?;
    let mut created_files = Vec::new();
    let import_result = async {
        for resource in &manifest.resources {
            let resource_bytes = {
                let mut entry = archive
                    .by_name(&resource.path)
                    .map_err(|_| AppError::InvalidArgument("主题包资源缺失".to_string()))?;
                let declared = entry.size();
                if declared == 0 || declared > MAX_THEME_RESOURCE_BYTES as u64 {
                    return Err(AppError::InvalidArgument("主题包资源大小无效".to_string()));
                }
                let mut bytes = Vec::with_capacity(declared as usize);
                entry.read_to_end(&mut bytes).map_err(|error| {
                    AppError::InvalidArgument(format!("读取主题资源失败：{error}"))
                })?;
                bytes
            };
            if format!("{:x}", Sha256::digest(&resource_bytes)) != resource.sha256 {
                return Err(AppError::InvalidArgument("主题资源校验失败".to_string()));
            }
            let extension = validate_resource_metadata(
                &resource.resource_key,
                &resource.file_name,
                &resource.mime_type,
            )?;
            let relative_path = theme_resource_relative_path(
                theme.id,
                &resource.resource_key,
                &resource.sha256,
                extension,
            );
            let existed_before = state.data_dir().join(&relative_path).exists();
            let uploaded = upload_theme_resource_for_user(
                state,
                user_id,
                ThemeResourceUploadInput {
                    theme_id: theme.id,
                    resource_key: resource.resource_key.clone(),
                    file_name: resource.file_name.clone(),
                    mime_type: resource.mime_type.clone(),
                    data_base64: base64::engine::general_purpose::STANDARD.encode(resource_bytes),
                },
            )
            .await?;
            if !existed_before {
                created_files.push(uploaded.file_path);
            }
        }
        state
            .theme_read_application()
            .export_theme(user_id, theme.id)
            .await
    }
    .await;
    match import_result {
        Ok(theme) => Ok(theme),
        Err(error) => {
            cleanup_failed_import(state, user_id, theme.id, &created_files).await;
            Err(error)
        }
    }
}

fn validate_manifest_resources(
    manifest: &ThemePackageManifest,
) -> Result<HashSet<String>, AppError> {
    let mut resource_keys = HashSet::with_capacity(manifest.resources.len());
    let mut paths = HashSet::with_capacity(manifest.resources.len());
    for resource in &manifest.resources {
        validate_resource_metadata(
            &resource.resource_key,
            &resource.file_name,
            &resource.mime_type,
        )?;
        if !resource_keys.insert(resource.resource_key.clone()) {
            return Err(AppError::InvalidArgument(
                "主题包资源标识重复或无效".to_string(),
            ));
        }
        let Some(file_name) = resource.path.strip_prefix("assets/") else {
            return Err(AppError::InvalidArgument("主题包资源路径无效".to_string()));
        };
        if file_name.is_empty()
            || file_name.contains('/')
            || file_name.contains('\\')
            || file_name.contains("..")
            || !paths.insert(resource.path.clone())
        {
            return Err(AppError::InvalidArgument(
                "主题包资源路径重复或无效".to_string(),
            ));
        }
        if resource.sha256.len() != 64
            || !resource
                .sha256
                .chars()
                .all(|character| character.is_ascii_hexdigit())
        {
            return Err(AppError::InvalidArgument("主题资源校验值无效".to_string()));
        }
    }
    Ok(resource_keys)
}

async fn cleanup_failed_import(
    state: &AppState,
    user_id: i64,
    theme_id: i64,
    created_files: &[String],
) {
    for file_path in created_files {
        if let Err(error) =
            remove_data_file_in_data_dir(state.data_dir(), file_path, "主题导入临时资源")
        {
            tracing::error!(theme_id, file_path, %error, "清理失败的主题导入资源失败");
        }
    }
    if let Err(error) = state
        .theme_mutation_application()
        .delete_theme(user_id, theme_id)
        .await
    {
        tracing::error!(theme_id, %error, "清理失败的主题导入记录失败");
    }
}

fn decode_package(input: &ThemePackageUploadInput) -> Result<Vec<u8>, AppError> {
    if input.file_name.trim().is_empty()
        || input.file_name.contains('/')
        || input.file_name.contains('\\')
        || !input.file_name.to_ascii_lowercase().ends_with(".zip")
    {
        return Err(AppError::InvalidArgument("主题包文件名无效".to_string()));
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(input.data_base64.trim())
        .map_err(|error| AppError::InvalidArgument(format!("主题包 base64 无效：{error}")))?;
    if bytes.is_empty() || bytes.len() > MAX_PACKAGE_BYTES {
        return Err(AppError::InvalidArgument("主题包大小超出限制".to_string()));
    }
    Ok(bytes)
}

fn safe_file_name(value: &str) -> String {
    let sanitized: String = value
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_control() || matches!(ch, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
            {
                '_'
            } else {
                ch
            }
        })
        .collect();
    let sanitized = sanitized.trim_matches([' ', '.']);
    if sanitized.is_empty() {
        "theme".to_string()
    } else {
        sanitized.to_string()
    }
}
