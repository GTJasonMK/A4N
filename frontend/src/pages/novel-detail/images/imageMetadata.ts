import type { GeneratedImage } from "../../../api/image";
import { asJsonRecord as asRecord } from "../../../utils/jsonRecord";

export interface MetadataField {
  label: string;
  value: string;
  mono?: boolean;
}

const formatImageTimestamp = (value: string | null | undefined): string => {
  if (!value) return "未知";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
};

const metadataValueText = (value: unknown): string => {
  if (value == null) return "无";
  if (typeof value === "string") return value.trim() || "无";
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  return JSON.stringify(value);
};

const field = (label: string, value: unknown, mono = false): MetadataField => ({
  label,
  value: metadataValueText(value),
  mono,
});

export const imageBaseMetadataFields = (image: GeneratedImage): MetadataField[] => [
  field("图片 ID", image.id, true),
  field("项目 ID", image.project_id, true),
  field("图片类型", image.image_type, true),
  field("章节 ID", image.chapter_id ?? "项目级", true),
  field("Panel ID", image.panel_id, true),
  field("模型", image.model, true),
  field("尺寸", `${image.width} x ${image.height}`, true),
  field("文件大小", image.file_size == null ? "未知" : `${image.file_size} bytes`, true),
  field("AI 评分", image.ai_score == null ? "未评分" : image.ai_score.toFixed(3), true),
  field("收藏", image.is_favorite ? "是" : "否"),
  field("创建时间", formatImageTimestamp(image.created_at)),
  field("文件路径", image.file_path, true),
];

export const imageGenerationMetadataFields = (image: GeneratedImage): MetadataField[] => {
  const generation = asRecord(asRecord(image.metadata_json)?.generation);
  if (!generation) return [];
  return [
    field("生成命令", generation.command, true),
    field("配置 ID", generation.config_id, true),
    field("配置名称", generation.config_name),
    field("后端", generation.backend, true),
    field("Base URL", generation.base_url, true),
    field("配置模型", generation.configured_model, true),
    field("实际模型", generation.provider_model, true),
    field("图片类型", generation.image_type, true),
    field("章节 ID", generation.chapter_id, true),
    field("Panel ID", generation.panel_id, true),
    field("尺寸", `${metadataValueText(generation.width)} x ${metadataValueText(generation.height)}`, true),
    field("生成时间", generation.generated_at),
  ];
};

export const imageSourceMetadataFields = (image: GeneratedImage): MetadataField[] => {
  const source = asRecord(asRecord(image.metadata_json)?.source);
  if (!source) return [];
  return [
    field("正文指纹", source.content_sha256, true),
    field("来源长度", source.content_len, true),
    field("摘要存在", source.summary_present),
    field("分析存在", source.analysis_present),
  ];
};

export const imageBusinessMetadataFields = (image: GeneratedImage): MetadataField[] => {
  const metadata = asRecord(image.metadata_json);
  if (!metadata) return [];
  const context = asRecord(metadata.generation_context);
  const fields: MetadataField[] = [
    field("Schema", metadata.metadata_schema_version, true),
    field("视频素材", metadata.chapter_video_material),
    field("视频素材库", metadata.video_library_member),
    field("视频素材角色", metadata.video_asset_role, true),
    field("漫画页", metadata.page_number, true),
    field("漫画提示词 ID", metadata.page_prompt_id, true),
    field("角色名", metadata.character_name),
    field("肖像 ID", metadata.portrait_id, true),
    field("角色档案 ID", metadata.profile_id, true),
  ].filter((item) => item.value !== "无");

  if (context) {
    fields.push(
      field("业务来源", context.source, true),
      field("界面来源", context.surface, true),
      field("来源提示词 ID", context.prompt_id, true),
      field("来源提示词类型", context.prompt_type, true),
      field("来源编号", context.shot_number, true),
      field("来源标题", context.title),
      field("来源场景", context.scene_summary),
      field("来源正文指纹", context.source_content_sha256, true),
    );
  }
  return fields.filter((item) => item.value !== "无");
};

export const imagePromptSnapshotFields = (image: GeneratedImage): MetadataField[] => {
  const snapshot = asRecord(asRecord(image.metadata_json)?.prompt_snapshot);
  if (!snapshot) return [];
  return [
    field("Visual Prompt", snapshot.visual_prompt),
    field("Motion Prompt", snapshot.motion_prompt),
    field("Negative Prompt", snapshot.negative_prompt),
    field("时长", snapshot.duration_secs, true),
    field("画幅", snapshot.aspect_ratio, true),
    field("参考图 ID", snapshot.reference_image_ids, true),
  ];
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
