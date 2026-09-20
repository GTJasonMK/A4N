#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"
cd "$PROJECT_ROOT"

if ! rg -q "pub trait OptimizeChatGenerationApplicationPort" crates/a4n-optimize-host-contract/src/lib.rs; then
    printf 'Optimize host contracts are missing\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator::optimize::(Optimize|GenerationTaskAccessError)' crates/a4n-runtime/src; then
    printf 'Runtime optimize actions must use a4n-optimize-host-contract\n' >&2
    exit 1
fi

if ! rg -q "pub trait PortraitGenerationApplicationPort" crates/a4n-portrait-host-contract/src/lib.rs; then
    printf 'Portrait host contracts are missing\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator::portrait::(Portrait|GenerationTaskAccessError)' crates/a4n-runtime/src; then
    printf 'Runtime portrait actions must use a4n-portrait-host-contract\n' >&2
    exit 1
fi

if ! rg -q "pub trait ChapterAnalysisGenerationApplicationPort" crates/a4n-chapter-analysis-host-contract/src/lib.rs; then
    printf 'Chapter analysis host contracts are missing\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator::(chapter_analysis|import_analysis)::(ChapterAnalysis|ImportAnalysis)' crates/a4n-runtime/src; then
    printf 'Runtime analysis actions must use dedicated host contracts\n' >&2
    exit 1
fi
if ! rg -n 'character_discovery_application\(\)' \
    crates/a4n-runtime/src/actions/character_ai/character_discovery_flow/request.rs \
    crates/a4n-runtime/src/actions/character_ai/character_discovery_flow/execution.rs || \
   ! rg -n 'character_profile_sync_application\(\)' \
    crates/a4n-runtime/src/actions/character_ai/sync_profile/preview.rs \
    crates/a4n-runtime/src/actions/character_ai/sync_profile/execution.rs \
    crates/a4n-runtime/src/actions/character_ai/sync_batch.rs; then
    printf 'Character workers must route through worker-specific ports\n' >&2
    exit 1
fi
if rg -n 'CharacterAiApplicationPort|character_ai_application\(\)' \
    crates/a4n-orchestrator/src/character.rs \
    crates/a4n-runtime/src/actions/character_ai \
    crates/a4n-runtime/src/state/app.rs; then
    printf 'The mixed CharacterAiApplicationPort must remain removed\n' >&2
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
   ! rg -n 'portrait_prompt_request_preview_application\(\)' \
    crates/a4n-runtime/src/actions/portrait_generation/preview/requests.rs || \
   ! rg -n 'portrait_image_request_preview_application\(\)' \
    crates/a4n-runtime/src/actions/portrait_generation/preview/requests.rs; then
    printf 'Portrait actions must route through split application ports\n' >&2
    exit 1
fi

if ! rg -n 'pub trait Portrait(PromptRequest|ImageRequest|Prompt|Upload|Generation)ApplicationPort' \
    crates/a4n-portrait-host-contract/src/lib.rs; then
    printf 'Portrait application ports are missing from the portrait owner\n' >&2
    exit 1
fi
if ! rg -n 'pub trait ProjectMutationApplicationPort' crates/a4n-api-contract/src/project.rs; then
    printf 'ProjectMutationApplicationPort contract is missing\n' >&2
    exit 1
fi

if ! rg -n 'chapter_generation_request_preview_application\(\)' \
    crates/a4n-runtime/src/actions/chapter_generation.rs || \
   ! rg -n 'chapter_generation_rag_preparation_application\(\)' \
    crates/a4n-runtime/src/actions/chapter_generation.rs; then
    printf 'Runtime chapter previews must route through worker-specific ports\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ChapterGenerationRequestPreviewApplicationPort' \
    crates/a4n-chapter-generation-contract/src/lib.rs || \
   ! rg -n 'pub trait ChapterGenerationRagPreparationApplicationPort' \
    crates/a4n-chapter-generation-contract/src/lib.rs; then
    printf 'Chapter-generation worker-specific preview contracts are missing\n' >&2
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
if ! rg -n 'import_reverse_preview_application\(\)' \
    crates/a4n-runtime/src/actions/import_reverse/request.rs || \
   ! rg -n 'import_reverse_generation_application\(\)' \
    crates/a4n-runtime/src/actions/import_reverse/api.rs || \
   ! rg -n 'import_reverse_confirmation_application\(\)' \
    crates/a4n-runtime/src/actions/import_reverse/api.rs; then
    printf 'Runtime import reverse actions must route through split application ports\n' >&2
    exit 1
fi
if ! rg -n 'pub trait ImportReversePreviewApplicationPort' \
    crates/a4n-novel-host-contract/src/lib.rs || \
   ! rg -n 'pub trait ImportReverseGenerationApplicationPort' \
    crates/a4n-novel-host-contract/src/lib.rs || \
   ! rg -n 'pub trait ImportReverseConfirmationApplicationPort' \
    crates/a4n-api-contract/src/import_reverse.rs; then
    printf 'Import-reverse application ports are missing\n' >&2
    exit 1
fi
if rg -n 'ImportReverseApplicationPort|import_reverse_application\(\)' \
    crates/a4n-api-contract/src/import_reverse.rs \
    crates/a4n-orchestrator/src/import_reverse.rs \
    crates/a4n-orchestrator/src/import_reverse \
    crates/a4n-runtime/src; then
    printf 'The mixed ImportReverseApplicationPort must remain removed\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator|OrchestratorContext|state\.orchestrator\(\)' \
    crates/a4n-runtime/src/actions/import_analysis \
    crates/a4n-runtime/src/actions/import_file.rs \
    crates/a4n-runtime/src/actions/import_review; then
    printf 'Runtime import actions must consume ImportAnalysisApplicationPort and ImportReviewApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'import_analysis_generation_application\(\)' \
    crates/a4n-runtime/src/actions/import_analysis/api.rs || \
   ! rg -n 'import_analysis_cancellation_application\(\)' \
    crates/a4n-runtime/src/actions/import_analysis/api.rs || \
   ! rg -n 'import_analysis_progress_application\(\)' \
    crates/a4n-runtime/src/actions/import_analysis/api.rs || \
   ! rg -n 'import_analysis_preview_application\(\)' \
    crates/a4n-runtime/src/actions/import_analysis/preview.rs; then
    printf 'Runtime import-analysis actions must route through split application ports\n' >&2
    exit 1
fi

if ! rg -n 'import_review_ingestion_application\(\)' \
    crates/a4n-runtime/src/actions/import_file.rs || \
   ! rg -n 'import_review_acceptance_application\(\)' \
    crates/a4n-runtime/src/actions/import_review/acceptance.rs || \
   ! rg -n 'import_review_mutation_application\(\)' \
    crates/a4n-runtime/src/actions/import_review/mutations.rs || \
   ! rg -n 'import_review_read_application\(\)' \
    crates/a4n-runtime/src/actions/import_review/queries.rs \
    crates/a4n-runtime/src/actions/import_review/dedup.rs; then
    printf 'Runtime import-review actions must route through the split application ports\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ImportAnalysisGenerationApplicationPort' \
    crates/a4n-import-analysis-host-contract/src/lib.rs || \
   ! rg -n 'pub trait ImportAnalysisPreviewApplicationPort' \
    crates/a4n-import-analysis-host-contract/src/lib.rs || \
   ! rg -n 'pub trait ImportAnalysisProgressApplicationPort' \
    crates/a4n-api-contract/src/import.rs; then
    printf 'Import-analysis application ports are missing\n' >&2
    exit 1
fi
if ! rg -n 'pub trait ImportAnalysisCancellationApplicationPort' \
    crates/a4n-api-contract/src/import.rs; then
    printf 'Import-analysis cancellation contract is missing\n' >&2
    exit 1
fi
if rg -n 'pub trait ImportAnalysisApplicationPort|import_analysis_application\(\)' \
    crates/a4n-orchestrator/src/import_analysis crates/a4n-api-contract/src/import.rs; then
    printf 'The mixed ImportAnalysisApplicationPort must remain removed\n' >&2
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

if ! rg -n 'pub trait ImportReviewIngestionApplicationPort' \
    crates/a4n-novel-host-contract/src/lib.rs; then
    printf 'ImportReviewIngestionApplicationPort contract is missing\n' >&2
    exit 1
fi
if ! rg -n 'pub trait ImportReviewAcceptanceApplicationPort' \
    crates/a4n-api-contract/src/import_review.rs || \
   ! rg -n 'pub trait ImportReviewMutationApplicationPort' \
    crates/a4n-api-contract/src/import_review.rs; then
    printf 'Import-review mutation ports are missing\n' >&2
    exit 1
fi
if rg -n 'ImportReviewApplicationPort|import_review_application\(\)' \
    crates/a4n-api-contract/src/import_review.rs \
    crates/a4n-orchestrator/src/import_review \
    crates/a4n-runtime/src; then
    printf 'The mixed ImportReviewApplicationPort must remain removed\n' >&2
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
   ! rg -n 'chapter_analysis_generation_preview_application\(\)' \
    crates/a4n-runtime/src/workflow/chapter_analysis/api/preview.rs || \
   ! rg -n 'chapter_analysis_resume_preview_application\(\)' \
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
    crates/a4n-chapter-analysis-host-contract/src/lib.rs || \
   ! rg -n 'pub trait ChapterAnalysisResumeApplicationPort' \
    crates/a4n-chapter-analysis-host-contract/src/lib.rs || \
   ! rg -n 'pub trait ChapterAnalysisGenerationPreviewApplicationPort' \
    crates/a4n-chapter-analysis-host-contract/src/lib.rs || \
   ! rg -n 'pub trait ChapterAnalysisResumePreviewApplicationPort' \
    crates/a4n-chapter-analysis-host-contract/src/lib.rs; then
    printf 'ChapterAnalysis split application ports are missing\n' >&2
    exit 1
fi
