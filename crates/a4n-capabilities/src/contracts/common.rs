use a4n_service_protocol::{ServiceError, ServiceHealth, ServiceRequest, ServiceResponse};
use async_trait::async_trait;

pub type CapabilityResult<T> = Result<T, ServiceError>;
pub type JsonRequest = ServiceRequest<serde_json::Value>;
pub type JsonResponse = ServiceResponse<serde_json::Value>;

#[async_trait]
pub trait CapabilityClient: Send + Sync {
    async fn health(&self) -> CapabilityResult<ServiceHealth>;
}
