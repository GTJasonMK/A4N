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
import { Modal } from "../../../components/ui/Modal";
import { asJsonRecord as asRecord } from "../../../utils/jsonRecord";
import { toMediaSrc } from "../../../utils/mediaSrc";
import { statusClass } from "./videoGenerationDisplay";
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
  return normalized ? IMAGE_TYPE_LABELS[normalized] ?? normalized : "素材";
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
  const deleteBlockedReason = textLocked ? textLockedReason : undefined;

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
    : `当前时长 ${durationSecs}s 不在默认视频模型支持范围内`;
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
  const importFrameImageTitle =
    importFrameImageBlockedReason ??
    `导入本地图片作为第 ${prompt.shot_number} 段${label}输入`;
  const title = prompt.title || `${label} ${prompt.shot_number}`;
  const promptLanguage = promptOutputLanguageOfPrompt(prompt);
  const generatedResultText = buildFrameResultText(prompt, frame, label);

  return (
    <li className="overflow-hidden rounded border border-book-border/50 bg-book-bg-paper/50 transition-colors hover:border-book-primary/50 hover:bg-book-bg-tertiary/35">
      <button
        type="button"
        onClick={() => setDetailOpen(true)}
        className="block min-w-0 w-full p-2 text-left focus-visible:outline focus-visible:outline-1 focus-visible:outline-book-primary"
        title={
          textLocked
            ? textLockedReason
            : imageLocked
              ? `已有${label}图片；编辑、优化或删除时需要选择如何处理下游图片`
              : `查看或编辑${label}成图请求`
        }
        aria-label={`查看或编辑${label}成图请求`}
      >
        <div className="mb-1.5 flex min-w-0 flex-wrap items-center gap-1 text-xs">
          <span className="shrink-0 font-mono text-book-text-main">#{prompt.shot_number}</span>
          <span className="min-w-[5rem] flex-1 truncate font-medium text-book-text-main" title={title}>
            {title}
          </span>
          <span className="pyqt-pill shrink-0 !text-[10px]">{prompt.duration_secs}s</span>
          <span className="pyqt-pill shrink-0 !text-[10px]">
            {PROMPT_OUTPUT_LANGUAGE_LABELS[promptLanguage]}
          </span>
          <span className={`pyqt-pill shrink-0 !text-[10px] ${statusClass(prompt.status)}`}>
            {prompt.status}
          </span>
          {generatedImageCount > 0 && (
            <span
              className="pyqt-pill shrink-0 !text-[10px] !text-success"
              title={imageLockedReason}
            >
              已有图 {generatedImageCount}
            </span>
          )}
          {referenceState.error ? (
            <span
              className="pyqt-pill shrink-0 !border-error/50 !text-[10px] !text-error"
              title={referenceState.error}
            >
              参考异常
            </span>
          ) : referenceState.ids.length > 0 && (
            <span className="pyqt-pill shrink-0 !text-[10px]">
              参考 {referenceState.ids.length}
            </span>
          )}
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
      <div className="flex flex-wrap items-center justify-end gap-1 border-t border-book-border/35 px-2 py-1.5">
        <button
          type="button"
          onClick={() => void onGenerateFrameImages(frameDraft)}
          disabled={generateFrameImageDisabled}
          className="pyqt-btn-primary inline-flex !min-h-9 shrink-0 items-center gap-1 !px-2 text-[10px] sm:!min-h-[24px]"
          title={generateFrameImageTitle}
        >
          {generating ? <Loader2 size={11} className="animate-spin" /> : <ImagePlus size={11} />}
          生成{label}图
        </button>
        <button
          type="button"
          onClick={() => void onImportFrameImage(frameDraft)}
          disabled={importFrameImageDisabled}
          className="pyqt-btn-secondary inline-flex !min-h-9 shrink-0 items-center gap-1 !px-2 text-[10px] sm:!min-h-[24px]"
          title={importFrameImageTitle}
        >
          {importing ? <Loader2 size={11} className="animate-spin" /> : <Upload size={11} />}
          导入图
        </button>
        <button
          type="button"
          onClick={() => void onDelete(prompt)}
          disabled={textLocked || acting === `delete-prompt-${prompt.id}`}
          title={deleteBlockedReason ?? `删除${promptLabel}`}
          className="inline-flex min-h-9 shrink-0 items-center gap-1 rounded border border-error/35 bg-transparent px-2 text-[10px] font-semibold text-error transition-colors hover:bg-error/10 focus-visible:outline-2 focus-visible:outline-error disabled:cursor-not-allowed disabled:opacity-45 sm:min-h-[24px]"
        >
          {acting === `delete-prompt-${prompt.id}` ? (
            <Loader2 size={11} className="animate-spin" />
          ) : (
            <Trash2 size={11} />
          )}
          删除
        </button>
      </div>

      <Modal
        isOpen={detailOpen}
        onClose={closeDetail}
        title={`${label}成图请求 #${prompt.shot_number}`}
        subtitle={title}
        icon={<PanelTop size={15} />}
        maxWidthClassName="max-w-5xl"
        bodyClassName="min-h-0 flex-1 overflow-y-auto custom-scrollbar px-4 py-4"
      >
        <div className="mb-3 flex flex-wrap items-start justify-between gap-2">
          <div className="min-w-0 flex-1">
            <div className="flex min-w-0 flex-wrap items-center gap-1 text-xs">
              <span className="font-mono text-book-text-main">#{prompt.shot_number}</span>
              <span className="min-w-0 truncate font-medium text-book-text-main">{title}</span>
              <span className={`pyqt-pill !text-[10px] ${statusClass(prompt.status)}`}>
                {prompt.status}
              </span>
              <span className="pyqt-pill !text-[10px]">{durationSecs}s</span>
              {generatedImageCount > 0 && (
                <span className="pyqt-pill !text-[10px] !text-success">
                  已有图 {generatedImageCount}
                </span>
              )}
              {referenceState.error ? (
                <span
                  className="pyqt-pill !border-error/50 !text-[10px] !text-error"
                  title={referenceState.error}
                >
                  参考异常
                </span>
              ) : referenceState.ids.length > 0 && (
                <span className="pyqt-pill !text-[10px]">
                  参考 {referenceState.ids.length}
                </span>
              )}
            </div>
          </div>
          <div className="flex shrink-0 items-center gap-1">
            <BookButton
              variant="secondary"
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
                保存
              </span>
            </BookButton>
            <BookButton
              variant="secondary"
              size="sm"
              onClick={() => void onOptimize(prompt)}
              disabled={optimizing || optimizeBlockedReason != null}
              title={optimizeBlockedReason ?? `按用户要求优化这条${promptLabel}`}
              className="!min-h-9 !px-3 sm:!min-h-7"
            >
              <span className="inline-flex items-center gap-1">
                {optimizing ? <Loader2 size={12} className="animate-spin" /> : <Wand2 size={12} />}
                优化
              </span>
            </BookButton>
            <BookButton
              variant="secondary"
              size="sm"
              onClick={() => void onPreviewFrameImages(frameDraft)}
              title={`查看${label}成图模型请求`}
              className="!min-h-9 !px-3 sm:!min-h-7"
            >
              <span className="inline-flex items-center gap-1">
                <Eye size={12} />
                模型请求
              </span>
            </BookButton>
          </div>
        </div>
        {referenceState.error && (
          <div className="mb-3 rounded border border-error/40 bg-error/10 px-2 py-1.5 text-xs text-error">
            参考图数据异常：{referenceState.error}
          </div>
        )}
        <section className="a4n-page-surface mb-3 overflow-hidden">
          <div className="flex items-center justify-between gap-2 border-b border-book-border/40 px-3 py-2 text-xs">
            <span className="font-semibold text-book-text-main">所用参考图</span>
            <span className="font-mono text-[10px] text-book-text-muted">
              {referenceImageItems.length} 张
              {missingReferenceImageCount > 0 ? ` · 缺失 ${missingReferenceImageCount}` : ""}
            </span>
          </div>
          {referenceImageItems.length === 0 ? (
            <div className="px-3 py-3 text-xs text-book-text-muted">当前成图请求未绑定参考图。</div>
          ) : (
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
                        title={`查看参考图 #${id}`}
                      >
                        <div className="relative flex aspect-square items-center justify-center overflow-hidden rounded border border-book-border/50 bg-book-bg-tertiary text-book-text-muted">
                          {src ? (
                            <img
                              src={src}
                              alt={`参考图 ${index + 1}`}
                              className="h-full w-full object-cover transition-transform duration-200 group-hover:scale-[1.03]"
                            />
                          ) : (
                            "无图"
                          )}
                          <span className="absolute left-1 top-1 rounded bg-book-bg-paper/90 px-1 font-mono text-[9px] text-book-text-main">
                            {index + 1}
                          </span>
                        </div>
                        <div className="mt-1 truncate font-mono text-book-text-main">#{id}</div>
                        <div className="truncate text-book-text-muted">
                          {videoFrameImageRoleLabel(role, imageTypeLabel(image.image_type))}
                        </div>
                      </button>
                    ) : (
                      <div>
                        <div className="relative flex aspect-square items-center justify-center overflow-hidden rounded border border-error/45 bg-error/10 text-error">
                          缺失
                          <span className="absolute left-1 top-1 rounded bg-book-bg-paper/90 px-1 font-mono text-[9px] text-book-text-main">
                            {index + 1}
                          </span>
                        </div>
                        <div className="mt-1 truncate font-mono text-error">#{id}</div>
                        <div className="truncate text-book-text-muted">未找到图片记录</div>
                      </div>
                    )}
                  </li>
                );
              })}
            </ul>
          )}
        </section>
        {framePromptDirty && !textLocked && (
          <div className="mb-3 rounded border border-warning/40 bg-warning/10 px-2 py-1.5 text-xs text-warning">
            当前成图提示词有未保存编辑；预览请求会使用草稿，真实成图前需要先保存。
          </div>
        )}
        {framePromptValidationError && !framePromptDirty && !textLocked && (
          <div className="mb-3 rounded border border-warning/40 bg-warning/10 px-2 py-1.5 text-xs text-warning">
            {framePromptValidationError}
          </div>
        )}
        {durationValidationError && !textLocked && (
          <div className="mb-3 rounded border border-warning/40 bg-warning/10 px-2 py-1.5 text-xs text-warning">
            {durationValidationError}
          </div>
        )}
        {prompt.scene_summary && (
          <div className="a4n-stat-tile mb-3 p-2 text-xs text-book-text-sub">
            {prompt.scene_summary}
          </div>
        )}
        <section className="a4n-page-surface mb-3 overflow-hidden">
          <div className="border-b border-book-border/40 px-3 py-2 text-xs font-semibold text-book-text-main">
            {label}结构化来源
          </div>
          <pre className="max-h-[32vh] min-h-[10rem] overflow-auto whitespace-pre-wrap break-words px-3 py-2 font-mono text-[11px] leading-relaxed text-book-text-main custom-scrollbar">
            {generatedResultText}
          </pre>
        </section>
        <label className="block text-[11px] text-book-text-muted">
          成图提示词 frame_prompt（可编辑，来源：{promptLabel}）
          <textarea
            value={framePrompt}
            onChange={(event) => setFramePrompt(event.target.value)}
            disabled={textLocked}
            className="book-control mt-1 min-h-[220px] w-full resize-y rounded-sm p-2 text-xs leading-relaxed"
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
                  <option value={durationSecs}>当前 {durationSecs}s（不支持）</option>
                )}
                {durationLimits.supportedDurations.map((duration) => (
                  <option key={duration} value={duration}>
                    {duration}s
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
              className="book-control mt-1 min-h-[96px] w-full resize-y rounded-sm p-2 text-xs leading-relaxed"
            />
          </label>
        </div>
      </Modal>
    </li>
  );
};
