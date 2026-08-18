use std::time::Duration;

use crate::registry::{
    EMBEDDING_SERVICE_NAME, EXPORT_SERVICE_NAME, IMAGE_SERVICE_NAME, IMPORT_SERVICE_NAME,
    LLM_SERVICE_NAME, PROMPT_SERVICE_NAME, RAG_SERVICE_NAME, VIDEO_SERVICE_NAME,
};

/// 统一的 sidecar 资源合同。
///
/// HTTP router、capability client 和 process/health manager 都从这里取默认值，
/// 避免每个 crate 各自维护一份 limit/timeout/concurrency 数字。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SidecarResourcePolicy {
    pub json_body_limit_bytes: usize,
    pub media_body_limit_bytes: usize,
    pub streaming_body_limit_bytes: usize,
    pub max_concurrent_requests: usize,
    pub connect_timeout: Duration,
    pub request_timeout: Duration,
    pub stream_idle_timeout: Duration,
    pub health_timeout: Duration,
}

impl SidecarResourcePolicy {
    pub const fn body_limit(self, kind: BodyLimitKind) -> usize {
        match kind {
            BodyLimitKind::Json => self.json_body_limit_bytes,
            BodyLimitKind::Media => self.media_body_limit_bytes,
            BodyLimitKind::Streaming => self.streaming_body_limit_bytes,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyLimitKind {
    Json,
    Media,
    Streaming,
}

pub const SIDECAR_CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
pub const SIDECAR_REQUEST_TIMEOUT: Duration = Duration::from_secs(120);
pub const SIDECAR_STREAM_IDLE_TIMEOUT: Duration = Duration::from_secs(45);
pub const SIDECAR_HEALTH_TIMEOUT: Duration = Duration::from_secs(5);
pub const SIDECAR_EVENT_QUEUE_CAPACITY: usize = 256;

const fn policy(
    json_body_limit_bytes: usize,
    media_body_limit_bytes: usize,
    streaming_body_limit_bytes: usize,
    max_concurrent_requests: usize,
) -> SidecarResourcePolicy {
    SidecarResourcePolicy {
        json_body_limit_bytes,
        media_body_limit_bytes,
        streaming_body_limit_bytes,
        max_concurrent_requests,
        connect_timeout: SIDECAR_CONNECT_TIMEOUT,
        request_timeout: SIDECAR_REQUEST_TIMEOUT,
        stream_idle_timeout: SIDECAR_STREAM_IDLE_TIMEOUT,
        health_timeout: SIDECAR_HEALTH_TIMEOUT,
    }
}

pub fn sidecar_resource_policy(service: &str) -> Option<SidecarResourcePolicy> {
    match service {
        PROMPT_SERVICE_NAME | LLM_SERVICE_NAME => {
            Some(policy(8 * 1024 * 1024, 8 * 1024 * 1024, 8 * 1024 * 1024, 8))
        }
        EMBEDDING_SERVICE_NAME => Some(policy(
            16 * 1024 * 1024,
            16 * 1024 * 1024,
            16 * 1024 * 1024,
            2,
        )),
        RAG_SERVICE_NAME => Some(policy(
            16 * 1024 * 1024,
            16 * 1024 * 1024,
            16 * 1024 * 1024,
            4,
        )),
        IMAGE_SERVICE_NAME | VIDEO_SERVICE_NAME => Some(policy(
            8 * 1024 * 1024,
            64 * 1024 * 1024,
            8 * 1024 * 1024,
            2,
        )),
        IMPORT_SERVICE_NAME | EXPORT_SERVICE_NAME => Some(policy(
            8 * 1024 * 1024,
            64 * 1024 * 1024,
            8 * 1024 * 1024,
            4,
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{BodyLimitKind, sidecar_resource_policy};
    use crate::SIDECAR_REQUEST_TIMEOUT;

    #[test]
    fn every_registered_sidecar_has_one_resource_policy() {
        for service in [
            "prompt",
            "llm",
            "embedding",
            "rag",
            "image",
            "video",
            "import",
            "export",
        ] {
            let policy = sidecar_resource_policy(service).expect("registered policy");
            assert_eq!(policy.request_timeout, SIDECAR_REQUEST_TIMEOUT);
            assert!(policy.max_concurrent_requests > 0);
            assert!(policy.body_limit(BodyLimitKind::Json) > 0);
        }
    }

    #[test]
    fn media_routes_have_a_larger_explicit_limit_than_json_routes() {
        for service in ["image", "video", "import", "export"] {
            let policy = sidecar_resource_policy(service).expect("media service policy");
            assert!(
                policy.body_limit(BodyLimitKind::Media) >= policy.body_limit(BodyLimitKind::Json)
            );
        }
    }
}
