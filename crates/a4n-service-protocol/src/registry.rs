use crate::health::{HealthStatus, ServiceHealth, ServiceVersion};
use crate::validation::RequiredSecretPolicy;

pub const PROTOCOL_VERSION: u32 = 2;
pub const PROMPT_RENDER_CAPABILITY_VERSION: &str = "prompt-render-v2";
pub const LLM_CHAT_CAPABILITY_VERSION: &str = "llm-chat-v2";
pub const EMBEDDING_CAPABILITY_VERSION: &str = "embedding-v2";
pub const RAG_INDEX_CAPABILITY_VERSION: &str = "rag-index-v2";
pub const IMAGE_PROVIDER_CAPABILITY_VERSION: &str = "image-provider-v2";
pub const VIDEO_PROVIDER_CAPABILITY_VERSION: &str = "video-provider-v2";
pub const IMPORT_TRANSFORM_CAPABILITY_VERSION: &str = "import-transform-v2";
pub const EXPORT_RENDER_CAPABILITY_VERSION: &str = "export-render-v2";

pub const PROMPT_SERVICE_NAME: &str = "prompt";
pub const LLM_SERVICE_NAME: &str = "llm";
pub const EMBEDDING_SERVICE_NAME: &str = "embedding";
pub const RAG_SERVICE_NAME: &str = "rag";
pub const IMAGE_SERVICE_NAME: &str = "image";
pub const VIDEO_SERVICE_NAME: &str = "video";
pub const IMPORT_SERVICE_NAME: &str = "import";
pub const EXPORT_SERVICE_NAME: &str = "export";

pub fn expected_capability_version_for_service(service: &str) -> Option<&'static str> {
    match service {
        PROMPT_SERVICE_NAME => Some(PROMPT_RENDER_CAPABILITY_VERSION),
        LLM_SERVICE_NAME => Some(LLM_CHAT_CAPABILITY_VERSION),
        EMBEDDING_SERVICE_NAME => Some(EMBEDDING_CAPABILITY_VERSION),
        RAG_SERVICE_NAME => Some(RAG_INDEX_CAPABILITY_VERSION),
        IMAGE_SERVICE_NAME => Some(IMAGE_PROVIDER_CAPABILITY_VERSION),
        VIDEO_SERVICE_NAME => Some(VIDEO_PROVIDER_CAPABILITY_VERSION),
        IMPORT_SERVICE_NAME => Some(IMPORT_TRANSFORM_CAPABILITY_VERSION),
        EXPORT_SERVICE_NAME => Some(EXPORT_RENDER_CAPABILITY_VERSION),
        _ => None,
    }
}

pub fn known_service_health(
    service: &'static str,
    service_version: impl Into<String>,
    status: HealthStatus,
) -> ServiceHealth {
    let capability_version = expected_capability_version_for_service(service)
        .expect("known sidecar service must have protocol capability version");
    ServiceHealth {
        service: service.to_string(),
        status,
        version: ServiceVersion {
            service_version: service_version.into(),
            protocol_version: PROTOCOL_VERSION,
            capability_version: capability_version.to_string(),
        },
    }
}

pub fn expected_capability_label_for_service(service: &str) -> Option<&'static str> {
    match service {
        PROMPT_SERVICE_NAME => Some("prompt capability"),
        LLM_SERVICE_NAME => Some("LLM capability"),
        EMBEDDING_SERVICE_NAME => Some("embedding capability"),
        RAG_SERVICE_NAME => Some("rag capability"),
        IMAGE_SERVICE_NAME => Some("image capability"),
        VIDEO_SERVICE_NAME => Some("video capability"),
        IMPORT_SERVICE_NAME => Some("import capability"),
        EXPORT_SERVICE_NAME => Some("export capability"),
        _ => None,
    }
}

pub fn required_secret_policy_for_service(service: &str) -> Option<RequiredSecretPolicy> {
    match service {
        PROMPT_SERVICE_NAME => Some(RequiredSecretPolicy::DenyAll {
            rejection_message: "prompt render sidecar 不接收 secret forwarding policy",
        }),
        LLM_SERVICE_NAME => Some(RequiredSecretPolicy::AllowAny),
        EMBEDDING_SERVICE_NAME => Some(RequiredSecretPolicy::DenyAll {
            rejection_message: "embedding sidecar 不接收 secret forwarding policy",
        }),
        RAG_SERVICE_NAME => Some(RequiredSecretPolicy::DenyAll {
            rejection_message: "rag index sidecar 不接收 secret forwarding policy",
        }),
        IMAGE_SERVICE_NAME => Some(RequiredSecretPolicy::AllowAny),
        VIDEO_SERVICE_NAME => Some(RequiredSecretPolicy::AllowAny),
        IMPORT_SERVICE_NAME => Some(RequiredSecretPolicy::DenyAll {
            rejection_message: "import transform sidecar 不接收 secret forwarding policy",
        }),
        EXPORT_SERVICE_NAME => Some(RequiredSecretPolicy::DenyAll {
            rejection_message: "export render sidecar 不接收 secret forwarding policy",
        }),
        _ => None,
    }
}
