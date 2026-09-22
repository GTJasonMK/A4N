#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"
cd "$PROJECT_ROOT"

if rg -n 'a4n_core::video_execution|a4n-core.*video' crates/host/a4n-api-contract/Cargo.toml crates/host/a4n-api-contract/src; then
    printf 'a4n-api-contract must consume the pure video contract crate directly\n' >&2
    exit 1
fi

if rg -n '^a4n-core\s*=' crates/host/a4n-api-contract/Cargo.toml; then
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

for package in a4n-llm-http a4n-llm-gateway a4n-prompt a4n-prompt-git a4n-prompt-service a4n-queue; do
    package_dir="$(cargo_package_dir "$package")"
    if rg -n 'a4n[-_]core' "$package_dir/Cargo.toml" "$package_dir/src"; then
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
    crates/platform/a4n-db/src/project/duplicate crates/host/a4n-orchestrator/src/project; then
    printf 'project media copy contracts must not be owned by DB or orchestrator facades\n' >&2
    exit 1
fi
if ! rg -n '\bpub enum ProjectMediaCopyError|\bpub trait ProjectMediaFileCopier' \
    crates/platform/a4n-domain/src/project.rs; then
    printf 'project media copy contracts must have one domain owner\n' >&2
    exit 1
fi
if rg -n 'a4n_orchestrator::project::(ProjectMediaCopyError|ProjectMediaFileCopier)' \
    crates/host/a4n-runtime/src/actions/project_duplication; then
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
    crates/platform/a4n-domain/src/embedding.rs crates/platform/a4n-domain/src/image.rs crates/platform/a4n-domain/src/json_contract \
    crates/platform/a4n-domain/src/rag.rs crates/platform/a4n-domain/src/text_location.rs crates/platform/a4n-domain/src/profile_ownership.rs; then
    printf 'pure embedding/image domain contracts must not leak ORM or runtime types\n' >&2
    exit 1
fi

storage_tree="$(cargo tree --locked -e normal,build -p a4n-storage)"
if rg -n '(^|[[:space:]])(a4n-(core|db|runtime|orchestrator|server|tauri)|sea-orm|sqlx|tokio|reqwest)([[:space:]]|v|$)' \
    <<<"$storage_tree"; then
    printf 'a4n-storage must remain independent of Host, database, and runtime crates\n' >&2
    exit 1
fi

rag_core_tree="$(cargo tree --locked -e normal,build -p a4n-rag-core)"
if rg -n '(^|[[:space:]])a4n-embedding([[:space:]]|v|$)' <<<"$rag_core_tree"; then
    printf 'a4n-rag-core must consume a4n-embedding-provider-contract instead of embedding implementations\n' >&2
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
    crates/host/a4n-orchestrator/src crates/host/a4n-runtime/src; then
    printf 'Canvas image/video generation inputs must remain in contract crates, not the orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'pub struct GenerateCanvasImageInput' crates/canvas/a4n-canvas-contract/src/lib.rs || \
   ! rg -n 'pub struct GenerateCanvasVideoInput' crates/host/a4n-api-contract/src/canvas.rs || \
   ! rg -n 'a4n_canvas_contract::GenerateCanvasImageInput|a4n_api_contract::canvas::GenerateCanvasVideoInput|GenerateCanvas(Image|Video)Input' \
    crates/host/a4n-orchestrator/src/canvas/video_generation.rs \
    crates/host/a4n-orchestrator/src/context/canvas.rs \
    crates/host/a4n-orchestrator/src/media/image/generation/contracts.rs \
    crates/host/a4n-orchestrator/src/media/image/generation.rs \
    crates/host/a4n-orchestrator/src/video/generation.rs \
    crates/host/a4n-runtime/src/actions/video_generation.rs \
    crates/host/a4n-runtime/src/actions/image_generation/mod.rs; then
    printf 'Canvas image/video generation must consume the narrow API/Canvas contracts directly\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator::canvas::Canvas(Image|Video|Audio)ImportHost(Input|Port)|pub (struct|trait) Canvas(Image|Video|Audio)ImportHost(Input|Port)' \
    crates/host/a4n-orchestrator/src/canvas crates/host/a4n-runtime/src; then
    printf 'Canvas media import Host contracts must remain in a4n-canvas-contract, not the orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'pub (struct|trait) Canvas(Image|Video|Audio)ImportHost(Input|Port)' \
    crates/canvas/a4n-canvas-contract/src/lib.rs || \
   ! rg -n 'a4n_canvas_contract::\{[^}]*Canvas(Image|Video)ImportHost' \
    crates/host/a4n-runtime/src/actions/canvas_workflow/media_import/image.rs \
    crates/host/a4n-runtime/src/actions/canvas_workflow/media_import/video.rs; then
    printf 'Canvas media import adapters must consume the narrow canvas contracts directly\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator|OrchestratorContext' \
    crates/host/a4n-runtime/src/actions/canvas_workflow/media_nodes/editing/host_port.rs; then
    printf 'Canvas media editing HostPort must consume a4n-canvas-contract directly\n' >&2
    exit 1
fi

if ! rg -n 'a4n_canvas_contract::CanvasEditingNodeHostPort|use a4n_canvas_contract::\{' \
    crates/host/a4n-orchestrator/src/canvas/editing_nodes/execution.rs \
    crates/host/a4n-orchestrator/src/context/canvas.rs \
    crates/host/a4n-runtime/src/actions/canvas_workflow/media_nodes/editing/host_port.rs; then
    printf 'Canvas editing execution and HostPort must consume the narrow canvas contract\n' >&2
    exit 1
fi

if rg -n 'a4n_db|a4n_db_models|CanvasGraphRecordDocument|canvas_node::Model|CanvasPreviewApplicationPort' \
    crates/host/a4n-runtime/src/actions/canvas_workflow/media_nodes/editing.rs; then
    printf 'Canvas editing action must not carry Canvas ORM records or the mixed execution port\n' >&2
    exit 1
fi

if ! rg -n 'canvas_editing_application\(\)' \
    crates/host/a4n-runtime/src/actions/canvas_workflow/media_nodes/editing.rs; then
    printf 'Canvas editing action must route through CanvasEditingNodeApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait CanvasEditingNodeApplicationPort' crates/canvas/a4n-canvas-contract/src/lib.rs; then
    printf 'CanvasEditingNodeApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'a4n_db::cleanup_outbox|a4n_db_models::models::task::cleanup_outbox|cleanup_outbox::(Model|ActiveModel|Entity|Column)' \
    crates/host/a4n-runtime/src --glob '*.rs'; then
    printf 'Runtime cleanup outbox code must consume pure cleanup claims instead of cleanup ORM types\n' >&2
    exit 1
fi

if rg -n 'a4n_db_models::models::ai::video_gen_config|video_gen_config::Model' \
    crates/host/a4n-runtime/src/actions/video_generation \
    crates/host/a4n-runtime/src/sidecar_clients/video \
    crates/host/a4n-runtime/src/runtime_config --glob '*.rs'; then
    printf 'Runtime video provider paths must consume VideoProviderConfig instead of video ORM models\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator::generation::GeneratedMediaArtifactFile|pub use artifacts::GeneratedMediaArtifactFile|pub struct GeneratedMediaArtifactFile' \
    crates/host/a4n-runtime/src crates/host/a4n-orchestrator/src/generation; then
    printf 'GeneratedMediaArtifactFile must remain a pure domain media fact, not an orchestrator export\n' >&2
    exit 1
fi

if ! rg -n 'a4n_domain::media::GeneratedMediaArtifactFile' \
    crates/host/a4n-runtime/src/file_utils.rs \
    crates/host/a4n-orchestrator/src/generation/artifacts.rs; then
    printf 'Media artifact inspection and event coordination must consume the domain artifact fact\n' >&2
    exit 1
fi

if rg -n 'a4n_db_models|a4n_db::' \
    crates/host/a4n-runtime/src/actions/video_generation/provider_request/record.rs; then
    printf 'Runtime video provider-request snapshots must consume domain video constants instead of DB models\n' >&2
    exit 1
fi

if ! rg -n 'VideoProviderConfig' \
    crates/provider/a4n-provider-contract/src/lib.rs \
    crates/host/a4n-orchestrator/src/video/provider/execution.rs \
    crates/host/a4n-runtime/src/sidecar_clients/video \
    crates/host/a4n-runtime/src/actions/video_generation/provider_host.rs; then
    printf 'VideoProviderConfig must remain the provider execution boundary\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator::video::VideoProviderPollTarget|pub enum VideoProviderPollTarget' \
    crates/host/a4n-runtime/src crates/host/a4n-orchestrator/src/video; then
    printf 'VideoProviderPollTarget must remain in a4n-video-host-contract, not the orchestrator facade\n' >&2
    exit 1
fi

if rg -U -n '^pub use a4n_video_contract::\{[^}]*\b(ChapterVideoGenerationHostPort|ReferenceMediaVideoGenerationHostPort|VideoGenerationHostPort)\b' \
    crates/host/a4n-orchestrator/src/video/mod.rs; then
    printf 'Video generation HostPorts must remain in a4n-video-host-contract, not the orchestrator facade\n' >&2
    exit 1
fi

if rg -n '^pub use a4n_video::VideoDurationCapability' \
    crates/host/a4n-orchestrator/src/video/mod.rs; then
    printf 'VideoDurationCapability must remain owned by a4n-video, not the orchestrator facade\n' >&2
    exit 1
fi

if rg -n '^pub use contracts::ProjectChapterVideoSource' \
    crates/host/a4n-orchestrator/src/video/mod.rs; then
    printf 'ProjectChapterVideoSource must remain an internal video owner type\n' >&2
    exit 1
fi

if rg -n '^pub use chapter_video_generation::GenerateChapterVideosBatchInput' \
    crates/host/a4n-orchestrator/src/video/mod.rs; then
    printf 'GenerateChapterVideosBatchInput must remain an internal video generation type\n' >&2
    exit 1
fi

if rg -U -n '^pub use chapter_video_request_rules::\{[^}]*\bBoundaryFrameImageIds\b' \
    crates/host/a4n-orchestrator/src/video/mod.rs; then
    printf 'BoundaryFrameImageIds must remain an internal video request type\n' >&2
    exit 1
fi

if rg -U -n '^pub use chapter_video_request_rules::\{[^}]*\bChapterVideoProviderRequestContext\b' \
    crates/host/a4n-orchestrator/src/video/mod.rs; then
    printf 'ChapterVideoProviderRequestContext must remain an internal video request type\n' >&2
    exit 1
fi

if rg -U -n '^pub use chapter_video_request_rules::\{[^}]*\bProviderPromptImageBinding\b' \
    crates/host/a4n-orchestrator/src/video/mod.rs; then
    printf 'ProviderPromptImageBinding must remain an internal video request type\n' >&2
    exit 1
fi

if rg -n '^pub use prompt_chain_rules::' crates/host/a4n-orchestrator/src/video/mod.rs; then
    printf 'Video prompt-chain rules must remain internal to the video owner\n' >&2
    exit 1
fi

if rg -n '^pub use assembly_push_rules::ASSEMBLY_PUSH_STATUS_PENDING' \
    crates/host/a4n-orchestrator/src/video/mod.rs; then
    printf 'Assembly push status constants must remain internal to the video owner\n' >&2
    exit 1
fi

if rg -n '^pub use boundary_frame_rules::' crates/host/a4n-orchestrator/src/video/mod.rs; then
    printf 'Boundary frame rules must remain internal to the video owner\n' >&2
    exit 1
fi

if rg -n 'latest_boundary_frame_image_id' crates/host/a4n-orchestrator/src/video/mod.rs; then
    printf 'Unused latest boundary frame facade must not return to the video module\n' >&2
    exit 1
fi

if rg -n '^pub use first_frame_chain_rules::' crates/host/a4n-orchestrator/src/video/mod.rs; then
    printf 'First-frame chain rules must remain internal to the video owner\n' >&2
    exit 1
fi

if rg -n '^pub use generation_run_rules::' crates/host/a4n-orchestrator/src/video/mod.rs; then
    printf 'Video generation run inputs must remain internal to the video owner\n' >&2
    exit 1
fi

if rg -n '^pub use job_status_rules::' crates/host/a4n-orchestrator/src/video/mod.rs \
    || rg -n 'CanvasVideoWorkflowContext' crates/host/a4n-orchestrator/src/video/mod.rs; then
    printf 'Video job-status rules must remain internal to the video owner\n' >&2
    exit 1
fi

if rg -n 'validate_generated_video_interrupted_status' \
    crates/host/a4n-orchestrator/src/video/mod.rs \
    crates/host/a4n-orchestrator/src/video/job_status_rules.rs; then
    printf 'Unused video interrupted-status helper must not return\n' >&2
    exit 1
fi

if rg -n '^pub use library::' crates/host/a4n-orchestrator/src/video/mod.rs; then
    printf 'Video library rules must remain internal to the video owner\n' >&2
    exit 1
fi

if rg -n '^pub use prompt_application::' crates/host/a4n-orchestrator/src/video/mod.rs; then
    printf 'Video prompt application inputs must remain internal to the video owner\n' >&2
    exit 1
fi

if rg -n 'VideoSegmentPlanDraftState' crates/host/a4n-orchestrator/src/video/mod.rs; then
    printf 'Unused video segment draft state facade must not return\n' >&2
    exit 1
fi

if rg -n 'pub use a4n_video_contract::VideoProviderValidationMode' \
    crates/host/a4n-orchestrator/src/video/reference_media_rules.rs \
    || rg -U -n 'pub use reference_media_rules::\{[^}]*\bVideoProviderValidationMode\b' \
    crates/host/a4n-orchestrator/src/video/mod.rs; then
    printf 'VideoProviderValidationMode must remain in a4n-video-contract, not the orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'a4n_video_host_contract::VideoProviderPollTarget|VideoProviderPollTarget' \
    crates/host/a4n-runtime/src/actions/video_generation/poll.rs \
    crates/host/a4n-orchestrator/src/video/records \
    crates/video/a4n-video-host-contract/src/lib.rs; then
    printf 'Video provider polling must consume the narrow video contract\n' >&2
    exit 1
fi

video_contract_tree="$(cargo tree --locked -e normal,build -p a4n-video-contract)"
if rg -n '(^|[[:space:]])(a4n-(core|db|db-models|runtime|orchestrator|server|tauri)|sea-orm|sqlx)([[:space:]]|v|$)' \
    <<<"$video_contract_tree"; then
    printf 'a4n-video-contract must remain independent of Host, database, ORM, and orchestrator crates\n' >&2
    exit 1
fi

video_host_contract_tree="$(cargo tree --locked -e normal,build -p a4n-video-host-contract)"
if rg -n '(^|[[:space:]])a4n-video([[:space:]]|v|$)' <<<"$video_host_contract_tree"; then
    printf 'a4n-video-host-contract must consume provider/sidecar contracts instead of a4n-video implementation\n' >&2
    exit 1
fi

if rg -n 'pub trait VideoProviderHostPort|a4n_orchestrator::video::VideoProviderHostPort' \
    crates/host/a4n-orchestrator/src crates/host/a4n-runtime/src; then
    printf 'VideoProviderHostPort must remain in a4n-video-host-contract, not the orchestrator facade\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator::video::VideoProviderExecutionApplicationPort' \
    crates/host/a4n-runtime/src; then
    printf 'Runtime video polling must consume the API contract application port directly\n' >&2
    exit 1
fi

image_provider_tree="$(cargo tree --locked -e normal,build -p a4n-image-providers)"
if rg -n '(^|[[:space:]])a4n-image([[:space:]]|v|$)' <<<"$image_provider_tree" \
    || rg -n 'a4n_image::' crates/image/a4n-image-providers/src crates/host/a4n-runtime/src/sidecar_clients/image; then
    printf 'Image providers and Runtime image adapters must consume a4n-image-provider-contract instead of a4n-image\n' >&2
    exit 1
fi

if ! rg -n 'pub trait VideoProviderExecutionApplicationPort' \
    crates/host/a4n-api-contract/src/video.rs || \
   ! rg -n 'a4n_video_host_contract::VideoProviderHostPort' \
    crates/host/a4n-api-contract/src/video.rs \
    crates/host/a4n-orchestrator/src/video/records \
    crates/host/a4n-runtime/src/actions/video_generation/provider_host.rs; then
    printf 'Video provider polling must use the narrow API and runtime contracts directly\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator::video::ImportedVideoProperties|pub struct ImportedVideoProperties' \
    crates/host/a4n-runtime/src crates/host/a4n-orchestrator/src/video; then
    printf 'ImportedVideoProperties must remain in a4n-video-host-contract, not the orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'a4n_video_host_contract::ImportedVideoProperties' \
    crates/host/a4n-runtime/src/actions/video_records/imports/source/probe.rs; then
    printf 'Video probing must return the narrow ImportedVideoProperties contract\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator::video::(VideoProviderRequestInput|VideoProviderRequestBody|PersistedVideoResult|VideoProviderResultTarget)|pub (struct|enum) (VideoProviderRequestInput|VideoProviderRequestBody|PersistedVideoResult|VideoProviderResultTarget)' \
    crates/host/a4n-runtime/src crates/host/a4n-orchestrator/src/video; then
    printf 'Video provider execution DTOs must remain in a4n-video-host-contract, not the orchestrator facade\n' >&2
    exit 1
fi

for video_contract_consumer in \
    crates/host/a4n-orchestrator/src/video/chapter_video_generation \
    crates/host/a4n-orchestrator/src/video/reference_media_generation.rs \
    crates/host/a4n-orchestrator/src/video/provider/execution.rs \
    crates/host/a4n-runtime/src/actions/video_generation/provider_host.rs \
    crates/host/a4n-runtime/src/actions/video_generation/job_lifecycle/completion/video.rs \
    crates/host/a4n-runtime/src/actions/video_generation/media/tail_frames/capture.rs \
    crates/host/a4n-runtime/src/actions/video_records/imports/chapter.rs; do
    if ! rg -n 'a4n_video_host_contract::' "$video_contract_consumer"; then
        printf '%s must consume video provider DTOs from a4n-video-host-contract directly\n' "$video_contract_consumer" >&2
        exit 1
    fi
done

if rg -n 'a4n_orchestrator::video::(VideoRequestPreviewApplicationPort|CanvasVideoRequestPreviewApplicationPort|ChapterVideoRequestPreviewApplicationPort|ReferenceMediaVideoRequestPreviewApplicationPort)|pub trait VideoRequestPreviewApplicationPort' \
    crates/host/a4n-runtime/src crates/host/a4n-orchestrator/src; then
    printf 'Video request previews must use source-specific API application ports and stable video preview contracts\n' >&2
    exit 1
fi

if ! rg -n 'pub trait CanvasVideoRequestPreviewApplicationPort' crates/host/a4n-api-contract/src/video.rs || \
   ! rg -n 'pub trait ChapterVideoRequestPreviewApplicationPort' crates/host/a4n-api-contract/src/video.rs || \
   ! rg -n 'pub trait ReferenceMediaVideoRequestPreviewApplicationPort' crates/host/a4n-api-contract/src/video.rs || \
   ! rg -n 'pub (struct|trait) (CanvasVideoRequestPreview|ChapterVideoRequestPreview|ReferenceMediaVideoRequestPreview|CanvasVideoPreviewHostPort|ChapterVideoPreviewHostPort|ReferenceMediaVideoPreviewHostPort)' \
    crates/video/a4n-video-host-contract/src/lib.rs; then
    printf 'Video request preview contracts are missing from their stable owners\n' >&2
    exit 1
fi
