import React, { useMemo } from "react";
import { Plus, RefreshCw, Trash2 } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import {
  VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES,
  VIDEO_WORKFLOW_MODE_CHAT_VIDEO,
  VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN,
  VIDEO_WORKFLOW_MODE_REFERENCE_MEDIA,
  VIDEO_WORKFLOW_MODE_STORYBOARD,
  type ChapterVideoPrompt,
  type VideoSegmentExecutionRecipe,
  type VideoSegmentWorkflowMode,
  type VideoTimelineItemSummary,
  type VideoTimelineSummary,
} from "../../../api/video";
import { Dropdown } from "../../../components/ui/Dropdown";
import { SegmentedControl } from "../../../components/ui/SegmentedControl";
import { getOperationStatusLabel } from "../../../utils/workflowPresentation";
import { EmptyHint } from "../shared";
import {
  promptOutputLanguageOfPrompt,
  PROMPT_OUTPUT_LANGUAGE_LABELS,
  videoGuidanceConfigurationError,
  VIDEO_WORKFLOW_MODE_LABELS,
  type VideoSegmentCardPlan,
} from "./videoTabUtils";
import { promptTitle } from "./videoGenerationDisplay";
import {
  gapLabel,
  insertedSegmentGapKey,
  timelineGaps,
  type InsertedSegmentGap,
} from "./InsertedSegmentTimelinePanel";

interface VideoPromptSetupPanelProps {
  prompts: ChapterVideoPrompt[];
  segmentCards: VideoSegmentCardPlan[];
  segmentRecipes: Map<number, VideoSegmentExecutionRecipe>;
  selectedSegmentNumber: number | null;
  planningDraftVisible: boolean;
  selectedPromptId: number | null;
  loading: boolean;
  acting: string | null;
  savingRecipeSegments: Set<number>;
  finalPromptBlockedReasons: Map<number, string>;
  timeline: VideoTimelineSummary | null;
  layout: "index" | "selected";
  onLoad: () => void | Promise<void>;
  onSelectPrompt: (id: number) => void | Promise<void>;
  onSelectSegment: (segmentNumber: number) => void | Promise<void>;
  onWorkflowModeChange: (
    mode: VideoSegmentWorkflowMode,
    shotNumber: number,
  ) => void | Promise<void>;
  onDeletePrompt: (prompt: ChapterVideoPrompt) => void | Promise<void>;
  onInsertSegment: (gap: InsertedSegmentGap) => void;
  onOpenInsertedSegment: (timelineItemId: number) => void;
}

export const VideoPromptSetupPanel: React.FC<VideoPromptSetupPanelProps> = ({
  prompts,
  segmentCards,
  segmentRecipes,
  selectedSegmentNumber,
  planningDraftVisible,
  selectedPromptId,
  loading,
  acting,
  savingRecipeSegments,
  finalPromptBlockedReasons,
  timeline,
  layout,
  onLoad,
  onSelectPrompt,
  onSelectSegment,
  onWorkflowModeChange,
  onDeletePrompt,
  onInsertSegment,
  onOpenInsertedSegment,
}) => {
  const promptsBySegment = useMemo(() => {
    const groups = new Map<number, ChapterVideoPrompt[]>();
    for (const prompt of prompts) {
      const rows = groups.get(prompt.shot_number) ?? [];
      rows.push(prompt);
      groups.set(prompt.shot_number, rows);
    }
    return groups;
  }, [prompts]);
  const selectedLayout = layout === "selected";

  const renderPromptRow = (prompt: ChapterVideoPrompt) => {
    const selected = selectedPromptId === prompt.id;
    const promptOutputLanguage = promptOutputLanguageOfPrompt(prompt);
    const metadata = [
      `${prompt.duration_secs} 秒`,
      promptOutputLanguage == null
        ? "输出语言无效"
        : PROMPT_OUTPUT_LANGUAGE_LABELS[promptOutputLanguage],
      getOperationStatusLabel(prompt.status),
      prompt.generated_video_id != null ? "已生成视频" : null,
    ].filter((value): value is string => Boolean(value));
    const deleteBlockedReason =
      prompt.status === "running"
        ? `片段 ${prompt.shot_number} 正在运行，不能删除`
        : null;

    return (
      <li
        key={prompt.id}
        className={
          selected
            ? "flex min-w-0 bg-book-primary/8"
            : "flex min-w-0 bg-book-bg-paper/95 hover:bg-book-bg-tertiary/45"
        }
      >
        <button
          type="button"
          onClick={() => void onSelectPrompt(prompt.id)}
          className="min-w-0 flex-1 px-3 py-2.5 text-left focus-visible:outline focus-visible:outline-2 focus-visible:outline-book-primary"
        >
          <div className="truncate text-xs font-medium text-book-text-main">
            {promptTitle(prompt)}
          </div>
          <div className="mt-1 text-caption text-book-text-muted">
            {metadata.join(" · ")}
          </div>
          <div className="mt-1 line-clamp-2 text-small leading-relaxed text-book-text-sub">
            {prompt.visual_prompt}
          </div>
        </button>
        {deleteBlockedReason == null &&
        acting !== `delete-prompt-${prompt.id}` ? (
          <div className="shrink-0 pr-2 pt-2">
            <Dropdown
              items={[
                {
                  label: "删除视频提示词",
                  danger: true,
                  icon: <Trash2 size={12} />,
                  onClick: () => void onDeletePrompt(prompt),
                },
              ]}
            />
          </div>
        ) : null}
      </li>
    );
  };

  const refreshButton = (
    <button
      type="button"
      onClick={() => void onLoad()}
      disabled={loading}
      className="pyqt-btn-icon !h-9 !w-9 shrink-0 sm:!h-7 sm:!w-7"
      title="刷新"
    >
      <RefreshCw size={12} className={loading ? "animate-spin" : undefined} />
    </button>
  );

  if (selectedLayout) {
    const card = segmentCards.find(
      (item) => item.segmentNumber === selectedSegmentNumber,
    );

    if (!card) {
      return (
        <section className="flex min-h-20 items-center gap-3">
          <div className="min-w-0 flex-1">
            <EmptyHint text="请先生成片段规划。规划完成后即可配置当前片段。" />
          </div>
          {refreshButton}
        </section>
      );
    }

    const shotNumber = card.segmentNumber;
    const recipe = segmentRecipes.get(shotNumber) ?? null;
    const variants = promptsBySegment.get(shotNumber) ?? [];
    const savingRecipe = savingRecipeSegments.has(shotNumber);
    const busy = acting != null || savingRecipe;
    const finalPromptBlockedReason =
      finalPromptBlockedReasons.get(shotNumber) ?? null;
    const modeInputError = recipe
      ? videoGuidanceConfigurationError(
          recipe.workflow_mode,
          shotNumber,
          recipe.guidance_mode,
        )
      : null;

    if (!recipe) {
      return (
        <section className="flex min-w-0 items-center gap-3">
          <div role="alert" className="min-w-0 flex-1 text-caption text-error">
            {planningDraftVisible
              ? "该片段已增量生成；等待片段规划完成落库后即可选择执行方案。"
              : "当前片段缺少执行方案，无法生成；请重新生成片段规划。"}
          </div>
          {refreshButton}
        </section>
      );
    }

    return (
      <section aria-label="当前片段配置" className="min-w-0">
        <div className="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-2">
          <div className="shrink-0">
            <div className="text-caption font-medium text-book-text-main">
              生成方式
            </div>
            <div className="mt-0.5 text-caption text-book-text-muted">
              切换后自动保存
            </div>
          </div>
          <SegmentedControl
            ariaLabel={`片段 ${shotNumber} 生成方案`}
            value={modeInputError ? null : recipe.workflow_mode}
            onChange={(mode) => void onWorkflowModeChange(mode, shotNumber)}
            options={[
              {
                id: VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES,
                label: "首尾帧",
                disabled: busy,
              },
              {
                id: VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN,
                label: "首帧续接",
                disabled: busy,
              },
              {
                id: VIDEO_WORKFLOW_MODE_STORYBOARD,
                label: "分镜",
                disabled: busy,
              },
              {
                id: VIDEO_WORKFLOW_MODE_REFERENCE_MEDIA,
                label: "参考媒体",
                disabled: busy,
              },
              {
                id: VIDEO_WORKFLOW_MODE_CHAT_VIDEO,
                label: "Chat",
                disabled: busy,
              },
            ]}
          />
          <div className="flex min-w-[12rem] flex-1 items-center gap-2 text-caption">
            {savingRecipe ? (
              <span className="inline-flex items-center gap-1 text-book-text-muted">
                <LoadingSpinner size="sm" /> 保存方案
              </span>
            ) : finalPromptBlockedReason ? (
              <span className="line-clamp-2 text-warning">
                {finalPromptBlockedReason}
              </span>
            ) : (
              <span className="text-book-text-muted">
                {recipe.workflow_mode === VIDEO_WORKFLOW_MODE_REFERENCE_MEDIA
                  ? "在下方配置媒体并生成视频"
                  : "在下方填充素材并继续生成"}
              </span>
            )}
            {recipe.adopted_video_id != null ? (
              <span className="ml-auto shrink-0 text-success">
                已采用视频 #{recipe.adopted_video_id}
              </span>
            ) : null}
          </div>
          {refreshButton}
        </div>

        {variants.length > 0 ? (
          <ul
            aria-label={`片段 ${shotNumber} 提示词版本`}
            className="mt-3 grid max-h-40 grid-cols-1 gap-px overflow-y-auto rounded-[var(--pyqt-radius-md)] border border-book-border/40 bg-book-border/35 custom-scrollbar"
          >
            {variants.map(renderPromptRow)}
          </ul>
        ) : null}
      </section>
    );
  }

  const segmentCardByNumber = new Map(
    segmentCards.map((card) => [card.segmentNumber, card]),
  );
  const nativeTimelineItems =
    timeline?.items.filter((item) => item.kind === "native") ?? [];
  const nativeSegmentNumbers = new Set(
    nativeTimelineItems.map((item) => item.segment_number),
  );
  const timelineMatchesSegments = Boolean(
    timeline &&
    nativeTimelineItems.length === segmentCards.length &&
    nativeSegmentNumbers.size === segmentCards.length &&
    nativeTimelineItems.every(
      (item) =>
        item.segment_number != null &&
        segmentCardByNumber.has(item.segment_number),
    ),
  );
  const timelineItemById = new Map(
    timeline?.items.map((item) => [item.id, item]) ?? [],
  );
  const reanchorItemIds = new Set(
    timeline?.items
      .filter((item) => item.status === "needs_reanchor")
      .map((item) => item.id) ?? [],
  );
  const gapBlocked = (gap: InsertedSegmentGap) =>
    (gap.previous_item_id != null &&
      reanchorItemIds.has(gap.previous_item_id)) ||
    (gap.next_item_id != null && reanchorItemIds.has(gap.next_item_id));

  const renderNativeCard = (
    card: VideoSegmentCardPlan,
    timelineItem?: VideoTimelineItemSummary,
  ) => {
    const shotNumber = card.segmentNumber;
    const recipe = segmentRecipes.get(shotNumber) ?? null;
    const variants = promptsBySegment.get(shotNumber) ?? [];
    const selectedCard = selectedSegmentNumber === shotNumber;
    const finalPromptBlockedReason =
      finalPromptBlockedReasons.get(shotNumber) ?? null;
    const cardStatus =
      recipe?.adopted_video_id != null
        ? "已采用成片"
        : variants.some((prompt) => prompt.generated_video_id != null)
          ? "已有候选视频"
          : variants.length > 0
            ? `${variants.length} 条提示词`
            : recipe?.workflow_mode === VIDEO_WORKFLOW_MODE_REFERENCE_MEDIA
              ? "无需最终提示词"
              : finalPromptBlockedReason
                ? "待补充输入"
                : "待生成提示词";

    return (
      <section
        key={timelineItem ? `timeline-${timelineItem.id}` : shotNumber}
        className={`a4n-flat-surface overflow-hidden ${
          selectedCard
            ? "border-book-primary/50 ring-1 ring-book-primary/25"
            : ""
        }`}
      >
        <button
          type="button"
          onClick={() => void onSelectSegment(shotNumber)}
          aria-current={selectedCard ? "true" : undefined}
          data-video-segment={shotNumber}
          title={selectedCard ? "再次点击打开片段配置" : "选择片段"}
          className={`block w-full min-w-0 p-3 text-left focus-visible:outline focus-visible:outline-2 focus-visible:outline-book-primary ${
            selectedCard ? "bg-book-primary/6" : "hover:bg-book-bg-tertiary/45"
          }`}
        >
          <div className="flex min-w-0 items-start gap-2">
            <div className="min-w-0 flex-1">
              <div className="flex items-baseline gap-2">
                <span className="text-xs font-semibold text-book-text-main">
                  片段 {timelineItem?.display_index ?? shotNumber}
                </span>
                {timelineItem && timelineItem.display_index !== shotNumber ? (
                  <span className="text-caption text-book-text-muted">
                    原生 #{shotNumber}
                  </span>
                ) : null}
                <span className="text-caption text-book-text-muted">
                  {card.durationSecs}s · {card.aspectRatio}
                </span>
              </div>
              <div className="mt-0.5 truncate text-xs font-medium text-book-text-main">
                {card.title}
              </div>
            </div>
            <span
              className={`shrink-0 rounded-full px-2 py-1 text-caption ${
                recipe?.adopted_video_id != null
                  ? "bg-success/10 text-success"
                  : finalPromptBlockedReason
                    ? "bg-warning-bg text-warning"
                    : "bg-book-bg-tertiary text-book-text-muted"
              }`}
            >
              {cardStatus}
            </span>
          </div>
          {card.sceneSummary ? (
            <div className="mt-1 line-clamp-1 text-caption leading-relaxed text-book-text-muted">
              {card.sceneSummary}
            </div>
          ) : null}
          <div className="mt-2 text-caption text-book-text-sub">
            {recipe
              ? VIDEO_WORKFLOW_MODE_LABELS[recipe.workflow_mode]
              : "缺少执行方案"}
          </div>
        </button>
      </section>
    );
  };

  const renderInsertedCard = (item: VideoTimelineItemSummary) => {
    const needsReanchor = item.status === "needs_reanchor";
    const promptMissing = item.plan_prompt_id == null;
    return (
      <section
        key={`timeline-${item.id}`}
        className="overflow-hidden rounded-[var(--pyqt-radius-md)] border border-book-primary/40 bg-book-primary/6"
      >
        <button
          type="button"
          onClick={() => onOpenInsertedSegment(item.id)}
          disabled={promptMissing}
          className="block w-full min-w-0 p-3 text-left hover:bg-book-primary/10 disabled:cursor-not-allowed disabled:opacity-60"
        >
          <div className="flex min-w-0 items-start gap-2">
            <div className="min-w-0 flex-1">
              <div className="flex items-baseline gap-2">
                <span className="text-xs font-semibold text-book-text-main">
                  片段 {item.display_index}
                </span>
                <span className="rounded-full bg-book-primary/12 px-1.5 py-0.5 text-caption text-book-primary">
                  插入
                </span>
              </div>
              <div className="mt-0.5 truncate text-xs font-medium text-book-text-main">
                {item.title ?? "未命名插入片段"}
              </div>
            </div>
            <span
              className={`shrink-0 rounded-full px-2 py-1 text-caption ${
                needsReanchor
                  ? "bg-warning-bg text-warning"
                  : item.generated_video_id != null
                    ? "bg-success/10 text-success"
                    : "bg-book-bg-tertiary text-book-text-muted"
              }`}
            >
              {promptMissing
                ? "数据不完整"
                : needsReanchor
                  ? "需重新定位"
                  : item.generated_video_id != null
                    ? "已有视频"
                    : "待生成视频"}
            </span>
          </div>
          <div className="mt-2 text-caption text-book-text-sub">
            点击编辑提示词、移动位置或生成视频
          </div>
        </button>
      </section>
    );
  };

  const renderGap = (
    gap: InsertedSegmentGap,
    items: VideoTimelineItemSummary[],
  ) => {
    const blocked = gapBlocked(gap);
    return (
      <button
        type="button"
        aria-label={`在${gapLabel(gap, items)}插入片段`}
        title={blocked ? "请先重新定位相邻的插入片段" : "插入片段"}
        disabled={loading || acting != null || blocked}
        onClick={() => onInsertSegment(gap)}
        className="group flex h-6 w-full items-center gap-2 px-1 text-caption text-book-text-muted transition-colors hover:text-book-primary disabled:cursor-not-allowed disabled:opacity-45"
      >
        <span className="min-w-0 flex-1 border-t border-dashed border-book-border/60 group-hover:border-book-primary/55" />
        <span className="inline-flex shrink-0 items-center gap-1">
          <Plus size={10} /> 插入
        </span>
        <span className="min-w-0 flex-1 border-t border-dashed border-book-border/60 group-hover:border-book-primary/55" />
      </button>
    );
  };

  return (
    <section className="flex h-full min-h-0 flex-col overflow-hidden">
      <div className="mb-3 flex items-center gap-2">
        <div className="min-w-0 flex-1">
          <div className="text-xs font-medium text-book-text-main">
            片段执行方案
          </div>
          <div className="mt-0.5 text-caption text-book-text-muted">
            点击片段进行配置；使用卡片间的“插入”补充镜头
          </div>
        </div>
        {refreshButton}
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto overflow-x-visible custom-scrollbar pr-2">
        {timelineMatchesSegments && timeline ? (
          <div>
            {timelineGaps(timeline.items).map((gap) => {
              const nextItem =
                gap.next_item_id == null
                  ? null
                  : (timelineItemById.get(gap.next_item_id) ?? null);
              return (
                <React.Fragment
                  key={`timeline-gap-${insertedSegmentGapKey(gap)}`}
                >
                  {renderGap(gap, timeline.items)}
                  {nextItem?.kind === "inserted"
                    ? renderInsertedCard(nextItem)
                    : nextItem?.segment_number != null
                      ? renderNativeCard(
                          segmentCardByNumber.get(nextItem.segment_number)!,
                          nextItem,
                        )
                      : null}
                </React.Fragment>
              );
            })}
          </div>
        ) : segmentCards.length === 0 ? (
          <EmptyHint text="请先生成片段规划。规划完成后，每个片段会在这里独立选择生成方案。" />
        ) : (
          <div className="space-y-2">
            {timeline ? (
              <div
                role="alert"
                className="rounded-lg border border-warning/35 bg-warning-bg px-2.5 py-2 text-caption text-warning"
              >
                时间线与当前片段规划尚未同步，插入入口暂时停用；刷新后再试。
              </div>
            ) : null}
            {segmentCards.map((card) => renderNativeCard(card))}
          </div>
        )}
      </div>
    </section>
  );
};
