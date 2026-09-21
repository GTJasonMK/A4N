import { extractErrorMessage } from "../../../api";
import {
  VIDEO_SEGMENT_WORKFLOW_MODES,
  VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES,
  VIDEO_WORKFLOW_MODE_CHAT_VIDEO,
  VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN,
  VIDEO_WORKFLOW_MODE_REFERENCE_MEDIA,
  VIDEO_WORKFLOW_MODE_SHARED_SEGMENT_PLAN,
  VIDEO_WORKFLOW_MODE_STORYBOARD,
  type ChapterVideoPrompt,
  type PromptOutputLanguage,
  type VideoSegmentExecutionRecipe,
  type VideoSegmentWorkflowMode,
} from "../../../api/video";
import { asJsonRecord as asRecord } from "../../../utils/jsonRecord";
import type { MessageTranslator } from "../../../i18n";
import {
  DEFAULT_VIDEO_OPTIONS,
  VIDEO_ASPECT_RATIOS,
  VIDEO_CHAT_VIDEO_CONTRACT_VERSION,
  VIDEO_FIRST_FRAME_CHAIN_CONTRACT_VERSION,
  VIDEO_FINAL_PROMPT_TYPE,
  VIDEO_FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
  VIDEO_FRAME_SEGMENT_PLAN_PROMPT_TYPE,
  VIDEO_STORYBOARD_VIDEO_PROMPT_CONTRACT_VERSION,
  VIDEO_SEGMENT_EXECUTION_RECIPE_SCHEMA_VERSION,
  type VideoAspectRatio,
  type VideoGenOptions,
} from "./videoTabConstants";
import {
  segmentPlanExecutionIdentityIsComplete,
  videoExecutionIdentityOfPrompt,
  videoExecutionSegmentIdentityMatchesPrompt,
  type VideoExecutionSegmentIdentityIndex,
} from "./videoExecutionIdentity";
import {
  promptHasVideoWorkflowContract,
  promptSourcePromptType,
  videoWorkflowModeOfPrompt,
} from "./videoImageUtils";
import { extractSourceContentHash, positiveIntegerOrNull } from "./videoTabShared";

export function videoSegmentRecipeGenerationFingerprintJson(
  recipe: VideoSegmentExecutionRecipe,
): string {
  return JSON.stringify(
    canonicalJsonValue(videoSegmentRecipeFingerprintValue(recipe, false)),
  );
}

export function videoSegmentRecipeFrameFingerprintJson(
  recipe: VideoSegmentExecutionRecipe,
): string {
  return JSON.stringify(
    canonicalJsonValue(videoSegmentRecipeFingerprintValue(recipe, true)),
  );
}

const canonicalJsonValue = (value: unknown): unknown => {
  if (Array.isArray(value)) return value.map(canonicalJsonValue);
  const object = asRecord(value);
  if (!object) return value;
  return Object.fromEntries(
    Object.keys(object)
      .sort()
      .map((key) => [key, canonicalJsonValue(object[key])]),
  );
};

const videoReferenceMediaBindingFingerprintValue = (
  row: NonNullable<
    VideoSegmentExecutionRecipe["reference_media"]
  >["image_rows"][number],
) => ({
  url: row.url,
  usage: row.usage,
  upload_sha256: row.upload_sha256,
  upload_file_name: row.upload_file_name,
  upload_mime_type: row.upload_mime_type,
});

function videoSegmentRecipeFingerprintValue(
  recipe: VideoSegmentExecutionRecipe,
  frameStage: boolean,
): Record<string, unknown> {
  const guidanceMode = frameStage ? "none" : recipe.guidance_mode;
  const referenceMedia =
    !frameStage &&
    recipe.workflow_mode === VIDEO_WORKFLOW_MODE_REFERENCE_MEDIA &&
    recipe.reference_media
      ? {
          prompt: recipe.reference_media.prompt,
          image_rows: recipe.reference_media.image_rows.map(
            videoReferenceMediaBindingFingerprintValue,
          ),
          video_rows: recipe.reference_media.video_rows.map(
            videoReferenceMediaBindingFingerprintValue,
          ),
          audio_rows: recipe.reference_media.audio_rows.map(
            videoReferenceMediaBindingFingerprintValue,
          ),
        }
      : null;
  return {
    schema_version: recipe.schema_version,
    workflow_mode: recipe.workflow_mode,
    style: recipe.style,
    prompt_output_language: recipe.prompt_output_language,
    duration_secs: recipe.duration_secs,
    duration_secs_options: recipe.duration_secs_options,
    aspect_ratio: recipe.aspect_ratio,
    reference_image_ids: recipe.reference_image_ids,
    video_library_image_ids: recipe.video_library_image_ids,
    guidance_mode: guidanceMode,
    guidance_reference_image_ids:
      guidanceMode === "keyframes" ? recipe.guidance_reference_image_ids : [],
    continuity_source_video_id:
      guidanceMode === "continuity" ? recipe.continuity_source_video_id : null,
    reference_media: referenceMedia,
    adopted_video_id: null,
  };
}

export function latestVideoSegmentPlanPrompt(
  prompts: ChapterVideoPrompt[],
  currentContentHash: string | null,
): ChapterVideoPrompt | null {
  if (!currentContentHash) return null;
  let latest: ChapterVideoPrompt | null = null;
  for (const prompt of prompts) {
    if (
      prompt.prompt_type === VIDEO_FRAME_SEGMENT_PLAN_PROMPT_TYPE &&
      prompt.workflow_mode === VIDEO_WORKFLOW_MODE_SHARED_SEGMENT_PLAN &&
      prompt.status === "completed" &&
      promptHasVideoWorkflowContract(
        prompt,
        VIDEO_FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
      ) &&
      segmentPlanExecutionIdentityIsComplete(prompt) &&
      extractSourceContentHash(prompt.metadata_json) === currentContentHash &&
      (latest == null || prompt.id > latest.id)
    ) {
      latest = prompt;
    }
  }
  return latest;
}

export function videoOptionsFromSegmentRecipe(
  recipe: VideoSegmentExecutionRecipe | null,
): VideoGenOptions {
  if (!recipe) return DEFAULT_VIDEO_OPTIONS;
  return {
    style: recipe.style,
    promptOutputLanguage: recipe.prompt_output_language,
    durationSecs: recipe.duration_secs,
    durationOptions: recipe.duration_secs_options,
    aspectRatio: recipe.aspect_ratio as VideoAspectRatio,
  };
}

export interface VideoSegmentCardPlan {
  segmentNumber: number;
  title: string;
  sceneSummary: string;
  durationSecs: number;
  aspectRatio: VideoAspectRatio;
  raw: Record<string, unknown>;
}

interface VideoSegmentRecipeReadResult {
  recipes: Map<number, VideoSegmentExecutionRecipe>;
  error: string | null;
}

export function videoSegmentCardsFromPlanPrompt(
  prompt: ChapterVideoPrompt | null | undefined,
  t: MessageTranslator,
): VideoSegmentCardPlan[] {
  const metadata = asRecord(prompt?.metadata_json);
  return videoSegmentCardsFromSegments(
    metadata && Array.isArray(metadata.segments) ? metadata.segments : [],
    t,
  );
}

export function videoSegmentCardsFromSegments(
  segments: unknown[],
  t: MessageTranslator,
): VideoSegmentCardPlan[] {
  return segments
    .map(asRecord)
    .filter((segment): segment is Record<string, unknown> => segment != null)
    .map((segment) => {
      const segmentNumber = segment.segment_number;
      if (
        typeof segmentNumber !== "number" ||
        !Number.isInteger(segmentNumber) ||
        segmentNumber <= 0
      ) {
        throw new Error(t("video.chapter.invalidSegmentNumber"));
      }
      const durationSecs = segment.duration_secs;
      const aspectRatio = segment.aspect_ratio;
      if (
        typeof durationSecs !== "number" ||
        !Number.isInteger(durationSecs) ||
        durationSecs <= 0
      ) {
        throw new Error(t("video.chapter.invalidSegmentDuration", { number: segmentNumber }));
      }
      if (
        typeof aspectRatio !== "string" ||
        !VIDEO_ASPECT_RATIOS.includes(aspectRatio as VideoAspectRatio)
      ) {
        throw new Error(t("video.chapter.invalidSegmentAspectRatio", { number: segmentNumber }));
      }
      const title =
        typeof segment.title === "string" ? segment.title.trim() : "";
      if (!title) throw new Error(t("video.chapter.invalidSegmentTitle", { number: segmentNumber }));
      const sceneSummary =
        typeof segment.scene_summary === "string"
          ? segment.scene_summary.trim()
          : "";
      if (!sceneSummary)
        throw new Error(t("video.chapter.invalidSegmentSummary", { number: segmentNumber }));
      return {
        segmentNumber,
        title,
        sceneSummary,
        durationSecs,
        aspectRatio: aspectRatio as VideoAspectRatio,
        raw: segment,
      };
    })
    .sort((left, right) => left.segmentNumber - right.segmentNumber);
}

export function readVideoSegmentRecipes(
  prompt: ChapterVideoPrompt | null | undefined,
  t: MessageTranslator,
): VideoSegmentRecipeReadResult {
  const metadata = asRecord(prompt?.metadata_json);
  const raw = metadata?.segment_recipes;
  if (!prompt) return { recipes: new Map(), error: null };
  if (raw == null) {
    return {
      recipes: new Map(),
      error: t("video.chapter.missingSegmentRecipes"),
    };
  }
  if (!Array.isArray(raw)) {
    return { recipes: new Map(), error: t("video.chapter.segmentRecipesNotArray") };
  }
  try {
    const recipes = new Map(
      parseVideoSegmentRecipeArray(raw, t).map((recipe) => [
        recipe.segment_number,
        recipe,
      ]),
    );
    return { recipes, error: null };
  } catch (error) {
    return {
      recipes: new Map(),
      error: extractErrorMessage(error, t("video.chapter.segmentRecipesReadFailed")),
    };
  }
}

function parseVideoSegmentRecipeArray(
  raw: unknown[],
  t: MessageTranslator,
): VideoSegmentExecutionRecipe[] {
  const recipes = new Map<number, VideoSegmentExecutionRecipe>();
  for (const [index, value] of raw.entries()) {
    const recipe = asRecord(value);
    if (!recipe) throw new Error(t("video.chapter.recipeNotObject", { index }));
    if (
      recipe.schema_version !== VIDEO_SEGMENT_EXECUTION_RECIPE_SCHEMA_VERSION
    ) {
      throw new Error(t("video.chapter.recipeSchemaInvalid", { index }));
    }
    const segmentNumber = recipe.segment_number;
    if (
      typeof segmentNumber !== "number" ||
      !Number.isInteger(segmentNumber) ||
      segmentNumber <= 0
    ) {
      throw new Error(t("video.chapter.recipeSegmentNumberInvalid", { index }));
    }
    if (recipes.has(segmentNumber)) {
      throw new Error(t("video.chapter.recipeDuplicateSegment", { number: segmentNumber }));
    }
    if (
      typeof recipe.workflow_mode !== "string" ||
      !VIDEO_SEGMENT_WORKFLOW_MODES.includes(
        recipe.workflow_mode as VideoSegmentWorkflowMode,
      )
    ) {
      throw new Error(t("video.chapter.recipeWorkflowInvalid", { number: segmentNumber }));
    }
    recipes.set(segmentNumber, value as VideoSegmentExecutionRecipe);
  }
  return [...recipes.values()];
}

export function finalVideoExecutionIdentityIsComplete(
  prompt: ChapterVideoPrompt | null | undefined,
): boolean {
  const identity = videoExecutionIdentityOfPrompt(prompt);
  if (!identity) return false;
  return (
    typeof identity.frame_set_sha256 === "string" &&
    identity.frame_set_sha256.trim().length > 0 &&
    typeof identity.prompt_output_language === "string" &&
    identity.prompt_output_language.trim().length > 0 &&
    asRecord(identity.generation_asset_fingerprint) != null
  );
}

export function currentFinalVideoPromptMatchesSegment(
  prompt: ChapterVideoPrompt,
  segmentPlanPrompt: ChapterVideoPrompt | null | undefined,
  segmentNumber: number,
  recipeFingerprint: string | null | undefined,
  workflowMode: VideoSegmentWorkflowMode | null | undefined,
  promptOutputLanguage: PromptOutputLanguage,
  identityIndex?: VideoExecutionSegmentIdentityIndex,
): boolean {
  const contractVersion =
    workflowMode === VIDEO_WORKFLOW_MODE_CHAT_VIDEO
      ? VIDEO_CHAT_VIDEO_CONTRACT_VERSION
      : workflowMode === VIDEO_WORKFLOW_MODE_STORYBOARD
        ? VIDEO_STORYBOARD_VIDEO_PROMPT_CONTRACT_VERSION
        : workflowMode === VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN
          ? VIDEO_FIRST_FRAME_CHAIN_CONTRACT_VERSION
          : workflowMode === VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES
            ? VIDEO_FRAME_SEGMENT_PLAN_CONTRACT_VERSION
            : null;
  const metadata = asRecord(prompt.metadata_json);
  return (
    contractVersion != null &&
    prompt.prompt_type === VIDEO_FINAL_PROMPT_TYPE &&
    videoWorkflowModeOfPrompt(prompt) === workflowMode &&
    promptHasVideoWorkflowContract(prompt, contractVersion) &&
    promptSourcePromptType(prompt) === VIDEO_FRAME_SEGMENT_PLAN_PROMPT_TYPE &&
    metadata?.prompt_output_language === promptOutputLanguage &&
    videoExecutionSegmentIdentityMatchesPrompt(
      prompt,
      segmentPlanPrompt,
      segmentNumber,
      recipeFingerprint,
      identityIndex,
    ) &&
    finalVideoExecutionIdentityIsComplete(prompt) &&
    videoExecutionIdentityOfPrompt(prompt)?.prompt_output_language ===
      promptOutputLanguage
  );
}

export function completedSegmentPlanPreviewItems(
  streamText: string,
): Record<string, unknown>[] {
  const byShot = new Map<
    string,
    {
      shotIndex: number | null;
      order: number;
      segments: Record<string, unknown>[];
    }
  >();
  const unscoped: Array<{
    shotIndex: number | null;
    order: number;
    segments: Record<string, unknown>[];
  }> = [];
  let order = 0;
  for (const line of streamText.split("\n")) {
    if (!line.trim()) continue;
    let value: Record<string, unknown> | null;
    try {
      value = asRecord(JSON.parse(line));
    } catch {
      break;
    }
    const group = Array.isArray(value?.segments)
      ? value.segments
          .map(asRecord)
          .filter((item): item is Record<string, unknown> => item != null)
      : [];
    if (group.length === 0) continue;
    const currentOrder = order;
    order += 1;
    const shotIndex = positiveIntegerOrNull(value?.shot_index);
    const shotIds = new Set(
      group
        .map((item) => item.source_shot_id)
        .filter(
          (value): value is string =>
            typeof value === "string" && value.length > 0,
        ),
    );
    if (shotIds.size === 1) {
      const sourceShotId = [...shotIds][0];
      const previous = byShot.get(sourceShotId);
      byShot.set(sourceShotId, {
        shotIndex: shotIndex ?? previous?.shotIndex ?? null,
        order: previous?.order ?? currentOrder,
        segments: group,
      });
    } else {
      unscoped.push({ shotIndex, order: currentOrder, segments: group });
    }
  }
  return [...byShot.values(), ...unscoped]
    .sort((left, right) => {
      if (left.shotIndex != null && right.shotIndex != null) {
        return left.shotIndex - right.shotIndex;
      }
      if (left.shotIndex != null) return -1;
      if (right.shotIndex != null) return 1;
      return left.order - right.order;
    })
    .flatMap((group) => group.segments)
    .map((segment, index) => ({
      ...segment,
      segment_number: index + 1,
    }));
}
