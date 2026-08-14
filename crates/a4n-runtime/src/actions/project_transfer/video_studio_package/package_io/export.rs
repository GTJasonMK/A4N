use std::path::Path;

use super::super::{MANIFEST_PATH, VideoStudioProjectPackage};
use crate::actions::project_transfer::package_files::project_package_media_zip_entries;

pub(in crate::actions::project_transfer) async fn package_zip_bytes(
    runtime_profile: crate::state::RuntimeProfile,
    capabilities: &a4n_capabilities::A4nCapabilities,
    data_dir: &Path,
    root_project_id: Option<i64>,
    package: &VideoStudioProjectPackage,
) -> Result<Vec<u8>, String> {
    let project_id = package.project.id;
    let manifest = crate::sidecar_clients::render_export_json_manifest_bytes(
        runtime_profile,
        capabilities,
        package,
        "视频画布项目导出包",
        Some(project_id),
        root_project_id,
    )
    .await?;
    let entries = project_package_media_zip_entries(
        data_dir,
        package
            .media_files
            .iter()
            .map(|media| (media.original_path.as_str(), media.package_path.as_str())),
        "视频画布项目媒体文件",
    )?;
    crate::sidecar_clients::render_export_zip_package(
        runtime_profile,
        capabilities,
        a4n_export_core::ExportZipPackageInput {
            manifest_path: MANIFEST_PATH.to_string(),
            manifest_bytes: manifest,
            manifest_label: "视频画布项目清单".to_string(),
            entries,
            package_label: "视频画布项目导出包".to_string(),
        },
        Some(project_id),
        root_project_id,
    )
    .await
}
