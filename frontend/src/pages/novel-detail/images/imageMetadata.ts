import type { GeneratedImage } from "../../../api/image";
import { imageTypeLabel } from "./imageTabUtils";
import { asJsonRecord as asRecord } from "../../../utils/jsonRecord";
import { formatUiDateTime } from "../../../utils/dateTime";

export interface MetadataField {
  label: string;
  value: string;
  mono?: boolean;
}

const formatImageTimestamp = (value: string | null | undefined): string => {
  return formatUiDateTime(value, "未知");
};

const metadataValueText = (value: unknown): string => {
  if (value == null) return "无";
  if (typeof value === "string") return value.trim() || "无";
  if (typeof value === "boolean") return value ? "是" : "否";
  if (typeof value === "number") return String(value);
  return JSON.stringify(value);
};

const field = (label: string, value: unknown, mono = false): MetadataField => ({
  label,
  value: metadataValueText(value),
  mono,
});

const GENERATION_SERVICE_LABELS: Record<string, string> = {
  openai: "OpenAI",
  comfyui: "ComfyUI",
  sd_webui: "Stable Diffusion WebUI",
  cliproxy: "CLIProxy",
  newapi: "New API",
};

const PROMPT_SOURCE_LABELS: Record<string, string> = {
  frame_segment_plan: "片段规划",
  frame_prompt: "画面提示词",
  first_frame_prompt: "首帧提示词",
  chat_video_frame_prompt: "Chat 视频画面提示词",
  storyboard_prompt: "分镜板提示词",
  video_prompt: "视频提示词",
  page_generation: "漫画页生成",
  panel_generation: "漫画画格生成",
  portrait_generation: "角色立绘生成",
  scene_generation: "场景图生成",
};

const VIDEO_ASSET_ROLE_LABELS: Record<string, string> = {
  reference: "参考素材",
  material: "视频素材",
  start_frame: "视频首帧",
  end_frame: "视频尾帧",
  storyboard: "视频分镜板",
  key_frame: "视频关键帧",
};

const enumLabel = (
  value: unknown,
  labels: Record<string, string>,
  fallback: string,
): string | null => {
  if (typeof value !== "string" || !value.trim()) return null;
  return labels[value.trim()] ?? fallback;
};

const presentFields = (fields: MetadataField[]): MetadataField[] =>
  fields.filter((item) => item.value !== "无");

const fileSizeLabel = (value: number | null): string => {
  if (value == null || value < 0) return "未知";
  if (value < 1024) return `${value} B`;
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KB`;
  return `${(value / 1024 / 1024).toFixed(1)} MB`;
};

export const imageBaseMetadataFields = (image: GeneratedImage): MetadataField[] => [
  field("图片类型", imageTypeLabel(image.image_type)),
  field("模型", image.model, true),
  field("尺寸", `${image.width} × ${image.height}`, true),
  field("文件大小", fileSizeLabel(image.file_size)),
  field("质量评分", image.ai_score == null ? "未评分" : image.ai_score.toFixed(3), true),
  field("收藏", image.is_favorite ? "是" : "否"),
  field("创建时间", formatImageTimestamp(image.created_at)),
];

export const imageGenerationMetadataFields = (image: GeneratedImage): MetadataField[] => {
  const generation = asRecord(asRecord(image.metadata_json)?.generation);
  if (!generation) return [];
  const generationSize =
    generation.width != null && generation.height != null
      ? `${metadataValueText(generation.width)} × ${metadataValueText(generation.height)}`
      : null;
  return presentFields([
    field("配置名称", generation.config_name),
    field(
      "生成服务",
      enumLabel(generation.backend, GENERATION_SERVICE_LABELS, "其他生成服务"),
    ),
    field("配置模型", generation.configured_model, true),
    field("实际模型", generation.provider_model, true),
    field(
      "图片类型",
      typeof generation.image_type === "string"
        ? imageTypeLabel(generation.image_type)
        : null,
    ),
    field("尺寸", generationSize, true),
    field(
      "生成时间",
      typeof generation.generated_at === "string"
        ? formatImageTimestamp(generation.generated_at)
        : null,
    ),
  ]);
};

export const imageSourceMetadataFields = (image: GeneratedImage): MetadataField[] => {
  const source = asRecord(asRecord(image.metadata_json)?.source);
  if (!source) return [];
  return presentFields([
    field(
      "正文长度",
      typeof source.content_len === "number" ? `${source.content_len.toLocaleString()} 字符` : null,
    ),
  ]);
};

export const imageBusinessMetadataFields = (image: GeneratedImage): MetadataField[] => {
  const metadata = asRecord(image.metadata_json);
  if (!metadata) return [];
  const context = asRecord(metadata.generation_context);
  const fields: MetadataField[] = [
    field("用于章节视频", metadata.chapter_video_material),
    field("加入视频素材库", metadata.video_library_member),
    field(
      "视频用途",
      enumLabel(metadata.video_asset_role, VIDEO_ASSET_ROLE_LABELS, "相关视频素材"),
    ),
    field(
      "漫画页",
      typeof metadata.page_number === "number" ? `第 ${metadata.page_number} 页` : null,
    ),
    field("角色名", metadata.character_name),
  ].filter((item) => item.value !== "无");

  if (context) {
    fields.push(
      field(
        "来源类型",
        enumLabel(context.prompt_type, PROMPT_SOURCE_LABELS, "其他生成来源"),
      ),
      field(
        "镜头",
        typeof context.shot_number === "number" ? `镜头 ${context.shot_number}` : null,
      ),
      field("来源标题", context.title),
      field("来源场景", context.scene_summary),
    );
  }
  return fields.filter((item) => item.value !== "无");
};

export const imagePromptSnapshotFields = (image: GeneratedImage): MetadataField[] => {
  const snapshot = asRecord(asRecord(image.metadata_json)?.prompt_snapshot);
  if (!snapshot) return [];
  const referenceImageCount = Array.isArray(snapshot.reference_image_ids)
    ? snapshot.reference_image_ids.length
    : null;
  return presentFields([
    field("视觉提示词", snapshot.visual_prompt),
    field("动态提示词", snapshot.motion_prompt),
    field("负面提示词", snapshot.negative_prompt),
    field(
      "时长",
      typeof snapshot.duration_secs === "number" ? `${snapshot.duration_secs} 秒` : null,
    ),
    field("画幅", snapshot.aspect_ratio, true),
    field("参考图", referenceImageCount == null ? null : `${referenceImageCount} 张`),
  ]);
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
