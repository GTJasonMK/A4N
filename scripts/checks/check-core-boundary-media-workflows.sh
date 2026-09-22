#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"
cd "$PROJECT_ROOT"

if rg -n 'a4n_db::video' \
    crates/host/a4n-runtime/src/cleanup_outbox/publication.rs \
    crates/host/a4n-runtime/src/actions/image_records/upload/record.rs; then
    printf 'Runtime image publication paths must consume media status contracts instead of the video DB facade\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator|OrchestratorContext' crates/host/a4n-runtime/src/actions/rag_search; then
    printf 'RAG search actions must consume a4n-rag-contract instead of the full orchestrator context\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator|OrchestratorContext|state\.orchestrator\(\)' \
    crates/host/a4n-runtime/src/rag_ingest --glob '*.rs'; then
    printf 'RAG ingest actions must consume a4n-rag-contract instead of the full orchestrator context\n' >&2
    exit 1
fi

if ! rg -n 'pub trait RagIngestPort' crates/rag/a4n-rag-contract/src/lib.rs; then
    printf 'RagIngestPort contract is missing\n' >&2
    exit 1
fi

if rg -U -n 'state\s*\.\s*orchestrator\(\)\s*\.\s*clear_project_rag' \
    crates/host/a4n-runtime/src/cleanup_outbox/claim.rs \
    crates/host/a4n-runtime/src/actions/generation_tasks/destructive_lifecycle.rs; then
    printf 'RAG project cleanup must consume RagIngestPort instead of the concrete orchestrator\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\)\.(image_publications_for_recovery|lock_image_publications|image_publication_by_id|generated_image_by_id|publish_imported_image_publication_for_user_validated|ensure_(portrait|profile_material)_publication_identity_for_user_validated|mark_image_publication_(reclaiming|reclaimed)|unreferenced_media_paths|media_path_prefix_has_persisted_reference|project_has_active_image_publication|protected_image_publication_cleanup_paths)' \
    crates/host/a4n-runtime/src/cleanup_outbox/publication.rs \
    crates/host/a4n-runtime/src/cleanup_outbox/claim.rs; then
    printf 'cleanup outbox media recovery must avoid direct media-owner calls\n' >&2
    exit 1
fi

if ! rg -n 'image_publication_recovery_application\(\)' \
    crates/host/a4n-runtime/src/cleanup_outbox/publication.rs; then
    printf 'cleanup outbox publication recovery must route through ImagePublicationRecoveryPort\n' >&2
    exit 1
fi
if ! rg -n 'image_publication_recovery_read_application\(\)' \
    crates/host/a4n-runtime/src/cleanup_outbox/publication.rs; then
    printf 'cleanup outbox publication reads must route through ImagePublicationRecoveryReadPort\n' >&2
    exit 1
fi
if rg -U -n 'image_publication_recovery_application\(\)\s*\.\s*(image_publications_for_recovery|image_publication_by_id|generated_image_identity)' \
    crates/host/a4n-runtime/src/cleanup_outbox/publication.rs; then
    printf 'cleanup outbox publication reads must not use the state-transition port\n' >&2
    exit 1
fi

if ! rg -n 'image_media_reference_application\(\)' \
    crates/host/a4n-runtime/src/cleanup_outbox/publication.rs \
    crates/host/a4n-runtime/src/cleanup_outbox/claim.rs; then
    printf 'cleanup outbox media reference checks must route through ImageMediaReferencePort\n' >&2
    exit 1
fi

if rg -U -n 'state\s*\.\s*orchestrator\(\)\s*\.\s*renew_claimed_cleanup' \
    crates/host/a4n-runtime/src/cleanup_outbox/lease.rs; then
    printf 'cleanup outbox lease renewal must consume CleanupOutboxApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'cleanup_outbox_application\(\)' \
    crates/host/a4n-runtime/src/cleanup_outbox/lease.rs; then
    printf 'cleanup outbox lease renewal must route through the injected cleanup application port\n' >&2
    exit 1
fi

if ! rg -n 'pub trait CleanupOutboxApplicationPort' \
    crates/platform/a4n-cleanup-contract/src/lib.rs; then
    printf 'CleanupOutboxApplicationPort contract is missing\n' >&2
    exit 1
fi

for package in a4n-blueprint-contract a4n-chapter-generation-contract a4n-outline-contract; do
    contract_tree="$(cargo tree --locked -e normal,build -p "$package")"
    if rg -n '(^|[[:space:]])a4n-prompt([[:space:]]|v|$)' <<<"$contract_tree"; then
        printf '%s must depend on prompt contracts, not the prompt implementation crate\n' "$package" >&2
        exit 1
    fi
done

if rg -n 'state\.orchestrator\(\)' \
    crates/host/a4n-runtime/src/actions/blueprint_records/manual.rs \
    crates/host/a4n-runtime/src/actions/blueprint_records/world_setting.rs \
    crates/host/a4n-runtime/src/actions/blueprint_records/versions.rs; then
    printf 'Runtime blueprint mutation actions must consume BlueprintMutationApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'blueprint_mutation_application\(\)' \
    crates/host/a4n-runtime/src/actions/blueprint_records/manual.rs \
    crates/host/a4n-runtime/src/actions/blueprint_records/world_setting.rs \
    crates/host/a4n-runtime/src/actions/blueprint_records/versions.rs; then
    printf 'Runtime blueprint mutation actions must route through BlueprintMutationApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait BlueprintMutationApplicationPort' \
    crates/host/a4n-api-contract/src/blueprint.rs; then
    printf 'BlueprintMutationApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'preview_blueprint_(generation|refine)_request_for_user_validated' \
    crates/host/a4n-runtime/src/actions/blueprint_records/generation.rs \
    crates/host/a4n-runtime/src/actions/blueprint_records/refinement.rs; then
    printf 'Runtime blueprint request previews must consume BlueprintRequestPreviewApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'blueprint_generation_preview_application\(\)' \
    crates/host/a4n-runtime/src/actions/blueprint_records/generation.rs || \
   ! rg -n 'blueprint_refinement_preview_application\(\)' \
    crates/host/a4n-runtime/src/actions/blueprint_records/refinement.rs; then
    printf 'Runtime blueprint request previews must route through worker-specific preview ports\n' >&2
    exit 1
fi

if ! rg -n 'pub trait BlueprintGenerationPreviewApplicationPort' \
    crates/planning/a4n-blueprint-contract/src/lib.rs || \
   ! rg -n 'pub trait BlueprintRefinementPreviewApplicationPort' \
    crates/planning/a4n-blueprint-contract/src/lib.rs; then
    printf 'Blueprint worker-specific preview contracts are missing\n' >&2
    exit 1
fi

if ! rg -n 'pub trait BlueprintGenerationApplicationPort' \
    crates/planning/a4n-blueprint-contract/src/lib.rs || \
   ! rg -n 'blueprint_generation_application\(\)' \
    crates/host/a4n-runtime/src/actions/blueprint_records/generation.rs; then
    printf 'Blueprint generation workers must route through BlueprintGenerationApplicationPort\n' >&2
    exit 1
fi
if ! rg -n 'pub trait BlueprintRefinementApplicationPort' \
    crates/planning/a4n-blueprint-contract/src/lib.rs || \
   ! rg -n 'blueprint_refinement_application\(\)' \
    crates/host/a4n-runtime/src/actions/blueprint_records/refinement.rs; then
    printf 'Blueprint refinement workers must route through BlueprintRefinementApplicationPort\n' >&2
    exit 1
fi
if rg -n 'blueprint_generation_application\(\)' \
    crates/host/a4n-runtime/src/actions/blueprint_records/refinement.rs; then
    printf 'Blueprint refinement workers must not use the generation port\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator|OrchestratorContext|state\.orchestrator\(\)|actions::access' \
    crates/host/a4n-runtime/src/actions/canvas_workflow/records_api.rs; then
    printf 'Canvas record actions must consume CanvasApplicationPort instead of the concrete orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'pub trait Canvas(Read|Mutation)ApplicationPort' crates/host/a4n-api-contract/src/canvas.rs; then
    printf 'Canvas read/mutation contracts are missing\n' >&2
    exit 1
fi

if rg -n 'CanvasApplicationPort|canvas_application\(\)' \
    crates/host/a4n-api-contract/src/canvas.rs crates/host/a4n-orchestrator/src crates/host/a4n-runtime/src; then
    printf 'The mixed CanvasApplicationPort must remain removed\n' >&2
    exit 1
fi

if ! rg -n 'canvas_read_application\(\)' \
    crates/host/a4n-runtime/src/actions/canvas_workflow/records_api.rs; then
    printf 'Canvas record reads must route through CanvasReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'canvas_mutation_application\(\)' \
    crates/host/a4n-runtime/src/actions/canvas_workflow/records_api.rs; then
    printf 'Canvas record mutations must route through CanvasMutationApplicationPort\n' >&2
    exit 1
fi

if rg -n 'a4n_db|a4n_db_models|CanvasGraphRecordDocument|canvas_node::Model|a4n_orchestrator::OrchestratorContext|state\.orchestrator\(\)' \
    crates/host/a4n-runtime/src/actions/canvas_workflow/preview; then
    printf 'Canvas preview actions must not carry Canvas ORM records or the concrete orchestrator\n' >&2
    exit 1
fi

if ! rg -n 'canvas_preview_application\(\)' \
    crates/host/a4n-runtime/src/actions/canvas_workflow/preview.rs; then
    printf 'Canvas preview actions must route through CanvasPreviewApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait CanvasPreviewApplicationPort' \
    crates/canvas/a4n-canvas-host-contract/src/lib.rs; then
    printf 'CanvasPreviewApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'a4n_db|a4n_db_models|CanvasGraphRecordDocument|canvas_node::Model|CanvasPreviewApplicationPort' \
    crates/host/a4n-runtime/src/actions/canvas_workflow/output_nodes.rs; then
    printf 'Canvas output action must not carry Canvas ORM records or the mixed execution port\n' >&2
    exit 1
fi

if ! rg -n 'canvas_output_application\(\)' \
    crates/host/a4n-runtime/src/actions/canvas_workflow/output_nodes.rs; then
    printf 'Canvas output action must route through CanvasOutputNodeApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait CanvasOutputNodeApplicationPort' \
    crates/canvas/a4n-canvas-contract/src/lib.rs; then
    printf 'CanvasOutputNodeApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'a4n_db|a4n_db_models|CanvasGraphRecordDocument|canvas_node::Model|CanvasPreviewApplicationPort' \
    crates/host/a4n-runtime/src/actions/canvas_workflow/probe_node.rs; then
    printf 'Canvas probe action must not carry Canvas ORM records or the mixed execution port\n' >&2
    exit 1
fi

if ! rg -n 'canvas_probe_application\(\)' \
    crates/host/a4n-runtime/src/actions/canvas_workflow/probe_node.rs; then
    printf 'Canvas probe action must route through CanvasProbeNodeApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait CanvasProbeNodeApplicationPort' \
    crates/canvas/a4n-canvas-contract/src/lib.rs; then
    printf 'CanvasProbeNodeApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'a4n_db|a4n_db_models|CanvasGraphRecordDocument|canvas_node::Model|CanvasPreviewApplicationPort|OrchestratorContext|state\.orchestrator\(\)' \
    crates/host/a4n-runtime/src/actions/canvas_workflow/text_nodes/render_text.rs; then
    printf 'Canvas RenderText action must not carry Canvas ORM records or the mixed execution port\n' >&2
    exit 1
fi

if ! rg -n 'canvas_render_text_application\(\)' \
    crates/host/a4n-runtime/src/actions/canvas_workflow/text_nodes/render_text.rs; then
    printf 'Canvas RenderText action must route through CanvasRenderTextNodeApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait CanvasRenderTextNodeApplicationPort' \
    crates/canvas/a4n-canvas-contract/src/lib.rs; then
    printf 'CanvasRenderTextNodeApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'a4n_db|a4n_db_models|CanvasGraphRecordDocument|canvas_node::Model|CanvasPreviewApplicationPort|OrchestratorContext|state\.orchestrator\(\)' \
    crates/host/a4n-runtime/src/actions/canvas_workflow/text_nodes/llm_text.rs; then
    printf 'Canvas LlmText action must not carry Canvas ORM records or the mixed execution port\n' >&2
    exit 1
fi

if ! rg -n 'canvas_llm_text_application\(\)' \
    crates/host/a4n-runtime/src/actions/canvas_workflow/text_nodes/llm_text.rs; then
    printf 'Canvas LlmText action must route through CanvasLlmTextNodeApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait CanvasLlmTextNodeApplicationPort' \
    crates/canvas/a4n-canvas-host-contract/src/lib.rs; then
    printf 'CanvasLlmTextNodeApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'a4n_db|a4n_db_models|CanvasGraphRecordDocument|canvas_node::Model|CanvasPreviewApplicationPort|OrchestratorContext|state\.orchestrator\(\)' \
    crates/host/a4n-runtime/src/actions/canvas_workflow/media_nodes/image.rs; then
    printf 'Canvas image action must not carry Canvas ORM records or the mixed execution port\n' >&2
    exit 1
fi

if ! rg -n 'canvas_image_application\(\)' \
    crates/host/a4n-runtime/src/actions/canvas_workflow/media_nodes/image.rs; then
    printf 'Canvas image action must route through CanvasImageNodeApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait CanvasImageNodeApplicationPort' \
    crates/canvas/a4n-canvas-host-contract/src/lib.rs; then
    printf 'CanvasImageNodeApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'a4n_db|a4n_db_models|CanvasGraphRecordDocument|canvas_node::Model|CanvasPreviewApplicationPort|OrchestratorContext|state\.orchestrator\(\)' \
    crates/host/a4n-runtime/src/actions/canvas_workflow/media_nodes/video.rs; then
    printf 'Canvas video action must not carry Canvas ORM records or the mixed execution port\n' >&2
    exit 1
fi

if ! rg -n 'canvas_video_application\(\)' \
    crates/host/a4n-runtime/src/actions/canvas_workflow/media_nodes/video.rs; then
    printf 'Canvas video action must route through CanvasVideoNodeApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait CanvasVideoNodeApplicationPort' \
    crates/canvas/a4n-canvas-host-contract/src/lib.rs; then
    printf 'CanvasVideoNodeApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\)' \
    crates/host/a4n-runtime/src/actions/canvas_workflow/execution/api.rs; then
    printf 'Canvas graph execution actions must consume CanvasGraphExecutionApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'canvas_graph_preparation_application\(\)' \
    crates/host/a4n-runtime/src/actions/canvas_workflow/execution/api.rs || \
   ! rg -n 'canvas_graph_execution_application\(\)' \
    crates/host/a4n-runtime/src/actions/canvas_workflow/execution/api.rs || \
   ! rg -n 'canvas_graph_failure_application\(\)' \
    crates/host/a4n-runtime/src/actions/canvas_workflow/execution/api.rs; then
    printf 'Canvas graph actions must route through preparation, execution and failure contracts\n' >&2
    exit 1
fi

if ! rg -n 'pub trait CanvasGraph(Preparation|Execution|Failure)ApplicationPort' \
    crates/canvas/a4n-canvas-contract/src/lib.rs; then
    printf 'Canvas graph preparation/execution/failure contracts are missing\n' >&2
    exit 1
fi

if ! rg -n 'pub trait Canvas(Image|Video|Audio)ImportApplicationPort' \
    crates/canvas/a4n-canvas-contract/src/lib.rs; then
    printf 'Canvas image/video/audio import contracts are missing\n' >&2
    exit 1
fi

if rg -n 'canvas_media_import_application\(\)|CanvasMediaImportApplicationPort' \
    crates/host/a4n-runtime/src/actions/canvas_workflow/media_import \
    crates/host/a4n-runtime/src/state/app.rs; then
    printf 'Canvas media import consumers must use media-kind-specific contracts\n' >&2
    exit 1
fi

if ! rg -n 'canvas_image_import_application\(\)' \
    crates/host/a4n-runtime/src/actions/canvas_workflow/media_import/image.rs || \
   ! rg -n 'canvas_video_import_application\(\)' \
    crates/host/a4n-runtime/src/actions/canvas_workflow/media_import/video.rs || \
   ! rg -n 'canvas_audio_import_application\(\)' \
    crates/host/a4n-runtime/src/actions/canvas_workflow/media_import/audio.rs; then
    printf 'Canvas media import actions must route through media-kind-specific contracts\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator::canvas::CanvasGraph|a4n_orchestrator::canvas::PreparedCanvasGraphRun' \
    crates/host/a4n-runtime/src/actions/canvas_workflow/execution \
    crates/host/a4n-runtime/src/state/app.rs; then
    printf 'Canvas graph execution consumers must use a4n-canvas-contract directly\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator::canvas' \
    crates/host/a4n-runtime/src/actions/canvas_workflow/execution; then
    printf 'Canvas graph execution consumers must not import the orchestrator Canvas facade\n' >&2
    exit 1
fi

if ! rg -n 'pub (enum CanvasGraphNodeExecutionKind|struct CanvasGraphNodeExecutionInput|struct PreparedCanvasGraphRun)' \
    crates/canvas/a4n-canvas-contract/src/lib.rs || \
   ! rg -n 'pub trait CanvasGraphPreparationHostPort' \
    crates/canvas/a4n-canvas-contract/src/lib.rs || \
   ! rg -n 'pub trait CanvasGraphExecutionHostPort' \
    crates/canvas/a4n-canvas-contract/src/lib.rs; then
    printf 'Canvas graph execution and preparation Host contracts are missing from a4n-canvas-contract\n' >&2
    exit 1
fi

if rg -n 'a4n_db|a4n_db_models|CanvasGraphRecordDocument|canvas_node::Model' \
    crates/canvas/a4n-canvas-contract/src/lib.rs \
    crates/host/a4n-runtime/src/actions/canvas_workflow/execution/host_port.rs; then
    printf 'Canvas graph execution HostPort must carry node IDs, not Canvas ORM records\n' >&2
    exit 1
fi

if ! rg -n 'node_id' \
    crates/canvas/a4n-canvas-contract/src/lib.rs \
    crates/host/a4n-runtime/src/actions/canvas_workflow/execution/host_port.rs; then
    printf 'Canvas graph execution HostPort node identity input is missing\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\)|OrchestratorContext' \
    crates/host/a4n-runtime/src/actions/canvas_workflow/text_nodes/host_port.rs; then
    printf 'Canvas text node HostPort must consume ProviderRuntime instead of the concrete orchestrator\n' >&2
    exit 1
fi

if rg -n 'queue_run_tracked' \
    crates/host/a4n-runtime/src/actions/canvas_workflow/text_nodes/host_port.rs; then
    printf 'Canvas text node HostPort must not acquire a second LLM queue slot\n' >&2
    exit 1
fi

if ! rg -q 'chat_with_config_id' \
    crates/host/a4n-runtime/src/actions/canvas_workflow/text_nodes/host_port.rs || \
   ! rg -q 'queue_start_llm_task' \
    crates/host/a4n-runtime/src/dynamic_llm/provider/preparation.rs || \
   ! rg -q 'concurrency_limit\(\)' \
    crates/host/a4n-runtime/src/dynamic_llm/provider/preparation.rs || \
   ! rg -q 'trace.workflow_run_id' \
    crates/host/a4n-runtime/src/provider_runtime.rs; then
    printf 'Canvas LLM calls must retain resolved model admission and workflow tracking through the shared provider path\n' >&2
    exit 1
fi

if rg -U -n 'state\s*\.\s*orchestrator\(\)\s*\.\s*queue_run_tracked_with_workflow_context' \
    crates/host/a4n-runtime/src/actions/canvas_workflow/media_nodes/editing/host_port.rs; then
    printf 'Canvas media editing HostPort must consume ProviderRuntime instead of the concrete orchestrator\n' >&2
    exit 1
fi

if ! rg -n 'provider_runtime\(\)' \
    crates/host/a4n-runtime/src/actions/canvas_workflow/media_nodes/editing/host_port.rs || \
   ! rg -n 'queue_run_tracked_with_workflow_context' \
    crates/host/a4n-runtime/src/actions/canvas_workflow/media_nodes/editing/host_port.rs; then
    printf 'Canvas media editing HostPort must route workflow-tracked provider calls through ProviderRuntime\n' >&2
    exit 1
fi

if rg -U -n 'state\s*\.\s*orchestrator\(\)\s*\.\s*queue_' \
    crates/host/a4n-runtime/src/actions/image_generation/generation_io/queue.rs; then
    printf 'Image generation queue helpers must consume ProviderRuntime instead of the concrete orchestrator\n' >&2
    exit 1
fi

if ! rg -n 'provider_runtime\(\)' \
    crates/host/a4n-runtime/src/actions/image_generation/generation_io/queue.rs || \
   ! rg -n 'queue_run_tracked_with_(workflow_context|cancel)' \
    crates/host/a4n-runtime/src/actions/image_generation/generation_io/queue.rs; then
    printf 'Image generation queue helpers must route tracked calls through ProviderRuntime\n' >&2
    exit 1
fi

if rg -U -n 'state\s*\.\s*orchestrator\(\)|OrchestratorContext|a4n_db_models::models::project::novel_project|novel_project::Model' \
    crates/host/a4n-runtime/src/actions/project_duplication/execution.rs; then
    printf 'Runtime project duplication must not expose project ORM records or call the concrete orchestrator\n' >&2
    exit 1
fi

if ! rg -n 'project_duplication_application\(\)' \
    crates/host/a4n-runtime/src/actions/project_duplication/execution.rs; then
    printf 'Runtime project duplication must route through ProjectDuplicationApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ProjectDuplicationApplicationPort' \
    crates/host/a4n-api-contract/src/project.rs; then
    printf 'ProjectDuplicationApplicationPort API contract is missing\n' >&2
    exit 1
fi
if rg -n 'pub trait ProjectDuplicationApplicationPort' \
    crates/host/a4n-orchestrator/src/project; then
    printf 'ProjectDuplicationApplicationPort must not be redefined in the orchestrator\n' >&2
    exit 1
fi

if ! rg -n 'pub trait Settings(Export|Import)ApplicationPort|pub trait SettingsAllHostPort|pub struct PromptRow' \
    crates/host/a4n-api-contract/src/config.rs; then
    printf 'split settings import/export contracts must live in the API contract\n' >&2
    exit 1
fi
if rg -n 'pub trait Settings(Export|Import)ApplicationPort|pub trait SettingsAllApplicationPort|pub struct PromptRow' \
    crates/host/a4n-orchestrator/src/config; then
    printf 'settings import/export contracts must not be redefined in the orchestrator\n' >&2
    exit 1
fi
if rg -n '(^|[^A-Za-z])SettingsAllApplicationPort($|[^A-Za-z])|(^|[^A-Za-z_])settings_all_application\(\)' \
    crates/host/a4n-api-contract/src crates/host/a4n-orchestrator/src crates/host/a4n-runtime/src; then
    printf 'The mixed SettingsAllApplicationPort must remain removed\n' >&2
    exit 1
fi
if ! rg -n 'settings_export_application\(\)' \
    crates/host/a4n-runtime/src/actions/settings_all/mod.rs || \
   ! rg -n 'settings_import_application\(\)' \
    crates/host/a4n-runtime/src/actions/settings_all/mod.rs; then
    printf 'Runtime settings actions must route through split import/export ports\n' >&2
    exit 1
fi

if rg -U -n 'state\s*\.\s*orchestrator\(\)|OrchestratorContext|a4n_db_models::models::project::novel_project|novel_project::Model' \
    crates/host/a4n-runtime/src/actions/project_records/reset \
    crates/host/a4n-runtime/src/actions/generation_tasks/destructive_lifecycle.rs; then
    printf 'Runtime project generated-data reset must not expose project ORM records or call the concrete orchestrator\n' >&2
    exit 1
fi

if ! rg -U -n 'generation_lifecycle_application\(\)\s*\.\s*reset_project_generated_data_validated' \
    crates/host/a4n-runtime/src/actions/generation_tasks/destructive_lifecycle.rs; then
    printf 'Runtime project generated-data reset must route through GenerationLifecycleApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait GenerationTaskReadApplicationPort' \
    crates/workflow/a4n-generation-host-contract/src/lib.rs || \
   ! rg -n 'pub trait GenerationTaskMutationApplicationPort' \
    crates/workflow/a4n-generation-host-contract/src/lib.rs || \
   ! rg -n 'pub trait GenerationLifecycleApplicationPort' \
    crates/workflow/a4n-generation-host-contract/src/lib.rs || \
   ! rg -n 'pub trait GenerationCleanupApplicationPort' \
    crates/workflow/a4n-generation-host-contract/src/lib.rs; then
    printf 'Generation task/lifecycle/cleanup contracts are missing\n' >&2
    exit 1
fi

if rg -n '(^|[^A-Za-z])GenerationApplicationPort($|[^A-Za-z])|(^|[^A-Za-z_])generation_application\(\)' \
    crates/host/a4n-orchestrator/src crates/host/a4n-runtime/src; then
    printf 'The mixed GenerationApplicationPort must remain removed\n' >&2
    exit 1
fi

if ! rg -n 'generation_task_read_application\(\)' \
    crates/host/a4n-runtime/src/actions/generation_tasks/mod.rs \
    crates/host/a4n-runtime/src/actions/portrait_generation/application.rs; then
    printf 'Runtime generation task reads must route through GenerationTaskReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'generation_task_mutation_application\(\)' \
    crates/host/a4n-runtime/src/actions/generation_tasks/mod.rs \
    crates/host/a4n-runtime/src/actions/blueprint_records/generation.rs \
    crates/host/a4n-runtime/src/actions/blueprint_records/refinement.rs; then
    printf 'Runtime generation task mutations must route through GenerationTaskMutationApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'generation_lifecycle_application\(\)' \
    crates/host/a4n-runtime/src/actions/generation_tasks/destructive_lifecycle.rs; then
    printf 'Runtime generation lifecycle must route through GenerationLifecycleApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'generation_cleanup_application\(\)' \
    crates/host/a4n-runtime/src/cleanup_outbox/worker.rs; then
    printf 'Runtime generation retention must route through GenerationCleanupApplicationPort\n' >&2
    exit 1
fi

if rg -n 'a4n_db::generation|a4n_db_models::models|PersistedGenerationTask|GenerationTaskPersistenceError' \
    crates/host/a4n-runtime/src/actions/generation_tasks; then
    printf 'Runtime generation task actions must consume domain lifecycle contracts, not DB models\n' >&2
    exit 1
fi

if ! rg -n 'a4n_domain::generation::GenerationTaskListCursor' \
    crates/host/a4n-runtime/src/actions/generation_tasks; then
    printf 'Runtime generation task history must consume the domain cursor contract\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\)|OrchestratorContext|a4n_db_models::models::ai::generated_image|generated_image::Model|prepare_imported_image_upload_for_user_validated|create_imported_image_publication_intent_for_user_validated|publish_imported_image_publication_for_user_validated|generated_image_by_id' \
    crates/host/a4n-runtime/src/actions/canvas_workflow/media_import/image.rs \
    crates/host/a4n-runtime/src/actions/image_records/upload.rs \
    crates/host/a4n-runtime/src/actions/image_records/upload/record.rs; then
    printf 'Canvas image import and image upload must not expose ORM records or call the concrete orchestrator\n' >&2
    exit 1
fi

if ! rg -n 'image_import_preparation_application\(\)' \
    crates/host/a4n-runtime/src/actions/image_records/upload.rs; then
    printf 'Image upload preparation must route through ImageImportPreparationApplicationPort\n' >&2
    exit 1
fi
if ! rg -n 'image_import_publication_application\(\)' \
    crates/host/a4n-runtime/src/actions/image_records/upload/record.rs; then
    printf 'Image publication must route through ImageImportPublicationApplicationPort\n' >&2
    exit 1
fi
if rg -n '\.image_import_application\(\)' crates/host/a4n-runtime/src; then
    printf 'Runtime image import must not use the mixed ImageImportApplicationPort\n' >&2
    exit 1
fi
if ! rg -n 'image_record_read_application\(\)' \
    crates/host/a4n-runtime/src/actions/image_records/upload/record.rs; then
    printf 'Published image lookup must route through ImageRecordReadApplicationPort\n' >&2
    exit 1
fi
if rg -n 'imported_image_summary' crates/host/a4n-runtime/src; then
    printf 'Runtime image import must not keep a read method on ImageImportApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ImageImport(Preparation|Publication)ApplicationPort|pub trait ImagePublicationRecoveryPort' \
    crates/image/a4n-image-host-contract/src/lib.rs; then
    printf 'image import preparation/publication and recovery contracts are missing\n' >&2
    exit 1
fi
if ! rg -n 'pub trait ImagePublicationRecoveryReadPort' crates/host/a4n-api-contract/src/image.rs; then
    printf 'image publication recovery read contract is missing from the API contract\n' >&2
    exit 1
fi
if rg -n 'pub trait ImagePublicationRecoveryReadPort' crates/host/a4n-orchestrator/src/media; then
    printf 'ImagePublicationRecoveryReadPort must not be redefined in the orchestrator\n' >&2
    exit 1
fi
if rg -n 'a4n_orchestrator::media::ImagePublicationRecoveryReadPort' crates/host/a4n-runtime/src; then
    printf 'Runtime image publication recovery reads must consume the API contract directly\n' >&2
    exit 1
fi
if ! rg -n 'pub trait ImageMediaReferencePort' crates/host/a4n-api-contract/src/image.rs; then
    printf 'image media reference contract is missing from the API contract\n' >&2
    exit 1
fi
if rg -n 'pub trait ImageMediaReferencePort' crates/host/a4n-orchestrator/src/media; then
    printf 'ImageMediaReferencePort must not be redefined in the orchestrator\n' >&2
    exit 1
fi
if rg -n 'a4n_orchestrator::media::ImageMediaReferencePort' crates/host/a4n-runtime/src; then
    printf 'Runtime image media reference adapters must consume the API contract directly\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ImageRecord(Read|Mutation)ApplicationPort' \
    crates/host/a4n-api-contract/src/image.rs; then
    printf 'image record read/mutation contracts are missing\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\)|OrchestratorContext|actions::access' \
    crates/host/a4n-runtime/src/actions/export_files.rs; then
    printf 'Runtime project export actions must consume ProjectExportApplicationPort instead of the concrete orchestrator or duplicate authorization\n' >&2
    exit 1
fi

if ! rg -n 'project_export_application\(\)' crates/host/a4n-runtime/src/actions/export_files.rs; then
    printf 'Runtime project export actions must route through ProjectExportApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub struct ExportProgress' crates/host/a4n-api-contract/src/export.rs; then
    printf 'ExportProgress contract is missing from the API contract\n' >&2
    exit 1
fi
if rg -n 'pub struct ExportProgress|a4n_orchestrator::export::ExportProgress' \
    crates/host/a4n-orchestrator/src/export crates/host/a4n-runtime/src; then
    printf 'Runtime export progress must consume the API contract directly\n' >&2
    exit 1
fi

if rg -n '(^|[^A-Za-z])ExportApplicationPort($|[^A-Za-z])|(^|[^A-Za-z_])export_application\(\)' \
    crates/host/a4n-api-contract/src/export.rs \
    crates/host/a4n-orchestrator/src \
    crates/host/a4n-runtime/src; then
    printf 'The mixed ExportApplicationPort must remain removed\n' >&2
    exit 1
fi
if ! rg -n 'pub trait ProjectExportApplicationPort' crates/host/a4n-api-contract/src/export.rs || \
   ! rg -n 'pub trait GalgameExportApplicationPort' crates/host/a4n-api-contract/src/export.rs; then
    printf 'Project and Galgame export contracts are missing from the API contract\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\)|OrchestratorContext|actions::access' \
    crates/host/a4n-runtime/src/actions/video_records/records.rs \
    crates/host/a4n-runtime/src/actions/video_records/deletion.rs \
    crates/host/a4n-runtime/src/actions/video_records/playback.rs \
    crates/host/a4n-runtime/src/actions/video_records/assembly/actions/status.rs \
    crates/host/a4n-runtime/src/actions/video_records/imports/processing/trim.rs; then
    printf 'Runtime video record actions must consume narrow application ports\n' >&2
    exit 1
fi

if rg -n 'VideoRecordApplicationPort|video_record_application\(\)' \
    crates/host/a4n-api-contract/src/video.rs \
    crates/host/a4n-orchestrator/src/video \
    crates/host/a4n-runtime/src; then
    printf 'the mixed VideoRecordApplicationPort must remain removed\n' >&2
    exit 1
fi

if ! rg -n 'video_record_read_application\(\)' \
    crates/host/a4n-runtime/src/actions/video_records/records.rs \
    crates/host/a4n-runtime/src/actions/video_records/playback.rs \
    crates/host/a4n-runtime/src/actions/video_generation/reference_media.rs \
    crates/host/a4n-runtime/src/actions/video_generation/media/reference_images; then
    printf 'video record reads must route through VideoRecordReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'video_record_mutation_application\(\)' \
    crates/host/a4n-runtime/src/actions/video_records/records.rs \
    crates/host/a4n-runtime/src/actions/video_records/deletion.rs; then
    printf 'ordinary video record mutations must route through VideoRecordMutationApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'video_chapter_import_application\(\)' \
    crates/host/a4n-runtime/src/actions/video_records/imports/chapter.rs || \
   ! rg -n 'video_reference_media_import_application\(\)' \
    crates/host/a4n-runtime/src/actions/video_records/imports/reference/media.rs || \
   ! rg -n 'video_assembly_import_application\(\)' \
    crates/host/a4n-runtime/src/actions/video_records/imports/reference/assembly_insert.rs || \
   ! rg -n 'video_canvas_import_application\(\)' \
    crates/host/a4n-runtime/src/actions/video_records/imports/canvas.rs || \
   ! rg -n 'video_trim_application\(\)' \
    crates/host/a4n-runtime/src/actions/video_records/imports/processing/trim.rs; then
    printf 'video imports and trims must route through media-kind-specific contracts\n' >&2
    exit 1
fi

if ! rg -n 'video_assembly_push_application\(\)' \
    crates/host/a4n-runtime/src/actions/video_records/assembly/actions/push.rs \
    crates/host/a4n-runtime/src/actions/video_records/assembly/actions/status.rs || \
   ! rg -n 'video_assembly_planning_application\(\)' \
    crates/host/a4n-runtime/src/actions/video_records/assembly/push.rs || \
   ! rg -n 'video_assembly_postprocess_application\(\)' \
    crates/host/a4n-runtime/src/actions/video_records/assembly/postprocess/source.rs \
    crates/host/a4n-runtime/src/actions/video_records/assembly/actions/edit.rs; then
    printf 'Assembly actions must route through focused planning and postprocess contracts\n' >&2
    exit 1
fi

if ! rg -n 'video_record_lifecycle_application\(\)' \
    crates/host/a4n-runtime/src/actions/video_records \
    || \
   ! rg -n 'video_record_completion_application\(\)' \
    crates/host/a4n-runtime/src/actions/video_records/completion.rs || \
   ! rg -n 'video_record_failure_application\(\)' \
    crates/host/a4n-runtime/src/actions/video_records/imports/canvas/assembly.rs || \
   ! rg -n 'video_tail_frame_application\(\)' \
    crates/host/a4n-runtime/src/actions/video_generation/media/tail_frames; then
    printf 'video completion and cleanup must route through focused lifecycle contracts\n' >&2
    exit 1
fi

if ! rg -n 'video_canvas_media_application\(\)' \
    crates/host/a4n-runtime/src/actions/video_records/imports/canvas; then
    printf 'canvas video media actions must route through VideoCanvasMediaApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait VideoRecordReadApplicationPort|pub trait VideoRecordMutationApplicationPort|pub trait (ChapterVideoImportApplicationPort|ReferenceMediaVideoImportApplicationPort|AssemblyVideoImportApplicationPort|CanvasVideoImportApplicationPort|VideoTrimApplicationPort)|pub trait VideoAssemblyPlanningApplicationPort|pub trait VideoAssemblyPostprocessApplicationPort|pub trait VideoAssemblyPushApplicationPort|pub trait VideoRecordCompletionApplicationPort|pub trait VideoTailFrameApplicationPort|pub trait VideoRecordFailureApplicationPort|pub trait VideoRecordLifecycleApplicationPort|pub trait VideoCanvasMediaApplicationPort' \
    crates/host/a4n-api-contract/src/video.rs; then
    printf 'split video record contracts are missing from the API contract\n' >&2
    exit 1
fi
