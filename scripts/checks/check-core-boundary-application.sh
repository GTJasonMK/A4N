#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"
cd "$PROJECT_ROOT"

if ! rg -n 'blueprint_read_application\(\)' \
    crates/host/a4n-runtime/src/actions/blueprint_records/generation.rs \
    crates/host/a4n-runtime/src/actions/blueprint_records/versions.rs; then
    printf 'Runtime blueprint read actions must consume BlueprintReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait BlueprintReadApplicationPort' crates/host/a4n-api-contract/src/blueprint.rs; then
    printf 'BlueprintReadApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator|OrchestratorContext|state\.orchestrator\(\)' \
    crates/host/a4n-runtime/src/actions/galgame_records/queries.rs; then
    printf 'Runtime Galgame read actions must consume GalgameReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'galgame_read_application\(\)' \
    crates/host/a4n-runtime/src/actions/galgame_records/queries.rs; then
    printf 'Runtime Galgame read actions must route through GalgameReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait GalgameReadApplicationPort' crates/host/a4n-api-contract/src/galgame.rs; then
    printf 'GalgameReadApplicationPort contract is missing\n' >&2
    exit 1
fi

if ! rg -n 'galgame_asset_plan_application\(\)' \
    crates/host/a4n-runtime/src/actions/galgame_records/asset_generation/mod.rs; then
    printf 'Runtime Galgame asset planning must route through GalgameAssetPlanApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'galgame_asset_generation_application\(\)' \
    crates/host/a4n-runtime/src/actions/galgame_records/asset_generation/mod.rs; then
    printf 'Runtime Galgame asset execution must route through GalgameAssetGenerationApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait GalgameAssetGenerationApplicationPort' \
    crates/galgame/a4n-galgame-host-contract/src/lib.rs; then
    printf 'Galgame asset-generation host contract is missing\n' >&2
    exit 1
fi

if ! rg -n 'pub trait GalgameAssetPlanApplicationPort' \
    crates/host/a4n-api-contract/src/galgame.rs; then
    printf 'GalgameAssetPlanApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator|OrchestratorContext|state\.orchestrator\(\)' \
    crates/host/a4n-runtime/src/actions/inspiration_records/history.rs; then
    printf 'Runtime inspiration history action must consume InspirationHistoryReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'inspiration_history_read_application\(\)' \
    crates/host/a4n-runtime/src/actions/inspiration_records/history.rs; then
    printf 'Runtime inspiration history action must route through InspirationHistoryReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait InspirationHistoryReadApplicationPort' crates/host/a4n-api-contract/src/inspiration.rs; then
    printf 'InspirationHistoryReadApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -U -n 'state\s*\.\s*orchestrator\(\)|OrchestratorContext|start_inspiration_generation_task|run_inspiration_generation_task' \
    crates/host/a4n-runtime/src/actions/inspiration_records/conversations.rs; then
    printf 'Runtime inspiration generation must not call the concrete orchestrator\n' >&2
    exit 1
fi

if ! rg -n 'inspiration_generation_application\(\)' \
    crates/host/a4n-runtime/src/actions/inspiration_records/conversations.rs; then
    printf 'Runtime inspiration generation must route through InspirationGenerationApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait InspirationGenerationApplicationPort' \
    crates/inspiration/a4n-inspiration-host-contract/src/lib.rs; then
    printf 'InspirationGenerationApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator|OrchestratorContext|state\.orchestrator\(\)|verify_project_editor' \
    crates/host/a4n-runtime/src/actions/inspiration_records/request.rs; then
    printf 'Runtime inspiration request preview action must consume the inspiration preview port\n' >&2
    exit 1
fi

if ! rg -n 'inspiration_request_application\(\)' \
    crates/host/a4n-runtime/src/actions/inspiration_records/request.rs; then
    printf 'Runtime inspiration request preview action must route through the inspiration preview port\n' >&2
    exit 1
fi

if ! rg -n 'pub trait InspirationRequestPreviewApplicationPort' \
    crates/inspiration/a4n-inspiration-host-contract/src/lib.rs; then
    printf 'InspirationRequestPreviewApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'OrchestratorContext|state\.orchestrator\(\)' \
    crates/host/a4n-runtime/src/actions/video_generation/config_state.rs; then
    printf 'Runtime video provider config resolution must not depend on the full OrchestratorContext\n' >&2
    exit 1
fi

if ! rg -n 'video_provider_config\(\)' \
    crates/host/a4n-runtime/src/actions/video_generation/provider_host.rs; then
    printf 'Runtime video provider config resolution must use the injected narrow port\n' >&2
    exit 1
fi

if ! rg -n 'pub trait VideoProviderExecutionApplicationPort' \
    crates/host/a4n-api-contract/src/video.rs; then
    printf 'VideoProviderExecutionApplicationPort contract is missing\n' >&2
    exit 1
fi

if ! rg -n 'video_provider_execution\(\)' \
    crates/host/a4n-runtime/src/actions/video_generation/poll.rs; then
    printf 'Runtime video polling must route through the provider execution port\n' >&2
    exit 1
fi

if ! rg -n 'pub trait VideoProviderConfigPort' crates/host/a4n-api-contract/src/video.rs; then
    printf 'VideoProviderConfigPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'pub use a4n_provider_contract::VideoProviderConfig' crates/host/a4n-orchestrator/src/video; then
    printf 'VideoProviderConfig must not be re-exported by the orchestrator video facade\n' >&2
    exit 1
fi

if rg -n 'pub use a4n_video::.*validate_video_(config_params|max_concurrent)|pub use self::validation::.*validate_video_(config_params|max_concurrent)' \
    crates/host/a4n-orchestrator/src/video; then
    printf 'pure video config validators must be consumed from a4n-video, not re-exported by the orchestrator facade\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator::media::ImageProviderConfigPort|crate::media::ImageProviderConfigPort' \
    crates/host/a4n-runtime crates/host/a4n-orchestrator; then
    printf 'ImageProviderConfigPort must not be reintroduced through the orchestrator media facade\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ImageProviderConfigPort' crates/host/a4n-api-contract/src/image.rs; then
    printf 'ImageProviderConfigPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\).*history|TaskReadApplicationPort\s*=\s*state\.orchestrator\(\)' \
    crates/host/a4n-runtime/src/actions/task_records/queries/tasks.rs; then
    printf 'Runtime task history must consume TaskReadApplicationPort instead of the concrete orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'task_read_application\(\)' \
    crates/host/a4n-runtime/src/actions/task_records/queries/tasks.rs; then
    printf 'Runtime task history must route through TaskReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait TaskReadApplicationPort' crates/host/a4n-api-contract/src/task.rs; then
    printf 'TaskReadApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\).*queue_stats' \
    crates/host/a4n-runtime/src/actions/task_records/queries/tasks.rs; then
    printf 'Runtime queue statistics must consume TaskQueueStatsApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'task_queue_stats_application\(\)' \
    crates/host/a4n-runtime/src/actions/task_records/queries/tasks.rs; then
    printf 'Runtime queue statistics must route through TaskQueueStatsApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait TaskQueueStatsApplicationPort' crates/host/a4n-api-contract/src/task.rs; then
    printf 'TaskQueueStatsApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\)\.(get_video_timeline|commit_inserted_segment_draft|move_video_timeline_item|delete_video_timeline_item|load_video_segment_plan_draft_validated|update_video_segment_plan_draft_recipe_validated|update_chapter_video_segment_recipe_validated|update_chapter_video_prompt_validated|delete_chapter_video_prompt_validated|optimize_chapter_video_prompt_validated)' \
    crates/host/a4n-runtime/src/actions/video_prompts.rs; then
    printf 'Runtime video prompt actions must consume the split video prompt application ports\n' >&2
    exit 1
fi

if ! rg -n 'video_prompt_(timeline|draft|mutation)_application\(\)' \
    crates/host/a4n-runtime/src/actions/video_prompts.rs; then
    printf 'Runtime video prompt actions must route through the split application ports\n' >&2
    exit 1
fi

if rg -n 'pub trait VideoPromptMutationApplicationPort|pub use .*VideoPromptMutationApplicationPort' \
    crates/host/a4n-orchestrator/src/video; then
    printf 'video prompt application ports must not be owned or re-exported by the orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'pub trait VideoPromptMutationApplicationPort|pub trait VideoPromptTimelineApplicationPort|pub trait VideoPromptDraftApplicationPort' \
    crates/host/a4n-api-contract/src/video.rs; then
    printf 'video prompt application ports must live in the API contract\n' >&2
    exit 1
fi

if ! rg -n 'pub trait VideoPromptTimelineApplicationPort|pub trait VideoPromptDraftApplicationPort' \
    crates/host/a4n-api-contract/src/video.rs; then
    printf 'video prompt timeline/draft ports must live in the API contract\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator::video::VideoPrompt(Timeline|Draft)ApplicationPort|VideoSegmentPlanDraftState' \
    crates/host/a4n-runtime/src crates/host/a4n-orchestrator/src/video; then
    printf 'video prompt timeline/draft consumers must not use orchestrator-only ports or duplicate draft state\n' >&2
    exit 1
fi

if ! rg -n 'pub trait VideoPromptDownstreamCleanupPort' \
    crates/host/a4n-api-contract/src/video.rs; then
    printf 'VideoPromptDownstreamCleanupPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator::video::VideoPromptDownstreamCleanupPort' \
    crates/host/a4n-runtime/src crates/host/a4n-server/src crates/host/a4n-tauri/src; then
    printf 'Runtime/Host must consume VideoPromptDownstreamCleanupPort from the API contract\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator::video::(CompleteVideoRecordInput|VideoRecordCompletion|VideoRecordCompletionError|VideoTailFrameInput|VideoTailFrameSummary|AssemblyPushClipInput|AssemblyPostprocessPlan|AssemblyPostprocessSourcePlan|AssemblyPostprocessEditRecordInput)' \
    crates/host/a4n-runtime/src crates/host/a4n-server/src crates/host/a4n-tauri/src; then
    printf 'Runtime/Host video record and assembly DTOs must consume the API contract\n' >&2
    exit 1
fi

if rg -n 'pub (struct|enum) (CompleteVideoRecordInput|VideoRecordCompletion|VideoRecordCompletionError|VideoTailFrameInput|VideoTailFrameSummary|AssemblyPushClipInput|AssemblyPostprocessPlan|AssemblyPostprocessSourcePlan|AssemblyPostprocessEditRecordInput)' \
    crates/host/a4n-orchestrator/src/video; then
    printf 'Video record and assembly DTOs must have one API contract owner\n' >&2
    exit 1
fi

if ! rg -n 'pub (struct|enum) (CompleteVideoRecordInput|VideoRecordCompletion|VideoRecordCompletionError|VideoTailFrameInput|VideoTailFrameSummary|AssemblyPushClipInput|AssemblyPostprocessPlan|AssemblyPostprocessSourcePlan|AssemblyPostprocessEditRecordInput)' \
    crates/host/a4n-api-contract/src/video/assembly.rs; then
    printf 'Video record and assembly DTO contracts are missing\n' >&2
    exit 1
fi

if ! rg -n 'pub struct GenerateChapterVideoInput' \
    crates/host/a4n-api-contract/src/video.rs; then
    printf 'GenerateChapterVideoInput contract is missing\n' >&2
    exit 1
fi

if rg -n 'GenerateChapterVideoInput' crates/host/a4n-orchestrator/src/video/mod.rs \
    || rg -n 'a4n_orchestrator::video::GenerateChapterVideoInput' crates/host/a4n-runtime/src; then
    printf 'Chapter video input must be consumed from the API contract\n' >&2
    exit 1
fi

if ! rg -n 'pub struct GenerateReferenceMediaVideoInput' \
    crates/host/a4n-api-contract/src/video.rs; then
    printf 'GenerateReferenceMediaVideoInput contract is missing\n' >&2
    exit 1
fi

if rg -n 'GenerateReferenceMediaVideoInput' crates/host/a4n-orchestrator/src/video/mod.rs \
    || rg -n 'a4n_orchestrator::video::GenerateReferenceMediaVideoInput' crates/host/a4n-runtime/src; then
    printf 'Reference-media video input must be consumed from the API contract\n' >&2
    exit 1
fi

if ! rg -n 'pub struct GeneratePageImageInput' \
    crates/host/a4n-api-contract/src/image_generation.rs; then
    printf 'GeneratePageImageInput contract is missing\n' >&2
    exit 1
fi

if rg -n 'GeneratePageImageInput' crates/host/a4n-orchestrator/src/media.rs \
    crates/host/a4n-orchestrator/src/media/image/generation.rs \
    || rg -n 'a4n_orchestrator::media::GeneratePageImageInput' crates/host/a4n-runtime/src; then
    printf 'Page image input must be consumed from the API contract\n' >&2
    exit 1
fi

if ! rg -n 'pub struct GenerateBatchImagesInput' \
    crates/host/a4n-api-contract/src/image_generation.rs; then
    printf 'GenerateBatchImagesInput contract is missing\n' >&2
    exit 1
fi

if rg -n 'GenerateBatchImagesInput' crates/host/a4n-orchestrator/src/media.rs \
    crates/host/a4n-orchestrator/src/media/image/generation.rs \
    || rg -n 'a4n_orchestrator::media::GenerateBatchImagesInput' crates/host/a4n-runtime/src; then
    printf 'Batch image input must be consumed from the API contract\n' >&2
    exit 1
fi

if ! rg -n 'pub struct GenerateSceneImageInput' \
    crates/host/a4n-api-contract/src/image_generation.rs; then
    printf 'GenerateSceneImageInput contract is missing\n' >&2
    exit 1
fi

if rg -n 'GenerateSceneImageInput' crates/host/a4n-orchestrator/src/media.rs \
    crates/host/a4n-orchestrator/src/media/image/generation.rs \
    || rg -n 'a4n_orchestrator::media::GenerateSceneImageInput' crates/host/a4n-runtime/src; then
    printf 'Scene image input must be consumed from the API contract\n' >&2
    exit 1
fi

if ! rg -n 'pub struct GenerateGalgameImageInput' \
    crates/host/a4n-api-contract/src/image_generation.rs; then
    printf 'GenerateGalgameImageInput contract is missing\n' >&2
    exit 1
fi

if rg -n 'GenerateGalgameImageInput' crates/host/a4n-orchestrator/src/media.rs \
    crates/host/a4n-orchestrator/src/media/image/generation.rs \
    || rg -n 'a4n_orchestrator::media::GenerateGalgameImageInput' crates/host/a4n-runtime/src; then
    printf 'Galgame image input must be consumed from the API contract\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ImageProfilePromptApplicationPort' \
    crates/host/a4n-api-contract/src/image_generation.rs; then
    printf 'ImageProfilePromptApplicationPort contract is missing\n' >&2
    exit 1
fi
if ! rg -n 'image_profile_prompt_application\(\)' \
    crates/host/a4n-runtime/src/actions/image_generation/profiles.rs; then
    printf 'Profile image prompt actions must route through ImageProfilePromptApplicationPort\n' >&2
    exit 1
fi
if rg -U -n 'image_generation_application\(\)\s*\.\s*(object_profile_prompt|scene_profile_prompt)' \
    crates/host/a4n-runtime/src/actions/image_generation/profiles.rs; then
    printf 'Profile image prompt actions must not use the image generation task port\n' >&2
    exit 1
fi
if ! rg -n 'pub trait ImageGenerationTaskApplicationPort' \
    crates/image/a4n-image-host-contract/src/lib.rs || \
   ! rg -n 'image_generation_task_application\(\)' \
    crates/host/a4n-runtime/src/actions/image_generation/generation_task.rs; then
    printf 'Queued image generation must route through ImageGenerationTaskApplicationPort\n' >&2
    exit 1
fi
if rg -n 'image_generation_application\(\)' \
    crates/host/a4n-runtime/src/actions/image_generation/generation_task.rs; then
    printf 'Queued image generation must not use the request-preparation port\n' >&2
    exit 1
fi
if ! rg -n 'pub trait ImageSceneRequestPreviewApplicationPort' \
    crates/image/a4n-image-host-contract/src/lib.rs || \
   ! rg -n 'image_scene_request_preview_application\(\)' \
    crates/host/a4n-runtime/src/actions/image_generation/direct.rs; then
    printf 'Scene image request previews must route through ImageSceneRequestPreviewApplicationPort\n' >&2
    exit 1
fi
if rg -U -n 'image_generation_application\(\)\s*\.\s*prepare_scene_request' \
    crates/host/a4n-runtime/src/actions/image_generation/direct.rs; then
    printf 'Scene image request previews must not use the mixed image generation port\n' >&2
    exit 1
fi
if ! rg -n 'pub trait ImageCanvasRequestPreviewApplicationPort' \
    crates/image/a4n-image-host-contract/src/lib.rs || \
   ! rg -n 'image_canvas_request_preview_application\(\)' \
    crates/host/a4n-runtime/src/actions/image_generation/direct.rs; then
    printf 'Canvas image request previews must route through ImageCanvasRequestPreviewApplicationPort\n' >&2
    exit 1
fi
if rg -U -n 'image_generation_application\(\)\s*\.\s*prepare_canvas_request' \
    crates/host/a4n-runtime/src/actions/image_generation/direct.rs; then
    printf 'Canvas image request previews must not use the mixed image generation port\n' >&2
    exit 1
fi
if ! rg -n 'pub trait ImagePageRequestPreviewApplicationPort' \
    crates/image/a4n-image-host-contract/src/lib.rs || \
   ! rg -n 'image_page_request_preview_application\(\)' \
    crates/host/a4n-runtime/src/actions/image_generation/page.rs; then
    printf 'Page image request previews must route through ImagePageRequestPreviewApplicationPort\n' >&2
    exit 1
fi
if rg -U -n 'image_generation_application\(\)\s*\.\s*prepare_page_request' \
    crates/host/a4n-runtime/src/actions/image_generation/page.rs; then
    printf 'Page image request previews must not use the mixed image generation port\n' >&2
    exit 1
fi
if ! rg -n 'pub trait ImageBatchRequestPreviewApplicationPort' \
    crates/image/a4n-image-host-contract/src/lib.rs || \
   ! rg -n 'image_batch_request_preview_application\(\)' \
    crates/host/a4n-runtime/src/actions/image_generation/batch_api.rs; then
    printf 'Batch image request previews must route through ImageBatchRequestPreviewApplicationPort\n' >&2
    exit 1
fi
if rg -U -n 'image_generation_application\(\)\s*\.\s*prepare_batch_request' \
    crates/host/a4n-runtime/src/actions/image_generation/batch_api.rs; then
    printf 'Batch image request previews must not use the mixed image generation port\n' >&2
    exit 1
fi
if ! rg -n 'pub trait ImageObjectProfileRequestPreviewApplicationPort' \
    crates/image/a4n-image-host-contract/src/lib.rs || \
   ! rg -n 'image_object_profile_request_preview_application\(\)' \
    crates/host/a4n-runtime/src/actions/image_generation/profiles.rs; then
    printf 'Object profile image request previews must route through ImageObjectProfileRequestPreviewApplicationPort\n' >&2
    exit 1
fi
if rg -U -n 'image_generation_application\(\)\s*\.\s*prepare_object_profile_request' \
    crates/host/a4n-runtime/src/actions/image_generation/profiles.rs; then
    printf 'Object profile image request previews must not use the mixed image generation port\n' >&2
    exit 1
fi
if ! rg -n 'pub trait GalgameImageGenerationApplicationPort' \
    crates/image/a4n-image-host-contract/src/lib.rs || \
   ! rg -n 'galgame_image_generation_application\(\)' \
    crates/host/a4n-runtime/src/actions/image_generation/direct.rs; then
    printf 'Galgame image generation must route through GalgameImageGenerationApplicationPort\n' >&2
    exit 1
fi
if rg -n '^[[:space:]]*\.image_generation_application\(\)' \
    crates/host/a4n-runtime/src/actions/image_generation; then
    printf 'Image generation actions must not use the removed mixed image generation port\n' >&2
    exit 1
fi
if ! rg -n 'pub trait OutlinePartGenerationPreviewApplicationPort' \
    crates/planning/a4n-outline-contract/src/lib.rs || \
   ! rg -n 'outline_part_generation_preview_application\(\)' \
    crates/host/a4n-runtime/src/actions/outline_records/parts/generation.rs; then
    printf 'Part outline generation previews must route through OutlinePartGenerationPreviewApplicationPort\n' >&2
    exit 1
fi
if rg -U -n 'outline_preview_application\(\)\s*\.\s*preview_(generate|regenerate)_parts' \
    crates/host/a4n-runtime/src/actions/outline_records/parts/generation.rs; then
    printf 'Part outline generation previews must not use the mixed outline preview port\n' >&2
    exit 1
fi
if ! rg -n 'pub trait OutlinePartMutationPreviewApplicationPort' \
    crates/planning/a4n-outline-contract/src/lib.rs || \
   ! rg -n 'outline_part_mutation_preview_application\(\)' \
    crates/host/a4n-runtime/src/actions/outline_records/parts; then
    printf 'Part outline mutation previews must route through OutlinePartMutationPreviewApplicationPort\n' >&2
    exit 1
fi
if rg -U -n 'outline_preview_application\(\)\s*\.\s*preview_(regenerate_last_part|continue_part_outlines|regenerate_part_outline)' \
    crates/host/a4n-runtime/src/actions/outline_records/parts; then
    printf 'Part outline mutation previews must not use the mixed outline preview port\n' >&2
    exit 1
fi
if ! rg -n 'pub trait OutlinePartChapterGenerationPreviewApplicationPort' \
    crates/planning/a4n-outline-contract/src/lib.rs || \
   ! rg -n 'outline_part_chapter_generation_preview_application\(\)' \
    crates/host/a4n-runtime/src/actions/outline_records/chapters/generation.rs; then
    printf 'Part chapter generation previews must route through OutlinePartChapterGenerationPreviewApplicationPort\n' >&2
    exit 1
fi
if rg -U -n 'outline_preview_application\(\)\s*\.\s*preview_generate_part_chapter_outlines' \
    crates/host/a4n-runtime/src/actions/outline_records/chapters/generation.rs; then
    printf 'Part chapter generation previews must not use the mixed outline preview port\n' >&2
    exit 1
fi
if ! rg -n 'pub trait OutlineBatchPartGenerationPreviewApplicationPort' \
    crates/planning/a4n-outline-contract/src/lib.rs || \
   ! rg -n 'outline_batch_part_generation_preview_application\(\)' \
    crates/host/a4n-runtime/src/actions/outline_records/batch/generation.rs; then
    printf 'Batch part generation previews must route through OutlineBatchPartGenerationPreviewApplicationPort\n' >&2
    exit 1
fi
if rg -U -n 'outline_preview_application\(\)\s*\.\s*preview_batch_generate_part_outlines' \
    crates/host/a4n-runtime/src/actions/outline_records/batch/generation.rs; then
    printf 'Batch part generation previews must not use the mixed outline preview port\n' >&2
    exit 1
fi
if ! rg -n 'pub trait OutlineChapterCountGenerationPreviewApplicationPort' \
    crates/planning/a4n-outline-contract/src/lib.rs || \
   ! rg -n 'outline_chapter_count_generation_preview_application\(\)' \
    crates/host/a4n-runtime/src/actions/outline_records/batch/by_count.rs; then
    printf 'Chapter-count generation previews must route through OutlineChapterCountGenerationPreviewApplicationPort\n' >&2
    exit 1
fi
if rg -U -n 'outline_preview_application\(\)\s*\.\s*preview_generate_chapter_outlines_by_count' \
    crates/host/a4n-runtime/src/actions/outline_records/batch/by_count.rs; then
    printf 'Chapter-count generation previews must not use the mixed outline preview port\n' >&2
    exit 1
fi
if ! rg -n 'pub trait OutlineChapterRegenerationPreviewApplicationPort' \
    crates/planning/a4n-outline-contract/src/lib.rs || \
   ! rg -n 'outline_chapter_regeneration_preview_application\(\)' \
    crates/host/a4n-runtime/src/actions/outline_records/chapters/regeneration.rs; then
    printf 'Chapter regeneration previews must route through OutlineChapterRegenerationPreviewApplicationPort\n' >&2
    exit 1
fi
if rg -U -n 'outline_preview_application\(\)\s*\.\s*preview_regenerate_chapter_outline' \
    crates/host/a4n-runtime/src/actions/outline_records/chapters/regeneration.rs; then
    printf 'Chapter regeneration previews must not use the mixed outline preview port\n' >&2
    exit 1
fi

if ! rg -n 'pub trait OutlineBeatMappingPreviewApplicationPort' \
    crates/planning/a4n-outline-contract/src/lib.rs || \
   ! rg -n 'outline_beat_mapping_preview_application\(\)' \
    crates/host/a4n-runtime/src/actions/outline_records/batch/beat_mapping.rs; then
    printf 'Beat mapping previews must route through OutlineBeatMappingPreviewApplicationPort\n' >&2
    exit 1
fi
if rg -U -n 'outline_preview_application\(\)\s*\.\s*preview_apply_beat_mapping' \
    crates/host/a4n-runtime/src/actions/outline_records/batch/beat_mapping.rs; then
    printf 'Beat mapping previews must not use the removed mixed outline preview port\n' >&2
    exit 1
fi

if rg -U -n 'pub use a4n_orchestrator::video::\{[^}]*\b(ChapterVideoPromptSummary|LoadVideoSegmentPlanDraftInput|OptimizeChapterVideoPromptInput|UpdateChapterVideoPromptInput|UpdateVideoSegmentPlanDraftRecipeInput)\b' \
    crates/host/a4n-runtime/src; then
    printf 'Video prompt DTOs must be consumed from the API contract without an orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'pub use a4n_api_contract::video::\{' \
    crates/host/a4n-runtime/src/actions/video_prompts.rs; then
    printf 'Video prompt actions must consume the API video contract directly\n' >&2
    exit 1
fi

if rg -U -n 'pub use a4n_orchestrator::character::\{[^}]*\b(ApplyImplicitUpdateDecisionInput|CheckImplicitUpdateInput|GetImplicitReviewInput|ImplicitReviewSummary|ImplicitUpdateDecisionSummary)\b' \
    crates/host/a4n-runtime/src; then
    printf 'Implicit character-review DTOs must be consumed from the API contract without an orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'pub use a4n_api_contract::character::\{' \
    crates/host/a4n-runtime/src/actions/character_implicit_review/mod.rs; then
    printf 'Implicit character-review actions must consume the API character contract directly\n' >&2
    exit 1
fi
if ! rg -n 'character_implicit_review_read_application\(\)' \
    crates/host/a4n-runtime/src/actions/character_implicit_review/api.rs; then
    printf 'Implicit character-review summary reads must route through the read application port\n' >&2
    exit 1
fi
if ! rg -n 'pub trait CharacterImplicitReviewReadApplicationPort' \
    crates/host/a4n-api-contract/src/character.rs; then
    printf 'CharacterImplicitReviewReadApplicationPort contract is missing\n' >&2
    exit 1
fi
if rg -n 'pub trait CharacterImplicitReviewReadApplicationPort' \
    crates/host/a4n-orchestrator/src/character; then
    printf 'CharacterImplicitReviewReadApplicationPort must not be redefined in the orchestrator\n' >&2
    exit 1
fi
if rg -n 'a4n_orchestrator::character::CharacterImplicitReviewReadApplicationPort' \
    crates/host/a4n-runtime/src; then
    printf 'Runtime implicit-review reads must consume the API contract directly\n' >&2
    exit 1
fi
for contract in \
    CharacterImplicitReviewCheckApplicationPort \
    CharacterImplicitReviewPreviewApplicationPort \
    CharacterImplicitReviewApplyApplicationPort; do
    if ! rg -q "pub trait $contract" crates/profiles/a4n-character-host-contract/src/lib.rs; then
        printf '%s contract is missing\n' "$contract" >&2
        exit 1
    fi
done
if ! rg -n 'character_implicit_review_check_application\(\)' \
    crates/host/a4n-runtime/src/actions/character_implicit_review/api.rs || \
   ! rg -n 'character_implicit_review_preview_application\(\)' \
    crates/host/a4n-runtime/src/actions/character_implicit_review/api.rs || \
   ! rg -n 'character_implicit_review_apply_application\(\)' \
    crates/host/a4n-runtime/src/actions/character_implicit_review/application.rs; then
    printf 'Implicit character-review actions must route through worker-specific ports\n' >&2
    exit 1
fi
if rg -n 'CharacterImplicitReviewApplicationPort|character_implicit_review_application\(\)' \
    crates/host/a4n-orchestrator/src/character.rs \
    crates/host/a4n-orchestrator/src/context/character.rs \
    crates/host/a4n-runtime/src/actions/character_implicit_review \
    crates/host/a4n-runtime/src/state/app.rs; then
    printf 'The mixed CharacterImplicitReviewApplicationPort must remain removed\n' >&2
    exit 1
fi

if rg -n 'pub use a4n_api_contract::source_sync::SourceSyncDecisionInput' \
    crates/host/a4n-orchestrator/src/source_sync.rs \
    crates/host/a4n-orchestrator/src/source_sync/contracts.rs; then
    printf 'Source-sync decision input must not be publicly re-exported by the orchestrator facade\n' >&2
    exit 1
fi

if rg -n '^pub use a4n_api_contract::optimize::OptimizationAgentPermissions' \
    crates/host/a4n-orchestrator/src/optimize/permissions.rs \
    || rg -n '^pub use permissions::\{[^}]*OptimizeAgentPermissions' \
    crates/host/a4n-orchestrator/src/optimize.rs; then
    printf 'Optimization permissions must be consumed from the API contract without an orchestrator facade\n' >&2
    exit 1
fi

if rg -n '^pub use a4n_api_contract::optimize::OptimizationPatch' \
    crates/host/a4n-orchestrator/src/optimize/patch/contracts.rs \
    || rg -n '^pub use patch::\{[^}]*OptimizationPatch' \
    crates/host/a4n-orchestrator/src/optimize.rs \
    || rg -n '^pub use contracts::\{[^}]*OptimizationPatch' \
    crates/host/a4n-orchestrator/src/optimize/patch.rs; then
    printf 'Optimization patches must be consumed from the API contract without an orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'pub trait OptimizeSessionReadApplicationPort' \
    crates/host/a4n-api-contract/src/optimize.rs; then
    printf 'OptimizeSessionReadApplicationPort contract is missing\n' >&2
    exit 1
fi
if ! rg -n 'optimize_session_read_application\(\)' \
    crates/host/a4n-runtime/src/actions/optimize/sessions.rs; then
    printf 'Optimize session reads must route through the API read port\n' >&2
    exit 1
fi
if rg -U -n 'optimize_application\(\)\s*\.\s*(list_sessions|get_session|list_turns|list_patch_sets)' \
    crates/host/a4n-runtime/src/actions/optimize; then
    printf 'Optimize session reads must not use the mixed application boundary\n' >&2
    exit 1
fi
if ! rg -n 'pub trait OptimizeSessionMutationApplicationPort' \
    crates/workflow/a4n-optimize-host-contract/src/lib.rs || \
   ! rg -n 'optimize_session_mutation_application\(\)' \
    crates/host/a4n-runtime/src/actions/optimize/sessions.rs \
    crates/host/a4n-runtime/src/actions/optimize/analysis.rs \
    crates/host/a4n-runtime/src/actions/optimize/chat.rs \
    crates/host/a4n-runtime/src/actions/optimize/patches/status.rs; then
    printf 'Optimize session mutations must route through the session mutation port\n' >&2
    exit 1
fi
if ! rg -n 'pub trait OptimizePatchApplicationPort' \
    crates/workflow/a4n-optimize-host-contract/src/lib.rs || \
   ! rg -n 'optimize_patch_application\(\)' \
    crates/host/a4n-runtime/src/actions/optimize/patches/application.rs; then
    printf 'Optimize patch application must route through the patch application port\n' >&2
    exit 1
fi

if ! rg -n 'pub trait OptimizePatchPreviewApplicationPort' \
    crates/workflow/a4n-optimize-host-contract/src/lib.rs || \
   ! rg -n 'optimize_patch_preview_application\(\)' \
    crates/host/a4n-runtime/src/actions/optimize/patches/proposal.rs; then
    printf 'Optimize patch previews must route through OptimizePatchPreviewApplicationPort\n' >&2
    exit 1
fi
if rg -n 'optimize_preview_application\(\)' \
    crates/host/a4n-runtime/src/actions/optimize/patches/proposal.rs; then
    printf 'Optimize patch previews must not use the removed mixed preview port\n' >&2
    exit 1
fi
if ! rg -n 'pub trait OptimizeAnalysisPreviewApplicationPort' \
    crates/workflow/a4n-optimize-host-contract/src/lib.rs || \
   ! rg -n 'optimize_analysis_preview_application\(\)' \
    crates/host/a4n-runtime/src/actions/optimize/analysis/request_preview.rs; then
    printf 'Optimize analysis previews must route through OptimizeAnalysisPreviewApplicationPort\n' >&2
    exit 1
fi
if rg -n 'optimize_preview_application\(\)' \
    crates/host/a4n-runtime/src/actions/optimize/analysis/request_preview.rs; then
    printf 'Optimize analysis previews must not use the mixed preview port\n' >&2
    exit 1
fi
if ! rg -n 'pub trait OptimizeChatPreviewApplicationPort' \
    crates/workflow/a4n-optimize-host-contract/src/lib.rs || \
   ! rg -n 'optimize_chat_preview_application\(\)' \
    crates/host/a4n-runtime/src/actions/optimize/chat/request_preview.rs; then
    printf 'Optimize chat previews must route through OptimizeChatPreviewApplicationPort\n' >&2
    exit 1
fi
if rg -n 'optimize_preview_application\(\)' \
    crates/host/a4n-runtime/src/actions/optimize/chat/request_preview.rs; then
    printf 'Optimize chat previews must not use the mixed preview port\n' >&2
    exit 1
fi
for contract in \
    OptimizeChatGenerationApplicationPort \
    OptimizeAnalysisGenerationApplicationPort \
    OptimizePatchGenerationApplicationPort; do
    if ! rg -q "pub trait $contract" crates/workflow/a4n-optimize-host-contract/src/lib.rs; then
        printf '%s contract is missing\n' "$contract" >&2
        exit 1
    fi
done
if ! rg -n 'optimize_chat_generation_application\(\)' \
    crates/host/a4n-runtime/src/actions/optimize/chat.rs || \
   ! rg -n 'optimize_analysis_generation_application\(\)' \
    crates/host/a4n-runtime/src/actions/optimize/analysis.rs || \
   ! rg -n 'optimize_patch_generation_application\(\)' \
    crates/host/a4n-runtime/src/actions/optimize/patches/proposal.rs; then
    printf 'Optimize generation workers must route through worker-specific ports\n' >&2
    exit 1
fi
if rg -n 'OptimizeApplicationPort|OptimizeGenerationApplicationPort|optimize_application\(\)|optimize_generation_application\(\)' \
    crates/host/a4n-api-contract/src/optimize.rs \
    crates/host/a4n-orchestrator/src/optimize \
    crates/host/a4n-runtime/src/actions/optimize \
    crates/host/a4n-runtime/src/state/app.rs; then
    printf 'The mixed OptimizeApplicationPort must remain removed\n' >&2
    exit 1
fi

for contract in CharacterDiscoveryApplicationPort CharacterProfileSyncApplicationPort; do
    if ! rg -q "pub trait $contract" crates/profiles/a4n-character-host-contract/src/lib.rs; then
        printf '%s contract is missing\n' "$contract" >&2
        exit 1
    fi
done

if ! rg -q "pub trait SceneProfileSyncApplicationPort" crates/profiles/a4n-scene-host-contract/src/lib.rs; then
    printf 'SceneProfileSyncApplicationPort contract is missing\n' >&2
    exit 1
fi

if ! rg -q "pub trait CanvasPreviewApplicationPort" crates/canvas/a4n-canvas-host-contract/src/lib.rs; then
    printf 'Canvas host contracts are missing\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator::canvas::(CanvasNodeRequestPreview|CanvasTextGeneration|Canvas(LlmText|Image|Video|Preview))' crates/host/a4n-runtime/src; then
    printf 'Runtime Canvas node actions must use a4n-canvas-host-contract\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator::scene_profile::(SceneProfileSyncApplicationPort|SceneProfileSyncHostPort|SceneProfileSyncProgress)' crates/host/a4n-runtime/src; then
    printf 'Runtime scene profile sync must use a4n-scene-host-contract\n' >&2
    exit 1
fi
