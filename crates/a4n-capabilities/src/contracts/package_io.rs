use a4n_export_core::{ExportRenderJsonInput, ExportRenderZipInput, ExportRenderZipOutput};
use a4n_import_core::{ImportReadMediaPackageZipInput, ImportReadMediaPackageZipOutput};
use a4n_service_protocol::{ServiceRequest, ServiceResponse};
use async_trait::async_trait;

use super::{CapabilityClient, CapabilityResult};

#[async_trait]
pub trait ImportClient: CapabilityClient {
    async fn read_media_package_zip(
        &self,
        request: ServiceRequest<ImportReadMediaPackageZipInput>,
    ) -> CapabilityResult<ServiceResponse<ImportReadMediaPackageZipOutput>>;
}

#[async_trait]
pub trait ExportClient: CapabilityClient {
    async fn render_json(
        &self,
        request: ServiceRequest<ExportRenderJsonInput>,
    ) -> CapabilityResult<ServiceResponse<String>>;
    async fn render_zip(
        &self,
        request: ServiceRequest<ExportRenderZipInput>,
    ) -> CapabilityResult<ServiceResponse<ExportRenderZipOutput>>;
}
