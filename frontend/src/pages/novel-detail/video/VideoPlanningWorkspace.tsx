import React, { useEffect, useMemo, useState } from "react";
import { ListChecks, RotateCcw, Trash2, Wand2 } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import type { ChapterVideoPrompt } from "../../../api/video";
import type { ShootingSceneV1 } from "../../../api/shootingScript";
import { BookButton } from "../../../components/ui/BookButton";
import { asArray, asJsonRecord } from "../../../utils/jsonRecord";
import { getOperationStatusLabel } from "../../../utils/workflowPresentation";
import { EmptyHint } from "../shared";
import { VideoDurationOptionsPicker } from "./VideoDurationOptionsPicker";
import {
  PROMPT_OUTPUT_LANGUAGE_LABELS,
  VIDEO_ASPECT_RATIOS,
  type VideoDurationLimits,
  type VideoGenOptions,
} from "./videoTabUtils";
import type { LoadedSegmentPlanDraft } from "./loadVideoTabData";

interface VideoPlanningWorkspaceProps {
  prompt: ChapterVideoPrompt | null;
  previewSegments: Record<string, unknown>[];
  draft: LoadedSegmentPlanDraft | null;
  options: VideoGenOptions;
  durationLimits: VideoDurationLimits;
  loading: boolean;
  loadError: string | null;
  dataReady: boolean;
  acting: string | null;
  shootingScenes: ShootingSceneV1[];
  onGenerate: (
    targetNarrativeUnitIndex: number,
    regenerateExisting: boolean,
  ) => void | Promise<void>;
  onOptionsChange: React.Dispatch<React.SetStateAction<VideoGenOptions>>;
  onRetryLoad: () => void | Promise<void>;
  onDelete: (prompt: ChapterVideoPrompt) => void | Promise<void>;
}

const textValue = (value: unknown): string | null =>
  typeof value === "string" && value.trim() ? value.trim() : null;

const numberValue = (value: unknown): number | null =>
  typeof value === "number" && Number.isFinite(value) ? value : null;

type PlanningSceneSelection = Pick<ShootingSceneV1, "narrative_unit_index"> & {
  shots: Array<Pick<ShootingSceneV1["shots"][number], "shot_id">>;
};

export const selectVideoPlanningUnitIndex = (
  shootingScenes: PlanningSceneSelection[],
  draftTarget: number | undefined,
  current: number | null,
  plannedShotIds: ReadonlySet<string>,
): number | null => {
  const available = new Set(
    shootingScenes.map((scene) => scene.narrative_unit_index),
  );
  if (draftTarget != null && available.has(draftTarget)) return draftTarget;

  const currentScene = shootingScenes.find(
    (scene) => scene.narrative_unit_index === current,
  );
  const currentScenePlanned =
    currentScene != null &&
    currentScene.shots.length > 0 &&
    currentScene.shots.every((shot) => plannedShotIds.has(shot.shot_id));
  if (currentScene != null && !currentScenePlanned) {
    return currentScene.narrative_unit_index;
  }

  return shootingScenes.find((scene) =>
    scene.shots.some((shot) => !plannedShotIds.has(shot.shot_id)),
  )?.narrative_unit_index ??
    currentScene?.narrative_unit_index ??
    shootingScenes[0]?.narrative_unit_index ??
    null;
};

const mergeVideoPlanningSegments = (
  persistedSegments: Record<string, unknown>[],
  previewSegments: Record<string, unknown>[],
  shootingScenes: PlanningSceneSelection[],
): Record<string, unknown>[] => {
  if (previewSegments.length === 0) return persistedSegments;
  if (persistedSegments.length === 0) return previewSegments;

  const previewShotIds = new Set(
    previewSegments
      .map((segment) => textValue(segment.source_shot_id))
      .filter((value): value is string => value != null),
  );
  const shotOrder = new Map(
    shootingScenes
      .flatMap((scene) => scene.shots)
      .map((shot, index) => [shot.shot_id, index] as const),
  );
  return [
    ...persistedSegments.filter((segment) => {
      const sourceShotId = textValue(segment.source_shot_id);
      return sourceShotId == null || !previewShotIds.has(sourceShotId);
    }),
    ...previewSegments,
  ]
    .sort((left, right) => {
      const leftShotId = textValue(left.source_shot_id);
      const rightShotId = textValue(right.source_shot_id);
      const leftShotOrder =
        (leftShotId == null ? undefined : shotOrder.get(leftShotId)) ??
        Number.MAX_SAFE_INTEGER;
      const rightShotOrder =
        (rightShotId == null ? undefined : shotOrder.get(rightShotId)) ??
        Number.MAX_SAFE_INTEGER;
      if (leftShotOrder !== rightShotOrder) {
        return leftShotOrder - rightShotOrder;
      }
      if (leftShotId != null && leftShotId === rightShotId) {
        return (
          (numberValue(left.segment_index_in_shot) ?? Number.MAX_SAFE_INTEGER) -
          (numberValue(right.segment_index_in_shot) ?? Number.MAX_SAFE_INTEGER)
        );
      }
      return 0;
    })
    .map((segment, index) => ({
      ...segment,
      segment_number: index + 1,
    }));
};

export const VideoPlanningWorkspace: React.FC<VideoPlanningWorkspaceProps> = ({
  prompt,
  previewSegments,
  draft,
  options,
  durationLimits,
  loading,
  loadError,
  dataReady,
  acting,
  shootingScenes,
  onGenerate,
  onOptionsChange,
  onRetryLoad,
  onDelete,
}) => {
  const persistedSegments = useMemo(
    () =>
      asArray(asJsonRecord(prompt?.metadata_json)?.segments)
        .map(asJsonRecord)
        .filter((item): item is Record<string, unknown> => item != null)
        .sort(
          (left, right) =>
            (numberValue(left.segment_number) ?? Number.MAX_SAFE_INTEGER) -
            (numberValue(right.segment_number) ?? Number.MAX_SAFE_INTEGER),
        ),
    [prompt?.metadata_json],
  );
  const draftRunning = draft?.workflowStatus === "running";
  const canContinueDraft = draft != null && !draftRunning;
  const busy = acting === "generate-segment-plan" || draftRunning;
  const showingPreview = previewSegments.length > 0;
  const generatingPreview = acting === "generate-segment-plan" || draftRunning;
  const segments = useMemo(
    () =>
      mergeVideoPlanningSegments(
        persistedSegments,
        previewSegments,
        shootingScenes,
      ),
    [persistedSegments, previewSegments, shootingScenes],
  );
  const plannedShotIds = useMemo(
    () =>
      new Set(
        segments
          .map((segment) => textValue(segment.source_shot_id))
          .filter((value): value is string => value != null),
      ),
    [segments],
  );
  const draftTarget = draft?.resumeOptions.target_narrative_unit_index;
  const [selectedUnitIndex, setSelectedUnitIndex] = useState<number | null>(null);
  useEffect(() => {
    setSelectedUnitIndex((current) => {
      return selectVideoPlanningUnitIndex(
        shootingScenes,
        draftTarget,
        current,
        plannedShotIds,
      );
    });
  }, [draftTarget, plannedShotIds, shootingScenes]);
  const selectedScene = shootingScenes.find(
    (scene) => scene.narrative_unit_index === selectedUnitIndex,
  );
  const selectedScenePlanned =
    selectedScene != null &&
    selectedScene.shots.length > 0 &&
    selectedScene.shots.every((shot) => plannedShotIds.has(shot.shot_id));
  const needsLoadRetry = Boolean(loadError && !prompt);
  const shotProgress =
    draft && draft.completedShotCount > 0
      ? `${draft.completedShotCount}/${draft.totalShotCount} 个镜头 · `
      : "";
  const workflowProgress = draft?.workflowProgress
    ? `${Math.round(draft.workflowProgress.percent)}%${draft.workflowProgress.message.trim() ? ` · ${draft.workflowProgress.message.trim()}` : ""} · `
    : "";

  return (
    <section className="flex h-full min-h-0 flex-col" aria-label="视频片段规划">
      {/* 头部：标题 + 状态 + 主操作 */}
      <div className="mb-4 flex min-w-0 flex-wrap items-center gap-3 border-b border-book-border/60 pb-4">
        <div className="min-w-0 flex-1">
          <div className="flex items-baseline gap-2">
            <h2 className="inline-flex items-center gap-1.5 text-base font-semibold text-book-text-main">
              <ListChecks size={16} className="text-book-primary" />
              片段规划
            </h2>
            <span className="text-small text-book-text-muted">
              {showingPreview
                ? `${generatingPreview ? "生成中草稿" : "已保存草稿"} · ${segments.length} 个片段`
                : prompt
                ? `${getOperationStatusLabel(prompt.status)} · ${segments.length} 个片段`
                : "未规划"}
            </span>
          </div>
          {(workflowProgress || shotProgress || loadError) && (
            <div className="mt-1 text-small text-book-text-muted">
              {workflowProgress}{shotProgress}
              {loadError && <span className="text-error">加载失败：{loadError}</span>}
            </div>
          )}
        </div>
        <div className="flex items-center gap-2">
          <select
            aria-label="选择要规划的拍摄场景"
            value={selectedUnitIndex ?? ""}
            onChange={(event) => setSelectedUnitIndex(Number(event.target.value))}
            disabled={busy || loading || draft != null || shootingScenes.length === 0}
            className="pyqt-input max-w-64 !px-2 !py-2 text-xs"
          >
            {shootingScenes.length === 0 ? (
              <option value="">尚无已完成场景</option>
            ) : (
              shootingScenes.map((scene) => {
                const planned =
                  scene.shots.length > 0 &&
                  scene.shots.every((shot) => plannedShotIds.has(shot.shot_id));
                return (
                  <option
                    key={scene.narrative_unit_index}
                    value={scene.narrative_unit_index}
                  >
                    场景 {scene.scene_index} · {scene.title}{planned ? " · 已规划" : ""}
                  </option>
                );
              })
            )}
          </select>
          <BookButton
            variant="primary"
            size="md"
            onClick={() =>
              void (needsLoadRetry
                ? onRetryLoad()
                : selectedUnitIndex == null
                  ? undefined
                  : onGenerate(selectedUnitIndex, selectedScenePlanned))
            }
            disabled={
              busy ||
              loading ||
              selectedUnitIndex == null ||
              (!dataReady && !needsLoadRetry)
            }
          >
            {busy || loading ? <LoadingSpinner size="sm" /> : selectedScenePlanned || needsLoadRetry ? <RotateCcw size={14} /> : <Wand2 size={14} />}
            {canContinueDraft
              ? selectedScenePlanned ? "继续重新规划" : "继续生成"
              : selectedScenePlanned ? "重新规划此场景" : needsLoadRetry ? "重试加载" : "规划此场景"}
          </BookButton>
          {prompt ? (
            <BookButton
              variant="ghost"
              size="md"
              onClick={() => void onDelete(prompt)}
              disabled={busy}
              title="删除片段规划及其受影响的下游记录"
            >
              <Trash2 size={14} />
            </BookButton>
          ) : null}
        </div>
      </div>

      {/* 配置区：左右结构 */}
      <details className="mb-4 rounded-xl border border-book-border/50 bg-book-bg-paper/60 open:shadow-[var(--shadow-card)]">
        <summary className="flex cursor-pointer list-none items-center gap-2 px-4 py-3 text-sm font-medium text-book-text-main hover:bg-book-bg-tertiary/40 [&::-webkit-details-marker]:hidden">
          <span className="flex-1">规划配置</span>
          <span className="text-small font-normal text-book-text-muted">
            {options.aspectRatio} · {PROMPT_OUTPUT_LANGUAGE_LABELS[options.promptOutputLanguage]} · {options.durationSecs}s
          </span>
        </summary>
        <div className="border-t border-book-border/40 px-4 py-3">
          <div className="grid gap-4 text-xs lg:grid-cols-[minmax(0,1fr)_minmax(0,1.2fr)]">
            {/* 左侧：参数配置 */}
            <div className="flex flex-col gap-2.5">
              <div className="grid gap-2.5 grid-cols-3">
                <label className="flex min-w-0 flex-col gap-1">
                  <span className="text-book-text-muted">画幅</span>
                  <select
                    value={options.aspectRatio}
                    onChange={(event) => onOptionsChange((previous) => ({
                      ...previous,
                      aspectRatio: event.target.value as VideoGenOptions["aspectRatio"],
                    }))}
                    disabled={busy}
                    className="pyqt-input !px-2 !py-1.5"
                  >
                    {VIDEO_ASPECT_RATIOS.map((ratio) => <option key={ratio} value={ratio}>{ratio}</option>)}
                  </select>
                </label>
                <label className="flex min-w-0 flex-col gap-1">
                  <span className="text-book-text-muted">提示词语言</span>
                  <select
                    value={options.promptOutputLanguage}
                    onChange={(event) => onOptionsChange((previous) => ({
                      ...previous,
                      promptOutputLanguage: event.target.value as VideoGenOptions["promptOutputLanguage"],
                    }))}
                    disabled={busy}
                    className="pyqt-input !px-2 !py-1.5"
                  >
                    {Object.entries(PROMPT_OUTPUT_LANGUAGE_LABELS).map(([value, label]) => (
                      <option key={value} value={value}>{label}</option>
                    ))}
                  </select>
                </label>
                <label className="flex min-w-0 flex-col gap-1">
                  <span className="text-book-text-muted">默认时长</span>
                  <select
                    value={options.durationSecs}
                    onChange={(event) => onOptionsChange((previous) => ({
                      ...previous,
                      durationSecs: Number(event.target.value),
                    }))}
                    disabled={busy}
                    className="pyqt-input !px-2 !py-1.5"
                  >
                    {options.durationOptions.map((duration) => (
                      <option key={duration} value={duration}>{duration} 秒</option>
                    ))}
                  </select>
                </label>
              </div>
              <div className="flex-1 flex flex-col">
                <div className="mb-1 text-book-text-muted">候选时长</div>
                <VideoDurationOptionsPicker
                  options={options}
                  durationLimits={durationLimits}
                  disabled={busy}
                  onOptionsChange={onOptionsChange}
                />
              </div>
            </div>

            {/* 右侧：风格模板 */}
            <label className="flex min-w-0 flex-col gap-1">
              <span className="text-book-text-muted">共享规划风格</span>
              <textarea
                value={options.style}
                onChange={(event) => onOptionsChange((previous) => ({ ...previous, style: event.target.value }))}
                disabled={busy}
                className="book-control flex-1 resize-none p-2 text-xs leading-relaxed"
                placeholder="描述视频的整体风格、氛围、细节要求…"
              />
            </label>
          </div>
        </div>
      </details>

      {/* 片段列表：紧凑卡片 */}
      <div className="min-h-0 flex-1 overflow-y-auto overflow-x-visible custom-scrollbar pr-2">
        {!prompt && !showingPreview ? (
          <EmptyHint text={loadError ? `加载已保存的片段规划失败：${loadError}` : loading ? "正在加载已保存的片段规划…" : busy ? "当前场景的片段规划完整校验并保存后会显示在这里。" : shootingScenes.length === 0 ? "请先生成至少一个拍摄场景。" : "请选择已完成的拍摄场景并生成片段规划。"} />
        ) : segments.length === 0 ? (
          <EmptyHint text="片段规划中没有可展示的片段。" />
        ) : (
          <ol className="space-y-2">
            {segments.map((segment, index) => {
              const number = numberValue(segment.segment_number) ?? index + 1;
              const title = textValue(segment.title) ?? `片段 ${number}`;
              const duration = numberValue(segment.duration_secs);
              const summary = textValue(segment.scene_summary);
              const blocking = textValue(segment.blocking_summary);
              return (
                <li
                  key={`${showingPreview ? "preview" : "saved"}:${number}:${index}`}
                  className="group rounded-lg border border-book-border/40 bg-book-bg-paper/80 px-4 py-3 transition-colors hover:border-book-primary/30 hover:bg-book-bg-tertiary/50"
                >
                  <div className="flex items-start gap-3">
                    <div className="flex shrink-0 items-baseline gap-2">
                      <span className="text-sm font-semibold text-book-primary">#{number}</span>
                      {duration ? (
                        <span className="text-caption text-book-text-muted tabular-nums">{duration}s</span>
                      ) : null}
                    </div>
                    <div className="min-w-0 flex-1">
                      <div className="text-sm font-medium text-book-text-main">{title}</div>
                      {summary ? (
                        <p className="mt-1.5 text-xs leading-relaxed text-book-text-sub line-clamp-2">
                          {summary}
                        </p>
                      ) : null}
                      {blocking ? (
                        <p className="mt-1 text-caption leading-relaxed text-book-text-muted line-clamp-1">
                          走位：{blocking}
                        </p>
                      ) : null}
                    </div>
                  </div>
                </li>
              );
            })}
          </ol>
        )}
      </div>
    </section>
  );
};
