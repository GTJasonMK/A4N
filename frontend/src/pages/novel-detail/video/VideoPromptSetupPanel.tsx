import React, { useMemo } from "react";
import { ArrowDown, ArrowUp, RefreshCw, Trash2, X } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import type { GeneratedImage } from "../../../api/image";
import type {
  ChapterVideoPrompt,
  GeneratedVideo,
  SegmentGuidanceMode,
  VideoSegmentExecutionRecipe,
} from "../../../api/video";
import { Dropdown } from "../../../components/ui/Dropdown";
import { SegmentedControl } from "../../../components/ui/SegmentedControl";
import { getOperationStatusLabel } from "../../../utils/workflowPresentation";
import {
  EmptyHint,
  chapterGeneratingMessage,
  type ChapterRow,
} from "../shared";
import {
  promptOutputLanguageOfPrompt,
  PROMPT_OUTPUT_LANGUAGE_LABELS,
  videoGuidanceConfigurationError,
  VIDEO_WORKFLOW_MODE_LABELS,
  type ChapterVideoWorkflowMode,
  type VideoSegmentCardPlan,
} from "./videoTabUtils";
import { promptTitle } from "./videoGenerationDisplay";

interface VideoPromptSetupPanelProps {
  chapter: ChapterRow;
  prompts: ChapterVideoPrompt[];
  segmentCards: VideoSegmentCardPlan[];
  segmentRecipes: Map<number, VideoSegmentExecutionRecipe>;
  selectedSegmentNumber: number | null;
  planningDraftVisible: boolean;
  selectedPromptId: number | null;
  loading: boolean;
  acting: string | null;
  chapterGenerating: boolean;
  savingRecipeSegments: Set<number>;
  selectableReferenceImages: GeneratedImage[];
  previousAdoptedVideos: GeneratedVideo[];
  finalPromptBlockedReasons: Map<number, string>;
  selectedSegmentMaterialsControl?: React.ReactNode;
  selectedSegmentWorkspace?: React.ReactNode;
  onLoad: () => void | Promise<void>;
  onSelectPrompt: (id: number) => void | Promise<void>;
  onSelectSegment: (segmentNumber: number) => boolean | Promise<boolean>;
  onWorkflowModeChange: (
    mode: ChapterVideoWorkflowMode,
    shotNumber: number,
  ) => void | Promise<void>;
  onUpdateGuidance: (
    segmentNumber: number,
    guidanceMode: SegmentGuidanceMode,
    referenceImageIds: number[],
    continuitySourceVideoId: number | null,
  ) => void | Promise<void>;
  onDeletePrompt: (prompt: ChapterVideoPrompt) => void | Promise<void>;
}

export const VideoPromptSetupPanel: React.FC<VideoPromptSetupPanelProps> = ({
  chapter,
  prompts,
  segmentCards,
  segmentRecipes,
  selectedSegmentNumber,
  planningDraftVisible,
  selectedPromptId,
  loading,
  acting,
  chapterGenerating,
  savingRecipeSegments,
  selectableReferenceImages,
  previousAdoptedVideos,
  finalPromptBlockedReasons,
  selectedSegmentMaterialsControl,
  selectedSegmentWorkspace,
  onLoad,
  onSelectPrompt,
  onSelectSegment,
  onWorkflowModeChange,
  onUpdateGuidance,
  onDeletePrompt,
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

  const renderPromptRow = (prompt: ChapterVideoPrompt) => {
    const selected = selectedPromptId === prompt.id;
    const metadata = [
      `${prompt.duration_secs} 秒`,
      PROMPT_OUTPUT_LANGUAGE_LABELS[promptOutputLanguageOfPrompt(prompt)],
      getOperationStatusLabel(prompt.status),
      prompt.generated_video_id != null ? "已生成视频" : null,
    ].filter((value): value is string => Boolean(value));
    const deleteBlockedReason = chapterGenerating
      ? chapterGeneratingMessage(chapter, "删除提示词")
      : prompt.status === "running"
        ? `片段 ${prompt.shot_number} 正在运行，不能删除`
        : null;

    return (
      <li
        key={prompt.id}
        className={
          selected
            ? "flex min-w-0 bg-book-primary/8"
            : "flex min-w-0 hover:bg-book-bg-tertiary/45"
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

  return (
    <section className="flex h-full min-h-0 flex-col overflow-hidden">
      <div className="mb-3 flex items-center gap-2">
        <div className="min-w-0 flex-1">
          <div className="text-xs font-medium text-book-text-main">
            片段执行方案
          </div>
          <div className="mt-0.5 text-caption text-book-text-muted">
            每个片段独立保存模式、参考素材和媒体引导
          </div>
        </div>
        <button
          type="button"
          onClick={() => void onLoad()}
          disabled={loading}
          className="pyqt-btn-icon !h-9 !w-9 sm:!h-7 sm:!w-7"
          title="刷新"
        >
          <RefreshCw
            size={12}
            className={loading ? "animate-spin" : undefined}
          />
        </button>
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto overflow-x-visible custom-scrollbar pr-2">
        {segmentCards.length === 0 ? (
          <EmptyHint text="请先生成片段规划。规划完成后，每个片段会在这里独立选择生成方案。" />
        ) : (
          <div className="space-y-2">
            {segmentCards.map((card) => {
              const shotNumber = card.segmentNumber;
              const recipe = segmentRecipes.get(shotNumber) ?? null;
              const variants = promptsBySegment.get(shotNumber) ?? [];
              const savingRecipe = savingRecipeSegments.has(shotNumber);
              const busy = acting != null || chapterGenerating || savingRecipe;
              const selectedCard = selectedSegmentNumber === shotNumber;
              const keyframesSupported =
                recipe != null &&
                videoGuidanceConfigurationError(
                  recipe.workflow_mode,
                  shotNumber,
                  "keyframes",
                ) == null;
              const systemFramesSupported =
                recipe != null &&
                videoGuidanceConfigurationError(
                  recipe.workflow_mode,
                  shotNumber,
                  "system_frames",
                ) == null;
              const continuitySupported =
                recipe != null &&
                videoGuidanceConfigurationError(
                  recipe.workflow_mode,
                  shotNumber,
                  "continuity",
                ) == null;
              const textOnlySupported =
                recipe != null &&
                videoGuidanceConfigurationError(
                  recipe.workflow_mode,
                  shotNumber,
                  "none",
                ) == null;
              const guidanceReady =
                recipe?.guidance_mode === "keyframes"
                  ? recipe.guidance_reference_image_ids.length > 0
                  : recipe?.guidance_mode === "continuity"
                    ? recipe.continuity_source_video_id != null
                    : true;
              const finalPromptBlockedReason =
                finalPromptBlockedReasons.get(shotNumber) ?? null;
              const cardStatus =
                recipe?.adopted_video_id != null
                  ? "已采用成片"
                  : variants.some((prompt) => prompt.generated_video_id != null)
                    ? "已有候选视频"
                    : variants.length > 0
                      ? `${variants.length} 条提示词`
                      : finalPromptBlockedReason
                        ? "待补充输入"
                        : "待生成提示词";

              return (
                <section
                  key={shotNumber}
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
                    className={`block w-full min-w-0 p-3 text-left focus-visible:outline focus-visible:outline-2 focus-visible:outline-book-primary ${
                      selectedCard
                        ? "bg-book-primary/6"
                        : "hover:bg-book-bg-tertiary/45"
                    }`}
                  >
                    <div className="flex min-w-0 items-start gap-2">
                      <div className="min-w-0 flex-1">
                        <div className="flex items-baseline gap-2">
                          <span className="text-xs font-semibold text-book-text-main">
                            片段 {shotNumber}
                          </span>
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
                      <div
                        className={`mt-1 text-caption leading-relaxed text-book-text-muted ${
                          selectedCard ? "line-clamp-2" : "line-clamp-1"
                        }`}
                      >
                        {card.sceneSummary}
                      </div>
                    ) : null}
                    <div className="mt-2 text-caption text-book-text-sub">
                      {recipe
                        ? VIDEO_WORKFLOW_MODE_LABELS[recipe.workflow_mode]
                        : "缺少执行方案"}
                    </div>
                  </button>

                  {recipe && selectedCard ? (
                    <div className="space-y-3 border-t border-book-border/40 px-3 py-3">
                      <div>
                        <div className="mb-1.5 text-caption font-medium text-book-text-main">
                          生成方式
                        </div>
                        <SegmentedControl
                          ariaLabel={`片段 ${shotNumber} 生成方案`}
                          value={recipe.workflow_mode}
                          onChange={(mode) =>
                            void onWorkflowModeChange(mode, shotNumber)
                          }
                          className="a4n-seg--segment-workflows"
                          options={[
                            {
                              id: "boundary_frames",
                              label: "首尾帧",
                              disabled: busy,
                            },
                            {
                              id: "first_frame_chain",
                              label: "首帧续接",
                              disabled: busy,
                            },
                            { id: "storyboard", label: "分镜", disabled: busy },
                            {
                              id: "reference_media",
                              label: "参考媒体",
                              disabled: busy,
                            },
                            { id: "chat_video", label: "Chat", disabled: busy },
                          ]}
                        />
                      </div>
                      {selectedSegmentMaterialsControl ? (
                        <div className="border-t border-book-border/35 pt-3">
                          {selectedSegmentMaterialsControl}
                        </div>
                      ) : null}
                      {recipe.workflow_mode !== "chat_video" &&
                      recipe.workflow_mode !== "reference_media" ? (
                        <div className="space-y-2 border-t border-book-border/35 pt-3">
                          <div className="text-caption font-medium text-book-text-main">
                            生成视频时的媒体输入
                          </div>
                          <SegmentedControl
                            ariaLabel={`片段 ${shotNumber} 媒体引导`}
                            value={recipe.guidance_mode}
                            onChange={(value) =>
                              void onUpdateGuidance(
                                shotNumber,
                                value as SegmentGuidanceMode,
                                value === "keyframes"
                                  ? recipe.guidance_reference_image_ids
                                  : [],
                                value === "continuity"
                                  ? recipe.continuity_source_video_id
                                  : null,
                              )
                            }
                            options={[
                              {
                                id: "system_frames",
                                label:
                                  recipe.workflow_mode === "storyboard"
                                    ? "分镜板"
                                    : recipe.workflow_mode ===
                                        "first_frame_chain"
                                      ? "系统首帧"
                                      : "系统首尾帧",
                                disabled: busy || !systemFramesSupported,
                              },
                              ...(recipe.workflow_mode ===
                                "first_frame_chain" && shotNumber > 1
                                ? [
                                    {
                                      id: "continuity" as const,
                                      label: "续接视频",
                                      disabled: busy || !continuitySupported,
                                    },
                                  ]
                                : []),
                              {
                                id: "keyframes",
                                  label: "关键帧",
                                disabled: busy || !keyframesSupported,
                              },
                              {
                                id: "none",
                                label: "纯文本",
                                disabled: busy || !textOnlySupported,
                              },
                            ]}
                          />
                          {recipe.guidance_mode === "keyframes" ? (
                            <div className="space-y-1.5">
                              {recipe.guidance_reference_image_ids.map(
                                (imageId, index) => {
                                  const image = selectableReferenceImages.find(
                                    (row) => row.id === imageId,
                                  );
                                  const updateIds = (ids: number[]) =>
                                    void onUpdateGuidance(
                                      shotNumber,
                                      "keyframes",
                                      ids,
                                      null,
                                    );
                                  return (
                                    <div
                                      key={imageId}
                                      className="flex min-w-0 items-center gap-1 text-caption"
                                    >
                                      <span className="min-w-0 flex-1 truncate">
                                        {image?.prompt || `图片 #${imageId}`}
                                      </span>
                                      <button
                                        type="button"
                                        className="pyqt-btn-icon !h-7 !w-7"
                                        disabled={busy || index === 0}
                                        onClick={() =>
                                          updateIds(
                                            recipe.guidance_reference_image_ids.map(
                                              (id, current) =>
                                                current === index - 1
                                                  ? imageId
                                                  : current === index
                                                    ? recipe
                                                        .guidance_reference_image_ids[
                                                        index - 1
                                                      ]
                                                    : id,
                                            ),
                                          )
                                        }
                                      >
                                        <ArrowUp size={11} />
                                      </button>
                                      <button
                                        type="button"
                                        className="pyqt-btn-icon !h-7 !w-7"
                                        disabled={
                                          busy ||
                                          index ===
                                            recipe.guidance_reference_image_ids
                                              .length -
                                              1
                                        }
                                        onClick={() =>
                                          updateIds(
                                            recipe.guidance_reference_image_ids.map(
                                              (id, current) =>
                                                current === index + 1
                                                  ? imageId
                                                  : current === index
                                                    ? recipe
                                                        .guidance_reference_image_ids[
                                                        index + 1
                                                      ]
                                                    : id,
                                            ),
                                          )
                                        }
                                      >
                                        <ArrowDown size={11} />
                                      </button>
                                      <button
                                        type="button"
                                        className="pyqt-btn-icon !h-7 !w-7"
                                        disabled={busy}
                                        onClick={() =>
                                          updateIds(
                                            recipe.guidance_reference_image_ids.filter(
                                              (id) => id !== imageId,
                                            ),
                                          )
                                        }
                                      >
                                        <X size={11} />
                                      </button>
                                    </div>
                                  );
                                },
                              )}
                              {recipe.guidance_reference_image_ids.length <
                              selectableReferenceImages.length ? (
                                <select
                                  value=""
                                  disabled={busy}
                                  onChange={(event) => {
                                    const imageId = Number(event.target.value);
                                    if (imageId > 0)
                                      void onUpdateGuidance(
                                        shotNumber,
                                        "keyframes",
                                        [
                                          ...recipe.guidance_reference_image_ids,
                                          imageId,
                                        ],
                                        null,
                                      );
                                  }}
                                  className="pyqt-input w-full !px-2 !py-1.5 text-caption"
                                >
                                  <option value="">添加关键帧参考图…</option>
                                  {selectableReferenceImages
                                    .filter(
                                      (image) =>
                                        !recipe.guidance_reference_image_ids.includes(
                                          image.id,
                                        ),
                                    )
                                    .map((image) => (
                                      <option key={image.id} value={image.id}>
                                        {image.prompt || `图片 #${image.id}`}
                                      </option>
                                    ))}
                                </select>
                              ) : null}
                              {!guidanceReady ? (
                                <div className="text-caption text-warning">
                                  请至少选择 1 张关键帧后再生成最终提示词。
                                </div>
                              ) : null}
                            </div>
                          ) : null}
                          {recipe.guidance_mode === "continuity" ? (
                            <select
                              value={recipe.continuity_source_video_id ?? ""}
                              disabled={busy}
                              onChange={(event) =>
                                void onUpdateGuidance(
                                  shotNumber,
                                  "continuity",
                                  [],
                                  Number(event.target.value) || null,
                                )
                              }
                              className="pyqt-input w-full !px-2 !py-1.5 text-caption"
                            >
                              <option value="">
                                选择更早片段已采用的视频…
                              </option>
                              {previousAdoptedVideos.map((video) => (
                                <option key={video.id} value={video.id}>
                                  片段{" "}
                                  {String(
                                    (
                                      video.metadata_json as Record<
                                        string,
                                        unknown
                                      > | null
                                    )?.shot_number ?? "?",
                                  )}{" "}
                                  · 视频 #{video.id}
                                </option>
                              ))}
                            </select>
                          ) : null}
                        </div>
                      ) : null}
                      <div className="flex min-w-0 items-center gap-2 border-t border-book-border/35 pt-3 text-caption">
                        {savingRecipe ? (
                          <span className="inline-flex items-center gap-1 text-book-text-muted">
                            <LoadingSpinner size="sm" /> 保存方案
                          </span>
                        ) : finalPromptBlockedReason &&
                          recipe.workflow_mode !== "reference_media" ? (
                          <span className="line-clamp-2 text-warning">
                            {finalPromptBlockedReason}
                          </span>
                        ) : (
                          <span className="text-book-text-muted">
                            {recipe.workflow_mode === "reference_media"
                              ? "在下方配置媒体并生成视频"
                              : "在右侧继续生成"}
                          </span>
                        )}
                        {recipe.adopted_video_id != null ? (
                          <span className="ml-auto shrink-0 text-success">
                            已采用视频 #{recipe.adopted_video_id}
                          </span>
                        ) : null}
                      </div>
                      {recipe.workflow_mode === "reference_media" &&
                      selectedSegmentWorkspace ? (
                        <div
                          data-segment-workspace={shotNumber}
                          className="min-h-[640px] border-t border-book-border/35 pt-3"
                        >
                          {selectedSegmentWorkspace}
                        </div>
                      ) : null}
                    </div>
                  ) : !recipe && selectedCard ? (
                    <div
                      role="alert"
                      className="border-t border-book-border/40 px-3 py-2 text-caption text-error"
                    >
                      {planningDraftVisible
                        ? "该片段已增量生成；等待片段规划完成落库后即可选择执行方案。"
                        : "当前片段缺少执行方案；请完成数据库迁移后重新加载，系统不会临时创建默认模式。"}
                    </div>
                  ) : null}

                  {selectedCard ? (
                    variants.length > 0 ? (
                      <ul className="divide-y divide-book-border/35 border-t border-book-border/40">
                        {variants.map(renderPromptRow)}
                      </ul>
                    ) : recipe?.workflow_mode === "reference_media" ? null : (
                      <div className="border-t border-book-border/40 px-3 py-2 text-caption text-book-text-muted">
                        尚未生成最终提示词，按右侧提示继续。
                      </div>
                    )
                  ) : null}
                </section>
              );
            })}
          </div>
        )}
      </div>
    </section>
  );
};
