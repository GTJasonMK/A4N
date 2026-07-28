use a4n_orchestrator::project::ProjectFamilyPackage;

use std::path::Path;

use crate::file_utils::canonicalize_data_file_path;

use super::MANIFEST_PATH;

pub(super) async fn package_zip_bytes(
    runtime_profile: crate::state::RuntimeProfile,
    capabilities: &a4n_capabilities::A4nCapabilities,
    data_dir: &Path,
    package: &ProjectFamilyPackage,
) -> Result<Vec<u8>, String> {
    let manifest = crate::sidecar_clients::render_export_json_manifest_bytes(
        runtime_profile,
        capabilities,
        package,
        "项目族导出包",
        Some(package.root_project_id),
        Some(package.root_project_id),
    )
    .await?;
    let entries = package_media_zip_entries(data_dir, package)?;
    crate::sidecar_clients::render_export_zip_package(
        runtime_profile,
        capabilities,
        a4n_export_core::ExportZipPackageInput {
            manifest_path: MANIFEST_PATH.to_string(),
            manifest_bytes: manifest,
            manifest_label: "项目族清单".to_string(),
            entries,
            package_label: "项目族导出包".to_string(),
        },
        Some(package.root_project_id),
        Some(package.root_project_id),
    )
    .await
}

fn package_media_zip_entries(
    data_dir: &Path,
    package: &ProjectFamilyPackage,
) -> Result<Vec<a4n_export_core::ExportZipEntryBytes>, String> {
    let mut entries = Vec::with_capacity(package.media_files.len());
    for media in &package.media_files {
        let source = canonicalize_data_file_path(data_dir, &media.original_path, "媒体文件")?;
        let bytes = std::fs::read(&source)
            .map_err(|err| format!("读取媒体文件失败: {}: {err}", source.display()))?;
        entries.push(a4n_export_core::ExportZipEntryBytes {
            path: media.package_path.clone(),
            bytes,
            label: format!("媒体文件 {}", media.package_path),
        });
    }
    Ok(entries)
}
