import type { GeneratedImage } from "../../../api/image";
import { imageTypeLabel } from "./imageTabUtils";
import { asJsonRecord as asRecord } from "../../../utils/jsonRecord";
import { formatDateTime, formatNumber, type Locale, type MessageKey, type MessageTranslator } from "../../../i18n";

export interface MetadataField {
  label: string;
  value: string;
  mono?: boolean;
}

const formatImageTimestamp = (
  value: string | null | undefined,
  t: MessageTranslator,
  locale: Locale,
): string => {
  return formatDateTime(value, locale, t("novel.imageLibrary.unknown"));
};

const metadataValueText = (value: unknown, t: MessageTranslator, locale: Locale): string => {
  if (value == null) return t("novel.imageLibrary.none");
  if (typeof value === "string") return value.trim() || t("novel.imageLibrary.none");
  if (typeof value === "boolean") return value ? t("novel.imageLibrary.yes") : t("novel.imageLibrary.no");
  if (typeof value === "number") return formatNumber(value, locale);
  return JSON.stringify(value);
};

const field = (label: string, value: unknown, t: MessageTranslator, locale: Locale, mono = false): MetadataField => ({
  label,
  value: metadataValueText(value, t, locale),
  mono,
});

const GENERATION_SERVICE_LABELS: Record<string, string> = {
  openai: "OpenAI",
  comfyui: "ComfyUI",
  sd_webui: "Stable Diffusion WebUI",
  cliproxy: "CLIProxy",
  newapi: "New API",
};

const VIDEO_ASSET_ROLE_LABELS: Record<string, MessageKey> = {
  reference: "novel.imageLibrary.referenceMaterial",
  material: "novel.imageLibrary.videoMaterial",
  start_frame: "novel.imageLibrary.videoStartFrame",
  end_frame: "novel.imageLibrary.videoEndFrame",
  storyboard: "novel.imageLibrary.videoStoryboard",
  key_frame: "novel.imageLibrary.videoKeyFrame",
};

const enumLabel = (
  value: unknown,
  labels: Record<string, MessageKey>,
  fallback: string,
  t: MessageTranslator,
): string | null => {
  if (typeof value !== "string" || !value.trim()) return null;
  const key = labels[value.trim()];
  return key ? t(key) : fallback;
};

const presentFields = (fields: MetadataField[], t: MessageTranslator): MetadataField[] =>
  fields.filter((item) => item.value !== t("novel.imageLibrary.none"));

const fileSizeLabel = (value: number | null, t: MessageTranslator, locale: Locale): string => {
  if (value == null || value < 0) return t("novel.imageLibrary.unknown");
  if (value < 1024) return `${formatNumber(value, locale)} B`;
  if (value < 1024 * 1024) return `${formatNumber(value / 1024, locale, { minimumFractionDigits: 1, maximumFractionDigits: 1 })} KB`;
  return `${formatNumber(value / 1024 / 1024, locale, { minimumFractionDigits: 1, maximumFractionDigits: 1 })} MB`;
};

export const imageBaseMetadataFields = (image: GeneratedImage, t: MessageTranslator, locale: Locale): MetadataField[] => [
  field(t("novel.imageLibrary.imageType"), imageTypeLabel(image.image_type, t), t, locale),
  field(t("novel.imageLibrary.model"), image.model, t, locale, true),
  field(t("novel.imageLibrary.size"), `${formatNumber(image.width, locale)} × ${formatNumber(image.height, locale)}`, t, locale, true),
  field(t("novel.imageLibrary.fileSize"), fileSizeLabel(image.file_size, t, locale), t, locale),
  field(t("novel.imageLibrary.qualityScore"), image.ai_score == null ? t("novel.imageLibrary.notRated") : formatNumber(image.ai_score, locale, { minimumFractionDigits: 3, maximumFractionDigits: 3 }), t, locale, true),
  field(t("novel.imageLibrary.favorite"), image.is_favorite ? t("novel.imageLibrary.yes") : t("novel.imageLibrary.no"), t, locale),
  field(t("novel.imageLibrary.createdAt"), formatImageTimestamp(image.created_at, t, locale), t, locale),
];

export const imageGenerationMetadataFields = (image: GeneratedImage, t: MessageTranslator, locale: Locale): MetadataField[] => {
  const generation = asRecord(asRecord(image.metadata_json)?.generation);
  if (!generation) return [];
  const generationSize =
    generation.width != null && generation.height != null
      ? `${metadataValueText(generation.width, t, locale)} × ${metadataValueText(generation.height, t, locale)}`
      : null;
  return presentFields([
    field(t("novel.imageLibrary.configName"), generation.config_name, t, locale),
    field(
      t("novel.imageLibrary.generationService"),
      typeof generation.backend === "string" && generation.backend.trim()
        ? GENERATION_SERVICE_LABELS[generation.backend.trim()] ?? t("novel.imageLibrary.otherGenerationService")
        : null,
      t,
      locale,
    ),
    field(t("novel.imageLibrary.configuredModel"), generation.configured_model, t, locale, true),
    field(t("novel.imageLibrary.providerModel"), generation.provider_model, t, locale, true),
    field(
      t("novel.imageLibrary.imageType"),
      typeof generation.image_type === "string"
        ? imageTypeLabel(generation.image_type, t)
        : null,
      t,
      locale,
    ),
    field(t("novel.imageLibrary.size"), generationSize, t, locale, true),
    field(
      t("novel.imageLibrary.generationTime"),
      typeof generation.generated_at === "string"
        ? formatImageTimestamp(generation.generated_at, t, locale)
        : null,
      t,
      locale,
    ),
  ], t);
};

export const imageSourceMetadataFields = (image: GeneratedImage, t: MessageTranslator, locale: Locale): MetadataField[] => {
  const source = asRecord(asRecord(image.metadata_json)?.source);
  if (!source) return [];
  return presentFields([
    field(
      t("novel.imageLibrary.contentLength"),
      typeof source.content_len === "number"
        ? `${formatNumber(source.content_len, locale)} ${t("novel.imageLibrary.characters")}`
        : null,
      t,
      locale,
    ),
  ], t);
};

export const imageBusinessMetadataFields = (image: GeneratedImage, t: MessageTranslator, locale: Locale): MetadataField[] => {
  const metadata = asRecord(image.metadata_json);
  if (!metadata) return [];
  const context = asRecord(metadata.generation_context);
  const fields: MetadataField[] = [
    field(t("novel.imageLibrary.chapterVideo"), metadata.chapter_video_material, t, locale),
    field(
      t("novel.imageLibrary.videoUsage"),
      enumLabel(metadata.video_asset_role, VIDEO_ASSET_ROLE_LABELS, t("novel.imageLibrary.relatedVideoMaterial"), t),
      t,
      locale,
    ),
    field(
      t("novel.imageLibrary.comicPage"),
      typeof metadata.page_number === "number" ? t("novel.imageLibrary.pageNumber", { number: metadata.page_number }) : null,
      t,
      locale,
    ),
    field(t("novel.imageLibrary.characterName"), metadata.character_name, t, locale),
  ].filter((item) => item.value !== t("novel.imageLibrary.none"));

  if (context) {
    fields.push(
      field(
        t("novel.imageLibrary.shot"),
        typeof context.shot_number === "number" ? `${t("novel.imageLibrary.shot")} ${context.shot_number}` : null,
        t,
        locale,
      ),
      field(t("novel.imageLibrary.sourceTitle"), context.title, t, locale),
      field(t("novel.imageLibrary.sourceScene"), context.scene_summary, t, locale),
    );
  }
  return fields.filter((item) => item.value !== t("novel.imageLibrary.none"));
};

export const imagePromptSnapshotFields = (image: GeneratedImage, t: MessageTranslator, locale: Locale): MetadataField[] => {
  const snapshot = asRecord(asRecord(image.metadata_json)?.prompt_snapshot);
  if (!snapshot) return [];
  const referenceImageCount = Array.isArray(snapshot.reference_image_ids)
    ? snapshot.reference_image_ids.length
    : null;
  return presentFields([
    field(t("novel.imageLibrary.visualPrompt"), snapshot.visual_prompt, t, locale),
    field(t("novel.imageLibrary.motionPrompt"), snapshot.motion_prompt, t, locale),
    field(t("novel.imageLibrary.negativePrompt"), snapshot.negative_prompt, t, locale),
    field(
      t("novel.imageLibrary.duration"),
      typeof snapshot.duration_secs === "number" ? t("novel.imageLibrary.seconds", { count: snapshot.duration_secs }) : null,
      t,
      locale,
    ),
    field(t("novel.imageLibrary.aspectRatio"), snapshot.aspect_ratio, t, locale, true),
    field(t("novel.imageLibrary.referenceImages"), referenceImageCount == null ? null : t("novel.imageLibrary.imageCount", { count: referenceImageCount }), t, locale),
  ], t);
};

export const imageMetadataRawJson = (image: GeneratedImage): string =>
  JSON.stringify(
    {
      record: {
        id: image.id,
        project_id: image.project_id,
        image_type: image.image_type,
        file_path: image.file_path,
        prompt: image.prompt,
        model: image.model,
        width: image.width,
        height: image.height,
        file_size: image.file_size,
        ai_score: image.ai_score,
        is_favorite: image.is_favorite,
        chapter_id: image.chapter_id,
        panel_id: image.panel_id,
        created_at: image.created_at,
      },
      metadata_json: image.metadata_json,
    },
    null,
    2,
  );
