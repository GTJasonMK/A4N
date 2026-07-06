import React, { useEffect, useMemo, useRef, useState } from "react";
import {
  ClipboardList,
  Eye,
  ListChecks,
  Loader2,
  RotateCcw,
  Trash2,
  Wand2,
} from "lucide-react";
import type { GeneratedImage } from "../../../api/image";
import type { ChapterVideoPrompt, VideoWorkflowMode } from "../../../api/video";
import { BookButton } from "../../../components/ui/BookButton";
import { BookCard } from "../../../components/ui/BookCard";
import { asArray, asJsonRecord as asRecord } from "../../../utils/jsonRecord";
import { StyleTemplatePicker } from "../StyleTemplatePicker";
import { EmptyHint } from "../shared";
import { VIDEO_STYLE_TEMPLATES } from "../styleTemplates";
import {
  VIDEO_ASPECT_RATIOS,
  PROMPT_OUTPUT_LANGUAGES,
  PROMPT_OUTPUT_LANGUAGE_LABELS,
  durationLimitsLabel,
  expectedFrameCountFromSegmentPlanPrompt,
  isVideoPromptFrameImage,
  normalizeDurationOptions,
  parseNumberInput,
  videoImageGenerationPromptId,
  videoTailFrameBoundaryNumber,
  type VideoDurationLimits,
  type VideoGenOptions,
} from "./videoTabUtils";
import { VideoFrameImageGallery } from "./VideoFrameImageGallery";
import {
  VideoFramePromptCard,
  type PromptUpdateInput,
} from "./VideoFramePromptCard";
import { VideoDurationOptionsPicker } from "./VideoDurationOptionsPicker";

interface VideoFrameWorkspaceProps {
  prompts: ChapterVideoPrompt[];
  generationPrompts: ChapterVideoPrompt[];
  workflowMode: VideoWorkflowMode;
  segmentPlanPrompt: ChapterVideoPrompt | null;
  videoPromptCount: number;
  totalFramePromptRecordCount: number;
  availableImages: GeneratedImage[];
  referenceLookupImages: GeneratedImage[];
  frameImages: GeneratedImage[];
  options: VideoGenOptions;
  durationLimits: VideoDurationLimits;
  promptInputSelectionError: string | null;
  acting: string | null;
  chapterGenerating: boolean;
  previewingPromptRequests: boolean;
  onOptionsChange: React.Dispatch<React.SetStateAction<VideoGenOptions>>;
  onGenerateSegments: (options?: { regenerateExisting?: boolean }) => void | Promise<void>;
  onDeleteSegmentPlan: (prompt: ChapterVideoPrompt) => void | Promise<void>;
  onGeneratePrompts: (options?: {
    regenerateExisting?: boolean;
    runUntilComplete?: boolean;
  }) => void | Promise<void>;
  onPreviewPromptRequests: () => void | Promise<void>;
  onDeleteAllFramePrompts: () => void | Promise<void>;
  onPreviewFrameImages: (prompt: ChapterVideoPrompt) => void | Promise<void>;
  onUpdatePrompt: (prompt: ChapterVideoPrompt, input: PromptUpdateInput) => void | Promise<void>;
  onOptimizePrompt: (prompt: ChapterVideoPrompt) => void | Promise<void>;
  onDeletePrompt: (prompt: ChapterVideoPrompt) => void | Promise<void>;
  onGenerateFrameImages: (prompt: ChapterVideoPrompt) => void | Promise<void>;
  onImportFrameImage: (prompt: ChapterVideoPrompt) => void | Promise<void>;
  onToggleFavorite: (image: GeneratedImage) => void | Promise<void>;
  onOpenImageDetails: (image: GeneratedImage) => void;
}

type FramePanelTab = "generate" | "segments" | "prompts";

const textValue = (value: unknown): string | null =>
  typeof value === "string" && value.trim() ? value.trim() : null;

const numberValue = (value: unknown): number | null =>
  typeof value === "number" && Number.isFinite(value) ? value : null;

const compactLabel = (value: unknown): string | null => {
  if (typeof value === "string") return value.trim() || null;
  if (typeof value === "number" && Number.isFinite(value)) return String(value);
  const record = asRecord(value);
  if (record) {
    return (
      textValue(record.anchor) ??
      textValue(record.name) ??
      textValue(record.entity) ??
      textValue(record.id)
    );
  }
  return null;
};

const compactJson = (value: unknown): string => {
  if (value == null) return "无";
  if (typeof value === "string") return value.trim() || "无";
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
};

const DurationControl: React.FC<{
  options: VideoGenOptions;
  durationLimits: VideoDurationLimits;
  disabled?: boolean;
  onOptionsChange: React.Dispatch<React.SetStateAction<VideoGenOptions>>;
}> = ({ options, durationLimits, disabled = false, onOptionsChange }) => {
  if (durationLimits.durationMode === "fixed" && durationLimits.supportedDurations.length > 0) {
    return (
      <select
        value={options.durationSecs}
        onChange={(event) => {
          if (disabled) return;
          const durationSecs = parseNumberInput(event.target.value);
          if (
            durationSecs !== null &&
            durationLimits.supportedDurations.includes(durationSecs)
          ) {
            onOptionsChange((prev) => ({
              ...prev,
              durationSecs,
              durationOptions: normalizeDurationOptions(durationSecs, prev.durationOptions),
            }));
          }
        }}
        disabled={disabled}
        className="pyqt-input !px-2 !py-1.5"
      >
        {durationLimits.supportedDurations.map((duration) => (
          <option key={duration} value={duration}>
            {duration}s
          </option>
        ))}
      </select>
    );
  }
  return (
    <input
      type="number"
      min={durationLimits.minDurationSecs}
      max={durationLimits.maxDurationSecs}
      value={options.durationSecs}
      onChange={(event) => {
        if (disabled) return;
        const durationSecs = parseNumberInput(event.target.value);
        if (durationSecs !== null) {
          onOptionsChange((prev) => ({
            ...prev,
            durationSecs,
            durationOptions: normalizeDurationOptions(durationSecs, prev.durationOptions),
          }));
        }
      }}
      disabled={disabled}
      className="pyqt-input !px-2 !py-1.5"
    />
  );
};

const SegmentPlanPanel: React.FC<{
  prompt: ChapterVideoPrompt | null;
  firstFrameMode: boolean;
  workflowMode: VideoWorkflowMode;
  frameLabel: string;
}> = ({ prompt, firstFrameMode, workflowMode, frameLabel }) => {
  const metadata = asRecord(prompt?.metadata_json);
  const segments = useMemo(
    () =>
      asArray(metadata?.segments)
        .map((segment) => asRecord(segment))
        .filter((segment): segment is Record<string, unknown> => segment != null)
        .sort((a, b) => {
          const numberA = numberValue(a.segment_number) ?? Number.MAX_SAFE_INTEGER;
          const numberB = numberValue(b.segment_number) ?? Number.MAX_SAFE_INTEGER;
          if (numberA !== numberB) return numberA - numberB;
          return (
            (numberValue(a.segment_index_in_shot) ?? 0) -
            (numberValue(b.segment_index_in_shot) ?? 0)
          );
        }),
    [metadata?.segments],
  );
  const totalDurationSecs = segments.reduce(
    (sum, segment) => sum + (numberValue(segment.duration_secs) ?? 0),
    0,
  );
  const expectedFrameCount =
    expectedFrameCountFromSegmentPlanPrompt(prompt, workflowMode) ?? 0;

  if (!prompt) {
    return (
      <div className="min-h-0 flex-1 overflow-y-auto custom-scrollbar pr-1">
        <EmptyHint text="当前章节还没有片段规划。" />
      </div>
    );
  }

  return (
    <div className="min-h-0 flex-1 overflow-y-auto custom-scrollbar pr-1">
      <div className="grid grid-cols-2 gap-2 text-[11px]">
        <div className="a4n-stat-tile px-2 py-1">
          <div className="text-book-text-muted">规划记录</div>
          <div className="min-w-0 truncate font-mono text-book-text-main">
            #{prompt.id} · {prompt.status}
          </div>
        </div>
        <div className="a4n-stat-tile px-2 py-1">
          <div className="text-book-text-muted">片段</div>
          <div className="font-mono text-book-text-main">{segments.length}</div>
        </div>
        <div className="a4n-stat-tile px-2 py-1">
          <div className="text-book-text-muted">总时长</div>
          <div className="font-mono text-book-text-main">{totalDurationSecs}s</div>
        </div>
        <div className="a4n-stat-tile px-2 py-1">
          <div className="text-book-text-muted">{firstFrameMode ? "初始首帧" : frameLabel}</div>
          <div className="font-mono text-book-text-main">{expectedFrameCount}</div>
        </div>
      </div>

      {segments.length === 0 ? (
        <div className="mt-2">
          <EmptyHint text="片段规划没有可展示的 segments。" />
        </div>
      ) : (
        <ol className="mt-3 space-y-2">
          {segments.map((segment, index) => {
            const segmentNumber = numberValue(segment.segment_number) ?? index + 1;
            const title = textValue(segment.title) ?? `片段 ${segmentNumber}`;
            const durationSecs = numberValue(segment.duration_secs);
            const sceneSummary = textValue(segment.scene_summary);
            const blockingSummary = textValue(segment.blocking_summary);
            const emotionalArc = asRecord(segment.emotional_arc);
            const emotionText = [
              textValue(emotionalArc?.start),
              textValue(emotionalArc?.turn),
              textValue(emotionalArc?.end),
            ]
              .filter(Boolean)
              .join(" → ");
            const videoIntent = asRecord(segment.video_intent);
            const hook = textValue(videoIntent?.hook);
            const visualPriority = textValue(videoIntent?.visual_priority);
            const sourceSpanIds = asArray(segment.source_span_ids).map(String);
            const spatialAnchors = asArray(segment.spatial_anchors)
              .map(compactLabel)
              .filter((label): label is string => label != null);
            const materialCoverage = asRecord(segment.material_coverage);
            const missingReferences = asArray(materialCoverage?.missing_or_weak_references)
              .map(compactLabel)
              .filter((label): label is string => label != null);
            const riskNote = textValue(materialCoverage?.risk_note);
            const shotId = numberValue(segment.analysis_shot_id);
            const segmentIndexInShot = numberValue(segment.segment_index_in_shot);
            const shotSegmentCount = numberValue(segment.shot_segment_count);
            return (
              <li
                key={`${segmentNumber}:${index}`}
                className="min-w-0 rounded border border-book-border/60 bg-book-bg-paper/55 p-2"
              >
                <div className="flex min-w-0 flex-wrap items-center gap-1.5">
                  <span className="shrink-0 font-mono text-xs text-book-text-muted">
                    #{segmentNumber}
                  </span>
                  <span className="min-w-[7rem] flex-1 truncate text-xs font-medium text-book-text-main">
                    {title}
                  </span>
                  {durationSecs != null && (
                    <span className="pyqt-pill shrink-0 !text-[10px]">{durationSecs}s</span>
                  )}
                  {shotId != null && (
                    <span className="pyqt-pill shrink-0 !text-[10px]">镜头 {shotId}</span>
                  )}
                  {segmentIndexInShot != null && shotSegmentCount != null && (
                    <span className="pyqt-pill shrink-0 !text-[10px]">
                      {segmentIndexInShot}/{shotSegmentCount}
                    </span>
                  )}
                </div>

                {sceneSummary && (
                  <p className="mt-1 line-clamp-3 break-words text-[11px] leading-relaxed text-book-text-sub">
                    {sceneSummary}
                  </p>
                )}

                <div className="mt-2 space-y-1 text-[11px] leading-relaxed">
                  {emotionText && <SegmentInfoLine label="情绪">{emotionText}</SegmentInfoLine>}
                  {hook && <SegmentInfoLine label="钩子">{hook}</SegmentInfoLine>}
                  {visualPriority && (
                    <SegmentInfoLine label="重点">{visualPriority}</SegmentInfoLine>
                  )}
                  {blockingSummary && (
                    <SegmentInfoLine label="走位">{blockingSummary}</SegmentInfoLine>
                  )}
                  {riskNote && <SegmentInfoLine label="素材">{riskNote}</SegmentInfoLine>}
                </div>

                {(spatialAnchors.length > 0 ||
                  sourceSpanIds.length > 0 ||
                  missingReferences.length > 0) && (
                  <div className="mt-2 flex min-w-0 flex-wrap gap-1 text-[10px] text-book-text-muted">
                    {spatialAnchors.slice(0, 4).map((anchor, anchorIndex) => (
                      <span
                        key={`${segmentNumber}:anchor:${anchorIndex}`}
                        className="pyqt-pill shrink-0 !text-[10px]"
                      >
                        {anchor}
                      </span>
                    ))}
                    {sourceSpanIds.slice(0, 3).map((id, idIndex) => (
                      <span
                        key={`${segmentNumber}:source:${idIndex}`}
                        className="pyqt-pill shrink-0 !text-[10px]"
                      >
                        {id}
                      </span>
                    ))}
                    {missingReferences.length > 0 && (
                      <span className="pyqt-pill shrink-0 !border-warning/50 !text-[10px] !text-warning">
                        弱覆盖 {missingReferences.length}
                      </span>
                    )}
                  </div>
                )}
              </li>
            );
          })}
        </ol>
      )}

      <details className="mt-3 rounded border border-book-border/60 bg-book-bg-paper/45 px-2 py-1.5">
        <summary className="cursor-pointer text-[11px] font-medium text-book-text-muted">
          原始 JSON
        </summary>
        <pre className="mt-2 max-h-72 overflow-auto whitespace-pre-wrap break-words rounded bg-book-bg-muted/60 p-2 text-[10px] leading-relaxed text-book-text-main custom-scrollbar">
          {compactJson(metadata?.segments ?? metadata ?? prompt.metadata_json)}
        </pre>
      </details>
    </div>
  );
};

function SegmentInfoLine({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div className="grid min-w-0 grid-cols-[34px_minmax(0,1fr)] gap-1">
      <span className="text-book-text-muted">{label}</span>
      <span className="min-w-0 break-words text-book-text-main">{children}</span>
    </div>
  );
}

export const VideoFrameWorkspace: React.FC<VideoFrameWorkspaceProps> = ({
  prompts,
  generationPrompts,
  workflowMode,
  segmentPlanPrompt,
  videoPromptCount,
  totalFramePromptRecordCount,
  availableImages,
  referenceLookupImages,
  frameImages,
  options,
  durationLimits,
  promptInputSelectionError,
  acting,
  chapterGenerating,
  previewingPromptRequests,
  onOptionsChange,
  onGenerateSegments,
  onDeleteSegmentPlan,
  onGeneratePrompts,
  onPreviewPromptRequests,
  onDeleteAllFramePrompts,
  onPreviewFrameImages,
  onUpdatePrompt,
  onOptimizePrompt,
  onDeletePrompt,
  onGenerateFrameImages,
  onImportFrameImage,
  onToggleFavorite,
  onOpenImageDetails,
}) => {
  const [framePanelTab, setFramePanelTab] = useState<FramePanelTab>(() =>
    prompts.length > 0 ? "prompts" : "generate",
  );
  const previousPromptCountRef = useRef(prompts.length);
  useEffect(() => {
    if (previousPromptCountRef.current === 0 && prompts.length > 0) {
      setFramePanelTab("prompts");
    }
    previousPromptCountRef.current = prompts.length;
  }, [prompts.length]);
  const firstFrameMode = workflowMode === "first_frame_chain";
  const chatVideoMode = workflowMode === "chat_video";
  const storyboardMode = workflowMode === "storyboard";
  const frameLabel = firstFrameMode
    ? "首帧"
    : storyboardMode
      ? "分镜板"
      : chatVideoMode
        ? "Chat 首尾帧"
        : "边界帧";
  const framePromptLabel = firstFrameMode
    ? "首帧提示词"
    : storyboardMode
      ? "分镜提示词"
    : chatVideoMode
      ? "Chat 首尾帧提示词"
      : "边界帧提示词";
  const segmentPlanLabel = firstFrameMode
    ? "首帧串联片段规划"
    : storyboardMode
      ? "分镜视频片段规划"
    : chatVideoMode
      ? "Chat 视频片段规划"
      : "视频片段规划";
  const hasVideoPrompts = videoPromptCount > 0;
  const runningFramePrompt =
    generationPrompts.find((prompt) => prompt.status === "running") ?? null;
  const promptById = useMemo(() => {
    const map = new Map<number, ChapterVideoPrompt>();
    for (const prompt of prompts) map.set(prompt.id, prompt);
    return map;
  }, [prompts]);
  const promptByFrameNumber = useMemo(() => {
    const map = new Map<number, ChapterVideoPrompt>();
    for (const prompt of prompts) map.set(prompt.shot_number, prompt);
    return map;
  }, [prompts]);
  const frameNumberByImageId = useMemo(() => {
    const map = new Map<number, number>();
    for (const image of frameImages) {
      const promptId = videoImageGenerationPromptId(image);
      const prompt = promptId == null ? null : promptById.get(promptId) ?? null;
      if (prompt && isVideoPromptFrameImage(image)) {
        map.set(image.id, prompt.shot_number);
        continue;
      }
      const tailBoundaryNumber = videoTailFrameBoundaryNumber(image);
      if (tailBoundaryNumber != null && promptByFrameNumber.has(tailBoundaryNumber)) {
        map.set(image.id, tailBoundaryNumber);
      }
    }
    return map;
  }, [frameImages, promptByFrameNumber, promptById]);
  const generatedImageCountsByPromptId = useMemo(() => {
    const counts = new Map<number, number>();
    for (const image of frameImages) {
      const frameNumber = frameNumberByImageId.get(image.id);
      if (frameNumber == null) continue;
      const prompt = promptByFrameNumber.get(frameNumber);
      if (!prompt) continue;
      counts.set(prompt.id, (counts.get(prompt.id) ?? 0) + 1);
    }
    return counts;
  }, [frameImages, frameNumberByImageId, promptByFrameNumber]);
  const generatedFrameImages = useMemo(
    () =>
      frameImages
        .filter((image) => frameNumberByImageId.has(image.id))
        .sort((a, b) => {
          const frameA = frameNumberByImageId.get(a.id) ?? Number.MAX_SAFE_INTEGER;
          const frameB = frameNumberByImageId.get(b.id) ?? Number.MAX_SAFE_INTEGER;
          if (frameA !== frameB) return frameA - frameB;
          return b.id - a.id;
        }),
    [frameImages, frameNumberByImageId],
  );
  const referenceImageById = useMemo(() => {
    const map = new Map<number, GeneratedImage>();
    for (const image of referenceLookupImages) map.set(image.id, image);
    for (const image of availableImages) map.set(image.id, image);
    for (const image of frameImages) map.set(image.id, image);
    return map;
  }, [availableImages, frameImages, referenceLookupImages]);
  const generatedFrameImageCount = generatedFrameImages.length;
  const segmentPlanSegmentCount = asArray(asRecord(segmentPlanPrompt?.metadata_json)?.segments).length;
  const expectedFramePromptCount = expectedFrameCountFromSegmentPlanPrompt(
    segmentPlanPrompt,
    workflowMode,
  );
  const completedFramePromptShotNumbers = useMemo(
    () => new Set(generationPrompts.map((prompt) => prompt.shot_number)),
    [generationPrompts],
  );
  const completedFramePromptPrefixCount = useMemo(() => {
    if (expectedFramePromptCount == null || expectedFramePromptCount <= 0) {
      return generationPrompts.length;
    }
    let count = 0;
    for (let shotNumber = 1; shotNumber <= expectedFramePromptCount; shotNumber += 1) {
      if (!completedFramePromptShotNumbers.has(shotNumber)) break;
      count += 1;
    }
    return count;
  }, [completedFramePromptShotNumbers, expectedFramePromptCount, generationPrompts.length]);
  const remainingFramePromptCount =
    expectedFramePromptCount == null
      ? null
      : Math.max(0, expectedFramePromptCount - completedFramePromptPrefixCount);
  const hiddenFramePromptRecordCount = Math.max(
    0,
    totalFramePromptRecordCount - prompts.length,
  );
  const segmentPlanRunningLocked = segmentPlanPrompt?.status === "running";
  const segmentPlanHasDownstream = totalFramePromptRecordCount > 0 || hasVideoPrompts;
  const segmentPlanDownstreamReason =
    totalFramePromptRecordCount > 0
      ? `已存在后续${framePromptLabel} ${totalFramePromptRecordCount} 条；请先在片段规划页顶部处理后续提示词`
      : hasVideoPrompts
        ? `已存在后续视频提示词 ${videoPromptCount} 条；请先处理后续视频提示词`
        : undefined;
  const segmentPlanGenerationBlockedReason =
    durationLimits.configError ??
    promptInputSelectionError ??
    (chapterGenerating ? `当前章节正在生成，不能生成${segmentPlanLabel}` : undefined) ??
    (!options.style.trim() ? `请填写${segmentPlanLabel}风格` : undefined) ??
    (segmentPlanRunningLocked ? `${segmentPlanLabel}正在运行，前置阶段已锁定` : undefined) ??
    segmentPlanDownstreamReason;
  const segmentPlanRegenerationBlockedReason =
    durationLimits.configError ??
    promptInputSelectionError ??
    (chapterGenerating ? `当前章节正在生成，不能重新生成${segmentPlanLabel}` : undefined) ??
    (!options.style.trim() ? `请填写${segmentPlanLabel}风格` : undefined) ??
    (segmentPlanRunningLocked ? `${segmentPlanLabel}正在运行，不能重新生成` : undefined) ??
    segmentPlanDownstreamReason;
  const deleteSegmentPlanBlockedReason =
    chapterGenerating
      ? `当前章节正在生成，不能删除${segmentPlanLabel}`
      : segmentPlanPrompt == null
        ? `没有可删除的${segmentPlanLabel}`
        : segmentPlanRunningLocked
          ? `${segmentPlanLabel}正在运行，不能删除`
          : undefined;
  const framePromptRunningLocked = runningFramePrompt != null;
  const framePromptGenerateLocked = framePromptRunningLocked;
  const framePromptLockedReason = runningFramePrompt
    ? `${framePromptLabel} #${runningFramePrompt.shot_number} 正在运行，前置阶段已锁定`
    : undefined;
  const framePromptGenerationBlockedReason =
    durationLimits.configError ??
    promptInputSelectionError ??
    (chapterGenerating ? `当前章节正在生成，不能生成${framePromptLabel}` : undefined) ??
    (!options.style.trim() ? `请填写${framePromptLabel}风格` : undefined) ??
    (!segmentPlanPrompt ? `请先生成${segmentPlanLabel}` : undefined) ??
    (segmentPlanRunningLocked ? `${segmentPlanLabel}正在运行，不能生成${framePromptLabel}` : undefined) ??
    framePromptLockedReason;
  const deleteAllFramePromptsBlockedReason =
    chapterGenerating
      ? `当前章节正在生成，不能删除${framePromptLabel}`
      : totalFramePromptRecordCount === 0
        ? `没有可删除的${framePromptLabel}记录`
        : undefined;
  const pendingImagePrompts = useMemo(
    () =>
      prompts.filter(
        (prompt) => (generatedImageCountsByPromptId.get(prompt.id) ?? 0) === 0,
      ),
    [generatedImageCountsByPromptId, prompts],
  );
  return (
    <div className="grid h-full min-h-0 gap-3 xl:grid-cols-[320px_minmax(0,1fr)]">
      <BookCard className="flex min-h-0 flex-col p-3">
        <div className="mb-3 grid min-w-0 grid-cols-3 overflow-hidden rounded-sm border border-book-border/40 bg-book-bg-paper/30">
          <button
            type="button"
            data-active={framePanelTab === "generate" ? "1" : undefined}
            onClick={() => setFramePanelTab("generate")}
            className="pyqt-tab min-w-0 flex-1 justify-center !px-2"
          >
            <Wand2 size={12} />
            <span className="truncate">生成片段</span>
          </button>
          <button
            type="button"
            data-active={framePanelTab === "segments" ? "1" : undefined}
            onClick={() => setFramePanelTab("segments")}
            className="pyqt-tab min-w-0 flex-1 justify-center !px-2"
          >
            <ClipboardList size={12} />
            <span className="truncate">片段规划</span>
            {segmentPlanSegmentCount > 0 && (
              <span className="font-mono text-[10px]">{segmentPlanSegmentCount}</span>
            )}
          </button>
          <button
            type="button"
            data-active={framePanelTab === "prompts" ? "1" : undefined}
            onClick={() => setFramePanelTab("prompts")}
            className="pyqt-tab min-w-0 flex-1 justify-center !px-2"
          >
            <ListChecks size={12} />
            <span className="truncate">{framePromptLabel}</span>
            <span className="font-mono text-[10px]">{prompts.length}</span>
          </button>
        </div>

        {framePanelTab === "generate" ? (
          <div className="min-h-0 flex-1 overflow-y-auto custom-scrollbar pr-1">
            <StyleTemplatePicker
              label="视频链路风格"
              value={options.style}
              templates={VIDEO_STYLE_TEMPLATES}
              rows={4}
              onChange={(style) => onOptionsChange((prev) => ({ ...prev, style }))}
            />
            <div className="mt-2 grid grid-cols-1 gap-2 text-xs sm:grid-cols-2">
              <label className="flex flex-col gap-1">
                <span className="text-book-text-muted">默认时长</span>
                <DurationControl
                  options={options}
                  durationLimits={durationLimits}
                  disabled={durationLimits.configError != null}
                  onOptionsChange={onOptionsChange}
                />
              </label>
              <label className="flex min-w-0 flex-col gap-1 sm:col-span-2">
                <span className="text-book-text-muted">可选时长</span>
                <VideoDurationOptionsPicker
                  options={options}
                  durationLimits={durationLimits}
                  disabled={durationLimits.configError != null}
                  onOptionsChange={onOptionsChange}
                />
              </label>
              <label className="flex flex-col gap-1">
                <span className="text-book-text-muted">画幅</span>
                <select
                  value={options.aspectRatio}
                  onChange={(event) =>
                    onOptionsChange((prev) => ({
                      ...prev,
                      aspectRatio: event.target.value as VideoGenOptions["aspectRatio"],
                    }))
                  }
                  className="pyqt-input !px-2 !py-1.5"
                >
                  {VIDEO_ASPECT_RATIOS.map((ratio) => (
                    <option key={ratio} value={ratio}>
                      {ratio}
                    </option>
                  ))}
                </select>
              </label>
              <label className="flex min-w-0 flex-col gap-1">
                <span className="text-book-text-muted">提示词语言</span>
                <select
                  value={options.promptOutputLanguage}
                  onChange={(event) =>
                    onOptionsChange((prev) => ({
                      ...prev,
                      promptOutputLanguage: PROMPT_OUTPUT_LANGUAGES.includes(
                        event.target.value as VideoGenOptions["promptOutputLanguage"],
                      )
                        ? (event.target.value as VideoGenOptions["promptOutputLanguage"])
                        : prev.promptOutputLanguage,
                    }))
                  }
                  className="pyqt-input !px-2 !py-1.5"
                >
                  {PROMPT_OUTPUT_LANGUAGES.map((language) => (
                    <option key={language} value={language}>
                      {PROMPT_OUTPUT_LANGUAGE_LABELS[language]}
                    </option>
                  ))}
                </select>
              </label>
            </div>
            <div className="a4n-stat-tile mt-2 px-2 py-1.5 text-[11px] text-book-text-sub">
              {durationLimitsLabel(durationLimits)}
            </div>
            {durationLimits.configError && (
              <div className="mt-2 rounded border border-error/40 bg-error/10 px-2 py-1.5 text-[11px] text-error">
                {durationLimits.configError}
              </div>
            )}
            {promptInputSelectionError && (
              <div className="mt-2 rounded border border-error/40 bg-error/10 px-2 py-1.5 text-[11px] text-error">
                {promptInputSelectionError}
              </div>
            )}
            <BookButton
              variant="primary"
              size="sm"
              onClick={() => void onGenerateSegments()}
              disabled={
                chapterGenerating ||
                acting === "generate-segment-plan" ||
                !options.style.trim() ||
                segmentPlanRunningLocked ||
                segmentPlanHasDownstream ||
                durationLimits.configError != null ||
                promptInputSelectionError != null
              }
              title={segmentPlanGenerationBlockedReason}
              className="mt-2 w-full"
            >
              <span className="inline-flex items-center gap-1">
                {acting === "generate-segment-plan" ? (
                  <Loader2 size={12} className="animate-spin" />
                ) : (
                  <Wand2 size={12} />
                )}
                生成/重试{segmentPlanLabel}
              </span>
            </BookButton>
            {segmentPlanPrompt && (
              <BookButton
                variant="secondary"
                size="sm"
                onClick={() => void onGenerateSegments({ regenerateExisting: true })}
                disabled={
                  chapterGenerating ||
                  acting === "generate-segment-plan" ||
                  !options.style.trim() ||
                  segmentPlanRunningLocked ||
                  segmentPlanHasDownstream ||
                  durationLimits.configError != null ||
                  promptInputSelectionError != null
                }
                title={segmentPlanRegenerationBlockedReason}
                className="mt-2 w-full"
              >
                <span className="inline-flex items-center gap-1">
                  {acting === "generate-segment-plan" ? (
                    <Loader2 size={12} className="animate-spin" />
                  ) : (
                    <RotateCcw size={12} />
                  )}
                  重新生成{segmentPlanLabel}
                </span>
              </BookButton>
            )}
            {segmentPlanPrompt && (
              <BookButton
                variant="danger"
                size="sm"
                onClick={() => void onDeleteSegmentPlan(segmentPlanPrompt)}
                disabled={
                  acting === `delete-prompt-${segmentPlanPrompt.id}` ||
                  deleteSegmentPlanBlockedReason != null
                }
                title={deleteSegmentPlanBlockedReason ?? segmentPlanDownstreamReason}
                className="mt-2 w-full"
              >
                <span className="inline-flex items-center gap-1">
                  {acting === `delete-prompt-${segmentPlanPrompt.id}` ? (
                    <Loader2 size={12} className="animate-spin" />
                  ) : (
                    <Trash2 size={12} />
                  )}
                  删除{segmentPlanLabel}
                </span>
              </BookButton>
            )}
            {segmentPlanPrompt && (
              <div className="a4n-stat-tile mt-2 px-2 py-1 text-[11px] text-book-text-muted">
                {segmentPlanLabel} #{segmentPlanPrompt.id} · {segmentPlanPrompt.status}
              </div>
            )}
            {hasVideoPrompts && (
              <div className="a4n-stat-tile mt-2 px-2 py-1 text-[11px] text-book-text-muted">
                已有视频提示词 {videoPromptCount} 条；编辑或删除被引用的前置提示词时，需要选择删除下游或保留下游继续。
              </div>
            )}
            {generatedFrameImageCount > 0 && (
              <div className="a4n-stat-tile mt-2 px-2 py-1 text-[11px] text-book-text-muted">
                已有图片 {generatedFrameImageCount} 张；生成/续跑可继续补齐缺失提示词，重新生成会由后端逐条保护已有图片记录
              </div>
            )}
            <div className="mt-3 grid grid-cols-2 gap-2 text-[11px]">
              <div className="a4n-stat-tile px-2 py-1">
                <div className="text-book-text-muted">可选参考图</div>
                <div className="font-mono text-book-text-main">{availableImages.length}</div>
              </div>
              <div className="a4n-stat-tile px-2 py-1">
                <div className="text-book-text-muted">已有图片</div>
                <div className="font-mono text-book-text-main">{generatedFrameImages.length}</div>
              </div>
            </div>
          </div>
        ) : framePanelTab === "segments" ? (
          <div className="flex min-h-0 flex-1 flex-col gap-2">
            <div className="shrink-0 space-y-2 border-b border-book-border/50 pb-2">
              <div className="grid grid-cols-2 gap-2 text-[11px] text-book-text-muted">
                <div className="a4n-stat-tile px-2 py-1">
                  {segmentPlanPrompt
                    ? `${segmentPlanLabel} ${segmentPlanSegmentCount} 段`
                    : `未生成${segmentPlanLabel}`}
                </div>
                <div className="a4n-stat-tile px-2 py-1">
                  {framePromptLabel} {prompts.length} 条
                </div>
              </div>
              <BookButton
                variant="primary"
                size="sm"
                onClick={() => void onGeneratePrompts()}
                disabled={
                  chapterGenerating ||
                  acting === "generate-frame-prompts" ||
                  !options.style.trim() ||
                  !segmentPlanPrompt ||
                  segmentPlanRunningLocked ||
                  framePromptGenerateLocked ||
                  durationLimits.configError != null ||
                  promptInputSelectionError != null
                }
                title={framePromptGenerationBlockedReason}
                className="w-full"
              >
                <span className="inline-flex items-center gap-1">
                  {acting === "generate-frame-prompts" ? (
                    <Loader2 size={12} className="animate-spin" />
                  ) : (
                    <Wand2 size={12} />
                  )}
                  生成/续跑{framePromptLabel}
                </span>
              </BookButton>
              {chatVideoMode &&
                remainingFramePromptCount != null &&
                remainingFramePromptCount > 0 && (
                  <BookButton
                    variant="secondary"
                    size="sm"
                    onClick={() => void onGeneratePrompts({ runUntilComplete: true })}
                    disabled={
                      chapterGenerating ||
                      acting === "generate-frame-prompts" ||
                      !options.style.trim() ||
                      !segmentPlanPrompt ||
                      segmentPlanRunningLocked ||
                      framePromptRunningLocked ||
                      durationLimits.configError != null ||
                      promptInputSelectionError != null
                    }
                    title={framePromptGenerationBlockedReason}
                    className="w-full"
                  >
                    <span className="inline-flex items-center gap-1">
                      {acting === "generate-frame-prompts" ? (
                        <Loader2 size={12} className="animate-spin" />
                      ) : (
                        <ListChecks size={12} />
                      )}
                      一键续跑当前语言剩余 {remainingFramePromptCount} 条
                    </span>
                  </BookButton>
                )}
            <div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
                <BookButton
                  variant="secondary"
                  size="sm"
                  onClick={() => void onPreviewPromptRequests()}
                  disabled={previewingPromptRequests || promptInputSelectionError != null}
                  title={promptInputSelectionError ?? undefined}
                  className="w-full"
                >
                  <span className="inline-flex items-center gap-1">
                    {previewingPromptRequests ? (
                      <Loader2 size={12} className="animate-spin" />
                    ) : (
                      <Eye size={12} />
                    )}
                    查看完整提示词
                  </span>
                </BookButton>
                <BookButton
                  variant="danger"
                  size="sm"
                  onClick={() => void onDeleteAllFramePrompts()}
                  disabled={
                    acting === "delete-frame-prompts" ||
                    deleteAllFramePromptsBlockedReason != null
                  }
                  title={deleteAllFramePromptsBlockedReason}
                  className="w-full"
                >
                  <span className="inline-flex items-center gap-1">
                    {acting === "delete-frame-prompts" ? (
                      <Loader2 size={12} className="animate-spin" />
                    ) : (
                      <Trash2 size={12} />
                    )}
                    删除全部
                  </span>
                </BookButton>
              </div>
            </div>
            <SegmentPlanPanel
              prompt={segmentPlanPrompt}
              firstFrameMode={firstFrameMode}
              workflowMode={workflowMode}
              frameLabel={frameLabel}
            />
          </div>
        ) : (
          <div className="min-h-0 flex-1 overflow-y-auto custom-scrollbar pr-1">
            <div className="mb-2 space-y-2 text-[11px] text-book-text-muted">
              <div className="a4n-stat-tile px-2 py-1">
                待生成图片 {pendingImagePrompts.length}/{prompts.length}
              </div>
            </div>
            {prompts.length === 0 ? (
              <EmptyHint
                text={
                  hiddenFramePromptRecordCount > 0
                    ? `当前章节已有 ${hiddenFramePromptRecordCount} 条首尾帧类提示词记录，但不属于当前生成模式或合同版本，请切换对应模式查看，或删除后重新生成。`
                    : `还没有${framePromptLabel}。`
                }
              />
            ) : (
              <ul className="space-y-2">
                {prompts.map((prompt) => {
                  const generatedImageCount = generatedImageCountsByPromptId.get(prompt.id) ?? 0;
                  return (
                    <VideoFramePromptCard
                      key={prompt.id}
                      prompt={prompt}
                      label={frameLabel}
                      promptLabel={framePromptLabel}
                      generatedImageCount={generatedImageCount}
                      referenceImageById={referenceImageById}
                      locked={false}
                      lockedReason={framePromptLockedReason}
                      acting={acting}
                      durationLimits={durationLimits}
                      onPreviewFrameImages={onPreviewFrameImages}
                      onUpdate={onUpdatePrompt}
                      onOptimize={onOptimizePrompt}
                      onDelete={onDeletePrompt}
                      onGenerateFrameImages={onGenerateFrameImages}
                      onImportFrameImage={onImportFrameImage}
                      onOpenImageDetails={onOpenImageDetails}
                    />
                  );
                })}
              </ul>
            )}
          </div>
        )}
      </BookCard>

      <VideoFrameImageGallery
        images={generatedFrameImages}
        promptById={promptById}
        frameNumberByImageId={frameNumberByImageId}
        label={frameLabel}
        onToggleFavorite={onToggleFavorite}
        onOpenImageDetails={onOpenImageDetails}
      />
    </div>
  );
};
