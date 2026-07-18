import React, {
  useEffect,
  useState,
} from "react";
import {
  Eye,
  ImagePlus,
  Loader2,
  PanelTop,
  Save,
  Trash2,
  Upload,
  Wand2,
} from "lucide-react";
import type { GeneratedImage } from "../../../api/image";
import type { ChapterVideoPrompt } from "../../../api/video";
import { BookButton } from "../../../components/ui/BookButton";
import { Dropdown } from "../../../components/ui/Dropdown";
import { Modal } from "../../../components/ui/Modal";
import { asJsonRecord as asRecord } from "../../../utils/jsonRecord";
import { toMediaSrc } from "../../../utils/mediaSrc";
import { getOperationStatusLabel } from "../../../utils/workflowPresentation";
import {
  isDurationAllowed,
  parseNumberInput,
  PROMPT_OUTPUT_LANGUAGE_LABELS,
  promptOutputLanguageOfPrompt,
  promptLengthError,
  readSavedReferenceImageIds,
  type VideoDurationLimits,
  VIDEO_FRAME_IMAGE_PROMPT_MIN_CHARS,
  videoFrameImageRoleLabel,
  videoFrameImageRoleOfImage,
} from "./videoTabUtils";

export type PromptUpdateInput = {
  visual_prompt?: string | null;
  motion_prompt?: string | null;
  negative_prompt?: string | null;
  duration_secs?: number | null;
  reference_image_ids_json?: unknown | null;
};

const stringValue = (value: unknown): string | null =>
  typeof value === "string" && value.trim() ? value.trim() : null;

const numberValue = (value: unknown): number | null =>
  typeof value === "number" && Number.isFinite(value) ? value : null;

interface IntegerArrayState {
  ids: number[];
  error: string | null;
  present: boolean;
}

const strictPositiveIntegerArray = (value: unknown, field: string): IntegerArrayState => {
  if (value == null) return { ids: [], error: null, present: false };
  if (!Array.isArray(value)) {
    return { ids: [], error: `${field} 必须是正整数数组`, present: true };
  }

  const ids: number[] = [];
  const seen = new Set<number>();
  for (const [index, item] of value.entries()) {
    if (typeof item !== "number" || !Number.isInteger(item) || item <= 0) {
      return { ids: [], error: `${field}[${index}] 必须是正整数`, present: true };
    }
    if (seen.has(item)) {
      return { ids: [], error: `${field} 包含重复 ID：${item}`, present: true };
    }
    seen.add(item);
    ids.push(item);
  }
  return { ids, error: null, present: true };
};

const prettyJson = (value: unknown): string => {
  if (value == null) return "";
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
};

const fieldLine = (label: string, value: string | number | number[] | null): string | null => {
  if (value == null) return null;
  const text = Array.isArray(value) ? value.join(", ") : String(value);
  return text.trim() ? `${label}: ${text}` : null;
};

const FRAME_STATE_LABELS: Record<string, string> = {
  character: "人物",
  characters: "人物",
  position: "位置",
  pose: "姿态",
  action: "动作",
  direction: "朝向",
  expression: "表情",
  lighting: "光线",
  key_light: "主光",
  fill_light: "辅光",
  color: "色彩",
  mood: "氛围",
};

const readableFrameState = (value: unknown, depth = 0): string => {
  if (value == null) return "";
  if (typeof value === "string") return value.trim();
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  if (Array.isArray(value)) {
    return value
      .map((item) => readableFrameState(item, depth + 1))
      .filter(Boolean)
      .join("；");
  }
  const record = asRecord(value);
  if (!record) return String(value);
  return Object.entries(record)
    .slice(0, depth > 0 ? 5 : 8)
    .map(([key, item]) => {
      const text = readableFrameState(item, depth + 1);
      return text ? `${FRAME_STATE_LABELS[key] ?? "信息"}：${text}` : "";
    })
    .filter(Boolean)
    .join("；");
};

const IMAGE_TYPE_LABELS: Record<string, string> = {
  portrait: "立绘",
  scene: "场景",
  manga_page: "整页漫画",
  cover: "封面",
  illustration: "插图",
  video_key_frame: "边界帧",
  video_start_frame: "首帧",
  video_storyboard_frame: "分镜板",
  video_tail_frame: "真实尾帧",
  scene_material: "场景素材",
  object_material: "物品素材",
};

const imageTypeLabel = (type: string | null | undefined): string => {
  const normalized = type?.trim();
  return normalized ? IMAGE_TYPE_LABELS[normalized] ?? "其他素材" : "素材";
};

const resolveFrameReferenceState = (
  prompt: ChapterVideoPrompt,
  frame: Record<string, unknown> | null,
): IntegerArrayState => {
  const frameReferenceState = strictPositiveIntegerArray(
    frame?.reference_image_ids,
    "frame.reference_image_ids",
  );
  const savedReferenceState = readSavedReferenceImageIds(prompt.reference_image_ids_json);
  return {
    ids: frameReferenceState.present
      ? frameReferenceState.ids
      : savedReferenceState.ids,
    error: frameReferenceState.error ?? savedReferenceState.error,
    present: frameReferenceState.present || savedReferenceState.ids.length > 0,
  };
};

const buildFrameResultText = (
  prompt: ChapterVideoPrompt,
  frame: Record<string, unknown> | null,
  label: string,
): string => {
  const metadata = asRecord(prompt.metadata_json);
  const blockingState = frame?.blocking_state ?? metadata?.blocking_state;
  const lightingState = frame?.lighting_state ?? metadata?.lighting_state;
  const segmentNumberState = strictPositiveIntegerArray(
    frame?.segment_numbers,
    "frame.segment_numbers",
  );
  const referenceState = resolveFrameReferenceState(prompt, frame);
  const sections = [
    fieldLine("frame_number", numberValue(frame?.frame_number) ?? prompt.shot_number),
    fieldLine("title", stringValue(frame?.title) ?? prompt.title),
    fieldLine("duration_secs", numberValue(frame?.duration_secs) ?? prompt.duration_secs),
    fieldLine("aspect_ratio", stringValue(frame?.aspect_ratio) ?? prompt.aspect_ratio),
    fieldLine("reference_image_ids", referenceState.ids),
    fieldLine("reference_image_ids_error", referenceState.error),
    fieldLine("segment_numbers", segmentNumberState.ids),
    fieldLine("segment_numbers_error", segmentNumberState.error),
    "",
    "scene_summary:",
    stringValue(frame?.scene_summary) ?? prompt.scene_summary ?? "",
    "",
    "frame_prompt:",
    stringValue(frame?.frame_prompt) ?? prompt.visual_prompt,
    "",
    "negative_prompt:",
    stringValue(frame?.negative_prompt) ?? prompt.negative_prompt ?? "",
    "",
    "blocking_state:",
    prettyJson(blockingState),
    "",
    "lighting_state:",
    prettyJson(lightingState),
    "",
    `raw_${label}:`,
    prettyJson(
      frame ??
        metadata?.boundary_frame ??
        metadata?.first_frame ??
        metadata?.storyboard_frame ??
        prompt.metadata_json,
    ),
  ];
  return sections.filter((section) => section != null).join("\n");
};

export const VideoFramePromptCard: React.FC<{
  prompt: ChapterVideoPrompt;
  label?: string;
  promptLabel?: string;
  generatedImageCount: number;
  referenceImageById: Map<number, GeneratedImage>;
  locked?: boolean;
  lockedReason?: string;
  acting: string | null;
  durationLimits: VideoDurationLimits;
  onPreviewFrameImages: (prompt: ChapterVideoPrompt) => void | Promise<void>;
  onUpdate: (prompt: ChapterVideoPrompt, input: PromptUpdateInput) => void | Promise<void>;
  onOptimize: (prompt: ChapterVideoPrompt) => void | Promise<void>;
  onDelete: (prompt: ChapterVideoPrompt) => void | Promise<void>;
  onGenerateFrameImages: (prompt: ChapterVideoPrompt) => void | Promise<void>;
  onImportFrameImage: (prompt: ChapterVideoPrompt) => void | Promise<void>;
  onOpenImageDetails: (image: GeneratedImage) => void;
}> = ({
  prompt,
  label = "边界帧",
  promptLabel = "边界帧提示词",
  generatedImageCount,
  referenceImageById,
  locked = false,
  lockedReason,
  acting,
  durationLimits,
  onPreviewFrameImages,
  onUpdate,
  onOptimize,
  onDelete,
  onGenerateFrameImages,
  onImportFrameImage,
  onOpenImageDetails,
}) => {
  const [detailOpen, setDetailOpen] = useState(false);
  const [framePrompt, setFramePrompt] = useState(prompt.visual_prompt);
  const [negativePrompt, setNegativePrompt] = useState(prompt.negative_prompt ?? "");
  const [durationSecs, setDurationSecs] = useState(prompt.duration_secs);
  const metadata = asRecord(prompt.metadata_json);
  const frame =
    asRecord(metadata?.boundary_frame) ??
    asRecord(metadata?.first_frame) ??
    asRecord(metadata?.storyboard_frame);
  const referenceState = resolveFrameReferenceState(prompt, frame);
  const referenceImageItems = referenceState.ids.map((id) => ({
    id,
    image: referenceImageById.get(id) ?? null,
  }));
  const missingReferenceImageCount = referenceImageItems.filter((item) => item.image == null).length;
  const saving = acting === `save-prompt-row-${prompt.id}`;
  const optimizing = acting === `optimize-prompt-${prompt.id}`;
  const generating = acting === `generate-frame-images-${prompt.id}`;
  const importing = acting === `import-frame-image-${prompt.id}`;
  const runningLocked = prompt.status === "running";
  const runningLockedReason = runningLocked
    ? `该${promptLabel}正在运行，不能编辑、删除或成图`
    : undefined;
  const imageLocked = generatedImageCount > 0;
  const imageLockedReason = imageLocked
    ? `该${promptLabel}已有 ${generatedImageCount} 张${label}图片，重新导入/成图前需先处理已有图片；编辑、优化或删除提示词时会让你选择如何处理下游图片`
    : undefined;
  const textLocked = locked || runningLocked;
  const textLockedReason = locked
    ? lockedReason
    : runningLockedReason;
  const frameImageLocked = imageLocked || runningLocked;
  const frameImageLockedReason = imageLocked ? imageLockedReason : runningLockedReason;

  useEffect(() => {
    setFramePrompt(prompt.visual_prompt);
    setNegativePrompt(prompt.negative_prompt ?? "");
    setDurationSecs(prompt.duration_secs);
  }, [prompt.duration_secs, prompt.negative_prompt, prompt.visual_prompt]);

  const resetDraftFromPrompt = () => {
    setFramePrompt(prompt.visual_prompt);
    setNegativePrompt(prompt.negative_prompt ?? "");
    setDurationSecs(prompt.duration_secs);
  };

  const closeDetail = () => {
    resetDraftFromPrompt();
    setDetailOpen(false);
  };

  const frameDraft: ChapterVideoPrompt = {
    ...prompt,
    visual_prompt: framePrompt.trim(),
    motion_prompt: null,
    negative_prompt: negativePrompt.trim() || null,
    duration_secs: durationSecs,
  };
  const trimmedFramePrompt = framePrompt.trim();
  const trimmedNegativePrompt = negativePrompt.trim();
  const framePromptValidationError = promptLengthError(
    promptLabel,
    trimmedFramePrompt,
    VIDEO_FRAME_IMAGE_PROMPT_MIN_CHARS,
  );
  const durationValidationError = isDurationAllowed(durationSecs, durationLimits)
    ? null
    : `当前时长 ${durationSecs} 秒不在默认视频模型支持范围内`;
  const framePromptDirty =
    trimmedFramePrompt !== prompt.visual_prompt.trim() ||
    trimmedNegativePrompt !== (prompt.negative_prompt?.trim() ?? "") ||
    durationSecs !== prompt.duration_secs;
  const optimizeBlockedReason = framePromptDirty
    ? "当前提示词有未保存编辑，请先保存或关闭后再优化"
    : textLocked
      ? textLockedReason
      : undefined;
  const generateFrameImageDisabled =
    framePromptValidationError != null ||
    durationValidationError != null ||
    framePromptDirty ||
    generating ||
    frameImageLocked;
  const generateFrameImageBlockedReason = framePromptDirty
    ? "当前成图提示词有未保存编辑，请先保存后再成图"
    : framePromptValidationError ??
      durationValidationError ??
      (frameImageLocked ? frameImageLockedReason : undefined);
  const generateFrameImageTitle =
    generateFrameImageBlockedReason ??
    `生成第 ${prompt.shot_number} 段${label}图片，用作后续视频生成的${label}输入`;
  const importFrameImageDisabled =
    framePromptDirty ||
    saving ||
    generating ||
    importing ||
    locked ||
    imageLocked ||
    runningLocked;
  const importFrameImageBlockedReason = saving
    ? "正在保存当前提示词，保存完成后再导入图片"
    : generating
      ? `正在生成${label}图片，不能同时导入`
      : framePromptDirty
        ? "当前成图提示词有未保存编辑，请先保存后再导入图片"
        : locked
          ? lockedReason
          : imageLocked
            ? imageLockedReason
            : runningLockedReason;
  const title = prompt.title || `${label} ${prompt.shot_number}`;
  const promptLanguage = promptOutputLanguageOfPrompt(prompt);
  const generatedResultText = buildFrameResultText(prompt, frame, label);
  const segmentNumberState = strictPositiveIntegerArray(
    frame?.segment_numbers,
    "frame.segment_numbers",
  );
  const sourceRows = [
    {
      label: "时长与画幅",
      value: `${numberValue(frame?.duration_secs) ?? prompt.duration_secs} 秒 · ${
        stringValue(frame?.aspect_ratio) ?? prompt.aspect_ratio ?? "未设置画幅"
      }`,
    },
    {
      label: "关联片段",
      value:
        segmentNumberState.ids.length > 0
          ? segmentNumberState.ids.join("、")
          : "未单独标注",
    },
    {
      label: "参考图",
      value: referenceState.ids.length > 0 ? `${referenceState.ids.length} 张` : "未使用",
    },
    {
      label: "人物与走位",
      value: readableFrameState(frame?.blocking_state ?? metadata?.blocking_state),
    },
    {
      label: "光线与氛围",
      value: readableFrameState(frame?.lighting_state ?? metadata?.lighting_state),
    },
  ].filter((item) => item.value);
  const showSavePrimary = saving || framePromptDirty;
  const showGeneratePrimary =
    !showSavePrimary &&
    (generating || generateFrameImageBlockedReason == null);
  const showImportAction = importing || importFrameImageBlockedReason == null;
  const showDeleteAction = !textLocked && acting !== `delete-prompt-${prompt.id}`;

  return (
    <li className="group flex min-w-0 items-start transition-colors hover:bg-book-bg-tertiary/45">
      <button
        type="button"
        onClick={() => setDetailOpen(true)}
        className="min-w-0 flex-1 px-3 py-3 text-left focus-visible:outline focus-visible:outline-2 focus-visible:outline-book-primary"
        title={
          textLocked
            ? textLockedReason
            : imageLocked
              ? `已有${label}图片；编辑、优化或删除时需要选择如何处理下游图片`
              : `查看或编辑${label}成图请求`
        }
        aria-label={`查看或编辑${label}成图请求`}
      >
        <div className="flex min-w-0 items-baseline gap-1.5">
          <span className="shrink-0 text-xs text-book-text-main">镜头 {prompt.shot_number}</span>
          <span className="min-w-0 flex-1 truncate text-xs font-medium text-book-text-main" title={title}>
            · {title}
          </span>
        </div>
        <div
          className={`mt-1 line-clamp-1 text-[10px] ${
            referenceState.error ? "text-error" : "text-book-text-muted"
          }`}
          title={referenceState.error ?? undefined}
        >
          {[
            `${prompt.duration_secs} 秒`,
            PROMPT_OUTPUT_LANGUAGE_LABELS[promptLanguage],
            getOperationStatusLabel(prompt.status),
            generatedImageCount > 0 ? `已有 ${generatedImageCount} 张图片` : null,
            referenceState.ids.length > 0 ? `${referenceState.ids.length} 张参考图` : null,
            referenceState.error ? "参考图信息异常" : null,
          ].filter((value): value is string => Boolean(value)).join(" · ")}
        </div>

        {prompt.scene_summary && (
          <div className="mb-1 line-clamp-1 text-[11px] text-book-text-sub">
            {prompt.scene_summary}
          </div>
        )}
        <div className="line-clamp-2 text-[11px] leading-relaxed text-book-text-sub">
          {prompt.visual_prompt || "暂无提示词"}
        </div>
      </button>
      <div className="flex shrink-0 items-start gap-1.5 pr-2 pt-2.5">
        {showGeneratePrimary && (
          <button
            type="button"
            onClick={() => void onGenerateFrameImages(frameDraft)}
            disabled={generateFrameImageDisabled}
            className="pyqt-btn-primary inline-flex !min-h-9 shrink-0 items-center gap-1 !px-2 text-[10px] sm:!min-h-7"
            title={generateFrameImageTitle}
          >
            {generating ? <Loader2 size={11} className="animate-spin" /> : <ImagePlus size={11} />}
            {generating ? "生成中" : "生成图"}
          </button>
        )}
        {(showImportAction || showDeleteAction) && (
          <Dropdown
            items={[
              ...(showImportAction
                ? [{
                    label: importing ? `正在导入${label}图` : `导入${label}图`,
                    icon: importing
                      ? <Loader2 size={13} className="animate-spin" />
                      : <Upload size={13} />,
                    disabled: importFrameImageDisabled,
                    onClick: () => void onImportFrameImage(frameDraft),
                  }]
                : []),
              ...(showDeleteAction
                ? [
                    ...(showImportAction ? [{ type: "divider" as const }] : []),
                    {
                      label: `删除${promptLabel}`,
                      danger: true,
                      icon: <Trash2 size={13} />,
                      onClick: () => void onDelete(prompt),
                    },
                  ]
                : []),
            ]}
          />
        )}
      </div>

      <Modal
        isOpen={detailOpen}
        onClose={closeDetail}
        title={`镜头 ${prompt.shot_number} · ${label}成图请求`}
        subtitle={title}
        icon={<PanelTop size={15} />}
        maxWidthClassName="max-w-5xl"
        bodyClassName="min-h-0 flex-1 overflow-y-auto custom-scrollbar px-4 py-4"
      >
        <div className="mb-3 flex min-w-0 flex-wrap items-center gap-2 border-b border-book-border/45 pb-3">
          <div className="min-w-0 flex-1 text-[11px] text-book-text-muted">
            {[
              `${durationSecs} 秒`,
              getOperationStatusLabel(prompt.status),
              generatedImageCount > 0 ? `已有 ${generatedImageCount} 张图片` : null,
              referenceState.ids.length > 0 ? `${referenceState.ids.length} 张参考图` : null,
              framePromptDirty ? "有未保存更改" : null,
            ].filter((value): value is string => Boolean(value)).join(" · ")}
          </div>
          <div className="flex shrink-0 items-center gap-2">
            {showSavePrimary && (
              <BookButton
                variant="primary"
                size="sm"
                onClick={() => void onUpdate(prompt, {
                  visual_prompt: trimmedFramePrompt,
                  motion_prompt: null,
                  negative_prompt: trimmedNegativePrompt || null,
                  duration_secs: durationSecs,
                })}
                disabled={
                  !trimmedFramePrompt ||
                  durationValidationError != null ||
                  saving ||
                  textLocked
                }
                title={textLocked ? textLockedReason : durationValidationError ?? undefined}
                className="!min-h-9 !px-3 sm:!min-h-7"
              >
                <span className="inline-flex items-center gap-1">
                  {saving ? <Loader2 size={12} className="animate-spin" /> : <Save size={12} />}
                  {saving ? "保存中" : "保存更改"}
                </span>
              </BookButton>
            )}
            {showGeneratePrimary && (
              <BookButton
                variant="primary"
                size="sm"
                onClick={() => void onGenerateFrameImages(frameDraft)}
                disabled={generateFrameImageDisabled}
                title={generateFrameImageTitle}
                className="!min-h-9 !px-3 sm:!min-h-7"
              >
                <span className="inline-flex items-center gap-1">
                  {generating ? <Loader2 size={12} className="animate-spin" /> : <ImagePlus size={12} />}
                  {generating ? "生成中" : `生成${label}图`}
                </span>
              </BookButton>
            )}
            <Dropdown
              items={[
                {
                  label: optimizing ? `正在优化${promptLabel}` : `优化${promptLabel}`,
                  icon: optimizing
                    ? <Loader2 size={13} className="animate-spin" />
                    : <Wand2 size={13} />,
                  disabled: optimizing || optimizeBlockedReason != null,
                  onClick: () => void onOptimize(prompt),
                },
                {
                  label: `查看${label}成图模型请求`,
                  icon: <Eye size={13} />,
                  onClick: () => void onPreviewFrameImages(frameDraft),
                },
                ...(showImportAction
                  ? [{
                      label: importing ? `正在导入${label}图` : `导入${label}图`,
                      icon: importing
                        ? <Loader2 size={13} className="animate-spin" />
                        : <Upload size={13} />,
                      disabled: importFrameImageDisabled,
                      onClick: () => void onImportFrameImage(frameDraft),
                    }]
                  : []),
                ...(showDeleteAction
                  ? [
                      { type: "divider" as const },
                      {
                        label: `删除${promptLabel}`,
                        icon: <Trash2 size={13} />,
                        danger: true,
                        onClick: () => void onDelete(prompt),
                      },
                    ]
                  : []),
              ]}
            />
          </div>
        </div>
        {runningLocked ? (
          <div className="a4n-flat-surface flex min-h-40 flex-col items-center justify-center gap-2 px-4 py-8 text-center">
            <Loader2 size={18} className="animate-spin text-book-primary" />
            <div className="text-sm font-medium text-book-text-main">
              当前{promptLabel}正在运行
            </div>
            <div className="max-w-md text-[11px] leading-relaxed text-book-text-muted">
              运行结束后再继续编辑或生成图片；当前阶段的输入和辅助操作已暂时收起。
            </div>
          </div>
        ) : (
          <>
        {referenceState.error && (
          <div role="alert" className="mb-3 rounded-lg border border-error/35 bg-error-bg px-2.5 py-2 text-xs text-error">
            参考图数据异常：{referenceState.error}
          </div>
        )}
        {referenceImageItems.length > 0 ? (
          <section className="a4n-flat-surface mb-3 overflow-hidden">
            <div className="flex min-w-0 items-center gap-2 border-b border-book-border/40 px-3 py-2.5">
              <div className="min-w-0 flex-1">
                <div className="text-xs font-medium text-book-text-main">所用参考图</div>
                <div className="mt-0.5 text-[10px] text-book-text-muted">
                  {referenceImageItems.length} 张已绑定
                  {missingReferenceImageCount > 0 ? ` · ${missingReferenceImageCount} 张缺失` : ""}
                </div>
              </div>
            </div>
            <ul className="grid grid-cols-2 gap-2 p-3 sm:grid-cols-3 md:grid-cols-4 xl:grid-cols-6">
              {referenceImageItems.map(({ id, image }, index) => {
                const src = image ? toMediaSrc(image.file_path) : "";
                const role = image ? videoFrameImageRoleOfImage(image) : null;
                return (
                  <li key={`${id}-${index}`} className="min-w-0 text-[10px]">
                    {image ? (
                      <button
                        type="button"
                        onClick={() => onOpenImageDetails(image)}
                        className="group block w-full text-left focus-visible:outline focus-visible:outline-1 focus-visible:outline-book-primary"
                        title={`查看参考图 ${index + 1}`}
                      >
                        <div className="relative flex aspect-square items-center justify-center overflow-hidden rounded-xl border border-book-border/50 bg-book-bg-tertiary text-book-text-muted">
                          {src ? (
                            <img
                              src={src}
                              alt={`参考图 ${index + 1}`}
                              className="h-full w-full object-cover transition-transform duration-200 group-hover:scale-[1.03]"
                            />
                          ) : (
                            "无图"
                          )}
                          <span className="absolute left-1 top-1 inline-flex h-5 min-w-5 items-center justify-center rounded-full bg-book-bg-paper/90 px-1 text-[10px] text-book-text-main">
                            {index + 1}
                          </span>
                        </div>
                        <div className="truncate text-book-text-muted">
                          {videoFrameImageRoleLabel(role, imageTypeLabel(image.image_type))}
                        </div>
                      </button>
                    ) : (
                      <div>
                        <div className="relative flex aspect-square items-center justify-center overflow-hidden rounded-xl border border-error/45 bg-error-bg text-error">
                          缺失
                          <span className="absolute left-1 top-1 inline-flex h-5 min-w-5 items-center justify-center rounded-full bg-book-bg-paper/90 px-1 text-[10px] text-book-text-main">
                            {index + 1}
                          </span>
                        </div>
                        <div className="truncate text-book-text-muted">未找到图片记录</div>
                      </div>
                    )}
                  </li>
                );
              })}
            </ul>
          </section>
        ) : null}
        {framePromptDirty && !textLocked && (
          <div role="status" className="mb-3 rounded-lg border border-warning/35 bg-warning-bg px-2.5 py-2 text-xs text-warning">
            当前成图提示词有未保存编辑；模型请求预览会使用草稿，真实成图前需要先保存。
          </div>
        )}
        {framePromptValidationError && !framePromptDirty && !textLocked && (
          <div role="status" className="mb-3 rounded-lg border border-warning/35 bg-warning-bg px-2.5 py-2 text-xs text-warning">
            {framePromptValidationError}
          </div>
        )}
        {durationValidationError && !textLocked && (
          <div role="status" className="mb-3 rounded-lg border border-warning/35 bg-warning-bg px-2.5 py-2 text-xs text-warning">
            {durationValidationError}
          </div>
        )}
        {prompt.scene_summary && (
          <div className="mb-3 border-l-2 border-book-border/70 pl-2 text-xs leading-relaxed text-book-text-sub">
            {prompt.scene_summary}
          </div>
        )}
        <section className="a4n-flat-surface mb-3 overflow-hidden">
          <div className="border-b border-book-border/40 px-3 py-2 text-xs font-semibold text-book-text-main">
            生成依据
          </div>
          <dl className="divide-y divide-book-border/35">
            {sourceRows.map((item) => (
              <div key={item.label} className="grid grid-cols-[88px_minmax(0,1fr)] gap-3 px-3 py-2 text-[11px]">
                <dt className="text-book-text-muted">{item.label}</dt>
                <dd className="min-w-0 break-words leading-relaxed text-book-text-sub">
                  {item.value}
                </dd>
              </div>
            ))}
          </dl>
        </section>
        <details className="mb-3 text-[10px] text-book-text-muted">
          <summary className="w-fit cursor-pointer hover:text-book-text-main">诊断信息</summary>
          <pre className="mt-1 whitespace-pre-wrap break-words rounded-xl bg-book-bg-tertiary/45 p-2 font-mono text-[10px] leading-relaxed text-book-text-sub">
            {generatedResultText}
          </pre>
        </details>
        <label className="block text-[11px] text-book-text-muted">
          成图提示词（可编辑，来源：{promptLabel}）
          <textarea
            value={framePrompt}
            onChange={(event) => setFramePrompt(event.target.value)}
            disabled={textLocked}
            className="book-control mt-1 min-h-[220px] w-full resize-y p-2 text-xs leading-relaxed"
          />
        </label>
        <div className="mt-3 grid gap-3 lg:grid-cols-[120px_minmax(0,1fr)]">
          <label className="block text-[11px] text-book-text-muted">
            关联时长
            {durationLimits.durationMode === "fixed" &&
            durationLimits.supportedDurations.length > 0 ? (
              <select
                value={durationSecs}
                onChange={(event) => {
                  const value = parseNumberInput(event.target.value);
                  if (
                    value !== null &&
                    durationLimits.supportedDurations.includes(value)
                  ) {
                    setDurationSecs(value);
                  }
                }}
                disabled={textLocked}
                className="pyqt-input mt-1 w-full !px-2 !py-1.5"
              >
                {durationValidationError && (
                  <option value={durationSecs}>当前 {durationSecs} 秒（不支持）</option>
                )}
                {durationLimits.supportedDurations.map((duration) => (
                  <option key={duration} value={duration}>
                    {duration} 秒
                  </option>
                ))}
              </select>
            ) : (
              <input
                type="number"
                min={durationLimits.minDurationSecs}
                max={durationLimits.maxDurationSecs}
                value={durationSecs}
                onChange={(event) => {
                  const value = parseNumberInput(event.target.value);
                  if (value !== null) setDurationSecs(value);
                }}
                disabled={textLocked}
                className="pyqt-input mt-1 w-full !px-2 !py-1.5"
              />
            )}
          </label>
          <label className="block text-[11px] text-book-text-muted">
            负面提示词
            <textarea
              value={negativePrompt}
              onChange={(event) => setNegativePrompt(event.target.value)}
              disabled={textLocked}
              className="book-control mt-1 min-h-[96px] w-full resize-y p-2 text-xs leading-relaxed"
            />
          </label>
        </div>
          </>
        )}
      </Modal>
    </li>
  );
};
