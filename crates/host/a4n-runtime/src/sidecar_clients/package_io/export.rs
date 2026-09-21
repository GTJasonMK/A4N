use a4n_export_client::ExportClient;
use a4n_export_contract::{
    EXPORT_RENDER_CAPABILITY_VERSION, ExportDocumentRenderer, ExportRenderDocumentInput,
    ExportRenderDocumentOutput,
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
) -> Result<Option<Arc<dyn ExportDocumentRenderer>>, String> {
    if !profile.uses_sidecars() {
        return Ok(None);
    }
    let client = client
        .ok_or_else(|| "export sidecar capability 未配置，无法启用 sidecar profile".to_string())?;
    Ok(Some(
        Arc::new(SidecarDocumentExportRenderer { client }) as Arc<dyn ExportDocumentRenderer>
    ))
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
    ) -> Result<ExportRenderDocumentOutput, String> {
        let response = self
            .client
            .render_document(pure_transform_typed_service_request(
                EXPORT_RENDER_CAPABILITY_VERSION,
                input,
                project_id,
                root_project_id,
            )?)
            .await
            .map_err(service_error_string)?;
        typed_response_payload(response, "export")
    }
}
