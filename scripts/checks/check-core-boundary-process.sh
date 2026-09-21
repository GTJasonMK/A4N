#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"
cd "$PROJECT_ROOT"

if rg -n '\.orchestrator\(\)|OrchestratorContext' \
    crates/host/a4n-runtime/src/actions/galgame_records/pipeline; then
    printf 'Runtime Galgame pipeline actions must consume split Galgame pipeline ports\n' >&2
    exit 1
fi

if ! rg -n 'galgame_pipeline_execution_application\(\)' \
    crates/host/a4n-runtime/src/actions/galgame_records/pipeline/execution/start.rs || \
   ! rg -n 'galgame_pipeline_preview_application\(\)' \
    crates/host/a4n-runtime/src/actions/galgame_records/pipeline/preview.rs || \
   ! rg -n 'galgame_pipeline_read_application\(\)' \
    crates/host/a4n-runtime/src/actions/galgame_records/pipeline/status.rs || \
   ! rg -n 'galgame_pipeline_cancellation_application\(\)' \
    crates/host/a4n-runtime/src/actions/galgame_records/pipeline/execution/cancel.rs; then
    printf 'Runtime Galgame pipeline actions must route through split application ports\n' >&2
    exit 1
fi

if rg -n 'pub trait GalgamePipeline(Application|Execution|Preview|Read|Cancellation)ApplicationPort|pub use .*GalgamePipeline(Application|Execution|Preview|Read|Cancellation)ApplicationPort' \
    crates/host/a4n-orchestrator/src/galgame; then
    printf 'Galgame pipeline ports must not be owned or re-exported by the orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'pub trait GalgamePipelineExecutionApplicationPort' \
    crates/host/a4n-api-contract/src/galgame.rs || \
   ! rg -n 'pub trait GalgamePipelinePreviewApplicationPort' \
    crates/host/a4n-api-contract/src/galgame.rs || \
   ! rg -n 'pub trait GalgamePipelineReadApplicationPort' \
    crates/host/a4n-api-contract/src/galgame.rs || \
   ! rg -n 'pub trait GalgamePipelineCancellationApplicationPort' \
    crates/host/a4n-api-contract/src/galgame.rs; then
    printf 'Galgame pipeline split contracts are missing from the API contract\n' >&2
    exit 1
fi

if ! rg -n 'pub struct GalgamePipelinePreviewStep' \
    crates/host/a4n-api-contract/src/galgame.rs; then
    printf 'Galgame pipeline preview contract is missing from the API contract\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator::galgame::(GalgamePipelineProgressUpdate|GalgameProgressReporter)|pub (struct|type) (GalgamePipelineProgressUpdate|GalgameProgressReporter)' \
    crates/host/a4n-runtime/src crates/host/a4n-server/src crates/host/a4n-tauri/src \
    crates/host/a4n-orchestrator/src/galgame; then
    printf 'Galgame pipeline progress contracts must be consumed from a4n-api-contract\n' >&2
    exit 1
fi

if ! rg -n 'pub struct GalgamePipelineProgressUpdate|pub type GalgameProgressReporter' \
    crates/host/a4n-api-contract/src/galgame.rs; then
    printf 'Galgame pipeline progress contracts are missing from a4n-api-contract\n' >&2
    exit 1
fi

if rg -n '\.orchestrator\(\)|OrchestratorContext|actions::access' \
    crates/host/a4n-runtime/src/actions/manga_pipeline; then
    printf 'Runtime Manga pipeline actions must consume split Manga pipeline ports\n' >&2
    exit 1
fi

if ! rg -n 'manga_pipeline_execution_application\(\)' \
    crates/host/a4n-runtime/src/actions/manga_pipeline/execution.rs || \
   ! rg -n 'manga_pipeline_preview_application\(\)' \
    crates/host/a4n-runtime/src/actions/manga_pipeline/preview.rs || \
   ! rg -n 'manga_pipeline_read_application\(\)' \
    crates/host/a4n-runtime/src/actions/manga_records/pipeline.rs || \
   ! rg -n 'manga_pipeline_cancellation_application\(\)' \
    crates/host/a4n-runtime/src/actions/manga_records/pipeline.rs; then
    printf 'Runtime Manga pipeline actions must route through split application ports\n' >&2
    exit 1
fi

if rg -n 'pub trait MangaPipeline(Application|Execution|Preview|Read|Cancellation)ApplicationPort|pub use .*MangaPipeline(Application|Execution|Preview|Read|Cancellation)ApplicationPort' \
    crates/host/a4n-orchestrator/src/manga; then
    printf 'Manga pipeline ports must not be owned or re-exported by the orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'pub trait MangaPipelineExecutionApplicationPort' \
    crates/host/a4n-api-contract/src/manga.rs || \
   ! rg -n 'pub trait MangaPipelinePreviewApplicationPort' \
    crates/host/a4n-api-contract/src/manga.rs || \
   ! rg -n 'pub trait MangaPipelineReadApplicationPort' \
    crates/host/a4n-api-contract/src/manga.rs || \
   ! rg -n 'pub trait MangaPipelineCancellationApplicationPort' \
    crates/host/a4n-api-contract/src/manga.rs; then
    printf 'Manga pipeline split contracts are missing from the API contract\n' >&2
    exit 1
fi

if ! rg -n 'pub struct MangaPipelinePreviewStep' \
    crates/host/a4n-api-contract/src/manga.rs; then
    printf 'Manga pipeline preview contract is missing from the API contract\n' >&2
    exit 1
fi

if rg -n 'pub trait MangaRecord(Application|Read|Mutation)ApplicationPort|pub use .*MangaRecord(Application|Read|Mutation)ApplicationPort' \
    crates/host/a4n-orchestrator/src/manga; then
    printf 'Manga record application ports must not be owned or re-exported by the orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'pub trait MangaRecord(Read|Mutation)ApplicationPort' \
    crates/host/a4n-api-contract/src/manga.rs; then
    printf 'Manga record read/mutation contracts are missing from the API contract\n' >&2
    exit 1
fi

if rg -n '\.orchestrator\(\)|OrchestratorContext' \
    crates/host/a4n-runtime/src/actions/manga_records/records.rs \
    crates/host/a4n-runtime/src/actions/manga_records/optimization; then
    printf 'Runtime Manga record actions must consume Manga record read/mutation ports\n' >&2
    exit 1
fi

if ! rg -n 'manga_record_(read|mutation)_application\(\)' \
    crates/host/a4n-runtime/src/actions/manga_records/records.rs \
    crates/host/a4n-runtime/src/actions/manga_records/optimization; then
    printf 'Runtime Manga record actions must route through Manga record read/mutation ports\n' >&2
    exit 1
fi

if ! rg -n 'pub trait MangaRecord(Read|Mutation)ApplicationPort' \
    crates/host/a4n-api-contract/src/manga.rs; then
    printf 'Manga record read/mutation contracts are missing from the API contract\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator::manga::MangaPromptDownstreamCleanupPort|pub trait MangaPromptDownstreamCleanupPort' \
    crates/host/a4n-runtime/src crates/host/a4n-orchestrator/src; then
    printf 'Manga prompt cleanup must use the API contract directly, without an orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'pub trait MangaPromptDownstreamCleanupPort' \
    crates/host/a4n-api-contract/src/manga.rs || \
   ! rg -n 'a4n_api_contract::manga::MangaPromptDownstreamCleanupPort' \
    crates/host/a4n-runtime/src/actions/manga_records.rs \
    crates/host/a4n-orchestrator/src/manga/application.rs \
    crates/host/a4n-orchestrator/src/manga/prompt_optimization/execution.rs; then
    printf 'Manga prompt cleanup consumers must use the narrow API contract directly\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator::manga::MangaPromptDownstreamCleanupPort|pub trait MangaPromptDownstreamCleanupPort' \
    crates/host/a4n-runtime/src crates/host/a4n-orchestrator/src; then
    printf 'Manga prompt cleanup must use the API contract directly, without an orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'pub trait MangaPromptDownstreamCleanupPort' \
    crates/host/a4n-api-contract/src/manga.rs || \
   ! rg -n 'a4n_api_contract::manga::MangaPromptDownstreamCleanupPort' \
    crates/host/a4n-runtime/src/actions/manga_records.rs \
    crates/host/a4n-orchestrator/src/manga/application.rs \
    crates/host/a4n-orchestrator/src/manga/prompt_optimization/execution.rs; then
    printf 'Manga prompt cleanup consumers must use the narrow API contract directly\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\)|OrchestratorContext|actions::config_records::shared::require_config_admin' \
    crates/host/a4n-runtime/src/actions/config_records/llm/records \
    crates/host/a4n-runtime/src/actions/config_records/image/records.rs \
    crates/host/a4n-runtime/src/actions/config_records/video/records.rs \
    crates/host/a4n-runtime/src/actions/config_records/embedding/records.rs \
    crates/host/a4n-runtime/src/actions/config_records/discovery.rs; then
    printf 'Runtime config CRUD actions must consume split configuration ports instead of the concrete orchestrator or duplicate authorization\n' >&2
    exit 1
fi

if rg -n '(^|[^A-Za-z])ConfigApplicationPort($|[^A-Za-z])|config_application\(\)' \
    crates/host/a4n-api-contract/src/config.rs \
    crates/host/a4n-orchestrator/src \
    crates/host/a4n-runtime/src; then
    printf 'The mixed ConfigApplicationPort must remain removed\n' >&2
    exit 1
fi

if ! rg -n 'pub trait Config(Read|Mutation)ApplicationPort' \
    crates/host/a4n-api-contract/src/config.rs; then
    printf 'Split configuration application contracts are missing\n' >&2
    exit 1
fi

if ! rg -n 'config_(read|mutation)_application\(\)' \
    crates/host/a4n-runtime/src/actions/config_records/llm/records \
    crates/host/a4n-runtime/src/actions/config_records/image/records.rs \
    crates/host/a4n-runtime/src/actions/config_records/video/records.rs \
    crates/host/a4n-runtime/src/actions/config_records/embedding/records.rs \
    crates/host/a4n-runtime/src/actions/config_records/discovery.rs; then
    printf 'Runtime config CRUD actions must route through split configuration ports\n' >&2
    exit 1
fi

for connectivity_file in \
    crates/host/a4n-runtime/src/actions/config_records/llm/connectivity.rs \
    crates/host/a4n-runtime/src/actions/config_records/embedding/connectivity.rs; do
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
    crates/host/a4n-runtime/src/actions/config_records/embedding/connectivity.rs; then
    printf 'Embedding connectivity must consume the provider runtime queue boundary\n' >&2
    exit 1
fi

if ! rg -n 'provider_runtime\(\)' \
    crates/host/a4n-runtime/src/actions/config_records/embedding/connectivity.rs; then
    printf 'Embedding connectivity must route queue tracking through ProviderRuntime\n' >&2
    exit 1
fi

if rg -U -n 'state\s*\.\s*orchestrator\(\)\s*\.\s*queue_cancel' \
    crates/host/a4n-runtime/src/actions/admin_records/tasks.rs; then
    printf 'Runtime admin task cancellation must consume TaskCancellationPort\n' >&2
    exit 1
fi

if ! rg -n 'task_cancellation_application\(\)' \
    crates/host/a4n-runtime/src/actions/admin_records/tasks.rs; then
    printf 'Runtime admin task cancellation must route through the injected task cancellation port\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator::ServiceHealthApplicationPort|state\.orchestrator\(\)|OrchestratorContext' \
    crates/host/a4n-runtime/src/actions/admin_records/tasks.rs; then
    printf 'Runtime service health must consume ServiceHealthApplicationPort from the API contract\n' >&2
    exit 1
fi

if ! rg -n 'service_health_application\(\)' \
    crates/host/a4n-runtime/src/actions/admin_records/tasks.rs \
    crates/host/a4n-runtime/src/state/app.rs; then
    printf 'Runtime service health must route through the injected service health port\n' >&2
    exit 1
fi

if rg -n 'pub trait ServiceHealthApplicationPort|pub use .*ServiceHealthApplicationPort' \
    crates/host/a4n-orchestrator/src; then
    printf 'ServiceHealthApplicationPort must not be owned or re-exported by the orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ServiceHealthApplicationPort' \
    crates/host/a4n-api-contract/src/service_health.rs; then
    printf 'ServiceHealthApplicationPort contract is missing from the API contract\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\)|OrchestratorContext|llm_provider_config_from_model' \
    crates/host/a4n-runtime/src/actions/request_preview/chat/budget.rs; then
    printf 'Runtime chat request preview must consume ProviderRuntime configuration DTOs\n' >&2
    exit 1
fi

if ! rg -n 'provider_runtime\(\)' \
    crates/host/a4n-runtime/src/actions/request_preview/chat/budget.rs; then
    printf 'Runtime chat request preview must route LLM configuration through ProviderRuntime\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\)|OrchestratorContext' \
    crates/host/a4n-runtime/src/actions/source_sync; then
    printf 'Runtime source-sync actions must consume source-sync ports from the API contract\n' >&2
    exit 1
fi

if ! rg -n 'source_sync_(read|mutation)_application\(\)' \
    crates/host/a4n-runtime/src/actions/source_sync \
    crates/host/a4n-runtime/src/state/app.rs; then
    printf 'Runtime source-sync actions must route through the injected source-sync port\n' >&2
    exit 1
fi

if rg -n 'pub trait SourceSync(Application|Read|Mutation)ApplicationPort|pub use .*SourceSync(Application|Read|Mutation)ApplicationPort' \
    crates/host/a4n-orchestrator/src; then
    printf 'source-sync application ports must not be owned or re-exported by the orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'pub trait SourceSync(Read|Mutation)ApplicationPort' \
    crates/host/a4n-api-contract/src/source_sync.rs; then
    printf 'source-sync read/mutation contracts are missing from the API contract\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\)|OrchestratorContext' \
    crates/host/a4n-runtime/src/actions/config_records/image/connectivity.rs; then
    printf 'Image connectivity must consume the provider runtime queue boundary\n' >&2
    exit 1
fi

if ! rg -n 'provider_runtime\(\)' \
    crates/host/a4n-runtime/src/actions/config_records/image/connectivity.rs; then
    printf 'Image connectivity must route queue tracking through ProviderRuntime\n' >&2
    exit 1
fi

if ! rg -U -n \
    '(?s)pub struct ImageGenerationProviderInput\s*\{.*?pub\s+config:\s*(a4n_provider_contract::)?ImageProviderConfig\b' \
    crates/image/a4n-image-host-contract/src/lib.rs; then
    printf 'ImageGenerationProviderInput.config must use ImageProviderConfig instead of the image ORM model\n' >&2
    exit 1
fi

image_provider_sidecar_files=(
    crates/host/a4n-runtime/src/sidecar_clients/image/contracts.rs
    crates/host/a4n-runtime/src/sidecar_clients/image/operations/routing.rs
    crates/host/a4n-runtime/src/sidecar_clients/image/operations/sidecar.rs
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
    crates/host/a4n-runtime/src/actions/config_records/llm/connectivity.rs \
    crates/host/a4n-runtime/src/actions/config_records/embedding/connectivity.rs; then
    printf 'LLM/Embedding connectivity actions must not duplicate concrete config lookups\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\)|OrchestratorContext' \
    crates/host/a4n-runtime/src/actions/embedding_model/download.rs; then
    printf 'Embedding model download must consume the provider runtime queue boundary\n' >&2
    exit 1
fi

if ! rg -n 'provider_runtime\(\)' \
    crates/host/a4n-runtime/src/actions/embedding_model/download.rs; then
    printf 'Embedding model download must route queue tracking through ProviderRuntime\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\)|OrchestratorContext' \
    crates/host/a4n-runtime/src/actions/prompt_records.rs; then
    printf 'Runtime prompt actions must consume prompt read/mutation ports instead of the concrete orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'pub trait Prompt(Read|Mutation)ApplicationPort' crates/host/a4n-api-contract/src/prompt.rs; then
    printf 'Prompt read/mutation contracts are missing\n' >&2
    exit 1
fi

if ! rg -n 'prompt_read_application\(\)' crates/host/a4n-runtime/src/actions/prompt_records.rs; then
    printf 'Runtime prompt reads must route through PromptReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'prompt_mutation_application\(\)' crates/host/a4n-runtime/src/actions/prompt_records.rs; then
    printf 'Runtime prompt mutations must route through PromptMutationApplicationPort\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator::prompt_records::PromptExecutionContext|pub struct PromptExecutionContext' \
    crates/host/a4n-runtime/src crates/host/a4n-orchestrator/src; then
    printf 'PromptExecutionContext must not be owned or re-exported by the orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'pub struct PromptExecutionContext' crates/host/a4n-api-contract/src/prompt.rs || \
   ! rg -n 'PromptExecutionContext' \
    crates/host/a4n-runtime/src/actions/prompt_records.rs \
    crates/host/a4n-orchestrator/src/prompt_records.rs; then
    printf 'Prompt consumers must use PromptExecutionContext from the API contract\n' >&2
    exit 1
fi

if rg -n '\.orchestrator\(\)|OrchestratorContext' \
    crates/host/a4n-runtime/src/actions/galgame_records/mutations.rs \
    crates/host/a4n-runtime/src/actions/galgame_records/media/binding.rs \
    crates/host/a4n-runtime/src/actions/galgame_records/media/audio.rs; then
    printf 'Runtime Galgame record actions must consume Galgame mutation ports\n' >&2
    exit 1
fi

if ! rg -n 'galgame_record_mutation_application\(\)' \
    crates/host/a4n-runtime/src/actions/galgame_records/mutations.rs || \
   ! rg -n 'galgame_media_mutation_application\(\)' \
    crates/host/a4n-runtime/src/actions/galgame_records/media/binding.rs \
    crates/host/a4n-runtime/src/actions/galgame_records/media/audio.rs; then
    printf 'Runtime Galgame record actions must route through the Galgame mutation ports\n' >&2
    exit 1
fi

if rg -n 'pub trait Galgame(Record|Media)MutationApplicationPort|pub use .*Galgame(Record|Media)MutationApplicationPort' \
    crates/host/a4n-orchestrator/src/galgame; then
    printf 'Galgame mutation ports must not be owned or re-exported by the orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'pub trait Galgame(Record|Media)MutationApplicationPort' \
    crates/host/a4n-api-contract/src/galgame.rs; then
    printf 'Galgame record/media mutation contracts are missing from the API contract\n' >&2
    exit 1
fi

if rg -n '\.orchestrator\(\)|OrchestratorContext' \
    crates/host/a4n-runtime/src/actions/galgame_records/webgal_export.rs; then
    printf 'Runtime WebGAL export must consume GalgameExportApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'galgame_export_application\(\)' \
    crates/host/a4n-runtime/src/actions/galgame_records/webgal_export.rs; then
    printf 'Runtime WebGAL export must route through GalgameExportApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait Package(Read|Mutation)ApplicationPort' crates/host/a4n-api-contract/src/package.rs; then
    printf 'Package read/mutation contracts are missing\n' >&2
    exit 1
fi

if rg -n 'PackageApplicationPort|package_application\(\)' \
    crates/host/a4n-api-contract/src/package.rs crates/host/a4n-orchestrator/src crates/host/a4n-runtime/src; then
    printf 'The mixed PackageApplicationPort must remain removed\n' >&2
    exit 1
fi

if ! rg -U -n 'package_read_application\(\)\s*\.\s*execution_enabled\(package_id\)' \
    crates/host/a4n-runtime/src/actions/package.rs; then
    printf 'Runtime package execution admission must consume PackageReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -U -n 'package_mutation_application\(\)\s*\.\s*ensure_execution_enabled\(package_id\)' \
    crates/host/a4n-runtime/src/actions/package.rs; then
    printf 'Runtime package execution guard must consume PackageMutationApplicationPort\n' >&2
    exit 1
fi

if ! rg -U -n 'package_mutation_application\(\)\s*\.\s*start_lifecycle\(' crates/host/a4n-runtime/src/actions/package.rs; then
    printf 'Runtime package lifecycle start must consume PackageMutationApplicationPort\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\)\.start_package_lifecycle' \
    crates/host/a4n-runtime/src/actions/package.rs; then
    printf 'Runtime package lifecycle start must not call the concrete orchestrator\n' >&2
    exit 1
fi

if ! rg -n 'package_read_application\(\)\.owned_services\(package_id\)' \
    crates/host/a4n-runtime/src/actions/package.rs; then
    printf 'Runtime package service startup must consume PackageReadApplicationPort\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\)\.package_service_names' \
    crates/host/a4n-runtime/src/actions/package.rs; then
    printf 'Runtime package service startup must not call the concrete orchestrator\n' >&2
    exit 1
fi

if ! rg -U -n 'package_read_application\(\)\s*\.\s*has_workload\(package_id\)' \
    crates/host/a4n-runtime/src/actions/package.rs; then
    printf 'Runtime package lifecycle workload checks must consume PackageReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -U -n 'package_read_application\(\)\s*\.\s*services_safe_to_stop\(package_id\)' \
    crates/host/a4n-runtime/src/actions/package.rs; then
    printf 'Runtime package service shutdown checks must consume PackageReadApplicationPort\n' >&2
    exit 1
fi

if rg -U -n 'state\s*\.\s*orchestrator\(\)\s*\.\s*package_services_safe_to_stop' \
    crates/host/a4n-runtime/src/actions/package.rs; then
    printf 'Runtime package service shutdown checks must not call the concrete orchestrator\n' >&2
    exit 1
fi

if ! rg -U -n 'package_mutation_application\(\)\s*\.\s*finalize_disable\(operation_id\)' \
    crates/host/a4n-runtime/src/actions/package.rs; then
    printf 'Runtime package disable finalization must consume PackageMutationApplicationPort\n' >&2
    exit 1
fi

if rg -U -n 'state\s*\.\s*orchestrator\(\)\s*\.\s*finalize_package_disable' \
    crates/host/a4n-runtime/src/actions/package.rs; then
    printf 'Runtime package disable finalization must not call the concrete orchestrator\n' >&2
    exit 1
fi

if ! rg -U -n 'package_read_application\(\)\s*\.\s*pending_lifecycle_operations\(\)' \
    crates/host/a4n-runtime/src/actions/package.rs; then
    printf 'Runtime package lifecycle recovery must consume PackageReadApplicationPort\n' >&2
    exit 1
fi

if rg -U -n 'state\s*\.\s*orchestrator\(\)\s*\.\s*pending_package_lifecycle_operations' \
    crates/host/a4n-runtime/src/actions/package.rs; then
    printf 'Runtime package lifecycle recovery must not call the concrete orchestrator\n' >&2
    exit 1
fi

if ! rg -n 'package_read_application\(\)\.enabled_services\(\)' \
    crates/host/a4n-runtime/src/state/app/bootstrap.rs; then
    printf 'Runtime package startup service discovery must consume PackageReadApplicationPort\n' >&2
    exit 1
fi

if rg -U -n 'state\s*\.\s*orchestrator\(\)\s*\.\s*enabled_package_service_names' \
    crates/host/a4n-runtime/src/state/app.rs \
    crates/host/a4n-runtime/src/state/app; then
    printf 'Runtime package startup service discovery must not call the concrete orchestrator\n' >&2
    exit 1
fi

if ! rg -U -n 'package_mutation_application\(\)\s*\.\s*advance_lifecycle\(operation_id\)' \
    crates/host/a4n-runtime/src/actions/package.rs; then
    printf 'Runtime package lifecycle advancement must consume PackageMutationApplicationPort\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\)\.package_workload_snapshot' \
    crates/host/a4n-runtime/src/actions/package.rs; then
    printf 'Runtime package lifecycle workload checks must not call the concrete orchestrator\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\)\.(package_execution_enabled|ensure_package_execution_enabled)' \
    crates/host/a4n-runtime/src/actions/package.rs; then
    printf 'Runtime package execution admission must not call the concrete orchestrator\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator|OrchestratorContext|state\.orchestrator\(\)' \
    crates/host/a4n-runtime/src/actions/auth_records/users.rs \
    crates/host/a4n-runtime/src/actions/auth_records/sessions.rs \
    crates/host/a4n-runtime/src/actions/auth_records/tokens.rs \
    crates/host/a4n-runtime/src/actions/oauth/flow.rs \
    crates/host/a4n-runtime/src/actions/theme; then
    printf 'Runtime auth/theme actions must consume auth read/mutation ports and Theme read/mutation ports\n' >&2
    exit 1
fi

if ! rg -n 'pub trait Auth(Read|Mutation)ApplicationPort' crates/host/a4n-api-contract/src/auth.rs; then
    printf 'Auth read/mutation contracts are missing\n' >&2
    exit 1
fi

if rg -n 'AuthApplicationPort|auth_application\(\)' \
    crates/host/a4n-api-contract/src/auth.rs crates/host/a4n-orchestrator/src crates/host/a4n-runtime/src; then
    printf 'The mixed AuthApplicationPort must remain removed\n' >&2
    exit 1
fi

if ! rg -n 'auth_read_application\(\)' \
    crates/host/a4n-runtime/src/actions/auth_records/users.rs \
    crates/host/a4n-runtime/src/actions/auth_records/sessions.rs \
    crates/host/a4n-runtime/src/actions/auth_records/tokens.rs; then
    printf 'Runtime auth reads must route through AuthReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'auth_mutation_application\(\)' \
    crates/host/a4n-runtime/src/actions/auth_records/users.rs \
    crates/host/a4n-runtime/src/actions/auth_records/sessions.rs \
    crates/host/a4n-runtime/src/actions/auth_records/tokens.rs \
    crates/host/a4n-runtime/src/actions/oauth/flow.rs; then
    printf 'Runtime auth mutations must route through AuthMutationApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait Theme(Read|Mutation)ApplicationPort' crates/host/a4n-api-contract/src/theme.rs; then
    printf 'Theme read/mutation contracts are missing\n' >&2
    exit 1
fi

if rg -n 'ThemeApplicationPort|theme_application\(\)' \
    crates/host/a4n-api-contract/src/theme.rs crates/host/a4n-orchestrator/src crates/host/a4n-runtime/src; then
    printf 'The mixed ThemeApplicationPort must remain removed\n' >&2
    exit 1
fi

if ! rg -n 'theme_read_application\(\)' \
    crates/host/a4n-runtime/src/actions/auth_records/tokens.rs \
    crates/host/a4n-runtime/src/actions/theme; then
    printf 'Runtime theme reads must route through ThemeReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'theme_mutation_application\(\)' \
    crates/host/a4n-runtime/src/actions/theme; then
    printf 'Runtime theme mutations must route through ThemeMutationApplicationPort\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator|OrchestratorContext|state\.orchestrator\(\)' \
    crates/host/a4n-runtime/src/actions/admin_records/permissions.rs \
    crates/host/a4n-runtime/src/actions/admin_records/projects.rs \
    crates/host/a4n-runtime/src/actions/admin_records/usage.rs \
    crates/host/a4n-runtime/src/actions/admin_records/users.rs \
    crates/host/a4n-runtime/src/actions/generation_analytics.rs; then
    printf 'Runtime administrative data actions must consume admin read/mutation ports instead of the concrete orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'pub trait Admin(Read|Mutation)ApplicationPort' crates/host/a4n-api-contract/src/admin.rs; then
    printf 'Admin read/mutation contracts are missing\n' >&2
    exit 1
fi

if rg -n 'AdminApplicationPort|admin_application\(\)' \
    crates/host/a4n-api-contract/src/admin.rs crates/host/a4n-orchestrator/src crates/host/a4n-runtime/src; then
    printf 'The mixed AdminApplicationPort must remain removed\n' >&2
    exit 1
fi

if ! rg -n 'admin_read_application\(\)' \
    crates/host/a4n-runtime/src/actions/admin_records \
    crates/host/a4n-runtime/src/actions/generation_analytics.rs; then
    printf 'Runtime admin reads must route through AdminReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'admin_mutation_application\(\)' \
    crates/host/a4n-runtime/src/actions/admin_records/users.rs; then
    printf 'Runtime admin mutations must route through AdminMutationApplicationPort\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator|OrchestratorContext|state\.orchestrator\(\)' \
    crates/host/a4n-runtime/src/actions/project_status.rs; then
    printf 'Runtime project status actions must consume ProjectStatusApplicationPort instead of persisted project models\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ProjectStatus(Read|Mutation)ApplicationPort' crates/host/a4n-api-contract/src/project.rs; then
    printf 'ProjectStatus read/mutation contracts are missing\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator|OrchestratorContext|state\.orchestrator\(\)' \
    crates/host/a4n-runtime/src/actions/project_collaboration.rs; then
    printf 'Runtime project collaboration actions must consume ProjectCollaborationApplicationPort instead of the concrete orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ProjectCollaborationApplicationPort' crates/host/a4n-api-contract/src/collaboration.rs; then
    printf 'ProjectCollaborationApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n '\.(verify_project_permission|verify_project_permissions|collaboration_root_project)\(|\.project_ids_with_permission\(\s*&' \
    crates/host/a4n-runtime/src --glob '*.rs'; then
    printf 'Runtime must consume ProjectAccessApplicationPort for project permission and root resolution\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ProjectAccessApplicationPort' crates/host/a4n-api-contract/src/collaboration.rs; then
    printf 'ProjectAccessApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'novel_project::Model|AccessibleProject(List|Record)|ProjectAccessInfo' \
    crates/host/a4n-runtime/src/actions/project_records/api/queries.rs; then
    printf 'Runtime project list/summary actions must not assemble persisted project projections\n' >&2
    exit 1
fi

if ! rg -n 'project_read_application\(\)' \
    crates/host/a4n-runtime/src/actions/project_records/api/queries.rs; then
    printf 'Runtime project list/summary actions must consume ProjectReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ProjectReadApplicationPort' crates/host/a4n-api-contract/src/project.rs; then
    printf 'ProjectReadApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'ProjectReadApplicationPort for OrchestratorContext' crates/host/a4n-orchestrator/src; then
    printf 'ProjectReadApplicationPort must be owned by the project read worker\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator|OrchestratorContext|state\.orchestrator\(\)' \
    crates/host/a4n-runtime/src/actions/chapter_scene/records.rs; then
    printf 'Runtime chapter scene record actions must consume split chapter-scene ports\n' >&2
    exit 1
fi

if rg -n '(^|[^A-Za-z])ChapterSceneApplicationPort($|[^A-Za-z])|(^|[^A-Za-z_])chapter_scene_application\(\)' \
    crates/host/a4n-api-contract/src/chapter.rs \
    crates/host/a4n-orchestrator/src \
    crates/host/a4n-runtime/src; then
    printf 'The mixed ChapterSceneApplicationPort must remain removed\n' >&2
    exit 1
fi

if ! rg -n 'chapter_scene_read_application\(\)' \
    crates/host/a4n-runtime/src/actions/chapter_scene/records.rs || \
   ! rg -n 'chapter_scene_mutation_application\(\)' \
    crates/host/a4n-runtime/src/actions/chapter_scene/records.rs; then
    printf 'Runtime chapter scene records must route through split read and mutation ports\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ChapterScene(Read|Mutation)ApplicationPort' \
    crates/host/a4n-api-contract/src/chapter.rs; then
    printf 'Chapter-scene split contracts are missing\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator::chapter_scene::ChapterSceneProgressReporter|pub type ChapterSceneProgressReporter' \
    crates/host/a4n-runtime/src crates/host/a4n-server/src crates/host/a4n-tauri/src \
    crates/host/a4n-orchestrator/src/chapter_scene; then
    printf 'Chapter-scene progress callbacks must be consumed from a4n-api-contract\n' >&2
    exit 1
fi

if ! rg -n 'pub type ChapterSceneProgressReporter' crates/host/a4n-api-contract/src/chapter.rs; then
    printf 'Chapter-scene progress callback contract is missing from a4n-api-contract\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator|OrchestratorContext|state\.orchestrator\(\)' \
    crates/host/a4n-runtime/src/actions/chapter_content/queries.rs; then
    printf 'Runtime chapter content read actions must consume ChapterContentReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'chapter_content_read_application\(\)' \
    crates/host/a4n-runtime/src/actions/chapter_content/queries.rs \
    crates/host/a4n-runtime/src/workflow/chapter_analysis/api; then
    printf 'Runtime chapter content summaries must consume ChapterContentReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ChapterContentReadApplicationPort' crates/host/a4n-api-contract/src/chapter.rs; then
    printf 'ChapterContentReadApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator|OrchestratorContext|state\.orchestrator\(\)' \
    crates/host/a4n-runtime/src/actions/chapter_content/edit.rs \
    crates/host/a4n-runtime/src/actions/chapter_content/import.rs; then
    printf 'Runtime chapter content mutations must consume ChapterContentMutationApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'chapter_content_mutation_application\(\)' \
    crates/host/a4n-runtime/src/actions/chapter_content/edit.rs \
    crates/host/a4n-runtime/src/actions/chapter_content/import.rs; then
    printf 'Runtime chapter content mutations must route through the mutation port\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ChapterContentMutationApplicationPort' \
    crates/host/a4n-api-contract/src/chapter.rs; then
    printf 'ChapterContentMutationApplicationPort contract is missing from the API contract\n' >&2
    exit 1
fi
if rg -n 'pub trait ChapterContentMutationApplicationPort' \
    crates/host/a4n-orchestrator/src/chapter_content; then
    printf 'ChapterContentMutationApplicationPort must not be redefined in the orchestrator\n' >&2
    exit 1
fi
if rg -n 'a4n_orchestrator::chapter_content::ChapterContentMutationApplicationPort' \
    crates/host/a4n-runtime/src; then
    printf 'Runtime chapter content mutations must consume the API contract directly\n' >&2
    exit 1
fi

if ! rg -n 'chapter_content_sync_application\(\)' \
    crates/host/a4n-runtime/src/actions/chapter_content/edit.rs \
    crates/host/a4n-runtime/src/actions/optimize/patches/host_port.rs \
    crates/host/a4n-runtime/src/workflow/chapter_cleanup.rs; then
    printf 'Runtime chapter content synchronization must route through ChapterContentSyncApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ChapterContentSyncApplicationPort' \
    crates/host/a4n-api-contract/src/chapter.rs; then
    printf 'ChapterContentSyncApplicationPort contract is missing from the API contract\n' >&2
    exit 1
fi
if rg -n 'pub trait ChapterContentSyncApplicationPort' \
    crates/host/a4n-orchestrator/src/chapter_content; then
    printf 'ChapterContentSyncApplicationPort must not be redefined in the orchestrator\n' >&2
    exit 1
fi
if rg -n 'a4n_orchestrator::chapter_content::ChapterContentSyncApplicationPort' \
    crates/host/a4n-runtime/src; then
    printf 'Runtime chapter content synchronization must consume the API contract directly\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ChapterDerivedCleanupHostPort' \
    crates/host/a4n-api-contract/src/chapter.rs; then
    printf 'ChapterDerivedCleanupHostPort contract is missing from the API contract\n' >&2
    exit 1
fi
if rg -n 'pub trait ChapterDerivedCleanupHostPort' \
    crates/host/a4n-orchestrator/src/chapter_content; then
    printf 'ChapterDerivedCleanupHostPort must not be redefined in the orchestrator\n' >&2
    exit 1
fi
if rg -n 'a4n_orchestrator::chapter_content::ChapterDerivedCleanupHostPort' \
    crates/host/a4n-runtime/src; then
    printf 'Runtime chapter cleanup adapters must consume the API contract directly\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator|OrchestratorContext|state\.orchestrator\(\)' \
    crates/host/a4n-runtime/src/actions/outline_records/chapters/records.rs \
    crates/host/a4n-runtime/src/actions/outline_records/parts/records.rs; then
    printf 'Runtime outline read actions must consume OutlineReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'outline_read_application\(\)' \
    crates/host/a4n-runtime/src/actions/outline_records/chapters/records.rs \
    crates/host/a4n-runtime/src/actions/outline_records/parts/records.rs; then
    printf 'Runtime outline read actions must route through OutlineReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait OutlineReadApplicationPort' crates/host/a4n-api-contract/src/outline.rs; then
    printf 'OutlineReadApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator|OrchestratorContext|state\.orchestrator\(\)' \
    crates/host/a4n-runtime/src/actions/outline_records/chapters/records.rs \
    crates/host/a4n-runtime/src/actions/outline_records/parts/records.rs; then
    printf 'Runtime outline mutation actions must consume OutlineMutationApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'outline_mutation_application\(\)' \
    crates/host/a4n-runtime/src/actions/outline_records/chapters/records.rs \
    crates/host/a4n-runtime/src/actions/outline_records/parts/records.rs; then
    printf 'Runtime outline mutation actions must route through OutlineMutationApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait OutlineMutationApplicationPort' crates/host/a4n-api-contract/src/outline.rs; then
    printf 'OutlineMutationApplicationPort contract is missing\n' >&2
    exit 1
fi
