#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"
cd "$PROJECT_ROOT"

if rg -n 'a4n_core::video_execution|a4n-core.*video' crates/a4n-api-contract/Cargo.toml crates/a4n-api-contract/src; then
    printf 'a4n-api-contract must consume the pure video contract crate directly\n' >&2
    exit 1
fi

if rg -n '^a4n-core\s*=' crates/a4n-api-contract/Cargo.toml; then
    printf 'a4n-api-contract must not depend on the Host core crate\n' >&2
    exit 1
fi

if rg -n 'a4n_core::event' crates --glob '*.rs'; then
    printf 'production callers must consume a4n-event and a4n-domain::snapshot directly\n' >&2
    exit 1
fi

if rg -n 'a4n_core::config::env|crate::config::env' crates --glob '*.rs'; then
    printf 'production callers must consume a4n-config directly\n' >&2
    exit 1
fi

config_tree="$(cargo tree --locked -e normal,build -p a4n-config)"
if rg -n '(^|[[:space:]])(a4n-(core|db|runtime|orchestrator)|sea-orm|sqlx|tokio|reqwest)([[:space:]]|v|$)' \
    <<<"$config_tree"; then
    printf 'a4n-config must remain a synchronous boundary parser without Host or runtime dependencies\n' >&2
    exit 1
fi

if rg -n 'a4n_core::|a4n-core' crates --glob '*.rs' --glob 'Cargo.toml'; then
    printf 'production crates must not depend on the removed Host core crate\n' >&2
    exit 1
fi

event_tree="$(cargo tree --locked -e normal,build -p a4n-event)"
if rg -n '(^|[[:space:]])(a4n-(core|db|runtime|orchestrator)|sea-orm|sqlx)([[:space:]]|v|$)' \
    <<<"$event_tree"; then
    printf 'a4n-event must remain independent of Host, database, ORM, and orchestrator crates\n' >&2
    exit 1
fi

for package in a4n-llm a4n-llm-http a4n-llm-gateway a4n-prompt a4n-prompt-git a4n-prompt-service a4n-queue; do
    if rg -n 'a4n[-_]core' "crates/$package/Cargo.toml" "crates/$package/src"; then
        printf '%s must depend on domain/contracts directly instead of a4n-core\n' "$package" >&2
        exit 1
    fi
    package_tree="$(cargo tree --locked --edges normal,build -p "$package")"
    if rg -n '(^|[[:space:]])(a4n-core|sea-orm)([[:space:]]|v|$)' <<<"$package_tree"; then
        printf '%s must not pull Host core or ORM through its error boundary\n' "$package" >&2
        exit 1
    fi
done

task_runtime_tree="$(cargo tree --locked -e normal,build -p a4n-task-runtime)"
if rg -n '(^|[[:space:]])(a4n-core|sea-orm)([[:space:]]|v|$)' <<<"$task_runtime_tree"; then
    printf 'a4n-task-runtime must remain independent of Host core and ORM\n' >&2
    exit 1
fi

domain_tree="$(cargo tree --locked -e normal,build -p a4n-domain)"
if rg -n '(^|[[:space:]])(a4n-(db|runtime|orchestrator)|sea-orm|sqlx|tokio|reqwest)([[:space:]]|v|$)' \
    <<<"$domain_tree"; then
    printf 'a4n-domain must remain free of ORM, host, and runtime dependencies\n' >&2
    exit 1
fi

if rg -n '\bpub (enum|trait) ProjectMedia(CopyError|FileCopier)' \
    crates/a4n-db/src/project/duplicate crates/a4n-orchestrator/src/project; then
    printf 'project media copy contracts must not be owned by DB or orchestrator facades\n' >&2
    exit 1
fi
if ! rg -n '\bpub enum ProjectMediaCopyError|\bpub trait ProjectMediaFileCopier' \
    crates/a4n-domain/src/project.rs; then
    printf 'project media copy contracts must have one domain owner\n' >&2
    exit 1
fi
if rg -n 'a4n_orchestrator::project::(ProjectMediaCopyError|ProjectMediaFileCopier)' \
    crates/a4n-runtime/src/actions/project_duplication; then
    printf 'project duplication media adapters must consume domain copy contracts directly\n' >&2
    exit 1
fi

progress_tree="$(cargo tree --locked -e normal,build -p a4n-progress)"
if rg -n '(^|[[:space:]])(a4n-(core|db|runtime|orchestrator)|sea-orm|sqlx|tokio|reqwest)([[:space:]]|v|$)' \
    <<<"$progress_tree"; then
    printf 'a4n-progress must remain a pure synchronous algorithm crate\n' >&2
    exit 1
fi

if rg -n '\b(sea_orm|DatabaseConnection|AppError|AppState|Runtime(Profile|Config))\b' \
    crates/a4n-domain/src/embedding.rs crates/a4n-domain/src/image.rs crates/a4n-domain/src/json_contract \
    crates/a4n-domain/src/rag.rs crates/a4n-domain/src/text_location.rs crates/a4n-domain/src/profile_ownership.rs; then
    printf 'pure embedding/image domain contracts must not leak ORM or runtime types\n' >&2
    exit 1
fi

storage_tree="$(cargo tree --locked -e normal,build -p a4n-storage)"
if rg -n '(^|[[:space:]])(a4n-(core|db|runtime|orchestrator|server|tauri)|sea-orm|sqlx|tokio|reqwest)([[:space:]]|v|$)' \
    <<<"$storage_tree"; then
    printf 'a4n-storage must remain independent of Host, database, and runtime crates\n' >&2
    exit 1
fi

rag_contract_tree="$(cargo tree --locked -e normal,build -p a4n-rag-contract)"
if rg -n '(^|[[:space:]])(a4n-(core|db|runtime|orchestrator)|sea-orm|sqlx)([[:space:]]|v|$)' \
    <<<"$rag_contract_tree"; then
    printf 'a4n-rag-contract must remain independent of Host, database, ORM, and orchestrator crates\n' >&2
    exit 1
fi

cleanup_contract_tree="$(cargo tree --locked -e normal,build -p a4n-cleanup-contract)"
if rg -n '(^|[[:space:]])(a4n-(core|db|db-models|runtime|orchestrator)|sea-orm|sqlx)([[:space:]]|v|$)' \
    <<<"$cleanup_contract_tree"; then
    printf 'a4n-cleanup-contract must remain independent of Host, database, ORM, and orchestrator crates\n' >&2
    exit 1
fi

canvas_contract_tree="$(cargo tree --locked -e normal,build -p a4n-canvas-contract)"
if rg -n '(^|[[:space:]])(a4n-(core|db|db-models|runtime|orchestrator)|sea-orm|sqlx|tokio)([[:space:]]|v|$)' \
    <<<"$canvas_contract_tree"; then
    printf 'a4n-canvas-contract must remain independent of Host, database, ORM, orchestrator, and runtime crates\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator::(canvas::GenerateCanvasVideoInput|media::GenerateCanvasImageInput)|pub struct GenerateCanvas(Image|Video)Input' \
    crates/a4n-orchestrator/src crates/a4n-runtime/src; then
    printf 'Canvas image/video generation inputs must remain in contract crates, not the orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'pub struct GenerateCanvasImageInput' crates/a4n-canvas-contract/src/lib.rs || \
   ! rg -n 'pub struct GenerateCanvasVideoInput' crates/a4n-api-contract/src/canvas.rs || \
   ! rg -n 'a4n_canvas_contract::GenerateCanvasImageInput|a4n_api_contract::canvas::GenerateCanvasVideoInput|GenerateCanvas(Image|Video)Input' \
    crates/a4n-orchestrator/src/canvas/preview.rs \
    crates/a4n-orchestrator/src/canvas/video_generation.rs \
    crates/a4n-orchestrator/src/context/canvas.rs \
    crates/a4n-orchestrator/src/media/image_generation/contracts.rs \
    crates/a4n-orchestrator/src/media/image_generation/generation.rs \
    crates/a4n-orchestrator/src/video/generation.rs \
    crates/a4n-runtime/src/actions/video_generation.rs \
    crates/a4n-runtime/src/actions/image_generation/mod.rs; then
    printf 'Canvas image/video generation must consume the narrow API/Canvas contracts directly\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator::canvas::Canvas(Image|Video|Audio)ImportHost(Input|Port)|pub (struct|trait) Canvas(Image|Video|Audio)ImportHost(Input|Port)' \
    crates/a4n-orchestrator/src/canvas crates/a4n-runtime/src; then
    printf 'Canvas media import Host contracts must remain in a4n-canvas-contract, not the orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'pub (struct|trait) Canvas(Image|Video|Audio)ImportHost(Input|Port)' \
    crates/a4n-canvas-contract/src/lib.rs || \
   ! rg -n 'a4n_canvas_contract::\{[^}]*Canvas(Image|Video)ImportHost' \
    crates/a4n-runtime/src/actions/canvas_workflow/media_import/image.rs \
    crates/a4n-runtime/src/actions/canvas_workflow/media_import/video.rs; then
    printf 'Canvas media import adapters must consume the narrow canvas contracts directly\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator|OrchestratorContext' \
    crates/a4n-runtime/src/actions/canvas_workflow/media_nodes/editing/host_port.rs; then
    printf 'Canvas media editing HostPort must consume a4n-canvas-contract directly\n' >&2
    exit 1
fi

if ! rg -n 'a4n_canvas_contract::CanvasEditingNodeHostPort|use a4n_canvas_contract::\{' \
    crates/a4n-orchestrator/src/canvas/editing_nodes/execution.rs \
    crates/a4n-orchestrator/src/context/canvas.rs \
    crates/a4n-runtime/src/actions/canvas_workflow/media_nodes/editing/host_port.rs; then
    printf 'Canvas editing execution and HostPort must consume the narrow canvas contract\n' >&2
    exit 1
fi

if rg -n 'a4n_db|a4n_db_models|CanvasGraphRecordDocument|canvas_node::Model|CanvasPreviewApplicationPort' \
    crates/a4n-runtime/src/actions/canvas_workflow/media_nodes/editing.rs; then
    printf 'Canvas editing action must not carry Canvas ORM records or the mixed execution port\n' >&2
    exit 1
fi

if ! rg -n 'canvas_editing_application\(\)' \
    crates/a4n-runtime/src/actions/canvas_workflow/media_nodes/editing.rs; then
    printf 'Canvas editing action must route through CanvasEditingNodeApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait CanvasEditingNodeApplicationPort' crates/a4n-canvas-contract/src/lib.rs; then
    printf 'CanvasEditingNodeApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'a4n_db::cleanup_outbox|a4n_db_models::models::task::cleanup_outbox|cleanup_outbox::(Model|ActiveModel|Entity|Column)' \
    crates/a4n-runtime/src --glob '*.rs'; then
    printf 'Runtime cleanup outbox code must consume pure cleanup claims instead of cleanup ORM types\n' >&2
    exit 1
fi

if rg -n 'a4n_db_models::models::ai::video_gen_config|video_gen_config::Model' \
    crates/a4n-runtime/src/actions/video_generation \
    crates/a4n-runtime/src/sidecar_clients/video \
    crates/a4n-runtime/src/runtime_config --glob '*.rs'; then
    printf 'Runtime video provider paths must consume VideoProviderConfig instead of video ORM models\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator::generation::GeneratedMediaArtifactFile|pub use artifacts::GeneratedMediaArtifactFile|pub struct GeneratedMediaArtifactFile' \
    crates/a4n-runtime/src crates/a4n-orchestrator/src/generation; then
    printf 'GeneratedMediaArtifactFile must remain a pure domain media fact, not an orchestrator export\n' >&2
    exit 1
fi

if ! rg -n 'a4n_domain::media::GeneratedMediaArtifactFile' \
    crates/a4n-runtime/src/file_utils/artifacts.rs \
    crates/a4n-orchestrator/src/generation/artifacts.rs; then
    printf 'Media artifact inspection and event coordination must consume the domain artifact fact\n' >&2
    exit 1
fi

if rg -n 'a4n_db_models|a4n_db::' \
    crates/a4n-runtime/src/actions/video_generation/provider_request/snapshot/record.rs; then
    printf 'Runtime video provider-request snapshots must consume domain video constants instead of DB models\n' >&2
    exit 1
fi

if ! rg -n 'VideoProviderConfig' \
    crates/a4n-provider-contract/src/lib.rs \
    crates/a4n-orchestrator/src/video/provider_execution.rs \
    crates/a4n-runtime/src/sidecar_clients/video \
    crates/a4n-runtime/src/actions/video_generation/provider_host.rs; then
    printf 'VideoProviderConfig must remain the provider execution boundary\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator::video::VideoProviderPollTarget|pub enum VideoProviderPollTarget' \
    crates/a4n-runtime/src crates/a4n-orchestrator/src/video; then
    printf 'VideoProviderPollTarget must remain in a4n-video-contract, not the orchestrator facade\n' >&2
    exit 1
fi

if rg -U -n '^pub use a4n_video_contract::\{[^}]*\b(ChapterVideoGenerationHostPort|ReferenceMediaVideoGenerationHostPort|VideoGenerationHostPort)\b' \
    crates/a4n-orchestrator/src/video/mod.rs; then
    printf 'Video generation HostPorts must remain in a4n-video-contract, not the orchestrator facade\n' >&2
    exit 1
fi

if rg -n '^pub use a4n_video::VideoDurationCapability' \
    crates/a4n-orchestrator/src/video/mod.rs; then
    printf 'VideoDurationCapability must remain owned by a4n-video, not the orchestrator facade\n' >&2
    exit 1
fi

if rg -n '^pub use contracts::ProjectChapterVideoSource' \
    crates/a4n-orchestrator/src/video/mod.rs; then
    printf 'ProjectChapterVideoSource must remain an internal video owner type\n' >&2
    exit 1
fi

if rg -n '^pub use chapter_video_generation::GenerateChapterVideosBatchInput' \
    crates/a4n-orchestrator/src/video/mod.rs; then
    printf 'GenerateChapterVideosBatchInput must remain an internal video generation type\n' >&2
    exit 1
fi

if rg -U -n '^pub use chapter_video_request_rules::\{[^}]*\bBoundaryFrameImageIds\b' \
    crates/a4n-orchestrator/src/video/mod.rs; then
    printf 'BoundaryFrameImageIds must remain an internal video request type\n' >&2
    exit 1
fi

if rg -U -n '^pub use chapter_video_request_rules::\{[^}]*\bChapterVideoProviderRequestContext\b' \
    crates/a4n-orchestrator/src/video/mod.rs; then
    printf 'ChapterVideoProviderRequestContext must remain an internal video request type\n' >&2
    exit 1
fi

if rg -U -n '^pub use chapter_video_request_rules::\{[^}]*\bProviderPromptImageBinding\b' \
    crates/a4n-orchestrator/src/video/mod.rs; then
    printf 'ProviderPromptImageBinding must remain an internal video request type\n' >&2
    exit 1
fi

if rg -n '^pub use prompt_chain_rules::' crates/a4n-orchestrator/src/video/mod.rs; then
    printf 'Video prompt-chain rules must remain internal to the video owner\n' >&2
    exit 1
fi

if rg -n '^pub use assembly_push_rules::ASSEMBLY_PUSH_STATUS_PENDING' \
    crates/a4n-orchestrator/src/video/mod.rs; then
    printf 'Assembly push status constants must remain internal to the video owner\n' >&2
    exit 1
fi

if rg -n '^pub use boundary_frame_rules::' crates/a4n-orchestrator/src/video/mod.rs; then
    printf 'Boundary frame rules must remain internal to the video owner\n' >&2
    exit 1
fi

if rg -n 'latest_boundary_frame_image_id' crates/a4n-orchestrator/src/video/mod.rs; then
    printf 'Unused latest boundary frame facade must not return to the video module\n' >&2
    exit 1
fi

if rg -n '^pub use first_frame_chain_rules::' crates/a4n-orchestrator/src/video/mod.rs; then
    printf 'First-frame chain rules must remain internal to the video owner\n' >&2
    exit 1
fi

if rg -n '^pub use generation_run_rules::' crates/a4n-orchestrator/src/video/mod.rs; then
    printf 'Video generation run inputs must remain internal to the video owner\n' >&2
    exit 1
fi

if rg -n '^pub use job_status_rules::' crates/a4n-orchestrator/src/video/mod.rs \
    || rg -n 'CanvasVideoWorkflowContext' crates/a4n-orchestrator/src/video/mod.rs; then
    printf 'Video job-status rules must remain internal to the video owner\n' >&2
    exit 1
fi

if rg -n 'validate_generated_video_interrupted_status' \
    crates/a4n-orchestrator/src/video/mod.rs \
    crates/a4n-orchestrator/src/video/job_status_rules.rs; then
    printf 'Unused video interrupted-status helper must not return\n' >&2
    exit 1
fi

if rg -n '^pub use library::' crates/a4n-orchestrator/src/video/mod.rs; then
    printf 'Video library rules must remain internal to the video owner\n' >&2
    exit 1
fi

if rg -n '^pub use prompt_application::' crates/a4n-orchestrator/src/video/mod.rs; then
    printf 'Video prompt application inputs must remain internal to the video owner\n' >&2
    exit 1
fi

if rg -n 'VideoSegmentPlanDraftState' crates/a4n-orchestrator/src/video/mod.rs; then
    printf 'Unused video segment draft state facade must not return\n' >&2
    exit 1
fi

if rg -n 'pub use a4n_video_contract::VideoProviderValidationMode' \
    crates/a4n-orchestrator/src/video/reference_media_rules.rs \
    || rg -U -n 'pub use reference_media_rules::\{[^}]*\bVideoProviderValidationMode\b' \
    crates/a4n-orchestrator/src/video/mod.rs; then
    printf 'VideoProviderValidationMode must remain in a4n-video-contract, not the orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'a4n_video_contract::VideoProviderPollTarget|VideoProviderPollTarget' \
    crates/a4n-runtime/src/actions/video_generation/poll.rs \
    crates/a4n-orchestrator/src/video/records.rs \
    crates/a4n-video-contract/src/contracts.rs; then
    printf 'Video provider polling must consume the narrow video contract\n' >&2
    exit 1
fi

video_contract_tree="$(cargo tree --locked -e normal,build -p a4n-video-contract)"
if rg -n '(^|[[:space:]])(a4n-(core|db|db-models|runtime|orchestrator|server|tauri)|sea-orm|sqlx)([[:space:]]|v|$)' \
    <<<"$video_contract_tree"; then
    printf 'a4n-video-contract must remain independent of Host, database, ORM, and orchestrator crates\n' >&2
    exit 1
fi

if rg -n 'pub trait VideoProviderHostPort|a4n_orchestrator::video::VideoProviderHostPort' \
    crates/a4n-orchestrator/src crates/a4n-runtime/src; then
    printf 'VideoProviderHostPort must remain in a4n-video-contract, not the orchestrator facade\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator::video::VideoProviderExecutionApplicationPort' \
    crates/a4n-runtime/src; then
    printf 'Runtime video polling must consume the API contract application port directly\n' >&2
    exit 1
fi

if ! rg -n 'pub trait VideoProviderExecutionApplicationPort' \
    crates/a4n-api-contract/src/video.rs || \
   ! rg -n 'a4n_video_contract::VideoProviderHostPort' \
    crates/a4n-api-contract/src/video.rs \
    crates/a4n-orchestrator/src/video/records.rs \
    crates/a4n-runtime/src/actions/video_generation/provider_host.rs; then
    printf 'Video provider polling must use the narrow API and runtime contracts directly\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator::video::ImportedVideoProperties|pub struct ImportedVideoProperties' \
    crates/a4n-runtime/src crates/a4n-orchestrator/src/video; then
    printf 'ImportedVideoProperties must remain in a4n-video-contract, not the orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'a4n_video_contract::ImportedVideoProperties' \
    crates/a4n-runtime/src/actions/video_records/imports/source/probe.rs; then
    printf 'Video probing must return the narrow ImportedVideoProperties contract\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator::video::(VideoProviderRequestInput|VideoProviderRequestBody|PersistedVideoResult|VideoProviderResultTarget)|pub (struct|enum) (VideoProviderRequestInput|VideoProviderRequestBody|PersistedVideoResult|VideoProviderResultTarget)' \
    crates/a4n-runtime/src crates/a4n-orchestrator/src/video; then
    printf 'Video provider execution DTOs must remain in a4n-video-contract, not the orchestrator facade\n' >&2
    exit 1
fi

for video_contract_consumer in \
    crates/a4n-orchestrator/src/video/chapter_video_generation.rs \
    crates/a4n-orchestrator/src/video/reference_media_generation.rs \
    crates/a4n-orchestrator/src/video/provider_execution.rs \
    crates/a4n-runtime/src/actions/video_generation/provider_host.rs \
    crates/a4n-runtime/src/actions/video_generation/job_lifecycle/completion/video.rs \
    crates/a4n-runtime/src/actions/video_generation/media/tail_frames/capture.rs \
    crates/a4n-runtime/src/actions/video_records/imports/chapter.rs; do
    if ! rg -n 'a4n_video_contract::' "$video_contract_consumer"; then
        printf '%s must consume video provider DTOs from a4n-video-contract directly\n' "$video_contract_consumer" >&2
        exit 1
    fi
done

if rg -n 'a4n_orchestrator::video::VideoRequestPreviewApplicationPort|pub trait VideoRequestPreviewApplicationPort|a4n_orchestrator::video::(CanvasVideoRequestPreview|ChapterVideoRequestPreview|ReferenceMediaVideoRequestPreview)' \
    crates/a4n-runtime/src crates/a4n-orchestrator/src; then
    printf 'Video request previews must use the API application port and stable video preview contracts\n' >&2
    exit 1
fi

if ! rg -n 'pub trait VideoRequestPreviewApplicationPort' crates/a4n-api-contract/src/video.rs || \
   ! rg -n 'pub (struct|trait) (CanvasVideoRequestPreview|ChapterVideoRequestPreview|ReferenceMediaVideoRequestPreview|CanvasVideoPreviewHostPort|ChapterVideoPreviewHostPort|ReferenceMediaVideoPreviewHostPort)' \
    crates/a4n-video-contract/src/contracts.rs; then
    printf 'Video request preview contracts are missing from their stable owners\n' >&2
    exit 1
fi

if rg -n 'a4n_db::video' \
    crates/a4n-runtime/src/cleanup_outbox/publication.rs \
    crates/a4n-runtime/src/actions/image_records/upload/record.rs; then
    printf 'Runtime image publication paths must consume media status contracts instead of the video DB facade\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator|OrchestratorContext' crates/a4n-runtime/src/actions/rag_search; then
    printf 'RAG search actions must consume a4n-rag-contract instead of the full orchestrator context\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator|OrchestratorContext|state\.orchestrator\(\)' \
    crates/a4n-runtime/src/rag_ingest --glob '*.rs'; then
    printf 'RAG ingest actions must consume a4n-rag-contract instead of the full orchestrator context\n' >&2
    exit 1
fi

if ! rg -n 'pub trait RagIngestPort' crates/a4n-rag-contract/src/lib.rs; then
    printf 'RagIngestPort contract is missing\n' >&2
    exit 1
fi

if rg -U -n 'state\s*\.\s*orchestrator\(\)\s*\.\s*clear_project_rag' \
    crates/a4n-runtime/src/cleanup_outbox/claim.rs \
    crates/a4n-runtime/src/actions/generation_tasks/destructive_lifecycle.rs; then
    printf 'RAG project cleanup must consume RagIngestPort instead of the concrete orchestrator\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\)\.(image_publications_for_recovery|lock_image_publications|image_publication_by_id|generated_image_by_id|publish_imported_image_publication_for_user_validated|ensure_(portrait|profile_material)_publication_identity_for_user_validated|mark_image_publication_(reclaiming|reclaimed)|unreferenced_media_paths|media_path_prefix_has_persisted_reference|project_has_active_image_publication|protected_image_publication_cleanup_paths)' \
    crates/a4n-runtime/src/cleanup_outbox/publication.rs \
    crates/a4n-runtime/src/cleanup_outbox/claim.rs; then
    printf 'cleanup outbox media recovery must avoid direct media-owner calls\n' >&2
    exit 1
fi

if ! rg -n 'image_publication_recovery_application\(\)' \
    crates/a4n-runtime/src/cleanup_outbox/publication.rs; then
    printf 'cleanup outbox publication recovery must route through ImagePublicationRecoveryPort\n' >&2
    exit 1
fi
if ! rg -n 'image_publication_recovery_read_application\(\)' \
    crates/a4n-runtime/src/cleanup_outbox/publication.rs; then
    printf 'cleanup outbox publication reads must route through ImagePublicationRecoveryReadPort\n' >&2
    exit 1
fi
if rg -U -n 'image_publication_recovery_application\(\)\s*\.\s*(image_publications_for_recovery|image_publication_by_id|generated_image_identity)' \
    crates/a4n-runtime/src/cleanup_outbox/publication.rs; then
    printf 'cleanup outbox publication reads must not use the state-transition port\n' >&2
    exit 1
fi

if ! rg -n 'image_media_reference_application\(\)' \
    crates/a4n-runtime/src/cleanup_outbox/publication.rs \
    crates/a4n-runtime/src/cleanup_outbox/claim.rs; then
    printf 'cleanup outbox media reference checks must route through ImageMediaReferencePort\n' >&2
    exit 1
fi

if rg -U -n 'state\s*\.\s*orchestrator\(\)\s*\.\s*renew_claimed_cleanup' \
    crates/a4n-runtime/src/cleanup_outbox/lease.rs; then
    printf 'cleanup outbox lease renewal must consume CleanupOutboxApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'cleanup_outbox_application\(\)' \
    crates/a4n-runtime/src/cleanup_outbox/lease.rs; then
    printf 'cleanup outbox lease renewal must route through the injected cleanup application port\n' >&2
    exit 1
fi

if ! rg -n 'pub trait CleanupOutboxApplicationPort' \
    crates/a4n-cleanup-contract/src/lib.rs; then
    printf 'CleanupOutboxApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\)' \
    crates/a4n-runtime/src/actions/blueprint_records/manual.rs \
    crates/a4n-runtime/src/actions/blueprint_records/world_setting.rs \
    crates/a4n-runtime/src/actions/blueprint_records/versions.rs; then
    printf 'Runtime blueprint mutation actions must consume BlueprintMutationApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'blueprint_mutation_application\(\)' \
    crates/a4n-runtime/src/actions/blueprint_records/manual.rs \
    crates/a4n-runtime/src/actions/blueprint_records/world_setting.rs \
    crates/a4n-runtime/src/actions/blueprint_records/versions.rs; then
    printf 'Runtime blueprint mutation actions must route through BlueprintMutationApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait BlueprintMutationApplicationPort' \
    crates/a4n-api-contract/src/blueprint.rs; then
    printf 'BlueprintMutationApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'preview_blueprint_(generation|refine)_request_for_user_validated' \
    crates/a4n-runtime/src/actions/blueprint_records/generation.rs \
    crates/a4n-runtime/src/actions/blueprint_records/refinement.rs; then
    printf 'Runtime blueprint request previews must consume BlueprintRequestPreviewApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'blueprint_request_preview_application\(\)' \
    crates/a4n-runtime/src/actions/blueprint_records/generation.rs \
    crates/a4n-runtime/src/actions/blueprint_records/refinement.rs; then
    printf 'Runtime blueprint request previews must route through BlueprintRequestPreviewApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait BlueprintRequestPreviewApplicationPort' \
    crates/a4n-orchestrator/src/blueprint.rs; then
    printf 'BlueprintRequestPreviewApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator|OrchestratorContext|state\.orchestrator\(\)|actions::access' \
    crates/a4n-runtime/src/actions/canvas_workflow/records_api.rs; then
    printf 'Canvas record actions must consume CanvasApplicationPort instead of the concrete orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'pub trait Canvas(Read|Mutation)ApplicationPort' crates/a4n-api-contract/src/canvas.rs; then
    printf 'Canvas read/mutation contracts are missing\n' >&2
    exit 1
fi

if rg -n 'CanvasApplicationPort|canvas_application\(\)' \
    crates/a4n-api-contract/src/canvas.rs crates/a4n-orchestrator/src crates/a4n-runtime/src; then
    printf 'The mixed CanvasApplicationPort must remain removed\n' >&2
    exit 1
fi

if ! rg -n 'canvas_read_application\(\)' \
    crates/a4n-runtime/src/actions/canvas_workflow/records_api.rs; then
    printf 'Canvas record reads must route through CanvasReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'canvas_mutation_application\(\)' \
    crates/a4n-runtime/src/actions/canvas_workflow/records_api.rs; then
    printf 'Canvas record mutations must route through CanvasMutationApplicationPort\n' >&2
    exit 1
fi

if rg -n 'a4n_db|a4n_db_models|CanvasGraphRecordDocument|canvas_node::Model|a4n_orchestrator::OrchestratorContext|state\.orchestrator\(\)' \
    crates/a4n-runtime/src/actions/canvas_workflow/preview; then
    printf 'Canvas preview actions must not carry Canvas ORM records or the concrete orchestrator\n' >&2
    exit 1
fi

if ! rg -n 'canvas_preview_application\(\)' \
    crates/a4n-runtime/src/actions/canvas_workflow/preview.rs; then
    printf 'Canvas preview actions must route through CanvasPreviewApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait CanvasPreviewApplicationPort' \
    crates/a4n-orchestrator/src/canvas/preview.rs; then
    printf 'CanvasPreviewApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'a4n_db|a4n_db_models|CanvasGraphRecordDocument|canvas_node::Model|CanvasPreviewApplicationPort' \
    crates/a4n-runtime/src/actions/canvas_workflow/output_nodes.rs; then
    printf 'Canvas output action must not carry Canvas ORM records or the mixed execution port\n' >&2
    exit 1
fi

if ! rg -n 'canvas_output_application\(\)' \
    crates/a4n-runtime/src/actions/canvas_workflow/output_nodes.rs; then
    printf 'Canvas output action must route through CanvasOutputNodeApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait CanvasOutputNodeApplicationPort' \
    crates/a4n-canvas-contract/src/lib.rs; then
    printf 'CanvasOutputNodeApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'a4n_db|a4n_db_models|CanvasGraphRecordDocument|canvas_node::Model|CanvasPreviewApplicationPort' \
    crates/a4n-runtime/src/actions/canvas_workflow/probe_node.rs; then
    printf 'Canvas probe action must not carry Canvas ORM records or the mixed execution port\n' >&2
    exit 1
fi

if ! rg -n 'canvas_probe_application\(\)' \
    crates/a4n-runtime/src/actions/canvas_workflow/probe_node.rs; then
    printf 'Canvas probe action must route through CanvasProbeNodeApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait CanvasProbeNodeApplicationPort' \
    crates/a4n-canvas-contract/src/lib.rs; then
    printf 'CanvasProbeNodeApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'a4n_db|a4n_db_models|CanvasGraphRecordDocument|canvas_node::Model|CanvasPreviewApplicationPort|OrchestratorContext|state\.orchestrator\(\)' \
    crates/a4n-runtime/src/actions/canvas_workflow/text_nodes/render_text.rs; then
    printf 'Canvas RenderText action must not carry Canvas ORM records or the mixed execution port\n' >&2
    exit 1
fi

if ! rg -n 'canvas_render_text_application\(\)' \
    crates/a4n-runtime/src/actions/canvas_workflow/text_nodes/render_text.rs; then
    printf 'Canvas RenderText action must route through CanvasRenderTextNodeApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait CanvasRenderTextNodeApplicationPort' \
    crates/a4n-canvas-contract/src/lib.rs; then
    printf 'CanvasRenderTextNodeApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'a4n_db|a4n_db_models|CanvasGraphRecordDocument|canvas_node::Model|CanvasPreviewApplicationPort|OrchestratorContext|state\.orchestrator\(\)' \
    crates/a4n-runtime/src/actions/canvas_workflow/text_nodes/llm_text.rs; then
    printf 'Canvas LlmText action must not carry Canvas ORM records or the mixed execution port\n' >&2
    exit 1
fi

if ! rg -n 'canvas_llm_text_application\(\)' \
    crates/a4n-runtime/src/actions/canvas_workflow/text_nodes/llm_text.rs; then
    printf 'Canvas LlmText action must route through CanvasLlmTextNodeApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait CanvasLlmTextNodeApplicationPort' \
    crates/a4n-orchestrator/src/canvas/text_generation.rs; then
    printf 'CanvasLlmTextNodeApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'a4n_db|a4n_db_models|CanvasGraphRecordDocument|canvas_node::Model|CanvasPreviewApplicationPort|OrchestratorContext|state\.orchestrator\(\)' \
    crates/a4n-runtime/src/actions/canvas_workflow/media_nodes/image.rs; then
    printf 'Canvas image action must not carry Canvas ORM records or the mixed execution port\n' >&2
    exit 1
fi

if ! rg -n 'canvas_image_application\(\)' \
    crates/a4n-runtime/src/actions/canvas_workflow/media_nodes/image.rs; then
    printf 'Canvas image action must route through CanvasImageNodeApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait CanvasImageNodeApplicationPort' \
    crates/a4n-orchestrator/src/canvas/image_generation.rs; then
    printf 'CanvasImageNodeApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'a4n_db|a4n_db_models|CanvasGraphRecordDocument|canvas_node::Model|CanvasPreviewApplicationPort|OrchestratorContext|state\.orchestrator\(\)' \
    crates/a4n-runtime/src/actions/canvas_workflow/media_nodes/video; then
    printf 'Canvas video action must not carry Canvas ORM records or the mixed execution port\n' >&2
    exit 1
fi

if ! rg -n 'canvas_video_application\(\)' \
    crates/a4n-runtime/src/actions/canvas_workflow/media_nodes/video; then
    printf 'Canvas video action must route through CanvasVideoNodeApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait CanvasVideoNodeApplicationPort' \
    crates/a4n-orchestrator/src/canvas/video_generation.rs; then
    printf 'CanvasVideoNodeApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\)' \
    crates/a4n-runtime/src/actions/canvas_workflow/execution/api.rs; then
    printf 'Canvas graph execution actions must consume CanvasGraphExecutionApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'canvas_graph_execution_application\(\)' \
    crates/a4n-runtime/src/actions/canvas_workflow/execution/api.rs; then
    printf 'Canvas graph execution actions must route through CanvasGraphExecutionApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait CanvasGraphExecutionApplicationPort' \
    crates/a4n-canvas-contract/src/lib.rs; then
    printf 'CanvasGraphExecutionApplicationPort contract is missing\n' >&2
    exit 1
fi

if ! rg -n 'pub trait CanvasMediaImportApplicationPort' \
    crates/a4n-canvas-contract/src/lib.rs; then
    printf 'CanvasMediaImportApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator::canvas::CanvasMediaImportApplicationPort' \
    crates/a4n-runtime/src/actions/canvas_workflow/media_import \
    crates/a4n-runtime/src/state/app.rs; then
    printf 'Canvas media import consumers must use a4n-canvas-contract directly\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator::canvas::CanvasGraph|a4n_orchestrator::canvas::PreparedCanvasGraphRun' \
    crates/a4n-runtime/src/actions/canvas_workflow/execution \
    crates/a4n-runtime/src/state/app.rs; then
    printf 'Canvas graph execution consumers must use a4n-canvas-contract directly\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator::canvas' \
    crates/a4n-runtime/src/actions/canvas_workflow/execution; then
    printf 'Canvas graph execution consumers must not import the orchestrator Canvas facade\n' >&2
    exit 1
fi

if ! rg -n 'pub (enum CanvasGraphNodeExecutionKind|struct CanvasGraphNodeExecutionInput|struct PreparedCanvasGraphRun|trait CanvasGraphExecutionHostPort)' \
    crates/a4n-canvas-contract/src/lib.rs; then
    printf 'Canvas graph execution Host contracts are missing from a4n-canvas-contract\n' >&2
    exit 1
fi

if rg -n 'a4n_db|a4n_db_models|CanvasGraphRecordDocument|canvas_node::Model' \
    crates/a4n-canvas-contract/src/lib.rs \
    crates/a4n-runtime/src/actions/canvas_workflow/execution/host_port.rs; then
    printf 'Canvas graph execution HostPort must carry node IDs, not Canvas ORM records\n' >&2
    exit 1
fi

if ! rg -n 'node_id' \
    crates/a4n-canvas-contract/src/lib.rs \
    crates/a4n-runtime/src/actions/canvas_workflow/execution/host_port.rs; then
    printf 'Canvas graph execution HostPort node identity input is missing\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\)|OrchestratorContext' \
    crates/a4n-runtime/src/actions/canvas_workflow/text_nodes/host_port.rs; then
    printf 'Canvas text node HostPort must consume ProviderRuntime instead of the concrete orchestrator\n' >&2
    exit 1
fi

if rg -n 'queue_run_tracked' \
    crates/a4n-runtime/src/actions/canvas_workflow/text_nodes/host_port.rs; then
    printf 'Canvas text node HostPort must not acquire a second LLM queue slot\n' >&2
    exit 1
fi

if ! rg -q 'chat_with_config_id' \
    crates/a4n-runtime/src/actions/canvas_workflow/text_nodes/host_port.rs || \
   ! rg -q 'queue_start_llm_task' \
    crates/a4n-runtime/src/dynamic_llm/provider/preparation.rs || \
   ! rg -q 'concurrency_limit\(\)' \
    crates/a4n-runtime/src/dynamic_llm/provider/preparation.rs || \
   ! rg -q 'trace.workflow_run_id' \
    crates/a4n-runtime/src/provider_runtime.rs; then
    printf 'Canvas LLM calls must retain resolved model admission and workflow tracking through the shared provider path\n' >&2
    exit 1
fi

if rg -U -n 'state\s*\.\s*orchestrator\(\)\s*\.\s*queue_run_tracked_with_workflow_context' \
    crates/a4n-runtime/src/actions/canvas_workflow/media_nodes/editing/host_port.rs; then
    printf 'Canvas media editing HostPort must consume ProviderRuntime instead of the concrete orchestrator\n' >&2
    exit 1
fi

if ! rg -n 'provider_runtime\(\)' \
    crates/a4n-runtime/src/actions/canvas_workflow/media_nodes/editing/host_port.rs || \
   ! rg -n 'queue_run_tracked_with_workflow_context' \
    crates/a4n-runtime/src/actions/canvas_workflow/media_nodes/editing/host_port.rs; then
    printf 'Canvas media editing HostPort must route workflow-tracked provider calls through ProviderRuntime\n' >&2
    exit 1
fi

if rg -U -n 'state\s*\.\s*orchestrator\(\)\s*\.\s*queue_' \
    crates/a4n-runtime/src/actions/image_generation/generation_io/queue.rs; then
    printf 'Image generation queue helpers must consume ProviderRuntime instead of the concrete orchestrator\n' >&2
    exit 1
fi

if ! rg -n 'provider_runtime\(\)' \
    crates/a4n-runtime/src/actions/image_generation/generation_io/queue.rs || \
   ! rg -n 'queue_run_tracked_with_(workflow_context|cancel)' \
    crates/a4n-runtime/src/actions/image_generation/generation_io/queue.rs; then
    printf 'Image generation queue helpers must route tracked calls through ProviderRuntime\n' >&2
    exit 1
fi

if rg -U -n 'state\s*\.\s*orchestrator\(\)|OrchestratorContext|a4n_db_models::models::project::novel_project|novel_project::Model' \
    crates/a4n-runtime/src/actions/project_duplication/execution.rs; then
    printf 'Runtime project duplication must not expose project ORM records or call the concrete orchestrator\n' >&2
    exit 1
fi

if ! rg -n 'project_duplication_application\(\)' \
    crates/a4n-runtime/src/actions/project_duplication/execution.rs; then
    printf 'Runtime project duplication must route through ProjectDuplicationApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ProjectDuplicationApplicationPort' \
    crates/a4n-api-contract/src/project.rs; then
    printf 'ProjectDuplicationApplicationPort API contract is missing\n' >&2
    exit 1
fi
if rg -n 'pub trait ProjectDuplicationApplicationPort' \
    crates/a4n-orchestrator/src/project; then
    printf 'ProjectDuplicationApplicationPort must not be redefined in the orchestrator\n' >&2
    exit 1
fi

if ! rg -n 'pub trait SettingsAll(Application|Host)Port|pub struct PromptRow' \
    crates/a4n-api-contract/src/config.rs; then
    printf 'settings import/export contracts must live in the API contract\n' >&2
    exit 1
fi
if rg -n 'pub trait SettingsAll(Application|Host)Port|pub struct PromptRow' \
    crates/a4n-orchestrator/src/config; then
    printf 'settings import/export contracts must not be redefined in the orchestrator\n' >&2
    exit 1
fi

if rg -U -n 'state\s*\.\s*orchestrator\(\)|OrchestratorContext|a4n_db_models::models::project::novel_project|novel_project::Model' \
    crates/a4n-runtime/src/actions/project_records/reset \
    crates/a4n-runtime/src/actions/generation_tasks/destructive_lifecycle.rs; then
    printf 'Runtime project generated-data reset must not expose project ORM records or call the concrete orchestrator\n' >&2
    exit 1
fi

if ! rg -U -n 'generation_lifecycle_application\(\)\s*\.\s*reset_project_generated_data_validated' \
    crates/a4n-runtime/src/actions/generation_tasks/destructive_lifecycle.rs; then
    printf 'Runtime project generated-data reset must route through GenerationLifecycleApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait GenerationTaskReadApplicationPort' \
    crates/a4n-orchestrator/src/context/generation.rs || \
   ! rg -n 'pub trait GenerationTaskMutationApplicationPort' \
    crates/a4n-orchestrator/src/context/generation.rs || \
   ! rg -n 'pub trait GenerationLifecycleApplicationPort' \
    crates/a4n-orchestrator/src/context/generation.rs || \
   ! rg -n 'pub trait GenerationCleanupApplicationPort' \
    crates/a4n-orchestrator/src/context/generation.rs; then
    printf 'Generation task/lifecycle/cleanup contracts are missing\n' >&2
    exit 1
fi

if rg -n '(^|[^A-Za-z])GenerationApplicationPort($|[^A-Za-z])|(^|[^A-Za-z_])generation_application\(\)' \
    crates/a4n-orchestrator/src crates/a4n-runtime/src; then
    printf 'The mixed GenerationApplicationPort must remain removed\n' >&2
    exit 1
fi

if ! rg -n 'generation_task_read_application\(\)' \
    crates/a4n-runtime/src/actions/generation_tasks/mod.rs \
    crates/a4n-runtime/src/actions/portrait_generation/application.rs; then
    printf 'Runtime generation task reads must route through GenerationTaskReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'generation_task_mutation_application\(\)' \
    crates/a4n-runtime/src/actions/generation_tasks/mod.rs \
    crates/a4n-runtime/src/actions/blueprint_records/generation.rs \
    crates/a4n-runtime/src/actions/blueprint_records/refinement.rs; then
    printf 'Runtime generation task mutations must route through GenerationTaskMutationApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'generation_lifecycle_application\(\)' \
    crates/a4n-runtime/src/actions/generation_tasks/destructive_lifecycle.rs; then
    printf 'Runtime generation lifecycle must route through GenerationLifecycleApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'generation_cleanup_application\(\)' \
    crates/a4n-runtime/src/cleanup_outbox/worker.rs; then
    printf 'Runtime generation retention must route through GenerationCleanupApplicationPort\n' >&2
    exit 1
fi

if rg -n 'a4n_db::generation|a4n_db_models::models|PersistedGenerationTask|GenerationTaskPersistenceError' \
    crates/a4n-runtime/src/actions/generation_tasks; then
    printf 'Runtime generation task actions must consume domain lifecycle contracts, not DB models\n' >&2
    exit 1
fi

if ! rg -n 'a4n_domain::generation::GenerationTaskListCursor' \
    crates/a4n-runtime/src/actions/generation_tasks; then
    printf 'Runtime generation task history must consume the domain cursor contract\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\)|OrchestratorContext|a4n_db_models::models::ai::generated_image|generated_image::Model|prepare_imported_image_upload_for_user_validated|create_imported_image_publication_intent_for_user_validated|publish_imported_image_publication_for_user_validated|generated_image_by_id' \
    crates/a4n-runtime/src/actions/canvas_workflow/media_import/image.rs \
    crates/a4n-runtime/src/actions/image_records/upload.rs \
    crates/a4n-runtime/src/actions/image_records/upload/record.rs; then
    printf 'Canvas image import and image upload must not expose ORM records or call the concrete orchestrator\n' >&2
    exit 1
fi

if ! rg -n 'image_import_application\(\)' \
    crates/a4n-runtime/src/actions/image_records/upload.rs \
    crates/a4n-runtime/src/actions/image_records/upload/record.rs; then
    printf 'Canvas image import and image upload must route through ImageImportApplicationPort\n' >&2
    exit 1
fi
if ! rg -n 'image_record_read_application\(\)' \
    crates/a4n-runtime/src/actions/image_records/upload/record.rs; then
    printf 'Published image lookup must route through ImageRecordReadApplicationPort\n' >&2
    exit 1
fi
if rg -n 'imported_image_summary' crates/a4n-runtime/src; then
    printf 'Runtime image import must not keep a read method on ImageImportApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ImageImportApplicationPort|pub trait ImagePublicationRecoveryPort' \
    crates/a4n-orchestrator/src/media/image_record_lifecycle.rs; then
    printf 'image import/recovery contracts are missing\n' >&2
    exit 1
fi
if ! rg -n 'pub trait ImagePublicationRecoveryReadPort' crates/a4n-api-contract/src/image.rs; then
    printf 'image publication recovery read contract is missing from the API contract\n' >&2
    exit 1
fi
if rg -n 'pub trait ImagePublicationRecoveryReadPort' crates/a4n-orchestrator/src/media; then
    printf 'ImagePublicationRecoveryReadPort must not be redefined in the orchestrator\n' >&2
    exit 1
fi
if rg -n 'a4n_orchestrator::media::ImagePublicationRecoveryReadPort' crates/a4n-runtime/src; then
    printf 'Runtime image publication recovery reads must consume the API contract directly\n' >&2
    exit 1
fi
if ! rg -n 'pub trait ImageMediaReferencePort' crates/a4n-api-contract/src/image.rs; then
    printf 'image media reference contract is missing from the API contract\n' >&2
    exit 1
fi
if rg -n 'pub trait ImageMediaReferencePort' crates/a4n-orchestrator/src/media; then
    printf 'ImageMediaReferencePort must not be redefined in the orchestrator\n' >&2
    exit 1
fi
if rg -n 'a4n_orchestrator::media::ImageMediaReferencePort' crates/a4n-runtime/src; then
    printf 'Runtime image media reference adapters must consume the API contract directly\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ImageRecord(Read|Mutation)ApplicationPort' \
    crates/a4n-api-contract/src/image.rs; then
    printf 'image record read/mutation contracts are missing\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\)|OrchestratorContext|actions::access' \
    crates/a4n-runtime/src/actions/export_files.rs; then
    printf 'Runtime export actions must consume ExportApplicationPort instead of the concrete orchestrator or duplicate authorization\n' >&2
    exit 1
fi

if ! rg -n 'export_application\(\)' crates/a4n-runtime/src/actions/export_files.rs; then
    printf 'Runtime export actions must route through ExportApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub struct ExportProgress' crates/a4n-api-contract/src/export.rs; then
    printf 'ExportProgress contract is missing from the API contract\n' >&2
    exit 1
fi
if rg -n 'pub struct ExportProgress|a4n_orchestrator::export::ExportProgress' \
    crates/a4n-orchestrator/src/export crates/a4n-runtime/src; then
    printf 'Runtime export progress must consume the API contract directly\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ExportApplicationPort' crates/a4n-api-contract/src/export.rs; then
    printf 'ExportApplicationPort contract is missing from the API contract\n' >&2
    exit 1
fi
if rg -n 'pub trait ExportApplicationPort' crates/a4n-orchestrator/src/export; then
    printf 'ExportApplicationPort must not be redefined in the orchestrator\n' >&2
    exit 1
fi
if rg -n 'a4n_orchestrator::export::ExportApplicationPort' crates/a4n-runtime/src; then
    printf 'Runtime export adapters must consume the API contract directly\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\)|OrchestratorContext|actions::access' \
    crates/a4n-runtime/src/actions/video_records/records.rs \
    crates/a4n-runtime/src/actions/video_records/deletion.rs \
    crates/a4n-runtime/src/actions/video_records/playback.rs \
    crates/a4n-runtime/src/actions/video_records/assembly/actions/status.rs \
    crates/a4n-runtime/src/actions/video_records/imports/processing/trim.rs; then
    printf 'Runtime video record actions must consume narrow application ports\n' >&2
    exit 1
fi

if rg -n 'VideoRecordApplicationPort|video_record_application\(\)' \
    crates/a4n-api-contract/src/video.rs \
    crates/a4n-orchestrator/src/video \
    crates/a4n-runtime/src; then
    printf 'the mixed VideoRecordApplicationPort must remain removed\n' >&2
    exit 1
fi

if ! rg -n 'video_record_read_application\(\)' \
    crates/a4n-runtime/src/actions/video_records/records.rs \
    crates/a4n-runtime/src/actions/video_records/playback.rs \
    crates/a4n-runtime/src/actions/video_generation/reference_media/api.rs \
    crates/a4n-runtime/src/actions/video_generation/media/reference_images; then
    printf 'video record reads must route through VideoRecordReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'video_record_mutation_application\(\)' \
    crates/a4n-runtime/src/actions/video_records/records.rs \
    crates/a4n-runtime/src/actions/video_records/deletion.rs; then
    printf 'ordinary video record mutations must route through VideoRecordMutationApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'video_import_application\(\)' \
    crates/a4n-runtime/src/actions/video_records/imports; then
    printf 'video imports and trims must route through VideoImportApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'video_assembly_application\(\)' \
    crates/a4n-runtime/src/actions/video_records/assembly; then
    printf 'Assembly actions must route through VideoAssemblyApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'video_record_lifecycle_application\(\)' \
    crates/a4n-runtime/src/actions/video_records \
    crates/a4n-runtime/src/actions/video_generation/media/tail_frames; then
    printf 'video completion and cleanup must route through VideoRecordLifecycleApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'video_canvas_media_application\(\)' \
    crates/a4n-runtime/src/actions/video_records/imports/canvas; then
    printf 'canvas video media actions must route through VideoCanvasMediaApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait VideoRecordReadApplicationPort|pub trait VideoRecordMutationApplicationPort|pub trait VideoImportApplicationPort|pub trait VideoAssemblyApplicationPort|pub trait VideoRecordLifecycleApplicationPort|pub trait VideoCanvasMediaApplicationPort' \
    crates/a4n-api-contract/src/video.rs; then
    printf 'split video record contracts are missing from the API contract\n' >&2
    exit 1
fi

if rg -n '\.orchestrator\(\)|OrchestratorContext' \
    crates/a4n-runtime/src/actions/galgame_records/pipeline; then
    printf 'Runtime Galgame pipeline actions must consume split Galgame pipeline ports\n' >&2
    exit 1
fi

if ! rg -n 'galgame_pipeline_execution_application\(\)' \
    crates/a4n-runtime/src/actions/galgame_records/pipeline/execution/start.rs || \
   ! rg -n 'galgame_pipeline_preview_application\(\)' \
    crates/a4n-runtime/src/actions/galgame_records/pipeline/preview.rs || \
   ! rg -n 'galgame_pipeline_read_application\(\)' \
    crates/a4n-runtime/src/actions/galgame_records/pipeline/status.rs || \
   ! rg -n 'galgame_pipeline_cancellation_application\(\)' \
    crates/a4n-runtime/src/actions/galgame_records/pipeline/execution/cancel.rs; then
    printf 'Runtime Galgame pipeline actions must route through split application ports\n' >&2
    exit 1
fi

if rg -n 'pub trait GalgamePipeline(Application|Execution|Preview|Read|Cancellation)ApplicationPort|pub use .*GalgamePipeline(Application|Execution|Preview|Read|Cancellation)ApplicationPort' \
    crates/a4n-orchestrator/src/galgame; then
    printf 'Galgame pipeline ports must not be owned or re-exported by the orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'pub trait GalgamePipelineExecutionApplicationPort' \
    crates/a4n-api-contract/src/galgame.rs || \
   ! rg -n 'pub trait GalgamePipelinePreviewApplicationPort' \
    crates/a4n-api-contract/src/galgame.rs || \
   ! rg -n 'pub trait GalgamePipelineReadApplicationPort' \
    crates/a4n-api-contract/src/galgame.rs || \
   ! rg -n 'pub trait GalgamePipelineCancellationApplicationPort' \
    crates/a4n-api-contract/src/galgame.rs; then
    printf 'Galgame pipeline split contracts are missing from the API contract\n' >&2
    exit 1
fi

if ! rg -n 'pub struct GalgamePipelinePreviewStep' \
    crates/a4n-api-contract/src/galgame.rs; then
    printf 'Galgame pipeline preview contract is missing from the API contract\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator::galgame::(GalgamePipelineProgressUpdate|GalgameProgressReporter)|pub (struct|type) (GalgamePipelineProgressUpdate|GalgameProgressReporter)' \
    crates/a4n-runtime/src crates/a4n-server/src crates/a4n-tauri/src \
    crates/a4n-orchestrator/src/galgame; then
    printf 'Galgame pipeline progress contracts must be consumed from a4n-api-contract\n' >&2
    exit 1
fi

if ! rg -n 'pub struct GalgamePipelineProgressUpdate|pub type GalgameProgressReporter' \
    crates/a4n-api-contract/src/galgame.rs; then
    printf 'Galgame pipeline progress contracts are missing from a4n-api-contract\n' >&2
    exit 1
fi

if rg -n '\.orchestrator\(\)|OrchestratorContext|actions::access' \
    crates/a4n-runtime/src/actions/manga_pipeline; then
    printf 'Runtime Manga pipeline actions must consume split Manga pipeline ports\n' >&2
    exit 1
fi

if ! rg -n 'manga_pipeline_execution_application\(\)' \
    crates/a4n-runtime/src/actions/manga_pipeline/execution.rs || \
   ! rg -n 'manga_pipeline_preview_application\(\)' \
    crates/a4n-runtime/src/actions/manga_pipeline/preview.rs || \
   ! rg -n 'manga_pipeline_read_application\(\)' \
    crates/a4n-runtime/src/actions/manga_records/pipeline.rs || \
   ! rg -n 'manga_pipeline_cancellation_application\(\)' \
    crates/a4n-runtime/src/actions/manga_records/pipeline.rs; then
    printf 'Runtime Manga pipeline actions must route through split application ports\n' >&2
    exit 1
fi

if rg -n 'pub trait MangaPipeline(Application|Execution|Preview|Read|Cancellation)ApplicationPort|pub use .*MangaPipeline(Application|Execution|Preview|Read|Cancellation)ApplicationPort' \
    crates/a4n-orchestrator/src/manga; then
    printf 'Manga pipeline ports must not be owned or re-exported by the orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'pub trait MangaPipelineExecutionApplicationPort' \
    crates/a4n-api-contract/src/manga.rs || \
   ! rg -n 'pub trait MangaPipelinePreviewApplicationPort' \
    crates/a4n-api-contract/src/manga.rs || \
   ! rg -n 'pub trait MangaPipelineReadApplicationPort' \
    crates/a4n-api-contract/src/manga.rs || \
   ! rg -n 'pub trait MangaPipelineCancellationApplicationPort' \
    crates/a4n-api-contract/src/manga.rs; then
    printf 'Manga pipeline split contracts are missing from the API contract\n' >&2
    exit 1
fi

if ! rg -n 'pub struct MangaPipelinePreviewStep' \
    crates/a4n-api-contract/src/manga.rs; then
    printf 'Manga pipeline preview contract is missing from the API contract\n' >&2
    exit 1
fi

if rg -n 'pub trait MangaRecord(Application|Read|Mutation)ApplicationPort|pub use .*MangaRecord(Application|Read|Mutation)ApplicationPort' \
    crates/a4n-orchestrator/src/manga; then
    printf 'Manga record application ports must not be owned or re-exported by the orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'pub trait MangaRecord(Read|Mutation)ApplicationPort' \
    crates/a4n-api-contract/src/manga.rs; then
    printf 'Manga record read/mutation contracts are missing from the API contract\n' >&2
    exit 1
fi

if rg -n '\.orchestrator\(\)|OrchestratorContext' \
    crates/a4n-runtime/src/actions/manga_records/records.rs \
    crates/a4n-runtime/src/actions/manga_records/optimization; then
    printf 'Runtime Manga record actions must consume Manga record read/mutation ports\n' >&2
    exit 1
fi

if ! rg -n 'manga_record_(read|mutation)_application\(\)' \
    crates/a4n-runtime/src/actions/manga_records/records.rs \
    crates/a4n-runtime/src/actions/manga_records/optimization; then
    printf 'Runtime Manga record actions must route through Manga record read/mutation ports\n' >&2
    exit 1
fi

if ! rg -n 'pub trait MangaRecord(Read|Mutation)ApplicationPort' \
    crates/a4n-api-contract/src/manga.rs; then
    printf 'Manga record read/mutation contracts are missing from the API contract\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator::manga::MangaPromptDownstreamCleanupPort|pub trait MangaPromptDownstreamCleanupPort' \
    crates/a4n-runtime/src crates/a4n-orchestrator/src; then
    printf 'Manga prompt cleanup must use the API contract directly, without an orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'pub trait MangaPromptDownstreamCleanupPort' \
    crates/a4n-api-contract/src/manga.rs || \
   ! rg -n 'a4n_api_contract::manga::MangaPromptDownstreamCleanupPort' \
    crates/a4n-runtime/src/actions/manga_records.rs \
    crates/a4n-orchestrator/src/manga/application.rs \
    crates/a4n-orchestrator/src/manga/prompt_optimization/execution.rs; then
    printf 'Manga prompt cleanup consumers must use the narrow API contract directly\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator::manga::MangaPromptDownstreamCleanupPort|pub trait MangaPromptDownstreamCleanupPort' \
    crates/a4n-runtime/src crates/a4n-orchestrator/src; then
    printf 'Manga prompt cleanup must use the API contract directly, without an orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'pub trait MangaPromptDownstreamCleanupPort' \
    crates/a4n-api-contract/src/manga.rs || \
   ! rg -n 'a4n_api_contract::manga::MangaPromptDownstreamCleanupPort' \
    crates/a4n-runtime/src/actions/manga_records.rs \
    crates/a4n-orchestrator/src/manga/application.rs \
    crates/a4n-orchestrator/src/manga/prompt_optimization/execution.rs; then
    printf 'Manga prompt cleanup consumers must use the narrow API contract directly\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\)|OrchestratorContext|actions::config_records::shared::require_config_admin' \
    crates/a4n-runtime/src/actions/config_records/llm/records \
    crates/a4n-runtime/src/actions/config_records/image/records.rs \
    crates/a4n-runtime/src/actions/config_records/video/records.rs \
    crates/a4n-runtime/src/actions/config_records/embedding/records.rs \
    crates/a4n-runtime/src/actions/config_records/discovery.rs; then
    printf 'Runtime config CRUD actions must consume split configuration ports instead of the concrete orchestrator or duplicate authorization\n' >&2
    exit 1
fi

if rg -n '(^|[^A-Za-z])ConfigApplicationPort($|[^A-Za-z])|config_application\(\)' \
    crates/a4n-api-contract/src/config.rs \
    crates/a4n-orchestrator/src \
    crates/a4n-runtime/src; then
    printf 'The mixed ConfigApplicationPort must remain removed\n' >&2
    exit 1
fi

if ! rg -n 'pub trait Config(Read|Mutation)ApplicationPort' \
    crates/a4n-api-contract/src/config.rs; then
    printf 'Split configuration application contracts are missing\n' >&2
    exit 1
fi

if ! rg -n 'config_(read|mutation)_application\(\)' \
    crates/a4n-runtime/src/actions/config_records/llm/records \
    crates/a4n-runtime/src/actions/config_records/image/records.rs \
    crates/a4n-runtime/src/actions/config_records/video/records.rs \
    crates/a4n-runtime/src/actions/config_records/embedding/records.rs \
    crates/a4n-runtime/src/actions/config_records/discovery.rs; then
    printf 'Runtime config CRUD actions must route through split configuration ports\n' >&2
    exit 1
fi

for connectivity_file in \
    crates/a4n-runtime/src/actions/config_records/llm/connectivity.rs \
    crates/a4n-runtime/src/actions/config_records/embedding/connectivity.rs; do
    if rg -n 'state\.orchestrator\(\)\.(llm|embedding)_config|state\.orchestrator\(\)\.config_' \
        "$connectivity_file"; then
        printf '%s must not read persisted provider config through the concrete orchestrator\n' \
            "$connectivity_file" >&2
        exit 1
    fi
    if ! rg -n 'config_read_application\(\)' "$connectivity_file"; then
        printf '%s must resolve persisted provider config through ConfigReadApplicationPort\n' \
            "$connectivity_file" >&2
        exit 1
    fi
done

if rg -n 'state\.orchestrator\(\)|OrchestratorContext' \
    crates/a4n-runtime/src/actions/config_records/embedding/connectivity.rs; then
    printf 'Embedding connectivity must consume the provider runtime queue boundary\n' >&2
    exit 1
fi

if ! rg -n 'provider_runtime\(\)' \
    crates/a4n-runtime/src/actions/config_records/embedding/connectivity.rs; then
    printf 'Embedding connectivity must route queue tracking through ProviderRuntime\n' >&2
    exit 1
fi

if rg -U -n 'state\s*\.\s*orchestrator\(\)\s*\.\s*queue_cancel' \
    crates/a4n-runtime/src/actions/admin_records/tasks.rs; then
    printf 'Runtime admin task cancellation must consume TaskCancellationPort\n' >&2
    exit 1
fi

if ! rg -n 'task_cancellation_application\(\)' \
    crates/a4n-runtime/src/actions/admin_records/tasks.rs; then
    printf 'Runtime admin task cancellation must route through the injected task cancellation port\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator::ServiceHealthApplicationPort|state\.orchestrator\(\)|OrchestratorContext' \
    crates/a4n-runtime/src/actions/admin_records/tasks.rs; then
    printf 'Runtime service health must consume ServiceHealthApplicationPort from the API contract\n' >&2
    exit 1
fi

if ! rg -n 'service_health_application\(\)' \
    crates/a4n-runtime/src/actions/admin_records/tasks.rs \
    crates/a4n-runtime/src/state/app.rs; then
    printf 'Runtime service health must route through the injected service health port\n' >&2
    exit 1
fi

if rg -n 'pub trait ServiceHealthApplicationPort|pub use .*ServiceHealthApplicationPort' \
    crates/a4n-orchestrator/src; then
    printf 'ServiceHealthApplicationPort must not be owned or re-exported by the orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ServiceHealthApplicationPort' \
    crates/a4n-api-contract/src/service_health.rs; then
    printf 'ServiceHealthApplicationPort contract is missing from the API contract\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\)|OrchestratorContext|llm_provider_config_from_model' \
    crates/a4n-runtime/src/actions/request_preview/chat/budget.rs; then
    printf 'Runtime chat request preview must consume ProviderRuntime configuration DTOs\n' >&2
    exit 1
fi

if ! rg -n 'provider_runtime\(\)' \
    crates/a4n-runtime/src/actions/request_preview/chat/budget.rs; then
    printf 'Runtime chat request preview must route LLM configuration through ProviderRuntime\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\)|OrchestratorContext' \
    crates/a4n-runtime/src/actions/source_sync; then
    printf 'Runtime source-sync actions must consume source-sync ports from the API contract\n' >&2
    exit 1
fi

if ! rg -n 'source_sync_(read|mutation)_application\(\)' \
    crates/a4n-runtime/src/actions/source_sync \
    crates/a4n-runtime/src/state/app.rs; then
    printf 'Runtime source-sync actions must route through the injected source-sync port\n' >&2
    exit 1
fi

if rg -n 'pub trait SourceSync(Application|Read|Mutation)ApplicationPort|pub use .*SourceSync(Application|Read|Mutation)ApplicationPort' \
    crates/a4n-orchestrator/src; then
    printf 'source-sync application ports must not be owned or re-exported by the orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'pub trait SourceSync(Read|Mutation)ApplicationPort' \
    crates/a4n-api-contract/src/source_sync.rs; then
    printf 'source-sync read/mutation contracts are missing from the API contract\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\)|OrchestratorContext' \
    crates/a4n-runtime/src/actions/config_records/image/connectivity.rs; then
    printf 'Image connectivity must consume the provider runtime queue boundary\n' >&2
    exit 1
fi

if ! rg -n 'provider_runtime\(\)' \
    crates/a4n-runtime/src/actions/config_records/image/connectivity.rs; then
    printf 'Image connectivity must route queue tracking through ProviderRuntime\n' >&2
    exit 1
fi

if ! rg -U -n \
    '(?s)pub struct ImageGenerationProviderInput\s*\{.*?pub\s+config:\s*(a4n_provider_contract::)?ImageProviderConfig\b' \
    crates/a4n-orchestrator/src/media/image_generation/contracts.rs; then
    printf 'ImageGenerationProviderInput.config must use ImageProviderConfig instead of the image ORM model\n' >&2
    exit 1
fi

image_provider_sidecar_files=(
    crates/a4n-runtime/src/sidecar_clients/image/contracts.rs
    crates/a4n-runtime/src/sidecar_clients/image/operations/routing.rs
    crates/a4n-runtime/src/sidecar_clients/image/operations/sidecar.rs
)
for image_provider_sidecar_file in "${image_provider_sidecar_files[@]}"; do
    if rg -n \
        '[[:alnum:]_]+[[:space:]]*:[[:space:]]*&?[[:space:]]*(a4n_db_models::models::ai::)?image_gen_config::Model\b' \
        "$image_provider_sidecar_file"; then
        printf '%s must not accept the image ORM model at the provider sidecar boundary\n' \
            "$image_provider_sidecar_file" >&2
        exit 1
    fi
    if ! rg -n \
        '[[:alnum:]_]+[[:space:]]*:[[:space:]]*&[[:space:]]*(a4n_provider_contract::)?ImageProviderConfig\b' \
        "$image_provider_sidecar_file"; then
        printf '%s must accept ImageProviderConfig at the provider sidecar boundary\n' \
            "$image_provider_sidecar_file" >&2
        exit 1
    fi
done

if rg -n 'state\.orchestrator\(\)\.(llm_config_by_id|embedding_config_by_id)' \
    crates/a4n-runtime/src/actions/config_records/llm/connectivity.rs \
    crates/a4n-runtime/src/actions/config_records/embedding/connectivity.rs; then
    printf 'LLM/Embedding connectivity actions must not duplicate concrete config lookups\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\)|OrchestratorContext' \
    crates/a4n-runtime/src/actions/embedding_model/download.rs; then
    printf 'Embedding model download must consume the provider runtime queue boundary\n' >&2
    exit 1
fi

if ! rg -n 'provider_runtime\(\)' \
    crates/a4n-runtime/src/actions/embedding_model/download.rs; then
    printf 'Embedding model download must route queue tracking through ProviderRuntime\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\)|OrchestratorContext' \
    crates/a4n-runtime/src/actions/prompt_records.rs; then
    printf 'Runtime prompt actions must consume prompt read/mutation ports instead of the concrete orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'pub trait Prompt(Read|Mutation)ApplicationPort' crates/a4n-orchestrator/src/prompt_records.rs; then
    printf 'Prompt read/mutation contracts are missing\n' >&2
    exit 1
fi

if ! rg -n 'prompt_read_application\(\)' crates/a4n-runtime/src/actions/prompt_records.rs; then
    printf 'Runtime prompt reads must route through PromptReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'prompt_mutation_application\(\)' crates/a4n-runtime/src/actions/prompt_records.rs; then
    printf 'Runtime prompt mutations must route through PromptMutationApplicationPort\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator::prompt_records::PromptExecutionContext|pub struct PromptExecutionContext' \
    crates/a4n-runtime/src crates/a4n-orchestrator/src; then
    printf 'PromptExecutionContext must not be owned or re-exported by the orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'pub struct PromptExecutionContext' crates/a4n-api-contract/src/prompt.rs || \
   ! rg -n 'PromptExecutionContext' \
    crates/a4n-runtime/src/actions/prompt_records.rs \
    crates/a4n-orchestrator/src/prompt_records.rs; then
    printf 'Prompt consumers must use PromptExecutionContext from the API contract\n' >&2
    exit 1
fi

if rg -n '\.orchestrator\(\)|OrchestratorContext' \
    crates/a4n-runtime/src/actions/galgame_records/mutations.rs \
    crates/a4n-runtime/src/actions/galgame_records/media/binding.rs \
    crates/a4n-runtime/src/actions/galgame_records/media/audio.rs; then
    printf 'Runtime Galgame record actions must consume Galgame mutation ports\n' >&2
    exit 1
fi

if ! rg -n 'galgame_record_mutation_application\(\)' \
    crates/a4n-runtime/src/actions/galgame_records/mutations.rs || \
   ! rg -n 'galgame_media_mutation_application\(\)' \
    crates/a4n-runtime/src/actions/galgame_records/media/binding.rs \
    crates/a4n-runtime/src/actions/galgame_records/media/audio.rs; then
    printf 'Runtime Galgame record actions must route through the Galgame mutation ports\n' >&2
    exit 1
fi

if rg -n 'pub trait Galgame(Record|Media)MutationApplicationPort|pub use .*Galgame(Record|Media)MutationApplicationPort' \
    crates/a4n-orchestrator/src/galgame; then
    printf 'Galgame mutation ports must not be owned or re-exported by the orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'pub trait Galgame(Record|Media)MutationApplicationPort' \
    crates/a4n-api-contract/src/galgame.rs; then
    printf 'Galgame record/media mutation contracts are missing from the API contract\n' >&2
    exit 1
fi

if rg -n '\.orchestrator\(\)|OrchestratorContext' \
    crates/a4n-runtime/src/actions/galgame_records/webgal_export.rs; then
    printf 'Runtime WebGAL export must consume ExportApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'export_application\(\)' \
    crates/a4n-runtime/src/actions/galgame_records/webgal_export.rs; then
    printf 'Runtime WebGAL export must route through ExportApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait Package(Read|Mutation)ApplicationPort' crates/a4n-api-contract/src/package.rs; then
    printf 'Package read/mutation contracts are missing\n' >&2
    exit 1
fi

if rg -n 'PackageApplicationPort|package_application\(\)' \
    crates/a4n-api-contract/src/package.rs crates/a4n-orchestrator/src crates/a4n-runtime/src; then
    printf 'The mixed PackageApplicationPort must remain removed\n' >&2
    exit 1
fi

if ! rg -U -n 'package_read_application\(\)\s*\.\s*execution_enabled\(package_id\)' \
    crates/a4n-runtime/src/actions/package.rs; then
    printf 'Runtime package execution admission must consume PackageReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -U -n 'package_mutation_application\(\)\s*\.\s*ensure_execution_enabled\(package_id\)' \
    crates/a4n-runtime/src/actions/package.rs; then
    printf 'Runtime package execution guard must consume PackageMutationApplicationPort\n' >&2
    exit 1
fi

if ! rg -U -n 'package_mutation_application\(\)\s*\.\s*start_lifecycle\(' crates/a4n-runtime/src/actions/package.rs; then
    printf 'Runtime package lifecycle start must consume PackageMutationApplicationPort\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\)\.start_package_lifecycle' \
    crates/a4n-runtime/src/actions/package.rs; then
    printf 'Runtime package lifecycle start must not call the concrete orchestrator\n' >&2
    exit 1
fi

if ! rg -n 'package_read_application\(\)\.owned_services\(package_id\)' \
    crates/a4n-runtime/src/actions/package.rs; then
    printf 'Runtime package service startup must consume PackageReadApplicationPort\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\)\.package_service_names' \
    crates/a4n-runtime/src/actions/package.rs; then
    printf 'Runtime package service startup must not call the concrete orchestrator\n' >&2
    exit 1
fi

if ! rg -U -n 'package_read_application\(\)\s*\.\s*has_workload\(package_id\)' \
    crates/a4n-runtime/src/actions/package.rs; then
    printf 'Runtime package lifecycle workload checks must consume PackageReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -U -n 'package_read_application\(\)\s*\.\s*services_safe_to_stop\(package_id\)' \
    crates/a4n-runtime/src/actions/package.rs; then
    printf 'Runtime package service shutdown checks must consume PackageReadApplicationPort\n' >&2
    exit 1
fi

if rg -U -n 'state\s*\.\s*orchestrator\(\)\s*\.\s*package_services_safe_to_stop' \
    crates/a4n-runtime/src/actions/package.rs; then
    printf 'Runtime package service shutdown checks must not call the concrete orchestrator\n' >&2
    exit 1
fi

if ! rg -U -n 'package_mutation_application\(\)\s*\.\s*finalize_disable\(operation_id\)' \
    crates/a4n-runtime/src/actions/package.rs; then
    printf 'Runtime package disable finalization must consume PackageMutationApplicationPort\n' >&2
    exit 1
fi

if rg -U -n 'state\s*\.\s*orchestrator\(\)\s*\.\s*finalize_package_disable' \
    crates/a4n-runtime/src/actions/package.rs; then
    printf 'Runtime package disable finalization must not call the concrete orchestrator\n' >&2
    exit 1
fi

if ! rg -U -n 'package_read_application\(\)\s*\.\s*pending_lifecycle_operations\(\)' \
    crates/a4n-runtime/src/actions/package.rs; then
    printf 'Runtime package lifecycle recovery must consume PackageReadApplicationPort\n' >&2
    exit 1
fi

if rg -U -n 'state\s*\.\s*orchestrator\(\)\s*\.\s*pending_package_lifecycle_operations' \
    crates/a4n-runtime/src/actions/package.rs; then
    printf 'Runtime package lifecycle recovery must not call the concrete orchestrator\n' >&2
    exit 1
fi

if ! rg -n 'package_read_application\(\)\.enabled_services\(\)' \
    crates/a4n-runtime/src/state/app.rs; then
    printf 'Runtime package startup service discovery must consume PackageReadApplicationPort\n' >&2
    exit 1
fi

if rg -U -n 'state\s*\.\s*orchestrator\(\)\s*\.\s*enabled_package_service_names' \
    crates/a4n-runtime/src/state/app.rs; then
    printf 'Runtime package startup service discovery must not call the concrete orchestrator\n' >&2
    exit 1
fi

if ! rg -U -n 'package_mutation_application\(\)\s*\.\s*advance_lifecycle\(operation_id\)' \
    crates/a4n-runtime/src/actions/package.rs; then
    printf 'Runtime package lifecycle advancement must consume PackageMutationApplicationPort\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\)\.package_workload_snapshot' \
    crates/a4n-runtime/src/actions/package.rs; then
    printf 'Runtime package lifecycle workload checks must not call the concrete orchestrator\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\)\.(package_execution_enabled|ensure_package_execution_enabled)' \
    crates/a4n-runtime/src/actions/package.rs; then
    printf 'Runtime package execution admission must not call the concrete orchestrator\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator|OrchestratorContext|state\.orchestrator\(\)' \
    crates/a4n-runtime/src/actions/auth_records/users.rs \
    crates/a4n-runtime/src/actions/auth_records/sessions.rs \
    crates/a4n-runtime/src/actions/auth_records/tokens.rs \
    crates/a4n-runtime/src/actions/oauth/flow.rs \
    crates/a4n-runtime/src/actions/theme; then
    printf 'Runtime auth/theme actions must consume auth read/mutation ports and Theme read/mutation ports\n' >&2
    exit 1
fi

if ! rg -n 'pub trait Auth(Read|Mutation)ApplicationPort' crates/a4n-api-contract/src/auth.rs; then
    printf 'Auth read/mutation contracts are missing\n' >&2
    exit 1
fi

if rg -n 'AuthApplicationPort|auth_application\(\)' \
    crates/a4n-api-contract/src/auth.rs crates/a4n-orchestrator/src crates/a4n-runtime/src; then
    printf 'The mixed AuthApplicationPort must remain removed\n' >&2
    exit 1
fi

if ! rg -n 'auth_read_application\(\)' \
    crates/a4n-runtime/src/actions/auth_records/users.rs \
    crates/a4n-runtime/src/actions/auth_records/sessions.rs \
    crates/a4n-runtime/src/actions/auth_records/tokens.rs; then
    printf 'Runtime auth reads must route through AuthReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'auth_mutation_application\(\)' \
    crates/a4n-runtime/src/actions/auth_records/users.rs \
    crates/a4n-runtime/src/actions/auth_records/sessions.rs \
    crates/a4n-runtime/src/actions/auth_records/tokens.rs \
    crates/a4n-runtime/src/actions/oauth/flow.rs; then
    printf 'Runtime auth mutations must route through AuthMutationApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait Theme(Read|Mutation)ApplicationPort' crates/a4n-api-contract/src/theme.rs; then
    printf 'Theme read/mutation contracts are missing\n' >&2
    exit 1
fi

if rg -n 'ThemeApplicationPort|theme_application\(\)' \
    crates/a4n-api-contract/src/theme.rs crates/a4n-orchestrator/src crates/a4n-runtime/src; then
    printf 'The mixed ThemeApplicationPort must remain removed\n' >&2
    exit 1
fi

if ! rg -n 'theme_read_application\(\)' \
    crates/a4n-runtime/src/actions/auth_records/tokens.rs \
    crates/a4n-runtime/src/actions/theme; then
    printf 'Runtime theme reads must route through ThemeReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'theme_mutation_application\(\)' \
    crates/a4n-runtime/src/actions/theme; then
    printf 'Runtime theme mutations must route through ThemeMutationApplicationPort\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator|OrchestratorContext|state\.orchestrator\(\)' \
    crates/a4n-runtime/src/actions/admin_records/permissions.rs \
    crates/a4n-runtime/src/actions/admin_records/projects.rs \
    crates/a4n-runtime/src/actions/admin_records/usage.rs \
    crates/a4n-runtime/src/actions/admin_records/users.rs \
    crates/a4n-runtime/src/actions/generation_analytics.rs; then
    printf 'Runtime administrative data actions must consume admin read/mutation ports instead of the concrete orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'pub trait Admin(Read|Mutation)ApplicationPort' crates/a4n-api-contract/src/admin.rs; then
    printf 'Admin read/mutation contracts are missing\n' >&2
    exit 1
fi

if rg -n 'AdminApplicationPort|admin_application\(\)' \
    crates/a4n-api-contract/src/admin.rs crates/a4n-orchestrator/src crates/a4n-runtime/src; then
    printf 'The mixed AdminApplicationPort must remain removed\n' >&2
    exit 1
fi

if ! rg -n 'admin_read_application\(\)' \
    crates/a4n-runtime/src/actions/admin_records \
    crates/a4n-runtime/src/actions/generation_analytics.rs; then
    printf 'Runtime admin reads must route through AdminReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'admin_mutation_application\(\)' \
    crates/a4n-runtime/src/actions/admin_records/users.rs; then
    printf 'Runtime admin mutations must route through AdminMutationApplicationPort\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator|OrchestratorContext|state\.orchestrator\(\)' \
    crates/a4n-runtime/src/actions/project_status.rs; then
    printf 'Runtime project status actions must consume ProjectStatusApplicationPort instead of persisted project models\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ProjectStatusApplicationPort' crates/a4n-api-contract/src/project.rs; then
    printf 'ProjectStatusApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator|OrchestratorContext|state\.orchestrator\(\)' \
    crates/a4n-runtime/src/actions/project_collaboration.rs; then
    printf 'Runtime project collaboration actions must consume ProjectCollaborationApplicationPort instead of the concrete orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ProjectCollaborationApplicationPort' crates/a4n-api-contract/src/collaboration.rs; then
    printf 'ProjectCollaborationApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n '\.(verify_project_permission|verify_project_permissions|collaboration_root_project)\(|\.project_ids_with_permission\(\s*&' \
    crates/a4n-runtime/src --glob '*.rs'; then
    printf 'Runtime must consume ProjectAccessApplicationPort for project permission and root resolution\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ProjectAccessApplicationPort' crates/a4n-api-contract/src/collaboration.rs; then
    printf 'ProjectAccessApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'novel_project::Model|AccessibleProject(List|Record)|ProjectAccessInfo' \
    crates/a4n-runtime/src/actions/project_records/api/queries.rs; then
    printf 'Runtime project list/summary actions must not assemble persisted project projections\n' >&2
    exit 1
fi

if ! rg -n 'project_read_application\(\)' \
    crates/a4n-runtime/src/actions/project_records/api/queries.rs; then
    printf 'Runtime project list/summary actions must consume ProjectReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ProjectReadApplicationPort' crates/a4n-api-contract/src/project.rs; then
    printf 'ProjectReadApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator|OrchestratorContext|state\.orchestrator\(\)' \
    crates/a4n-runtime/src/actions/chapter_scene/records.rs; then
    printf 'Runtime chapter scene record actions must consume ChapterSceneApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ChapterSceneApplicationPort' crates/a4n-api-contract/src/chapter.rs; then
    printf 'ChapterSceneApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator::chapter_scene::ChapterSceneProgressReporter|pub type ChapterSceneProgressReporter' \
    crates/a4n-runtime/src crates/a4n-server/src crates/a4n-tauri/src \
    crates/a4n-orchestrator/src/chapter_scene; then
    printf 'Chapter-scene progress callbacks must be consumed from a4n-api-contract\n' >&2
    exit 1
fi

if ! rg -n 'pub type ChapterSceneProgressReporter' crates/a4n-api-contract/src/chapter.rs; then
    printf 'Chapter-scene progress callback contract is missing from a4n-api-contract\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator|OrchestratorContext|state\.orchestrator\(\)' \
    crates/a4n-runtime/src/actions/chapter_content/queries.rs; then
    printf 'Runtime chapter content read actions must consume ChapterContentReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'chapter_content_read_application\(\)' \
    crates/a4n-runtime/src/actions/chapter_content/queries.rs \
    crates/a4n-runtime/src/workflow/chapter_analysis/api; then
    printf 'Runtime chapter content summaries must consume ChapterContentReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ChapterContentReadApplicationPort' crates/a4n-api-contract/src/chapter.rs; then
    printf 'ChapterContentReadApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator|OrchestratorContext|state\.orchestrator\(\)' \
    crates/a4n-runtime/src/actions/chapter_content/edit.rs \
    crates/a4n-runtime/src/actions/chapter_content/import.rs; then
    printf 'Runtime chapter content mutations must consume ChapterContentMutationApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'chapter_content_mutation_application\(\)' \
    crates/a4n-runtime/src/actions/chapter_content/edit.rs \
    crates/a4n-runtime/src/actions/chapter_content/import.rs; then
    printf 'Runtime chapter content mutations must route through the mutation port\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ChapterContentMutationApplicationPort' \
    crates/a4n-api-contract/src/chapter.rs; then
    printf 'ChapterContentMutationApplicationPort contract is missing from the API contract\n' >&2
    exit 1
fi
if rg -n 'pub trait ChapterContentMutationApplicationPort' \
    crates/a4n-orchestrator/src/chapter_content; then
    printf 'ChapterContentMutationApplicationPort must not be redefined in the orchestrator\n' >&2
    exit 1
fi
if rg -n 'a4n_orchestrator::chapter_content::ChapterContentMutationApplicationPort' \
    crates/a4n-runtime/src; then
    printf 'Runtime chapter content mutations must consume the API contract directly\n' >&2
    exit 1
fi

if ! rg -n 'chapter_content_sync_application\(\)' \
    crates/a4n-runtime/src/actions/chapter_content/edit.rs \
    crates/a4n-runtime/src/actions/optimize/patches/host_port.rs \
    crates/a4n-runtime/src/workflow/chapter_cleanup.rs; then
    printf 'Runtime chapter content synchronization must route through ChapterContentSyncApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ChapterContentSyncApplicationPort' \
    crates/a4n-api-contract/src/chapter.rs; then
    printf 'ChapterContentSyncApplicationPort contract is missing from the API contract\n' >&2
    exit 1
fi
if rg -n 'pub trait ChapterContentSyncApplicationPort' \
    crates/a4n-orchestrator/src/chapter_content; then
    printf 'ChapterContentSyncApplicationPort must not be redefined in the orchestrator\n' >&2
    exit 1
fi
if rg -n 'a4n_orchestrator::chapter_content::ChapterContentSyncApplicationPort' \
    crates/a4n-runtime/src; then
    printf 'Runtime chapter content synchronization must consume the API contract directly\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ChapterDerivedCleanupHostPort' \
    crates/a4n-api-contract/src/chapter.rs; then
    printf 'ChapterDerivedCleanupHostPort contract is missing from the API contract\n' >&2
    exit 1
fi
if rg -n 'pub trait ChapterDerivedCleanupHostPort' \
    crates/a4n-orchestrator/src/chapter_content; then
    printf 'ChapterDerivedCleanupHostPort must not be redefined in the orchestrator\n' >&2
    exit 1
fi
if rg -n 'a4n_orchestrator::chapter_content::ChapterDerivedCleanupHostPort' \
    crates/a4n-runtime/src; then
    printf 'Runtime chapter cleanup adapters must consume the API contract directly\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator|OrchestratorContext|state\.orchestrator\(\)' \
    crates/a4n-runtime/src/actions/outline_records/chapters/records.rs \
    crates/a4n-runtime/src/actions/outline_records/parts/records.rs; then
    printf 'Runtime outline read actions must consume OutlineReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'outline_read_application\(\)' \
    crates/a4n-runtime/src/actions/outline_records/chapters/records.rs \
    crates/a4n-runtime/src/actions/outline_records/parts/records.rs; then
    printf 'Runtime outline read actions must route through OutlineReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait OutlineReadApplicationPort' crates/a4n-api-contract/src/outline.rs; then
    printf 'OutlineReadApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator|OrchestratorContext|state\.orchestrator\(\)' \
    crates/a4n-runtime/src/actions/outline_records/chapters/records.rs \
    crates/a4n-runtime/src/actions/outline_records/parts/records.rs; then
    printf 'Runtime outline mutation actions must consume OutlineMutationApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'outline_mutation_application\(\)' \
    crates/a4n-runtime/src/actions/outline_records/chapters/records.rs \
    crates/a4n-runtime/src/actions/outline_records/parts/records.rs; then
    printf 'Runtime outline mutation actions must route through OutlineMutationApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait OutlineMutationApplicationPort' crates/a4n-api-contract/src/outline.rs; then
    printf 'OutlineMutationApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator|OrchestratorContext|state\.orchestrator\(\)' \
    crates/a4n-runtime/src/actions/object_profile_records/queries.rs \
    crates/a4n-runtime/src/actions/scene_profile_records/queries.rs; then
    printf 'Runtime profile list actions must consume profile read ports instead of the concrete orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ObjectProfileReadApplicationPort' crates/a4n-api-contract/src/profile.rs; then
    printf 'ObjectProfileReadApplicationPort contract is missing\n' >&2
    exit 1
fi

if ! rg -n 'pub trait SceneProfileReadApplicationPort' crates/a4n-api-contract/src/profile.rs; then
    printf 'SceneProfileReadApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'profiles_with_timeline_state' \
    crates/a4n-runtime/src/actions/character_records/profiles.rs; then
    printf 'Runtime character profile list must consume CharacterProfileReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'character_profile_read_application\(\)' \
    crates/a4n-runtime/src/actions/character_records/profiles.rs; then
    printf 'Runtime character profile list must route through CharacterProfileReadApplicationPort\n' >&2
    exit 1
fi

if rg -n 'require_profile_for_user|state\.orchestrator\(\)|delete_character_profile_records_validated|CharacterRecordLifecycleError' \
    crates/a4n-runtime/src/actions/character_records/profiles.rs; then
    printf 'Runtime character profile deletion must not use ORM/profile helpers or the concrete lifecycle result\n' >&2
    exit 1
fi

if ! rg -n 'character_profile_deletion_application\(\)' \
    crates/a4n-runtime/src/actions/character_records/profiles.rs; then
    printf 'Runtime character profile deletion must route through CharacterProfileDeletionApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait CharacterProfileDeletionApplicationPort' \
    crates/a4n-api-contract/src/character.rs; then
    printf 'CharacterProfileDeletionApplicationPort contract is missing\n' >&2
    exit 1
fi

if ! rg -n 'pub trait CharacterProfileReadApplicationPort' \
    crates/a4n-api-contract/src/character.rs; then
    printf 'CharacterProfileReadApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'require_profile_for_user|state\.orchestrator\(\)|deletion_marks_by_profile_validated|deletion_mark_summary|CharacterAttributeDeletion(Execution|Request)Input' \
    crates/a4n-runtime/src/actions/character_records/deletion_marks.rs; then
    printf 'Runtime character deletion-mark actions must consume character read/deletion ports and owner DTO projection\n' >&2
    exit 1
fi

if ! rg -n 'character_profile_read_application\(\)' \
    crates/a4n-runtime/src/actions/character_records/deletion_marks.rs || \
   ! rg -n '\.deletion_marks\(user_id, input\)' \
    crates/a4n-runtime/src/actions/character_records/deletion_marks.rs; then
    printf 'Runtime character deletion-mark reads must route through CharacterProfileReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'character_profile_attribute_deletion_application\(\)' \
    crates/a4n-runtime/src/actions/character_records/deletion_marks.rs; then
    printf 'Runtime character deletion-mark mutations must route through CharacterProfileAttributeDeletionApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait CharacterProfileAttributeDeletionApplicationPort' \
    crates/a4n-api-contract/src/character.rs; then
    printf 'CharacterProfileAttributeDeletionApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'require_profile_for_user|state\.orchestrator\(\)|CharacterTimelineRollbackLifecycleResult' \
    crates/a4n-runtime/src/actions/character_records/snapshots/rollback.rs; then
    printf 'Runtime character timeline rollback must not use ORM/profile helpers or the concrete orchestrator lifecycle result\n' >&2
    exit 1
fi

if ! rg -n 'character_profile_timeline_mutation_application\(\)' \
    crates/a4n-runtime/src/actions/character_records/snapshots/rollback.rs; then
    printf 'Runtime character timeline rollback must route through CharacterProfileTimelineMutationApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait CharacterProfileTimelineMutationApplicationPort' \
    crates/a4n-api-contract/src/character.rs; then
    printf 'CharacterProfileTimelineMutationApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'require_profile_for_user|state\.orchestrator\(\)|CharacterAttributeValueChangeInput' \
    crates/a4n-runtime/src/actions/character_records/attributes.rs; then
    printf 'Runtime character attribute mutations must not use ORM/profile helpers, domain mutation inputs, or the concrete orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'character_profile_attribute_mutation_application\(\)' \
    crates/a4n-runtime/src/actions/character_records/attributes.rs; then
    printf 'Runtime character attribute mutations must route through CharacterProfileAttributeMutationApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait CharacterProfileAttributeMutationApplicationPort' \
    crates/a4n-api-contract/src/character.rs; then
    printf 'CharacterProfileAttributeMutationApplicationPort contract is missing\n' >&2
    exit 1
fi

if ! rg -n 'impl CharacterProfileReadApplicationPort for OrchestratorContext' \
    crates/a4n-orchestrator/src/context/character.rs; then
    printf 'CharacterProfileReadApplicationPort owner implementation is missing\n' >&2
    exit 1
fi

if rg -n 'verify_project_editor_string' \
    crates/a4n-runtime/src/actions/character_records/profiles.rs; then
    printf 'Runtime character profile reads must leave Editor authorization in CharacterProfileReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n '\.list\(user_id, input\.project_id\)' \
    crates/a4n-runtime/src/actions/character_records/profiles.rs; then
    printf 'Runtime character profile list must pass identity through CharacterProfileReadApplicationPort\n' >&2
    exit 1
fi

if rg -n 'find_profile_by_name_for_user|profile_by_project_name' \
    crates/a4n-runtime/src/actions/character_records/profiles.rs; then
    printf 'Runtime character profile field reads must consume CharacterProfileReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'character_arc_by_name|voice_fingerprint_by_name' \
    crates/a4n-runtime/src/actions/character_records/profiles.rs; then
    printf 'Runtime character profile field reads must route through the character read port\n' >&2
    exit 1
fi

if rg -n 'create_profile_validated|update_initial_profile_validated' \
    crates/a4n-runtime/src/actions/character_records/profiles.rs; then
    printf 'Runtime character profile create/update actions must consume CharacterProfileMutationApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'character_profile_mutation_application\(\)' \
    crates/a4n-runtime/src/actions/character_records/profiles.rs; then
    printf 'Runtime character profile create/update actions must route through CharacterProfileMutationApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait CharacterProfileMutationApplicationPort' \
    crates/a4n-api-contract/src/character.rs; then
    printf 'CharacterProfileMutationApplicationPort contract is missing\n' >&2
    exit 1
fi

if ! rg -n 'impl CharacterProfileMutationApplicationPort for OrchestratorContext' \
    crates/a4n-orchestrator/src/context/character.rs; then
    printf 'CharacterProfileMutationApplicationPort owner implementation is missing\n' >&2
    exit 1
fi

if rg -n 'CharacterAttributeEventHistoryFilter|attribute_event_summary' \
    crates/a4n-runtime/src/actions/character_records/attributes.rs; then
    printf 'Runtime character attribute history must consume the character read port and owner DTO projection\n' >&2
    exit 1
fi

if ! rg -n 'character_profile_read_application\(\)' \
    crates/a4n-runtime/src/actions/character_records/attributes.rs; then
    printf 'Runtime character attribute history must route through CharacterProfileReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n '\.attribute_history\(user_id, input\)' \
    crates/a4n-runtime/src/actions/character_records/attributes.rs; then
    printf 'Runtime character attribute history must pass identity and input through the read port\n' >&2
    exit 1
fi

if rg -n 'behavior_record_summary|behavior_records_by_profile' \
    crates/a4n-runtime/src/actions/character_records/behaviors.rs; then
    printf 'Runtime character behavior history must consume the character read port and owner DTO projection\n' >&2
    exit 1
fi

if ! rg -n 'character_profile_read_application\(\)' \
    crates/a4n-runtime/src/actions/character_records/behaviors.rs || \
   ! rg -n '\.behavior_history\(user_id, input\)' \
    crates/a4n-runtime/src/actions/character_records/behaviors.rs; then
    printf 'Runtime character behavior history must route through CharacterProfileReadApplicationPort\n' >&2
    exit 1
fi

if rg -n 'snapshot_references_for_user_validated|snapshot_reference_summary' \
    crates/a4n-runtime/src/actions/character_records/snapshots.rs; then
    printf 'Runtime character snapshot lists must consume the character read port and owner DTO projection\n' >&2
    exit 1
fi

if ! rg -n 'character_profile_read_application\(\)' \
    crates/a4n-runtime/src/actions/character_records/snapshots.rs || \
   ! rg -n '\.snapshots\(user_id, input\)' \
    crates/a4n-runtime/src/actions/character_records/snapshots.rs; then
    printf 'Runtime character snapshot lists must route through CharacterProfileReadApplicationPort\n' >&2
    exit 1
fi

if rg -n 'profile_snapshot_for_user_at_chapter_validated|profile_state_summary_from_snapshot' \
    crates/a4n-runtime/src/actions/character_records/snapshots.rs; then
    printf 'Runtime character snapshot state reads must consume the character read port and owner DTO projection\n' >&2
    exit 1
fi

if ! rg -n 'character_profile_read_application\(\)' \
    crates/a4n-runtime/src/actions/character_records/snapshots.rs || \
   ! rg -n '\.state_at_chapter\(user_id, input\)' \
    crates/a4n-runtime/src/actions/character_records/snapshots.rs; then
    printf 'Runtime character snapshot state reads must route through CharacterProfileReadApplicationPort\n' >&2
    exit 1
fi

if rg -n 'character_snapshot_for_user_by_hash_validated|character_snapshot_by_hash' \
    crates/a4n-runtime/src/actions/character_records/snapshots.rs; then
    printf 'Runtime character snapshot hash reads must consume the character read port\n' >&2
    exit 1
fi

if ! rg -n 'character_profile_read_application\(\)' \
    crates/a4n-runtime/src/actions/character_records/snapshots.rs || \
   ! rg -n '\.snapshot_by_hash\(user_id, input\)' \
    crates/a4n-runtime/src/actions/character_records/snapshots.rs; then
    printf 'Runtime character snapshot hash reads must route through CharacterProfileReadApplicationPort\n' >&2
    exit 1
fi

if rg -n 'diff_profile_snapshots_for_user_validated|diff_profile_snapshots_for_user' \
    crates/a4n-runtime/src/actions/character_records/snapshots.rs; then
    printf 'Runtime character snapshot diffs must consume the character read port\n' >&2
    exit 1
fi

if ! rg -n 'character_profile_read_application\(\)' \
    crates/a4n-runtime/src/actions/character_records/snapshots.rs || \
   ! rg -n '\.diff_chapters\(user_id, input\)' \
    crates/a4n-runtime/src/actions/character_records/snapshots.rs; then
    printf 'Runtime character snapshot diffs must route through CharacterProfileReadApplicationPort\n' >&2
    exit 1
fi

if rg -n 'character_sync_conflict_for_user_validated|CharacterSyncConflictReadModel' \
    crates/a4n-runtime/src/actions/character_records/snapshots.rs; then
    printf 'Runtime character sync conflict reads must consume the character read port\n' >&2
    exit 1
fi

if ! rg -n 'character_profile_read_application\(\)' \
    crates/a4n-runtime/src/actions/character_records/snapshots.rs || \
   ! rg -n '\.sync_conflict\(user_id, input\)' \
    crates/a4n-runtime/src/actions/character_records/snapshots.rs; then
    printf 'Runtime character sync conflict reads must route through CharacterProfileReadApplicationPort\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator|OrchestratorContext|state\.orchestrator\(\)' \
    crates/a4n-runtime/src/actions/object_profile_records/mutations.rs \
    crates/a4n-runtime/src/actions/scene_profile_records/mutations.rs; then
    printf 'Runtime profile mutation actions must consume profile mutation ports instead of the concrete orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'object_profile_mutation_application\(\)' \
    crates/a4n-runtime/src/actions/object_profile_records/mutations.rs; then
    printf 'Runtime object profile mutation actions must route through ObjectProfileMutationApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'scene_profile_mutation_application\(\)' \
    crates/a4n-runtime/src/actions/scene_profile_records/mutations.rs; then
    printf 'Runtime scene profile mutation actions must route through SceneProfileMutationApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ObjectProfileMutationApplicationPort' crates/a4n-api-contract/src/profile.rs; then
    printf 'ObjectProfileMutationApplicationPort contract is missing\n' >&2
    exit 1
fi

if ! rg -n 'pub trait SceneProfileMutationApplicationPort' crates/a4n-api-contract/src/profile.rs; then
    printf 'SceneProfileMutationApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\)|OrchestratorContext' \
    crates/a4n-runtime/src/actions/profile_materials/list.rs \
    crates/a4n-runtime/src/actions/profile_materials/primary.rs \
    crates/a4n-runtime/src/actions/profile_materials/upload/api.rs; then
    printf 'Runtime profile-material actions must consume ProfileMaterialApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'profile_material_application\(\)' \
    crates/a4n-runtime/src/actions/profile_materials/list.rs \
    crates/a4n-runtime/src/actions/profile_materials/primary.rs; then
    printf 'Runtime profile-material record actions must route through the API ProfileMaterialApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'profile_material_upload_application\(\)' \
    crates/a4n-runtime/src/actions/profile_materials/upload/api.rs; then
    printf 'Runtime profile-material upload must route through ProfileMaterialUploadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ProfileMaterialApplicationPort' \
    crates/a4n-api-contract/src/profile.rs || \
   ! rg -n 'pub trait ProfileMaterialUploadApplicationPort' \
    crates/a4n-orchestrator/src/profile_material.rs; then
    printf 'Profile-material split contracts are missing\n' >&2
    exit 1
fi

if rg -n 'shooting_script_application\(\)|ShootingScriptApplicationPort' \
    crates/a4n-runtime/src crates/a4n-orchestrator/src crates/a4n-orchestrator/src/lib.rs; then
    printf 'Shooting-script records and generation must use split application ports\n' >&2
    exit 1
fi

if ! rg -n 'shooting_script_record_read_application\(\)' \
    crates/a4n-runtime/src/actions/shooting_script_records/read.rs; then
    printf 'Runtime shooting-script record reads must route through the read port\n' >&2
    exit 1
fi

if ! rg -n 'shooting_script_record_mutation_application\(\)' \
    crates/a4n-runtime/src/actions/shooting_script_records/edit.rs; then
    printf 'Runtime shooting-script record edits must route through the mutation port\n' >&2
    exit 1
fi

if ! rg -n 'shooting_script_generation_application\(\)' \
    crates/a4n-runtime/src/actions/shooting_script_records/generation.rs; then
    printf 'Runtime shooting-script generation actions must route through the generation port\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ShootingScriptRecord(Read|Mutation)ApplicationPort' \
    crates/a4n-api-contract/src/shooting_script.rs; then
    printf 'ShootingScriptRecord read/mutation contracts are missing\n' >&2
    exit 1
fi

if rg -n 'ShootingScriptRecordApplicationPort' \
    crates/a4n-api-contract/src/shooting_script.rs \
    crates/a4n-orchestrator/src crates/a4n-runtime/src; then
    printf 'The mixed ShootingScriptRecordApplicationPort must remain removed\n' >&2
    exit 1
fi

if ! rg -n 'blueprint_read_application\(\)' \
    crates/a4n-runtime/src/actions/blueprint_records/generation.rs \
    crates/a4n-runtime/src/actions/blueprint_records/versions.rs; then
    printf 'Runtime blueprint read actions must consume BlueprintReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait BlueprintReadApplicationPort' crates/a4n-api-contract/src/blueprint.rs; then
    printf 'BlueprintReadApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator|OrchestratorContext|state\.orchestrator\(\)' \
    crates/a4n-runtime/src/actions/galgame_records/queries.rs; then
    printf 'Runtime Galgame read actions must consume GalgameReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'galgame_read_application\(\)' \
    crates/a4n-runtime/src/actions/galgame_records/queries.rs; then
    printf 'Runtime Galgame read actions must route through GalgameReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait GalgameReadApplicationPort' crates/a4n-api-contract/src/galgame.rs; then
    printf 'GalgameReadApplicationPort contract is missing\n' >&2
    exit 1
fi

if ! rg -n 'galgame_asset_plan_application\(\)' \
    crates/a4n-runtime/src/actions/galgame_records/asset_generation/execution.rs; then
    printf 'Runtime Galgame asset planning must route through GalgameAssetPlanApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'galgame_asset_generation_application\(\)' \
    crates/a4n-runtime/src/actions/galgame_records/asset_generation/execution.rs; then
    printf 'Runtime Galgame asset execution must route through GalgameAssetGenerationApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait GalgameAssetPlanApplicationPort' \
    crates/a4n-api-contract/src/galgame.rs; then
    printf 'GalgameAssetPlanApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator|OrchestratorContext|state\.orchestrator\(\)' \
    crates/a4n-runtime/src/actions/inspiration_records/history.rs; then
    printf 'Runtime inspiration history action must consume InspirationApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'inspiration_application\(\)' \
    crates/a4n-runtime/src/actions/inspiration_records/history.rs; then
    printf 'Runtime inspiration history action must route through InspirationApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait InspirationApplicationPort' crates/a4n-api-contract/src/inspiration.rs; then
    printf 'InspirationApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -U -n 'state\s*\.\s*orchestrator\(\)|OrchestratorContext|start_inspiration_generation_task|run_inspiration_generation_task' \
    crates/a4n-runtime/src/actions/inspiration_records/conversations.rs; then
    printf 'Runtime inspiration generation must not call the concrete orchestrator\n' >&2
    exit 1
fi

if ! rg -n 'inspiration_generation_application\(\)' \
    crates/a4n-runtime/src/actions/inspiration_records/conversations.rs; then
    printf 'Runtime inspiration generation must route through InspirationGenerationApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait InspirationGenerationApplicationPort' \
    crates/a4n-orchestrator/src/inspiration/application.rs; then
    printf 'InspirationGenerationApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator|OrchestratorContext|state\.orchestrator\(\)|verify_project_editor' \
    crates/a4n-runtime/src/actions/inspiration_records/request.rs; then
    printf 'Runtime inspiration request preview action must consume the inspiration preview port\n' >&2
    exit 1
fi

if ! rg -n 'inspiration_request_application\(\)' \
    crates/a4n-runtime/src/actions/inspiration_records/request.rs; then
    printf 'Runtime inspiration request preview action must route through the inspiration preview port\n' >&2
    exit 1
fi

if ! rg -n 'pub trait InspirationRequestPreviewApplicationPort' \
    crates/a4n-orchestrator/src/inspiration/application.rs; then
    printf 'InspirationRequestPreviewApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'OrchestratorContext|state\.orchestrator\(\)' \
    crates/a4n-runtime/src/actions/video_generation/config_state.rs; then
    printf 'Runtime video provider config resolution must not depend on the full OrchestratorContext\n' >&2
    exit 1
fi

if ! rg -n 'video_provider_config\(\)' \
    crates/a4n-runtime/src/actions/video_generation/provider_host.rs; then
    printf 'Runtime video provider config resolution must use the injected narrow port\n' >&2
    exit 1
fi

if ! rg -n 'pub trait VideoProviderExecutionApplicationPort' \
    crates/a4n-api-contract/src/video.rs; then
    printf 'VideoProviderExecutionApplicationPort contract is missing\n' >&2
    exit 1
fi

if ! rg -n 'video_provider_execution\(\)' \
    crates/a4n-runtime/src/actions/video_generation/poll.rs; then
    printf 'Runtime video polling must route through the provider execution port\n' >&2
    exit 1
fi

if ! rg -n 'pub trait VideoProviderConfigPort' crates/a4n-api-contract/src/video.rs; then
    printf 'VideoProviderConfigPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'pub use a4n_provider_contract::VideoProviderConfig' crates/a4n-orchestrator/src/video; then
    printf 'VideoProviderConfig must not be re-exported by the orchestrator video facade\n' >&2
    exit 1
fi

if rg -n 'pub use a4n_video::.*validate_video_(config_params|max_concurrent)|pub use self::validation::.*validate_video_(config_params|max_concurrent)' \
    crates/a4n-orchestrator/src/video; then
    printf 'pure video config validators must be consumed from a4n-video, not re-exported by the orchestrator facade\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator::media::ImageProviderConfigPort|crate::media::ImageProviderConfigPort' \
    crates/a4n-runtime crates/a4n-orchestrator; then
    printf 'ImageProviderConfigPort must not be reintroduced through the orchestrator media facade\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ImageProviderConfigPort' crates/a4n-api-contract/src/image.rs; then
    printf 'ImageProviderConfigPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\).*history|TaskReadApplicationPort\s*=\s*state\.orchestrator\(\)' \
    crates/a4n-runtime/src/actions/task_records/queries/tasks.rs; then
    printf 'Runtime task history must consume TaskReadApplicationPort instead of the concrete orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'task_read_application\(\)' \
    crates/a4n-runtime/src/actions/task_records/queries/tasks.rs; then
    printf 'Runtime task history must route through TaskReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait TaskReadApplicationPort' crates/a4n-api-contract/src/task.rs; then
    printf 'TaskReadApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\).*queue_stats' \
    crates/a4n-runtime/src/actions/task_records/queries/tasks.rs; then
    printf 'Runtime queue statistics must consume TaskQueueStatsApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'task_queue_stats_application\(\)' \
    crates/a4n-runtime/src/actions/task_records/queries/tasks.rs; then
    printf 'Runtime queue statistics must route through TaskQueueStatsApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait TaskQueueStatsApplicationPort' crates/a4n-api-contract/src/task.rs; then
    printf 'TaskQueueStatsApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\)\.(get_video_timeline|commit_inserted_segment_draft|move_video_timeline_item|delete_video_timeline_item|load_video_segment_plan_draft_validated|update_video_segment_plan_draft_recipe_validated|update_chapter_video_segment_recipe_validated|update_chapter_video_prompt_validated|delete_chapter_video_prompt_validated|optimize_chapter_video_prompt_validated)' \
    crates/a4n-runtime/src/actions/video_prompts.rs; then
    printf 'Runtime video prompt actions must consume the split video prompt application ports\n' >&2
    exit 1
fi

if ! rg -n 'video_prompt_(timeline|draft|mutation)_application\(\)' \
    crates/a4n-runtime/src/actions/video_prompts.rs; then
    printf 'Runtime video prompt actions must route through the split application ports\n' >&2
    exit 1
fi

if rg -n 'pub trait VideoPromptMutationApplicationPort|pub use .*VideoPromptMutationApplicationPort' \
    crates/a4n-orchestrator/src/video; then
    printf 'video prompt application ports must not be owned or re-exported by the orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'pub trait VideoPromptMutationApplicationPort|pub trait VideoPromptTimelineApplicationPort|pub trait VideoPromptDraftApplicationPort' \
    crates/a4n-api-contract/src/video.rs; then
    printf 'video prompt application ports must live in the API contract\n' >&2
    exit 1
fi

if ! rg -n 'pub trait VideoPromptTimelineApplicationPort|pub trait VideoPromptDraftApplicationPort' \
    crates/a4n-api-contract/src/video.rs; then
    printf 'video prompt timeline/draft ports must live in the API contract\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator::video::VideoPrompt(Timeline|Draft)ApplicationPort|VideoSegmentPlanDraftState' \
    crates/a4n-runtime/src crates/a4n-orchestrator/src/video; then
    printf 'video prompt timeline/draft consumers must not use orchestrator-only ports or duplicate draft state\n' >&2
    exit 1
fi

if ! rg -n 'pub trait VideoPromptDownstreamCleanupPort' \
    crates/a4n-api-contract/src/video.rs; then
    printf 'VideoPromptDownstreamCleanupPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator::video::VideoPromptDownstreamCleanupPort' \
    crates/a4n-runtime/src crates/a4n-server/src crates/a4n-tauri/src; then
    printf 'Runtime/Host must consume VideoPromptDownstreamCleanupPort from the API contract\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator::video::(CompleteVideoRecordInput|VideoRecordCompletion|VideoRecordCompletionError|VideoTailFrameInput|VideoTailFrameSummary|AssemblyPushClipInput|AssemblyPostprocessPlan|AssemblyPostprocessSourcePlan|AssemblyPostprocessEditRecordInput)' \
    crates/a4n-runtime/src crates/a4n-server/src crates/a4n-tauri/src; then
    printf 'Runtime/Host video record and assembly DTOs must consume the API contract\n' >&2
    exit 1
fi

if rg -n 'pub (struct|enum) (CompleteVideoRecordInput|VideoRecordCompletion|VideoRecordCompletionError|VideoTailFrameInput|VideoTailFrameSummary|AssemblyPushClipInput|AssemblyPostprocessPlan|AssemblyPostprocessSourcePlan|AssemblyPostprocessEditRecordInput)' \
    crates/a4n-orchestrator/src/video; then
    printf 'Video record and assembly DTOs must have one API contract owner\n' >&2
    exit 1
fi

if ! rg -n 'pub (struct|enum) (CompleteVideoRecordInput|VideoRecordCompletion|VideoRecordCompletionError|VideoTailFrameInput|VideoTailFrameSummary|AssemblyPushClipInput|AssemblyPostprocessPlan|AssemblyPostprocessSourcePlan|AssemblyPostprocessEditRecordInput)' \
    crates/a4n-api-contract/src/video.rs; then
    printf 'Video record and assembly DTO contracts are missing\n' >&2
    exit 1
fi

if ! rg -n 'pub struct GenerateChapterVideoInput' \
    crates/a4n-api-contract/src/video.rs; then
    printf 'GenerateChapterVideoInput contract is missing\n' >&2
    exit 1
fi

if rg -n 'GenerateChapterVideoInput' crates/a4n-orchestrator/src/video/mod.rs \
    || rg -n 'a4n_orchestrator::video::GenerateChapterVideoInput' crates/a4n-runtime/src; then
    printf 'Chapter video input must be consumed from the API contract\n' >&2
    exit 1
fi

if ! rg -n 'pub struct GenerateReferenceMediaVideoInput' \
    crates/a4n-api-contract/src/video.rs; then
    printf 'GenerateReferenceMediaVideoInput contract is missing\n' >&2
    exit 1
fi

if rg -n 'GenerateReferenceMediaVideoInput' crates/a4n-orchestrator/src/video/mod.rs \
    || rg -n 'a4n_orchestrator::video::GenerateReferenceMediaVideoInput' crates/a4n-runtime/src; then
    printf 'Reference-media video input must be consumed from the API contract\n' >&2
    exit 1
fi

if ! rg -n 'pub struct GeneratePageImageInput' \
    crates/a4n-api-contract/src/image_generation.rs; then
    printf 'GeneratePageImageInput contract is missing\n' >&2
    exit 1
fi

if rg -n 'GeneratePageImageInput' crates/a4n-orchestrator/src/media.rs \
    crates/a4n-orchestrator/src/media/image_generation.rs \
    || rg -n 'a4n_orchestrator::media::GeneratePageImageInput' crates/a4n-runtime/src; then
    printf 'Page image input must be consumed from the API contract\n' >&2
    exit 1
fi

if ! rg -n 'pub struct GenerateBatchImagesInput' \
    crates/a4n-api-contract/src/image_generation.rs; then
    printf 'GenerateBatchImagesInput contract is missing\n' >&2
    exit 1
fi

if rg -n 'GenerateBatchImagesInput' crates/a4n-orchestrator/src/media.rs \
    crates/a4n-orchestrator/src/media/image_generation.rs \
    || rg -n 'a4n_orchestrator::media::GenerateBatchImagesInput' crates/a4n-runtime/src; then
    printf 'Batch image input must be consumed from the API contract\n' >&2
    exit 1
fi

if ! rg -n 'pub struct GenerateSceneImageInput' \
    crates/a4n-api-contract/src/image_generation.rs; then
    printf 'GenerateSceneImageInput contract is missing\n' >&2
    exit 1
fi

if rg -n 'GenerateSceneImageInput' crates/a4n-orchestrator/src/media.rs \
    crates/a4n-orchestrator/src/media/image_generation.rs \
    || rg -n 'a4n_orchestrator::media::GenerateSceneImageInput' crates/a4n-runtime/src; then
    printf 'Scene image input must be consumed from the API contract\n' >&2
    exit 1
fi

if ! rg -n 'pub struct GenerateGalgameImageInput' \
    crates/a4n-api-contract/src/image_generation.rs; then
    printf 'GenerateGalgameImageInput contract is missing\n' >&2
    exit 1
fi

if rg -n 'GenerateGalgameImageInput' crates/a4n-orchestrator/src/media.rs \
    crates/a4n-orchestrator/src/media/image_generation.rs \
    || rg -n 'a4n_orchestrator::media::GenerateGalgameImageInput' crates/a4n-runtime/src; then
    printf 'Galgame image input must be consumed from the API contract\n' >&2
    exit 1
fi

if rg -U -n 'pub use a4n_orchestrator::video::\{[^}]*\b(ChapterVideoPromptSummary|LoadVideoSegmentPlanDraftInput|OptimizeChapterVideoPromptInput|UpdateChapterVideoPromptInput|UpdateVideoSegmentPlanDraftRecipeInput)\b' \
    crates/a4n-runtime/src; then
    printf 'Video prompt DTOs must be consumed from the API contract without an orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'pub use a4n_api_contract::video::\{' \
    crates/a4n-runtime/src/actions/video_prompts.rs; then
    printf 'Video prompt actions must consume the API video contract directly\n' >&2
    exit 1
fi

if rg -U -n 'pub use a4n_orchestrator::character::\{[^}]*\b(ApplyImplicitUpdateDecisionInput|CheckImplicitUpdateInput|GetImplicitReviewInput|ImplicitReviewSummary|ImplicitUpdateDecisionSummary)\b' \
    crates/a4n-runtime/src; then
    printf 'Implicit character-review DTOs must be consumed from the API contract without an orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'pub use a4n_api_contract::character::\{' \
    crates/a4n-runtime/src/actions/character_implicit_review/mod.rs; then
    printf 'Implicit character-review actions must consume the API character contract directly\n' >&2
    exit 1
fi
if ! rg -n 'character_implicit_review_read_application\(\)' \
    crates/a4n-runtime/src/actions/character_implicit_review/api.rs; then
    printf 'Implicit character-review summary reads must route through the read application port\n' >&2
    exit 1
fi
if ! rg -n 'pub trait CharacterImplicitReviewReadApplicationPort' \
    crates/a4n-api-contract/src/character.rs; then
    printf 'CharacterImplicitReviewReadApplicationPort contract is missing\n' >&2
    exit 1
fi
if rg -n 'pub trait CharacterImplicitReviewReadApplicationPort' \
    crates/a4n-orchestrator/src/character; then
    printf 'CharacterImplicitReviewReadApplicationPort must not be redefined in the orchestrator\n' >&2
    exit 1
fi
if rg -n 'a4n_orchestrator::character::CharacterImplicitReviewReadApplicationPort' \
    crates/a4n-runtime/src; then
    printf 'Runtime implicit-review reads must consume the API contract directly\n' >&2
    exit 1
fi

if rg -n 'pub use a4n_api_contract::source_sync::SourceSyncDecisionInput' \
    crates/a4n-orchestrator/src/source_sync.rs \
    crates/a4n-orchestrator/src/source_sync/contracts.rs; then
    printf 'Source-sync decision input must not be publicly re-exported by the orchestrator facade\n' >&2
    exit 1
fi

if rg -n '^pub use a4n_api_contract::optimize::OptimizationAgentPermissions' \
    crates/a4n-orchestrator/src/optimize/permissions.rs \
    || rg -n '^pub use permissions::\{[^}]*OptimizeAgentPermissions' \
    crates/a4n-orchestrator/src/optimize.rs; then
    printf 'Optimization permissions must be consumed from the API contract without an orchestrator facade\n' >&2
    exit 1
fi

if rg -n '^pub use a4n_api_contract::optimize::OptimizationPatch' \
    crates/a4n-orchestrator/src/optimize/patch/contracts.rs \
    || rg -n '^pub use patch::\{[^}]*OptimizationPatch' \
    crates/a4n-orchestrator/src/optimize.rs \
    || rg -n '^pub use contracts::\{[^}]*OptimizationPatch' \
    crates/a4n-orchestrator/src/optimize/patch.rs; then
    printf 'Optimization patches must be consumed from the API contract without an orchestrator facade\n' >&2
    exit 1
fi

if rg -n '^pub use a4n_api_contract::generation_tasks::GenerationTaskStart' \
    crates/a4n-orchestrator/src/context/generation.rs \
    || rg -U -n '^pub use context::generation::\{[^}]*\bGenerationTaskStart\b' \
    crates/a4n-orchestrator/src/lib.rs; then
    printf 'GenerationTaskStart must remain an internal orchestrator alias\n' >&2
    exit 1
fi

if rg -U -n '^pub use a4n_api_contract::shooting_script::\{' \
    crates/a4n-orchestrator/src/shooting_script.rs; then
    printf 'Shooting-script DTOs must not be publicly re-exported by the orchestrator facade\n' >&2
    exit 1
fi

if rg -U -n 'pub use a4n_api_contract::character::\{[^}]*\b(ApplyImplicitUpdateDecisionInput|CheckImplicitUpdateInput|GetImplicitReviewInput|ImplicitReviewSummary|ImplicitUpdateDecisionSummary)\b' \
    crates/a4n-orchestrator/src/character.rs \
    crates/a4n-orchestrator/src/character/implicit_review.rs; then
    printf 'Implicit character-review DTOs must not be publicly re-exported by the orchestrator facade\n' >&2
    exit 1
fi

if rg -n 'pub use .*GeneratedImageSummary' \
    crates/a4n-orchestrator/src/media.rs \
    crates/a4n-runtime/src/actions/image_generation/mod.rs \
    || rg -n 'actions::image_generation::GeneratedImageSummary' crates/a4n-runtime/src; then
    printf 'GeneratedImageSummary must be consumed from the image API contract without runtime or orchestrator facades\n' >&2
    exit 1
fi

if ! rg -n 'a4n_api_contract::image::GeneratedImageSummary|image::GeneratedImageSummary' \
    crates/a4n-orchestrator/src/media/image_generation \
    crates/a4n-orchestrator/src/media/image_record_lifecycle.rs \
    crates/a4n-runtime/src/actions/image_records \
    crates/a4n-runtime/src/actions/video_records/imports/canvas/frame.rs; then
    printf 'GeneratedImageSummary consumers must use the image API contract directly\n' >&2
    exit 1
fi

if ! rg -n 'pub struct GeneratePortraitInput' \
    crates/a4n-api-contract/src/portrait.rs; then
    printf 'GeneratePortraitInput contract is missing\n' >&2
    exit 1
fi

if rg -U -n 'pub use (application|contracts)::\{[^}]*GeneratePortraitInput' \
    crates/a4n-orchestrator/src/portrait.rs \
    crates/a4n-orchestrator/src/portrait/application.rs \
    crates/a4n-orchestrator/src/portrait/application/contracts.rs \
    || rg -U -n 'a4n_orchestrator::(?:\{[^;]*)?portrait::\{[^}]*GeneratePortraitInput' \
    crates/a4n-runtime/src crates/a4n-orchestrator/src/galgame; then
    printf 'Portrait generation input must be consumed from the API contract\n' >&2
    exit 1
fi

if ! rg -n 'pub struct GeneratePortraitPromptInput' \
    crates/a4n-api-contract/src/portrait.rs; then
    printf 'GeneratePortraitPromptInput contract is missing\n' >&2
    exit 1
fi

if rg -U -n 'pub use (application|contracts)::\{[^}]*GeneratePortraitPromptInput' \
    crates/a4n-orchestrator/src/portrait.rs \
    crates/a4n-orchestrator/src/portrait/application.rs \
    crates/a4n-orchestrator/src/portrait/application/contracts.rs \
    || rg -U -n 'a4n_orchestrator::(?:\{[^;]*)?portrait::\{[^}]*GeneratePortraitPromptInput' \
    crates/a4n-runtime/src crates/a4n-orchestrator/src/portrait; then
    printf 'Portrait prompt input must be consumed from the API contract\n' >&2
    exit 1
fi

if ! rg -n 'pub struct GenerateAgeVariantInput' \
    crates/a4n-api-contract/src/portrait.rs; then
    printf 'GenerateAgeVariantInput contract is missing\n' >&2
    exit 1
fi

if rg -U -n 'pub use (application|contracts)::\{[^}]*GenerateAgeVariantInput' \
    crates/a4n-orchestrator/src/portrait.rs \
    crates/a4n-orchestrator/src/portrait/application.rs \
    crates/a4n-orchestrator/src/portrait/application/contracts.rs \
    || rg -U -n 'a4n_orchestrator::(?:\{[^;]*)?portrait::\{[^}]*GenerateAgeVariantInput' \
    crates/a4n-runtime/src crates/a4n-orchestrator/src/portrait; then
    printf 'Portrait age-variant input must be consumed from the API contract\n' >&2
    exit 1
fi

if ! rg -n 'pub struct GenerateMissingPortraitsInput' \
    crates/a4n-api-contract/src/portrait.rs; then
    printf 'GenerateMissingPortraitsInput contract is missing\n' >&2
    exit 1
fi

if rg -U -n 'pub use (application|contracts)::\{[^}]*GenerateMissingPortraitsInput' \
    crates/a4n-orchestrator/src/portrait.rs \
    crates/a4n-orchestrator/src/portrait/application.rs \
    crates/a4n-orchestrator/src/portrait/application/contracts.rs \
    || rg -U -n 'a4n_orchestrator::(?:\{[^;]*)?portrait::\{[^}]*GenerateMissingPortraitsInput' \
    crates/a4n-runtime/src crates/a4n-orchestrator/src/portrait; then
    printf 'Missing portrait input must be consumed from the API contract\n' >&2
    exit 1
fi

if ! rg -n 'pub struct RegeneratePortraitInput' \
    crates/a4n-api-contract/src/portrait.rs; then
    printf 'RegeneratePortraitInput contract is missing\n' >&2
    exit 1
fi

if rg -U -n 'pub use (application|contracts)::\{[^}]*RegeneratePortraitInput' \
    crates/a4n-orchestrator/src/portrait.rs \
    crates/a4n-orchestrator/src/portrait/application.rs \
    crates/a4n-orchestrator/src/portrait/application/contracts.rs \
    || rg -U -n 'a4n_orchestrator::(?:\{[^;]*)?portrait::\{[^}]*RegeneratePortraitInput' \
    crates/a4n-runtime/src crates/a4n-orchestrator/src/portrait; then
    printf 'Portrait regeneration input must be consumed from the API contract\n' >&2
    exit 1
fi

if ! rg -n 'pub struct UploadPortraitInput' \
    crates/a4n-api-contract/src/portrait.rs; then
    printf 'UploadPortraitInput contract is missing\n' >&2
    exit 1
fi

if rg -U -n 'pub use (application|contracts)::\{[^}]*UploadPortraitInput' \
    crates/a4n-orchestrator/src/portrait.rs \
    crates/a4n-orchestrator/src/portrait/application.rs \
    crates/a4n-orchestrator/src/portrait/application/contracts.rs \
    || rg -U -n 'a4n_orchestrator::(?:\{[^;]*)?portrait::\{[^}]*UploadPortraitInput' \
    crates/a4n-runtime/src crates/a4n-orchestrator/src/portrait; then
    printf 'Portrait upload input must be consumed from the API contract\n' >&2
    exit 1
fi

if ! rg -n 'pub struct PortraitSummary' \
    crates/a4n-api-contract/src/portrait.rs; then
    printf 'PortraitSummary contract is missing\n' >&2
    exit 1
fi

if rg -n 'pub use .*PortraitSummary' \
    crates/a4n-orchestrator/src/portrait.rs \
    crates/a4n-orchestrator/src/portrait/application.rs \
    crates/a4n-orchestrator/src/portrait/application/contracts.rs \
    || rg -U -n 'a4n_orchestrator::(?:\{[^;]*)?portrait::\{[^}]*PortraitSummary' \
    crates/a4n-runtime/src crates/a4n-orchestrator/src; then
    printf 'PortraitSummary must be consumed directly from the API contract\n' >&2
    exit 1
fi

if ! rg -n 'pub enum PortraitType' crates/a4n-api-contract/src/portrait.rs; then
    printf 'PortraitType contract is missing\n' >&2
    exit 1
fi

if rg -n 'pub use .*PortraitType' \
    crates/a4n-orchestrator/src/portrait.rs \
    crates/a4n-orchestrator/src/portrait/application.rs \
    crates/a4n-orchestrator/src/portrait/application/contracts.rs \
    || rg -U -n 'a4n_orchestrator::(?:\{[^;]*)?portrait::\{[^}]*PortraitType' \
    crates/a4n-runtime/src crates/a4n-orchestrator/src; then
    printf 'PortraitType must be consumed directly from the API contract\n' >&2
    exit 1
fi

if rg -n 'portrait_generation_application\(\)' \
    crates/a4n-runtime/src/actions/portrait_generation/preview/requests.rs; then
    printf 'Portrait prompt, upload and preview actions must use their narrow application ports\n' >&2
    exit 1
fi

if ! rg -n 'pub trait PortraitRecordReadApplicationPort|pub trait PortraitRecordMutationApplicationPort' \
    crates/a4n-api-contract/src/portrait.rs; then
    printf 'Portrait record read/mutation contracts are missing\n' >&2
    exit 1
fi

if rg -n 'PortraitRecordApplicationPort' \
    crates/a4n-api-contract/src/portrait.rs \
    crates/a4n-orchestrator/src crates/a4n-runtime/src; then
    printf 'The mixed PortraitRecordApplicationPort must remain removed\n' >&2
    exit 1
fi

if ! rg -n 'portrait_record_read_application\(\)' \
    crates/a4n-runtime/src/actions/portrait_records.rs \
    crates/a4n-runtime/src/actions/portrait_generation; then
    printf 'Portrait record reads must route through PortraitRecordReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'portrait_record_mutation_application\(\)' \
    crates/a4n-runtime/src/actions/portrait_records.rs; then
    printf 'Portrait record mutations must route through PortraitRecordMutationApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'portrait_prompt_application\(\)' \
    crates/a4n-runtime/src/actions/portrait_generation/application.rs || \
   ! rg -n 'portrait_upload_application\(\)' \
    crates/a4n-runtime/src/actions/portrait_generation/application.rs || \
   ! rg -n 'portrait_request_preview_application\(\)' \
    crates/a4n-runtime/src/actions/portrait_generation/preview/requests.rs; then
    printf 'Portrait actions must route through split application ports\n' >&2
    exit 1
fi

if ! rg -n 'pub trait Portrait(RequestPreview|Prompt|Upload|Generation)ApplicationPort' \
    crates/a4n-orchestrator/src/portrait.rs; then
    printf 'Portrait application ports are missing from the portrait owner\n' >&2
    exit 1
fi
if ! rg -n 'pub trait ProjectMutationApplicationPort' crates/a4n-api-contract/src/project.rs; then
    printf 'ProjectMutationApplicationPort contract is missing\n' >&2
    exit 1
fi

if ! rg -n 'chapter_generation_preview_application\(\)' \
    crates/a4n-runtime/src/actions/chapter_generation.rs; then
    printf 'Runtime chapter previews must route through ChapterGenerationPreviewApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ChapterGenerationPreviewApplicationPort' \
    crates/a4n-orchestrator/src/chapter_generation.rs; then
    printf 'ChapterGenerationPreviewApplicationPort contract is missing\n' >&2
    exit 1
fi

if ! rg -n 'project_mutation_application\(\)' \
    crates/a4n-runtime/src/actions/project_records/api/mutations.rs; then
    printf 'Runtime project create/update actions must consume ProjectMutationApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ProjectAssetGraphApplicationPort' crates/a4n-api-contract/src/project.rs; then
    printf 'ProjectAssetGraphApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n '\b(a4n_orchestrator|OrchestratorContext|ProjectFamilyPackage|VideoStudioProjectPackage|ImportMediaPayload|MediaFileEntry)\b' \
    crates/a4n-runtime/src/actions/project_transfer; then
    printf 'Runtime project transfer actions must consume the pure project-transfer contract, not package models or the orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'ProjectPackageTransferHostPort' \
    crates/a4n-runtime/src/actions/project_transfer/host_port.rs || \
   ! rg -n 'ProjectFamilyTransferApplicationPort|VideoStudioProjectTransferApplicationPort' \
    crates/a4n-api-contract/src/project_transfer.rs; then
    printf 'Project transfer Host and application ports must live in the API contract\n' >&2
    exit 1
fi

if rg -n 'Project(Family|VideoStudio)TransferHostPort|ProjectPackageFileHostPort|Read(ProjectFamily|VideoStudio)ProjectPackage' \
    crates/a4n-orchestrator/src/project crates/a4n-orchestrator/src/context \
    --glob '*.rs' --glob '!**/tests.rs' --glob '!**/tests/**'; then
    printf 'Project transfer must not retain package-specific Host compatibility traits or read wrappers\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ImportReverseReadApplicationPort' \
    crates/a4n-api-contract/src/import_reverse.rs; then
    printf 'ImportReverseReadApplicationPort contract is missing\n' >&2
    exit 1
fi
if rg -n 'pub trait ImportReverseReadApplicationPort' \
    crates/a4n-orchestrator/src/import_reverse; then
    printf 'ImportReverseReadApplicationPort must not be redefined in the orchestrator\n' >&2
    exit 1
fi
if rg -n 'a4n_orchestrator::import_reverse::ImportReverseReadApplicationPort' \
    crates/a4n-runtime/src; then
    printf 'Runtime import reverse reads must consume the API contract directly\n' >&2
    exit 1
fi
if ! rg -n 'import_reverse_read_application\(\)' \
    crates/a4n-runtime/src/actions/import_reverse/api.rs; then
    printf 'Runtime import reverse candidate reads must route through ImportReverseReadApplicationPort\n' >&2
    exit 1
fi
if rg -U -n 'import_reverse_application\(\)\s*\.\s*latest' \
    crates/a4n-runtime/src/actions/import_reverse/api.rs; then
    printf 'Runtime import reverse candidate reads must not use the generation boundary\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator|OrchestratorContext|state\.orchestrator\(\)' \
    crates/a4n-runtime/src/actions/import_analysis \
    crates/a4n-runtime/src/actions/import_file.rs \
    crates/a4n-runtime/src/actions/import_review; then
    printf 'Runtime import actions must consume ImportAnalysisApplicationPort and ImportReviewApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'import_analysis_application\(\)' \
    crates/a4n-runtime/src/actions/import_analysis/api.rs || \
   ! rg -n 'import_analysis_progress_application\(\)' \
    crates/a4n-runtime/src/actions/import_analysis/api.rs || \
   ! rg -n 'import_analysis_preview_application\(\)' \
    crates/a4n-runtime/src/actions/import_analysis/preview.rs; then
    printf 'Runtime import-analysis actions must route through split application ports\n' >&2
    exit 1
fi

if ! rg -n 'import_review_application\(\)' \
    crates/a4n-runtime/src/actions/import_file.rs \
    crates/a4n-runtime/src/actions/import_review/acceptance.rs \
    crates/a4n-runtime/src/actions/import_review/mutations.rs || \
   ! rg -n 'import_review_read_application\(\)' \
    crates/a4n-runtime/src/actions/import_review/queries.rs \
    crates/a4n-runtime/src/actions/import_review/dedup.rs; then
    printf 'Runtime import-review actions must route through the split application ports\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ImportAnalysisApplicationPort' \
    crates/a4n-orchestrator/src/import_analysis/application.rs || \
   ! rg -n 'pub trait ImportAnalysisPreviewApplicationPort' \
    crates/a4n-orchestrator/src/import_analysis/application.rs || \
   ! rg -n 'pub trait ImportAnalysisProgressApplicationPort' \
    crates/a4n-api-contract/src/import.rs; then
    printf 'Import-analysis application ports are missing\n' >&2
    exit 1
fi
if rg -n 'pub trait ImportAnalysisProgressApplicationPort' \
    crates/a4n-orchestrator/src/import_analysis; then
    printf 'ImportAnalysisProgressApplicationPort must not be redefined in the orchestrator\n' >&2
    exit 1
fi
if rg -n 'a4n_orchestrator::import_analysis::ImportAnalysisProgressApplicationPort' \
    crates/a4n-runtime/src; then
    printf 'Runtime import-analysis progress must consume the API contract directly\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ImportReviewApplicationPort' \
    crates/a4n-orchestrator/src/import_review/mod.rs; then
    printf 'ImportReviewApplicationPort contract is missing\n' >&2
    exit 1
fi
if ! rg -n 'pub trait ImportReviewReadApplicationPort' \
    crates/a4n-api-contract/src/import_review.rs; then
    printf 'ImportReviewReadApplicationPort contract is missing\n' >&2
    exit 1
fi
if rg -n 'pub trait ImportReviewReadApplicationPort' \
    crates/a4n-orchestrator/src/import_review; then
    printf 'ImportReviewReadApplicationPort must not be redefined in the orchestrator\n' >&2
    exit 1
fi
if rg -n 'a4n_orchestrator::import_review::ImportReviewReadApplicationPort' \
    crates/a4n-runtime/src; then
    printf 'Runtime import-review reads must consume the API contract directly\n' >&2
    exit 1
fi

if rg -n 'OrchestratorContext|state\.orchestrator\(\)' \
    crates/a4n-runtime/src/workflow/chapter_analysis; then
    printf 'Runtime chapter analysis must consume split application ports and RagIngestPort\n' >&2
    exit 1
fi

if ! rg -n 'chapter_analysis_generation_application\(\)' \
    crates/a4n-runtime/src/workflow/chapter_analysis/api/generation.rs \
    crates/a4n-runtime/src/workflow/chapter_analysis/execution.rs || \
   ! rg -n 'chapter_analysis_resume_application\(\)' \
    crates/a4n-runtime/src/workflow/chapter_analysis/api/resume.rs || \
   ! rg -n 'chapter_analysis_preview_application\(\)' \
    crates/a4n-runtime/src/workflow/chapter_analysis/api/preview.rs; then
    printf 'Runtime chapter analysis must route through split application ports\n' >&2
    exit 1
fi

if ! rg -n 'rag_ingest\(\)' \
    crates/a4n-runtime/src/workflow/chapter_analysis/host_port.rs; then
    printf 'Runtime chapter analysis RAG synchronization must route through RagIngestPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ChapterAnalysisGenerationApplicationPort' \
    crates/a4n-orchestrator/src/chapter_analysis/application.rs || \
   ! rg -n 'pub trait ChapterAnalysisResumeApplicationPort' \
    crates/a4n-orchestrator/src/chapter_analysis/application.rs || \
   ! rg -n 'pub trait ChapterAnalysisPreviewApplicationPort' \
    crates/a4n-orchestrator/src/chapter_analysis/application.rs; then
    printf 'ChapterAnalysis split application ports are missing\n' >&2
    exit 1
fi

if rg -n -U 'orchestrator\(\)\s*\.agent\(\)\s*\.(ensure_main_thread|create_thread|fork_thread|update_thread|archive_thread|list_threads|show_thread|list_messages|show_run|send_message|control_run|send_message_to_child|control_child_agent|spawn_agent|events|search_tools|describe_tool|list_memories|create_memory|retract_memory|review_delegation|resolve_approval|verify_chapters_in_project)' \
    crates/a4n-runtime/src/actions/agent_runtime.rs; then
    printf 'Runtime Agent user/API actions must consume AgentApplicationPort instead of the concrete orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'pub trait AgentApplicationPort' crates/a4n-api-contract/src/agent.rs; then
    printf 'AgentApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n -U 'orchestrator\(\)\s*\.agent\(\)' \
    crates/a4n-runtime/src/actions/agent_runtime.rs \
    crates/a4n-runtime/src/actions/agent_runtime; then
    printf 'Runtime Agent worker must consume the injected Agent owner instead of the full OrchestratorContext\n' >&2
    exit 1
fi

if rg -n 'a4n_db_models::models::agent|a4n_db::models::agent' \
    crates/a4n-runtime/src/actions/agent_runtime.rs \
    crates/a4n-runtime/src/actions/agent_runtime; then
    printf 'Runtime Agent worker must consume orchestrator worker snapshots instead of Agent ORM models\n' >&2
    exit 1
fi

if rg -n 'state\.application\(\)\.(search_tools|describe_tool)' \
    crates/a4n-agent-runtime/src/engine; then
    printf 'Agent worker tool discovery must consume AgentWorkerReadApplicationPort\n' >&2
    exit 1
fi
if ! rg -n 'search_tools_for_worker|describe_tool_for_worker' \
    crates/a4n-agent-application/src/agent/worker_read.rs \
    crates/a4n-agent-runtime/src/engine/mod.rs; then
    printf 'Agent worker tool discovery read port is missing\n' >&2
    exit 1
fi
if rg -U -n 'ensure_argument_resource_scope\(\s*self\.state\.application\(\)' \
    crates/a4n-agent-runtime/src/engine/mod.rs; then
    printf 'Agent worker resource scope checks must consume AgentWorkerReadApplicationPort\n' >&2
    exit 1
fi
if ! rg -n 'verify_chapters_in_project_for_worker' \
    crates/a4n-agent-application/src/agent/worker_read.rs \
    crates/a4n-agent-runtime/src/engine/mod.rs; then
    printf 'Agent worker resource scope read port is missing\n' >&2
    exit 1
fi
if rg -U -n 'state\.application\(\)\s*\.\s*show_run' \
    crates/a4n-agent-runtime/src/engine; then
    printf 'Agent worker terminal status must consume AgentWorkerCompletionApplicationPort\n' >&2
    exit 1
fi
if ! rg -n 'run_is_terminal' \
    crates/a4n-agent-application/src/agent/worker_completion.rs \
    crates/a4n-agent-runtime/src/engine/mod.rs; then
    printf 'Agent worker terminal status completion port is missing\n' >&2
    exit 1
fi
if rg -n 'state\.application\(\)\.review_delegation' \
    crates/a4n-agent-runtime/src/engine; then
    printf 'Agent worker delegation review must consume AgentWorkerDelegationApplicationPort\n' >&2
    exit 1
fi
if ! rg -n 'review_delegation' \
    crates/a4n-agent-application/src/agent/worker_delegation.rs \
    crates/a4n-agent-runtime/src/engine/mod.rs; then
    printf 'Agent worker delegation review port is missing\n' >&2
    exit 1
fi

if ! rg -n 'pub trait AgentWorkerBudgetApplicationPort' \
    crates/a4n-agent-runtime-contract/src/ports.rs; then
    printf 'Agent worker budget application port contract is missing\n' >&2
    exit 1
fi

if ! rg -n 'AgentWorkerBudgetSnapshot' \
    crates/a4n-agent-runtime-contract/src/lib.rs || \
   ! rg -n 'agent_worker_budget_application\(\)' \
    crates/a4n-runtime/src/state/app.rs \
    crates/a4n-runtime/src/actions/agent_runtime.rs || \
   ! rg -n 'worker_budget_snapshot' \
    crates/a4n-runtime/src/actions/agent_runtime.rs; then
    printf 'Agent worker budget must use its typed narrow accessor and snapshot\n' >&2
    exit 1
fi

if rg -n 'run_budget_snapshot' crates/a4n-runtime/src/actions/agent_runtime.rs; then
    printf 'Runtime Agent worker must not call the broad run_budget_snapshot owner method\n' >&2
    exit 1
fi

if ! rg -n 'pub trait AgentRunLeaseRecoveryApplicationPort' \
    crates/a4n-agent-runtime-contract/src/ports.rs; then
    printf 'Agent run lease-recovery application port contract is missing\n' >&2
    exit 1
fi

if ! rg -n 'agent_run_lease_recovery_application\(\)' \
    crates/a4n-runtime/src/state/app.rs \
    crates/a4n-runtime/src/actions/agent_runtime.rs || \
   ! rg -U -n 'run_lease_recovery_application\s*\.\s*recover_expired_runs\(16' \
    crates/a4n-runtime/src/actions/agent_runtime.rs; then
    printf 'Agent expired-run recovery must use its typed narrow accessor\n' >&2
    exit 1
fi

if rg -n 'agent\.recover_expired_runs' \
    crates/a4n-runtime/src/actions/agent_runtime.rs \
    crates/a4n-runtime/src/actions/agent_runtime; then
    printf 'Agent expired-run recovery must not use the broad worker owner accessor\n' >&2
    exit 1
fi

if ! rg -n 'pub trait AgentWorkerLeaseApplicationPort' \
    crates/a4n-agent-runtime-contract/src/ports.rs; then
    printf 'Agent worker-lease application port contract is missing\n' >&2
    exit 1
fi

if ! rg -n 'agent_worker_lease_application\(\)' \
    crates/a4n-runtime/src/state/app.rs \
    crates/a4n-runtime/src/actions/agent_runtime.rs || \
   ! rg -U -n 'renew_agent\s*\.\s*renew_worker_lease\(\s*&renew_claim\s*\)' \
    crates/a4n-runtime/src/actions/agent_runtime.rs; then
    printf 'Agent worker lease renewal must use its typed narrow accessor\n' >&2
    exit 1
fi

if rg -n '(^|[^[:alnum:]_])agent\.renew_worker_lease' \
    crates/a4n-runtime/src/actions/agent_runtime.rs \
    crates/a4n-runtime/src/actions/agent_runtime; then
    printf 'Agent worker lease renewal must not use the broad worker owner accessor\n' >&2
    exit 1
fi

if ! rg -n 'pub trait AgentWorkerClaimApplicationPort' \
    crates/a4n-agent-runtime-contract/src/ports.rs; then
    printf 'Agent worker-claim application port contract is missing\n' >&2
    exit 1
fi

if ! rg -n 'agent_worker_claim_application\(\)' \
    crates/a4n-runtime/src/state/app.rs \
    crates/a4n-runtime/src/actions/agent_runtime.rs || \
   ! rg -U -n 'claim_application\s*\.\s*claim_worker_run\(\s*&worker_id\s*\)' \
    crates/a4n-runtime/src/actions/agent_runtime.rs; then
    printf 'Agent worker claim must use its typed narrow accessor\n' >&2
    exit 1
fi

if rg -n '(^|[^[:alnum:]_])agent\.claim_worker_run' \
    crates/a4n-runtime/src/actions/agent_runtime.rs \
    crates/a4n-runtime/src/actions/agent_runtime; then
    printf 'Agent worker claim must not use the broad worker owner accessor\n' >&2
    exit 1
fi

if ! rg -n 'pub trait AgentWorkerFailureApplicationPort' \
    crates/a4n-agent-runtime-contract/src/ports.rs; then
    printf 'Agent worker-failure application port contract is missing\n' >&2
    exit 1
fi

if ! rg -n 'agent_worker_failure_application\(\)' \
    crates/a4n-runtime/src/state/app.rs \
    crates/a4n-runtime/src/actions/agent_runtime.rs || \
   ! rg -U -n 'failure_application\s*\.\s*fail_child_run\(\s*&claimed' \
    crates/a4n-runtime/src/actions/agent_runtime.rs; then
    printf 'Agent worker failure persistence must use its typed narrow accessor\n' >&2
    exit 1
fi

if rg -n '(^|[^[:alnum:]_])agent\.fail_child_run' \
    crates/a4n-runtime/src/actions/agent_runtime.rs \
    crates/a4n-runtime/src/actions/agent_runtime; then
    printf 'Agent worker failure persistence must not use the broad worker owner accessor\n' >&2
    exit 1
fi

if ! rg -n 'pub trait AgentApprovalExpiryApplicationPort' \
    crates/a4n-agent-runtime-contract/src/ports.rs; then
    printf 'Agent approval-expiry application port contract is missing\n' >&2
    exit 1
fi

if ! rg -n 'agent_approval_expiry_application\(\)' \
    crates/a4n-runtime/src/state/app.rs \
    crates/a4n-runtime/src/actions/agent_runtime.rs || \
   ! rg -U -n 'approval_expiry_application\s*\.\s*expire_pending_approvals\(32\)' \
    crates/a4n-runtime/src/actions/agent_runtime.rs; then
    printf 'Agent approval expiry scan must use its typed narrow accessor\n' >&2
    exit 1
fi

if rg -n 'agent\.expire_pending_approvals' \
    crates/a4n-runtime/src/actions/agent_runtime.rs; then
    printf 'Agent approval expiry scan must not use the broad worker owner accessor\n' >&2
    exit 1
fi

if ! rg -n 'Agent(Execution|Inbox|Message)Snapshot' \
    crates/a4n-agent-application/src/agent.rs \
    crates/a4n-runtime/src/actions/agent_runtime.rs; then
    printf 'Agent worker snapshot contracts are missing\n' >&2
    exit 1
fi

if ! rg -n 'pub trait AgentEditorWaitApplicationPort' \
    crates/a4n-agent-runtime-contract/src/ports.rs; then
    printf 'Agent editor-wait application port contract is missing\n' >&2
    exit 1
fi

if ! rg -n 'agent_editor_wait_application\(\)' \
    crates/a4n-runtime/src/state/app.rs \
    crates/a4n-runtime/src/actions/agent_runtime.rs || \
   ! rg -n 'editor_wait_application\.expire_editor_requests\(32\)' \
    crates/a4n-runtime/src/actions/agent_runtime.rs; then
    printf 'Agent editor-wait expiry scan must use its typed narrow accessor\n' >&2
    exit 1
fi

if rg -n 'agent\.expire_editor_requests' \
    crates/a4n-runtime/src/actions/agent_runtime.rs \
    crates/a4n-runtime/src/actions/agent_runtime; then
    printf 'Agent editor-wait expiry scan must not use the broad worker owner accessor\n' >&2
    exit 1
fi

if rg -n 'pub async fn expire_editor_requests' \
    crates/a4n-agent-application/src/agent.rs; then
    printf 'Agent editor-wait owner wrapper must live in editor_wait.rs\n' >&2
    exit 1
fi

if ! rg -n 'pub trait AgentExternalTaskWaitApplicationPort' \
    crates/a4n-agent-runtime-contract/src/ports.rs; then
    printf 'Agent external-task wait port is missing\n' >&2
    exit 1
fi

if ! rg -n 'AgentExternalTaskWaitSnapshot' \
    crates/a4n-agent-runtime-contract/src/lib.rs \
    crates/a4n-runtime/src/actions/agent_runtime/task_wait.rs; then
    printf 'Agent external-task wait snapshot contract is missing\n' >&2
    exit 1
fi

if ! rg -n 'pub trait AgentGenerationCancelApplicationPort' \
    crates/a4n-agent-runtime-contract/src/ports.rs; then
    printf 'Agent generation-cancel application port contract is missing\n' >&2
    exit 1
fi

if ! rg -n 'AgentGenerationCancelSnapshot' \
    crates/a4n-agent-runtime-contract/src/lib.rs \
    crates/a4n-runtime/src/actions/agent_runtime/task_wait.rs || \
   ! rg -n 'agent_generation_cancel_application\(\)' \
    crates/a4n-runtime/src/state/app.rs \
    crates/a4n-runtime/src/actions/agent_runtime/task_wait.rs || \
   ! rg -U -n 'generation_cancel_application\s*\.\s*generation_cancel_candidates' \
    crates/a4n-runtime/src/actions/agent_runtime/task_wait.rs || \
   ! rg -n 'cancel_generation_task' \
    crates/a4n-runtime/src/actions/agent_runtime/task_wait.rs; then
    printf 'Agent generation cancellation must use its typed narrow accessor and snapshot\n' >&2
    exit 1
fi

if rg -n 'agent\.(active_owned_generation_tasks|generation_owner)\(' \
    crates/a4n-runtime/src/actions/agent_runtime/task_wait.rs; then
    printf 'Agent generation cancellation must not use broad owner queries from Runtime\n' >&2
    exit 1
fi

if ! rg -n 'pub trait AgentWorkflowCancelApplicationPort' \
    crates/a4n-agent-runtime-contract/src/ports.rs; then
    printf 'Agent Workflow-cancel application port contract is missing\n' >&2
    exit 1
fi

if ! rg -n 'AgentWorkflowCancelSnapshot' \
    crates/a4n-agent-runtime-contract/src/lib.rs \
    crates/a4n-runtime/src/actions/agent_runtime/task_wait.rs || \
   ! rg -n 'agent_workflow_cancel_application\(\)' \
    crates/a4n-runtime/src/state/app.rs \
    crates/a4n-runtime/src/actions/agent_runtime/task_wait.rs || \
   ! rg -U -n 'workflow_cancel_application\s*\.\s*workflow_cancel_candidates' \
    crates/a4n-runtime/src/actions/agent_runtime/task_wait.rs || \
   ! rg -n 'workflow_cancellation_application\(\)' \
    crates/a4n-runtime/src/actions/agent_runtime/task_wait.rs; then
    printf 'Agent Workflow cancellation must use its typed narrow accessor and preserve the cancellation owner\n' >&2
    exit 1
fi

if rg -n 'active_owned_workflows|workflow_owner|generation_owner_stopped' \
    crates/a4n-runtime/src/actions/agent_runtime/task_wait.rs; then
    printf 'Agent Workflow cancellation must not use broad owner queries from Runtime\n' >&2
    exit 1
fi

if ! rg -n 'agent_external_task_wait_application\(\)' \
    crates/a4n-runtime/src/actions/agent_runtime/task_wait.rs; then
    printf 'Agent external-task wait recovery must use its narrow application port\n' >&2
    exit 1
fi

if rg -n 'AgentExecutionSnapshot|run_budget_snapshot' \
    crates/a4n-runtime/src/actions/agent_runtime/task_wait.rs; then
    printf 'Agent external-task wait recovery must not consume broad execution snapshots or budget owner methods\n' >&2
    exit 1
fi

if rg -n 'scope_json' crates/a4n-runtime/src/actions/agent_runtime/task_wait.rs; then
    printf 'Agent external-task wait recovery must consume parsed scope snapshots instead of persistence JSON\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\)' crates/a4n-runtime/src/actions/agent_runtime/task_wait.rs; then
    printf 'Agent wait recovery must consume WorkflowCancellationPort instead of the full orchestrator\n' >&2
    exit 1
fi

if ! rg -n 'workflow_cancellation_application\(\)' \
    crates/a4n-runtime/src/actions/agent_runtime/task_wait.rs; then
    printf 'Agent wait recovery must route workflow cancellation through its narrow port\n' >&2
    exit 1
fi

if ! rg -n 'pub trait WorkflowTaskLookupApplicationPort|pub trait WorkflowCancellationPort' \
    crates/a4n-api-contract/src/workflow.rs; then
    printf 'Workflow runtime ports must live in the API contract\n' >&2
    exit 1
fi

if rg -n 'pub trait WorkflowTaskLookupApplicationPort|pub trait WorkflowCancellationPort' \
    crates/a4n-orchestrator/src/workflow.rs; then
    printf 'Workflow runtime ports must not be redefined in the orchestrator\n' >&2
    exit 1
fi

if ! rg -n 'pub trait TaskCancellationPort' crates/a4n-api-contract/src/task.rs; then
    printf 'TaskCancellationPort must live in the API contract\n' >&2
    exit 1
fi

if rg -n 'pub trait TaskCancellationPort' crates/a4n-orchestrator/src/task_queue.rs; then
    printf 'TaskCancellationPort must not be redefined in the orchestrator queue module\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\)\.(register_task_handle|remove_task_handle|task_handle|cancel_task_handle|cancel_owned_task_handle|cancel_or_remember_task_handle|active_task_snapshots)' \
    crates/a4n-runtime/src/actions/task_records; then
    printf 'Runtime task handle actions must use the a4n-task-runtime owner directly\n' >&2
    exit 1
fi

if ! rg -n 'state\.task_manager\(\)' \
    crates/a4n-runtime/src/actions/task_records/handles.rs \
    crates/a4n-runtime/src/actions/task_records/access/task.rs \
    crates/a4n-runtime/src/actions/task_records/queries/workflows \
    crates/a4n-runtime/src/actions/task_records/workflows/cancellation.rs; then
    printf 'Runtime task handle actions must route through AppState task manager\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\)' crates/a4n-runtime/src/actions/storage_paths/paths.rs; then
    printf 'Runtime storage path display must consume AppState configuration and a4n-storage paths directly\n' >&2
    exit 1
fi

if rg -n 'existing_project_ids\(state\.orchestrator\(\)\)|project_read:\s*&a4n_orchestrator::OrchestratorContext' \
    crates/a4n-runtime/src/actions/storage_paths/orphans.rs; then
    printf 'Runtime orphan scans must use ProjectReadApplicationPort for project IDs\n' >&2
    exit 1
fi

if rg -U -n 'state\s*\.\s*orchestrator\(\)\s*\.\s*project_by_id' \
    crates/a4n-runtime/src/actions/storage_paths/orphans.rs; then
    printf 'Runtime orphan scans must use the project read port for existence checks\n' >&2
    exit 1
fi

if rg -U -n 'state\s*\.\s*orchestrator\(\)\s*\.\s*(project_ids_with_incomplete_cleanup|media_path_prefix_has_persisted_reference)' \
    crates/a4n-runtime/src/actions/storage_paths/orphans.rs; then
    printf 'Runtime orphan cleanup must use cleanup and image application ports\n' >&2
    exit 1
fi

if ! rg -n 'cleanup_outbox_application\(\)' \
    crates/a4n-runtime/src/actions/storage_paths/orphans.rs || \
   ! rg -n 'image_media_reference_application\(\)' \
    crates/a4n-runtime/src/actions/storage_paths/orphans.rs; then
    printf 'Runtime orphan cleanup must route cleanup and media reference checks through application ports\n' >&2
    exit 1
fi

printf 'pure crate dependency boundaries are valid\n'
