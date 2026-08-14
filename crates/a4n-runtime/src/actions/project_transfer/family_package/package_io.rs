use a4n_orchestrator::project::ProjectFamilyPackage;

use std::path::Path;

use super::MANIFEST_PATH;
use crate::actions::project_transfer::package_files::project_package_media_zip_entries;

pub(in crate::actions::project_transfer) async fn package_zip_bytes(
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
    let entries = project_package_media_zip_entries(
        data_dir,
        package
            .media_files
            .iter()
            .map(|media| (media.original_path.as_str(), media.package_path.as_str())),
        "媒体文件",
    )?;
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
