use a4n_service_protocol::{
    EMBEDDING_SERVICE_NAME, EXPORT_SERVICE_NAME, IMAGE_SERVICE_NAME, IMPORT_SERVICE_NAME,
    LLM_SERVICE_NAME, PROMPT_SERVICE_NAME, RAG_SERVICE_NAME, VIDEO_SERVICE_NAME,
};

const PROMPT_SERVICE_URL_ENV: &str = "A4N_PROMPT_SERVICE_URL";
const LLM_GATEWAY_URL_ENV: &str = "A4N_LLM_GATEWAY_URL";
const EMBEDDING_SERVICE_URL_ENV: &str = "A4N_EMBEDDING_SERVICE_URL";
const RAG_SERVICE_URL_ENV: &str = "A4N_RAG_SERVICE_URL";
const IMAGE_SERVICE_URL_ENV: &str = "A4N_IMAGE_SERVICE_URL";
const VIDEO_SERVICE_URL_ENV: &str = "A4N_VIDEO_SERVICE_URL";
const IMPORT_SERVICE_URL_ENV: &str = "A4N_IMPORT_SERVICE_URL";
const EXPORT_SERVICE_URL_ENV: &str = "A4N_EXPORT_SERVICE_URL";

pub(in crate::sidecar_clients::service_manager) const SIDECAR_BIN_DIR_ENV: &str =
    "A4N_SIDECAR_BIN_DIR";
pub(in crate::sidecar_clients::service_manager) const MANAGED_SIDECARS_ENV: &str =
    "A4N_MANAGED_SIDECARS";

const PROMPT_SERVICE_ADDR_ENV: &str = "A4N_PROMPT_SERVICE_ADDR";
const LLM_GATEWAY_ADDR_ENV: &str = "A4N_LLM_GATEWAY_ADDR";
const EMBEDDING_SERVICE_ADDR_ENV: &str = "A4N_EMBEDDING_SERVICE_ADDR";
const RAG_SERVICE_ADDR_ENV: &str = "A4N_RAG_SERVICE_ADDR";
const IMAGE_SERVICE_ADDR_ENV: &str = "A4N_IMAGE_SERVICE_ADDR";
const VIDEO_SERVICE_ADDR_ENV: &str = "A4N_VIDEO_SERVICE_ADDR";
const IMPORT_SERVICE_ADDR_ENV: &str = "A4N_IMPORT_SERVICE_ADDR";
const EXPORT_SERVICE_ADDR_ENV: &str = "A4N_EXPORT_SERVICE_ADDR";

const DEFAULT_PROMPT_SERVICE_URL: &str = "http://127.0.0.1:47641";
const DEFAULT_LLM_GATEWAY_URL: &str = "http://127.0.0.1:47642";
const DEFAULT_EMBEDDING_SERVICE_URL: &str = "http://127.0.0.1:47643";
const DEFAULT_RAG_SERVICE_URL: &str = "http://127.0.0.1:47644";
const DEFAULT_IMAGE_SERVICE_URL: &str = "http://127.0.0.1:47645";
const DEFAULT_VIDEO_SERVICE_URL: &str = "http://127.0.0.1:47646";
const DEFAULT_IMPORT_SERVICE_URL: &str = "http://127.0.0.1:47647";
const DEFAULT_EXPORT_SERVICE_URL: &str = "http://127.0.0.1:47648";

const DEFAULT_PROMPT_SERVICE_ADDR: &str = "127.0.0.1:47641";
const DEFAULT_LLM_GATEWAY_ADDR: &str = "127.0.0.1:47642";
const DEFAULT_EMBEDDING_SERVICE_ADDR: &str = "127.0.0.1:47643";
const DEFAULT_RAG_SERVICE_ADDR: &str = "127.0.0.1:47644";
const DEFAULT_IMAGE_SERVICE_ADDR: &str = "127.0.0.1:47645";
const DEFAULT_VIDEO_SERVICE_ADDR: &str = "127.0.0.1:47646";
const DEFAULT_IMPORT_SERVICE_ADDR: &str = "127.0.0.1:47647";
const DEFAULT_EXPORT_SERVICE_ADDR: &str = "127.0.0.1:47648";

#[derive(Debug, Clone, Copy)]
pub(in crate::sidecar_clients::service_manager) struct SidecarServiceSpec {
    pub(in crate::sidecar_clients::service_manager) name: &'static str,
    pub(in crate::sidecar_clients::service_manager) url_env: &'static str,
    pub(in crate::sidecar_clients::service_manager) default_url: &'static str,
    pub(in crate::sidecar_clients::service_manager) addr_env: &'static str,
    pub(in crate::sidecar_clients::service_manager) default_addr: &'static str,
}

pub(in crate::sidecar_clients::service_manager) fn sidecar_service_specs() -> Vec<SidecarServiceSpec>
{
    vec![
        SidecarServiceSpec {
            name: PROMPT_SERVICE_NAME,
            url_env: PROMPT_SERVICE_URL_ENV,
            default_url: DEFAULT_PROMPT_SERVICE_URL,
            addr_env: PROMPT_SERVICE_ADDR_ENV,
            default_addr: DEFAULT_PROMPT_SERVICE_ADDR,
        },
        SidecarServiceSpec {
            name: LLM_SERVICE_NAME,
            url_env: LLM_GATEWAY_URL_ENV,
            default_url: DEFAULT_LLM_GATEWAY_URL,
            addr_env: LLM_GATEWAY_ADDR_ENV,
            default_addr: DEFAULT_LLM_GATEWAY_ADDR,
        },
        SidecarServiceSpec {
            name: EMBEDDING_SERVICE_NAME,
            url_env: EMBEDDING_SERVICE_URL_ENV,
            default_url: DEFAULT_EMBEDDING_SERVICE_URL,
            addr_env: EMBEDDING_SERVICE_ADDR_ENV,
            default_addr: DEFAULT_EMBEDDING_SERVICE_ADDR,
        },
        SidecarServiceSpec {
            name: RAG_SERVICE_NAME,
            url_env: RAG_SERVICE_URL_ENV,
            default_url: DEFAULT_RAG_SERVICE_URL,
            addr_env: RAG_SERVICE_ADDR_ENV,
            default_addr: DEFAULT_RAG_SERVICE_ADDR,
        },
        SidecarServiceSpec {
            name: IMAGE_SERVICE_NAME,
            url_env: IMAGE_SERVICE_URL_ENV,
            default_url: DEFAULT_IMAGE_SERVICE_URL,
            addr_env: IMAGE_SERVICE_ADDR_ENV,
            default_addr: DEFAULT_IMAGE_SERVICE_ADDR,
        },
        SidecarServiceSpec {
            name: VIDEO_SERVICE_NAME,
            url_env: VIDEO_SERVICE_URL_ENV,
            default_url: DEFAULT_VIDEO_SERVICE_URL,
            addr_env: VIDEO_SERVICE_ADDR_ENV,
            default_addr: DEFAULT_VIDEO_SERVICE_ADDR,
        },
        SidecarServiceSpec {
            name: IMPORT_SERVICE_NAME,
            url_env: IMPORT_SERVICE_URL_ENV,
            default_url: DEFAULT_IMPORT_SERVICE_URL,
            addr_env: IMPORT_SERVICE_ADDR_ENV,
            default_addr: DEFAULT_IMPORT_SERVICE_ADDR,
        },
        SidecarServiceSpec {
            name: EXPORT_SERVICE_NAME,
            url_env: EXPORT_SERVICE_URL_ENV,
            default_url: DEFAULT_EXPORT_SERVICE_URL,
            addr_env: EXPORT_SERVICE_ADDR_ENV,
            default_addr: DEFAULT_EXPORT_SERVICE_ADDR,
        },
    ]
}

pub(in crate::sidecar_clients::service_manager) fn capability_service_names() -> Vec<&'static str> {
    vec![
        PROMPT_SERVICE_NAME,
        LLM_SERVICE_NAME,
        EMBEDDING_SERVICE_NAME,
        RAG_SERVICE_NAME,
        IMAGE_SERVICE_NAME,
        VIDEO_SERVICE_NAME,
        IMPORT_SERVICE_NAME,
        EXPORT_SERVICE_NAME,
    ]
}
