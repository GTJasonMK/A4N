const API_EMBEDDING_BACKEND: &str = "api";
const FASTEMBED_EMBEDDING_BACKEND: &str = "fastembed";
const LOCAL_HASH_EMBEDDING_BACKEND: &str = "local_hash";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingBackendKind {
    Api,
    FastEmbed,
    LocalHash,
}

impl EmbeddingBackendKind {
    pub const fn identity_backend_name(self) -> &'static str {
        match self {
            Self::Api => "Api",
            Self::FastEmbed => "FastEmbed",
            Self::LocalHash => "LocalHash",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmbeddingProviderCapabilities {
    pub backend: &'static str,
    pub display_name: &'static str,
    pub kind: EmbeddingBackendKind,
    pub requires_base_url: bool,
    pub accepts_api_key: bool,
}

pub const EMBEDDING_PROVIDER_CAPABILITIES: &[EmbeddingProviderCapabilities] = &[
    EmbeddingProviderCapabilities {
        backend: API_EMBEDDING_BACKEND,
        display_name: "OpenAI 兼容 API",
        kind: EmbeddingBackendKind::Api,
        requires_base_url: true,
        accepts_api_key: true,
    },
    EmbeddingProviderCapabilities {
        backend: FASTEMBED_EMBEDDING_BACKEND,
        display_name: "FastEmbed 本地模型",
        kind: EmbeddingBackendKind::FastEmbed,
        requires_base_url: false,
        accepts_api_key: false,
    },
    EmbeddingProviderCapabilities {
        backend: LOCAL_HASH_EMBEDDING_BACKEND,
        display_name: "本地哈希（开发）",
        kind: EmbeddingBackendKind::LocalHash,
        requires_base_url: false,
        accepts_api_key: false,
    },
];

pub fn embedding_provider_capabilities(value: &str) -> Option<EmbeddingProviderCapabilities> {
    let value = value.trim();
    EMBEDDING_PROVIDER_CAPABILITIES
        .iter()
        .copied()
        .find(|capabilities| capabilities.backend.eq_ignore_ascii_case(value))
}
