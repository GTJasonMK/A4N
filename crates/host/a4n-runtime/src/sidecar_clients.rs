mod embedding;
mod image;
mod llm;
mod model_discovery;
mod package_io;
mod rag;
mod request_context;
mod response;
mod service_manager;
mod video;

pub(crate) use embedding::{
    create_embedding_provider, delete_embedding_model, download_embedding_model_events,
    embedding_model_download_info, list_embedding_models, prepare_embedding_connectivity_test,
    test_embedding_config_via_sidecar,
};
pub(crate) use image::{generate_image, preview_image_provider_request};
pub(crate) use llm::create_llm_provider;
use llm::{missing_sidecar_capability_message, required_sidecar_client};
pub(crate) use model_discovery::{ModelDiscoveryKind, discover_models};
pub(crate) use package_io::{
    document_export_renderer, parse_import_document, read_import_media_package_zip,
};
pub(crate) use rag::{
    chapter_rag_cleanup_operation_trace_id, character_profile_delete_rag_operation_trace_id,
    character_profile_rollback_rag_operation_trace_id, create_sidecar_rag_store,
    rag_client_ingest_operation_trace_id, rag_ingest_operation_trace_id, rag_operation_timestamp,
    scene_rag_sync_operation_trace_id, scope_rag_operation_trace,
    validate_rag_client_ingest_operation_trace_id, workflow_rag_operation_trace_id,
};
pub(crate) use request_context::{
    SidecarRequestContext, global_typed_provider_service_request,
    global_typed_service_request_without_secrets, pure_transform_typed_service_request,
    service_request_with_typed_context, sidecar_context_for_llm_trace,
    typed_service_request_with_context_provider_secret_policy,
};
use request_context::{SidecarWorkflowScope, sidecar_context_for_workflow_operation_with_root};
use response::{
    GenerationEvent, GenerationStreamError, collect_generation_output, parse_generation_event,
    service_app_error, service_error_app_error, service_error_string,
};
pub(crate) use response::{response_payload, typed_response_payload};
pub(crate) use service_manager::{
    RuntimeCapabilities, runtime_capabilities, runtime_service_manager,
};
pub(crate) use video::{
    delete_video_job, generate_video, poll_video, preview_video_provider_request, video_job_status,
};
