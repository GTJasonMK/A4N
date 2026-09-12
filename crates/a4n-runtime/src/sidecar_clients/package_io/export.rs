use a4n_export_client::ExportClient;
use a4n_export_contract::{
    ExportRenderJsonInput, ExportRenderZipInput, ExportRenderZipOutput, ExportZipPackageInput,
};
use serde::Serialize;
use std::sync::Arc;

use crate::state::RuntimeProfile;

use super::super::{
    pure_transform_typed_service_request, service_error_string, typed_response_payload,
};

pub(crate) async fn render_export_json(
    profile: RuntimeProfile,
    client: Option<Arc<dyn ExportClient>>,
    input: ExportRenderJsonInput,
    project_id: Option<i64>,
    root_project_id: Option<i64>,
) -> Result<String, String> {
    if profile.uses_sidecars() {
        let client = client
            .ok_or_else(|| "export sidecar capability 未配置，无法执行导出渲染".to_string())?;
        let response = client
            .render_json(pure_transform_typed_service_request(
                a4n_service_protocol::EXPORT_RENDER_CAPABILITY_VERSION,
                input,
                project_id,
                root_project_id,
            )?)
            .await
            .map_err(service_error_string)?;
        return typed_response_payload(response, "export");
    }

    if input.pretty {
        serde_json::to_string_pretty(&input.payload)
    } else {
        serde_json::to_string(&input.payload)
    }
    .map_err(|err| err.to_string())
}

pub(crate) async fn render_export_json_manifest_bytes<T>(
    profile: RuntimeProfile,
    client: Option<Arc<dyn ExportClient>>,
    payload: &T,
    label: &str,
    project_id: Option<i64>,
    root_project_id: Option<i64>,
) -> Result<Vec<u8>, String>
where
    T: Serialize,
{
    if profile.uses_sidecars() {
        let payload =
            serde_json::to_value(payload).map_err(|err| format!("{label}序列化失败：{err}"))?;
        let rendered = render_export_json(
            profile,
            client,
            ExportRenderJsonInput {
                payload,
                pretty: true,
            },
            project_id,
            root_project_id,
        )
        .await?;
        return Ok(rendered.into_bytes());
    }

    serde_json::to_vec_pretty(payload).map_err(|err| format!("{label}序列化失败：{err}"))
}

pub(crate) async fn render_export_zip_package(
    profile: RuntimeProfile,
    client: Option<Arc<dyn ExportClient>>,
    input: ExportZipPackageInput,
    project_id: Option<i64>,
    root_project_id: Option<i64>,
) -> Result<Vec<u8>, String> {
    if profile.uses_sidecars() {
        let client = client
            .ok_or_else(|| "export sidecar capability 未配置，无法执行 zip 渲染".to_string())?;
        let response = client
            .render_zip(pure_transform_typed_service_request(
                a4n_service_protocol::EXPORT_RENDER_CAPABILITY_VERSION,
                ExportRenderZipInput::from_package_input(input),
                project_id,
                root_project_id,
            )?)
            .await
            .map_err(service_error_string)?;
        let output: ExportRenderZipOutput = typed_response_payload(response, "export")?;
        if output.content_type != "application/zip" {
            return Err(format!(
                "export sidecar 返回非法 content_type: {}",
                output.content_type
            ));
        }
        return output.into_zip_bytes().map_err(|err| err.to_string());
    }

    let entries = std::iter::once(a4n_archive_core::ArchiveEntry {
        path: input.manifest_path,
        bytes: input.manifest_bytes,
        label: input.manifest_label,
    })
    .chain(
        input
            .entries
            .into_iter()
            .map(|entry| a4n_archive_core::ArchiveEntry {
                path: entry.path,
                bytes: entry.bytes,
                label: entry.label,
            }),
    );
    a4n_archive_core::write_zip(entries, input.package_label).map_err(|err| err.to_string())
}
