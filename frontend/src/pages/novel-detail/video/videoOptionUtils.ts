import {
  type GeneratedImage,
} from "../../../api/image";
import {
  type GeneratedVideo,
  type PromptOutputLanguage,
  type VideoGenConfig,
} from "../../../api/video";
import { asJsonRecord as asRecord } from "../../../utils/jsonRecord";
import { isSafeInteger } from "../../../utils/numberRange";
import type { ChapterRow } from "../shared";
import type { MessageTranslator } from "../../../i18n";
import {
  MAX_VIDEO_DURATION_OPTION_COUNT,
  GENERIC_VIDEO_DURATION_LIMITS,
  PROMPT_OUTPUT_LANGUAGES,
  VIDEO_ASPECT_RATIOS,
  type VideoAspectRatio,
  type VideoDurationLimits,
  type VideoGenOptions,
} from "./videoTabConstants";
import {
  isProjectReusableVideoMaterialImage,
  isVideoFrameInputImage,
  profileMaterialKindOfImage,
} from "./videoImageUtils";
import { extractSourceContentHash } from "./videoTabShared";

export const staleVideoPromptMessage = (
  frameStageLabel: string,
  t: MessageTranslator,
): string =>
  t("video.chapter.stalePrompt", { stage: frameStageLabel });

export const promptCharCount = (value: string | null | undefined): number =>
  Array.from(value?.trim() ?? "").length;

const textEncoder = new TextEncoder();

export const promptByteCount = (value: string | null | undefined): number =>
  textEncoder.encode(value?.trim() ?? "").length;

export const promptLengthError = (
  label: string,
  value: string | null | undefined,
  minChars: number,
  t: MessageTranslator,
): string | null => {
  const text = value?.trim() ?? "";
  if (!text) return t("video.chapter.framePromptEmpty", { label });
  const charCount = Array.from(text).length;
  return charCount < minChars
    ? t("video.chapter.framePromptTooShort", {
        label,
        min: minChars,
        count: charCount,
      })
    : null;
};

export const parseNumberInput = (value: unknown): number | null => {
  if (typeof value === "number") {
    return isSafeInteger(value) ? value : null;
  }
  if (typeof value !== "string") return null;
  const text = value.trim();
  if (!/^\d+$/.test(text)) return null;
  const parsed = Number(text);
  return isSafeInteger(parsed) ? parsed : null;
};

const strictVideoOptionInt = (
  value: unknown,
  field: string,
  min: number,
  max: number,
  t: MessageTranslator,
): number => {
  if (
    !isSafeInteger(value) ||
    (value as number) < min ||
    (value as number) > max
  ) {
    throw new Error(t("video.chapter.invalidVideoOptionRange", { field, min, max }));
  }
  return value as number;
};

const durationOptionsValidationError = (
  defaultDurationSecs: number,
  options: number[],
  t: MessageTranslator,
): string | null => {
  if (
    !Array.isArray(options) ||
    options.some(
      (duration) =>
        !isSafeInteger(duration) || duration < 1 || duration > 60,
    )
  ) {
    return t("video.chapter.invalidDurationOptionsValues");
  }
  if (
    options.length === 0 ||
    options.length > MAX_VIDEO_DURATION_OPTION_COUNT
  ) {
    return t("video.chapter.invalidDurationOptionsCount");
  }
  if (!options.includes(defaultDurationSecs)) {
    return t("video.chapter.missingDefaultDuration");
  }
  if (
    options.some(
      (duration, index) => index > 0 && duration <= options[index - 1],
    )
  ) {
    return t("video.chapter.unorderedDurationOptions");
  }
  return null;
};

export const videoOptionsValidationError = (
  options: VideoGenOptions,
  t: MessageTranslator,
): string | null => {
  if (typeof options.style !== "string") {
    return t("video.chapter.invalidVideoOptionType", { field: "style" });
  }
  if (
    !isSafeInteger(options.durationSecs) ||
    options.durationSecs < 1 ||
    options.durationSecs > 60
  ) {
    return t("video.chapter.invalidVideoOptionRange", {
      field: "durationSecs",
      min: 1,
      max: 60,
    });
  }
  const durationOptionsError = durationOptionsValidationError(
    options.durationSecs,
    options.durationOptions,
    t,
  );
  if (durationOptionsError) return durationOptionsError;
  if (!VIDEO_ASPECT_RATIOS.includes(options.aspectRatio)) {
    return t("video.chapter.invalidAspectRatio");
  }
  if (!PROMPT_OUTPUT_LANGUAGES.includes(options.promptOutputLanguage)) {
    return t("video.chapter.invalidPromptOutputLanguage");
  }
  return null;
};

export const parseVideoOptions = (
  raw: string,
  t: MessageTranslator,
): VideoGenOptions => {
  let parsedValue: unknown;
  try {
    parsedValue = JSON.parse(raw);
  } catch {
    throw new Error(t("video.chapter.invalidVideoOptionsObject"));
  }
  const parsed = asRecord(parsedValue);
  if (!parsed) throw new Error(t("video.chapter.invalidVideoOptionsObject"));
  const allowedFields = new Set([
    "style",
    "promptOutputLanguage",
    "durationSecs",
    "durationOptions",
    "aspectRatio",
  ]);
  const unknownField = Object.keys(parsed).find(
    (field) => !allowedFields.has(field),
  );
  if (unknownField) {
    throw new Error(t("video.chapter.invalidVideoOptionField", { field: unknownField }));
  }

  if (typeof parsed.style !== "string") {
    throw new Error(t("video.chapter.invalidVideoOptionType", { field: "style" }));
  }
  const style = parsed.style;
  const promptOutputLanguage =
    typeof parsed.promptOutputLanguage === "string" &&
    PROMPT_OUTPUT_LANGUAGES.includes(
      parsed.promptOutputLanguage as PromptOutputLanguage,
    )
      ? (parsed.promptOutputLanguage as PromptOutputLanguage)
      : null;
  if (!promptOutputLanguage) {
    throw new Error(t("video.chapter.invalidPromptOutputLanguage"));
  }

  const durationSecs = strictVideoOptionInt(
    parsed.durationSecs,
    "durationSecs",
    1,
    60,
    t,
  );
  if (!Array.isArray(parsed.durationOptions)) {
    throw new Error(t("video.chapter.invalidDurationOptionsArray"));
  }
  const durationOptions = parsed.durationOptions.map((value) =>
    strictVideoOptionInt(value, "durationOptions", 1, 60, t),
  );

  if (!VIDEO_ASPECT_RATIOS.includes(parsed.aspectRatio as VideoAspectRatio)) {
    throw new Error(t("video.chapter.invalidAspectRatio"));
  }

  const options: VideoGenOptions = {
    style,
    promptOutputLanguage,
    durationSecs,
    durationOptions,
    aspectRatio: parsed.aspectRatio as VideoAspectRatio,
  };
  const validationError = videoOptionsValidationError(options, t);
  if (validationError) {
    throw new Error(validationError);
  }
  return options;
};

export const serializeVideoOptions = (
  options: VideoGenOptions,
  t: MessageTranslator,
): string => {
  const validationError = videoOptionsValidationError(options, t);
  if (validationError) throw new Error(validationError);
  return JSON.stringify({
    style: options.style,
    promptOutputLanguage: options.promptOutputLanguage,
    durationSecs: options.durationSecs,
    durationOptions: options.durationOptions,
    aspectRatio: options.aspectRatio,
  });
};

export function mergeDurationOptions(
  defaultDurationSecs: number,
  options: number[],
): number[] {
  return [...new Set([...options, defaultDurationSecs])].sort((a, b) => a - b);
}

export function durationChoicesForLimits(
  limits: VideoDurationLimits,
): number[] {
  if (limits.configError) return [];
  if (limits.durationMode === "fixed") return limits.supportedDurations;
  const baseChoices = [5, 10, 15];
  return mergeDurationOptions(
    limits.defaultDurationSecs,
    baseChoices.filter(
      (duration) =>
        duration >= limits.minDurationSecs &&
        duration <= limits.maxDurationSecs,
    ),
  );
}

export function durationOptionsAllowed(
  defaultDurationSecs: number,
  options: number[],
  limits: VideoDurationLimits,
  t: MessageTranslator,
): boolean {
  return (
    durationOptionsValidationError(defaultDurationSecs, options, t) == null &&
    options.every((duration) => isDurationAllowed(duration, limits))
  );
}

export function videoDurationLimitsFromConfig(
  config: VideoGenConfig | null,
): VideoDurationLimits {
  if (!config) return GENERIC_VIDEO_DURATION_LIMITS;
  return {
    model: config.model,
    durationMode: config.duration_mode,
    defaultDurationSecs: config.default_duration_secs,
    minDurationSecs: config.min_duration_secs,
    maxDurationSecs: config.max_duration_secs,
    supportedDurations:
      config.duration_mode === "fixed"
        ? config.supported_durations_json
        : [],
    configError: null,
  };
}

export function isDurationAllowed(
  durationSecs: number,
  limits: VideoDurationLimits,
): boolean {
  if (limits.configError) return false;
  if (!isSafeInteger(durationSecs)) return false;
  if (
    durationSecs < limits.minDurationSecs ||
    durationSecs > limits.maxDurationSecs
  )
    return false;
  return (
    limits.durationMode !== "fixed" ||
    limits.supportedDurations.includes(durationSecs)
  );
}

export function durationLimitsLabel(
  limits: VideoDurationLimits,
  t: MessageTranslator,
): string {
  if (limits.configError) {
    return t("video.chapter.invalidDurationConfig", { error: limits.configError });
  }
  if (limits.durationMode === "fixed") {
    return t("video.chapter.fixedDurations", {
      values: limits.supportedDurations.join(" / "),
    });
  }
  return t("video.chapter.durationRange", {
    min: limits.minDurationSecs,
    max: limits.maxDurationSecs,
  });
}

export const parseNullableNumber = (
  raw: string,
  t: MessageTranslator,
): number | null => {
  const parsed = JSON.parse(raw) as unknown;
  if (parsed === null) return null;
  if (typeof parsed === "number" && Number.isFinite(parsed) && parsed > 0)
    return parsed;
  throw new Error(t("video.chapter.invalidPersistedVideoId"));
};

export const serializeNullableNumber = (value: number | null): string =>
  JSON.stringify(
    Number.isFinite(value) && value != null && value > 0 ? value : null,
  );

const isTerminalVideoStatus = (status: string | null | undefined): boolean => {
  if (typeof status !== "string") return false;
  const normalized = status.trim().toLowerCase();
  if (!normalized) return false;
  return (
    normalized === "completed" ||
    normalized === "failed" ||
    normalized === "cancelled"
  );
};

export const hasPendingProviderJob = (video: GeneratedVideo): boolean =>
  Boolean(video.provider_job_id) && !isTerminalVideoStatus(video.status);

export const isSelectableReferenceImage = (
  img: GeneratedImage,
  chapter: ChapterRow | null,
  currentContentHash: string | null,
): boolean => {
  if (isVideoFrameInputImage(img)) return false;
  if (isProjectReusableVideoMaterialImage(img)) return true;
  if (img.chapter_id == null) return true;
  if (!chapter || img.chapter_id !== chapter.id) return false;
  if (!currentContentHash) return true;
  return extractSourceContentHash(img.metadata_json) === currentContentHash;
};

export const referenceImageScopeLabel = (
  img: GeneratedImage,
  chapter: ChapterRow | null,
  currentContentHash: string | null,
  t: MessageTranslator,
): string => {
  const profileMaterialKind = profileMaterialKindOfImage(img);
  if (profileMaterialKind === "scene") return t("video.chapter.sceneMaterial");
  if (profileMaterialKind === "object") return t("video.chapter.objectMaterial");
  if (isProjectReusableVideoMaterialImage(img)) return t("video.chapter.profileMaterial");
  if (img.chapter_id == null) return t("video.chapter.projectMaterial");
  if (!chapter || img.chapter_id !== chapter.id) return t("video.chapter.otherChapter");
  if (
    currentContentHash &&
    extractSourceContentHash(img.metadata_json) !== currentContentHash
  ) {
    return t("video.chapter.fingerprintMismatch");
  }
  return t("video.chapter.chapterImage");
};
