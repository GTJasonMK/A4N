use a4n_export_core::{
    ExportRenderJsonInput, ExportRenderJsonOutput, ExportRenderZipInput, ExportRenderZipOutput,
};
use a4n_import_core::{
    ImportNormalizeJsonInput, ImportNormalizeJsonOutput, ImportReadMediaPackageZipInput,
    ImportReadMediaPackageZipOutput,
};
use a4n_service_protocol::{ServiceRequest, ServiceResponse};
use async_trait::async_trait;

use super::{CapabilityClient, CapabilityResult};

#[async_trait]
pub trait ImportClient: CapabilityClient {
    async fn normalize_json(
        &self,
        request: ServiceRequest<ImportNormalizeJsonInput>,
    ) -> CapabilityResult<ServiceResponse<ImportNormalizeJsonOutput>>;
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
    ) -> CapabilityResult<ServiceResponse<ExportRenderJsonOutput>>;
    async fn render_zip(
        &self,
        request: ServiceRequest<ExportRenderZipInput>,
    ) -> CapabilityResult<ServiceResponse<ExportRenderZipOutput>>;
}
