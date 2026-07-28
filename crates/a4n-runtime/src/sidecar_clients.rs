mod embedding;
mod image;
mod llm;
mod package_io;
mod rag;
mod request_context;
mod response;
mod service_manager;
mod video;

pub(crate) use embedding::{
    create_embedding_provider, download_embedding_model_events, embedding_model_operation,
    test_embedding_config_via_sidecar,
};
pub(crate) use image::{
    discover_image_models, generate_image, preview_image_provider_request, probe_image_config,
};
pub(crate) use llm::{create_llm_provider, discover_llm_models};
use llm::{missing_sidecar_capability_message, model_discovery_payload, required_sidecar_client};
pub(crate) use package_io::{
    normalize_import_json, read_import_media_package_zip, render_export_json,
    render_export_json_manifest_bytes, render_export_zip_package,
};
pub(crate) use rag::{
    chapter_rag_cleanup_operation_trace_id, character_profile_delete_rag_operation_trace_id,
    character_profile_rollback_rag_operation_trace_id, create_sidecar_rag_store,
    rag_client_ingest_operation_trace_id, rag_ingest_operation_trace_id,
    scene_rag_sync_operation_trace_id, scope_rag_operation_trace,
    validate_rag_client_ingest_operation_trace_id, workflow_rag_operation_trace_id,
};
pub(crate) use request_context::{
    SidecarRequestContext, global_provider_service_request, global_service_request_without_secrets,
    pure_transform_service_request, service_request_with_context,
    service_request_with_context_provider_secret_policy, sidecar_context_for_llm_trace,
};
use request_context::{sidecar_context_for_workflow_operation_with_root, value_as_i64};
pub(crate) use response::response_payload;
use response::{
    service_app_error, service_error_app_error, service_error_string, service_response_payload,
};
pub(crate) use service_manager::{
    runtime_capabilities, runtime_service_manager, start_managed_sidecars,
};
pub(crate) use video::{
    delete_video_job, discover_video_models, generate_video, poll_video,
    preview_video_provider_request, probe_video_config, video_job_status,
};
