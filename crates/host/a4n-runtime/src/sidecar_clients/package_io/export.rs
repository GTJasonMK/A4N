use a4n_export_client::ExportClient;
use a4n_export_contract::{
    EXPORT_RENDER_CAPABILITY_VERSION, ExportDocumentRenderer, ExportRenderDocumentInput,
    ExportRenderDocumentOutput, ExportRenderFailure, ExportRenderObserver,
};
use async_trait::async_trait;
use std::sync::Arc;

use crate::state::RuntimeProfile;

use super::super::{
    pure_transform_typed_service_request, service_error_string, typed_response_payload,
};

pub(crate) fn document_export_renderer(
    profile: RuntimeProfile,
    client: Option<Arc<dyn ExportClient>>,
) -> Result<Arc<dyn ExportDocumentRenderer>, String> {
    if !profile.uses_sidecars() {
        #[cfg(feature = "embedded-transforms")]
        return Ok(Arc::new(a4n_export_core::LocalDocumentExportRenderer));
        #[cfg(not(feature = "embedded-transforms"))]
        return Err("本地文档导出需要 embedded-transforms 构建功能".into());
    }
    let client = client
        .ok_or_else(|| "export sidecar capability 未配置，无法启用 sidecar profile".to_string())?;
    Ok(Arc::new(SidecarDocumentExportRenderer { client }))
}

struct SidecarDocumentExportRenderer {
    client: Arc<dyn ExportClient>,
}

#[async_trait]
impl ExportDocumentRenderer for SidecarDocumentExportRenderer {
    async fn render_document(
        &self,
        input: ExportRenderDocumentInput,
        project_id: Option<i64>,
        root_project_id: Option<i64>,
        _progress: Option<&dyn ExportRenderObserver>,
    ) -> Result<ExportRenderDocumentOutput, ExportRenderFailure> {
        let response = self
            .client
            .render_document(
                pure_transform_typed_service_request(
                    EXPORT_RENDER_CAPABILITY_VERSION,
                    input,
                    project_id,
                    root_project_id,
                )
                .map_err(ExportRenderFailure::Failed)?,
            )
            .await
            .map_err(|error| ExportRenderFailure::Failed(service_error_string(error)))?;
        typed_response_payload(response, "export").map_err(ExportRenderFailure::Failed)
    }
}
