import React, { useMemo } from "react";
import { RefreshCcw, Trash2, Wand2 } from "lucide-react";
import type { ChapterVideoPrompt, VideoWorkflowMode } from "../../../api/video";
import { asJsonRecord as asRecord } from "../../../utils/jsonRecord";
import type { VideoGenOptions } from "./videoTabUtils";

interface VideoShotAnalysisWorkspaceProps {
  prompt: ChapterVideoPrompt | null;
  segmentPlanPrompt: ChapterVideoPrompt | null;
  workflowMode: VideoWorkflowMode;
  options: VideoGenOptions;
  acting: string | null;
  chapterGenerating: boolean;
  onGenerate: (options?: { regenerateExisting?: boolean }) => void;
  onDeletePrompt: (prompt: ChapterVideoPrompt) => void;
}

const textValue = (value: unknown): string | null =>
  typeof value === "string" && value.trim() ? value.trim() : null;

const numberValue = (value: unknown): number | null =>
  typeof value === "number" && Number.isFinite(value) ? value : null;

const booleanValue = (value: unknown): boolean | null =>
  typeof value === "boolean" ? value : null;

const arrayValue = (value: unknown): unknown[] => (Array.isArray(value) ? value : []);

const compactJson = (value: unknown): string => {
  if (value == null) return "无";
  if (typeof value === "string") return value.trim() || "无";
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
};

const workflowLabel = (mode: VideoWorkflowMode): string => {
  if (mode === "first_frame_chain") return "首帧串联";
  if (mode === "storyboard") return "分镜";
  if (mode === "chat_video") return "Chat 生视频";
  if (mode === "reference_media") return "参考媒体";
  return "首尾帧";
};

interface ShotSegmentIndex {
  segments: unknown[];
  segmentByBeatId: Map<number, unknown>;
  segmentByIndexInShot: Map<number, unknown>;
}

const buildShotSegmentIndex = (segments: unknown[]): Map<number, ShotSegmentIndex> => {
  const byShotId = new Map<number, ShotSegmentIndex>();
  for (const segment of segments) {
    const segmentObject = asRecord(segment);
    const shotId = numberValue(segmentObject?.analysis_shot_id);
    if (shotId == null) continue;
    let index = byShotId.get(shotId);
    if (!index) {
      index = {
        segments: [],
        segmentByBeatId: new Map<number, unknown>(),
        segmentByIndexInShot: new Map<number, unknown>(),
      };
      byShotId.set(shotId, index);
    }
    index.segments.push(segment);

    const segmentIndex = numberValue(segmentObject?.segment_index_in_shot);
    if (segmentIndex != null && !index.segmentByIndexInShot.has(segmentIndex)) {
      index.segmentByIndexInShot.set(segmentIndex, segment);
    }
    for (const beatIdValue of arrayValue(segmentObject?.covered_beat_ids)) {
      const beatId = numberValue(beatIdValue);
      if (beatId != null && !index.segmentByBeatId.has(beatId)) {
        index.segmentByBeatId.set(beatId, segment);
      }
    }
  }
  return byShotId;
};

export function VideoShotAnalysisWorkspace({
  prompt,
  segmentPlanPrompt,
  workflowMode,
  options,
  acting,
  chapterGenerating,
  onGenerate,
  onDeletePrompt,
}: VideoShotAnalysisWorkspaceProps) {
  const metadata = asRecord(prompt?.metadata_json);
  const shots = arrayValue(metadata?.cinematic_shots);
  const planningQuality = asRecord(metadata?.planning_quality);
  const totalRequiredSegments = numberValue(planningQuality?.total_required_segments);
  const segmentMetadata = asRecord(segmentPlanPrompt?.metadata_json);
  const segments = arrayValue(segmentMetadata?.segments);
  const segmentIndexByShotId = useMemo(() => buildShotSegmentIndex(segments), [segments]);
  const reusedFromMode =
    prompt != null && prompt.workflow_mode !== workflowMode
      ? workflowLabel(prompt.workflow_mode)
      : null;
  const generating = acting === "generate-shot-analysis";
  const canRegenerate = prompt != null && segmentPlanPrompt == null && !chapterGenerating;
  const regenerateBlockedReason = segmentPlanPrompt
    ? "已存在后续片段规划，请先删除片段规划"
    : chapterGenerating
      ? "章节正在生成中"
      : null;

  return (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-3 overflow-hidden">
      <div className="flex shrink-0 flex-col gap-2 rounded border border-book-border/70 bg-book-bg-paper/70 px-3 py-2 sm:flex-row sm:items-center sm:justify-between">
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <span className="text-sm font-medium text-book-text-main">
              {workflowLabel(workflowMode)}镜头分析
            </span>
            <span className="pyqt-pill !text-[10px]">默认 {options.durationSecs}s</span>
            <span className="pyqt-pill !text-[10px]">
              候选 {options.durationOptions.join(" / ")}s
            </span>
            {prompt ? (
              <span className="pyqt-pill !text-[10px]">镜头 {shots.length}</span>
            ) : (
              <span className="pyqt-pill !text-[10px]">未生成</span>
            )}
            {prompt ? (
              <span className="pyqt-pill !text-[10px]">
                {reusedFromMode ? "全模式共享" : "共享镜头分析"}
              </span>
            ) : null}
            {totalRequiredSegments != null ? (
              <span className="pyqt-pill !text-[10px]">预算 {totalRequiredSegments} 段</span>
            ) : null}
            {segmentPlanPrompt ? (
              <span className="pyqt-pill !text-[10px]">已规划片段</span>
            ) : null}
          </div>
        </div>
        <div className="flex shrink-0 flex-wrap items-center gap-2">
          {!prompt ? (
            <button
              type="button"
              className="pyqt-btn-primary inline-flex !min-h-9 items-center gap-1.5 !px-2.5 text-xs sm:!min-h-[28px]"
              disabled={generating || chapterGenerating}
              onClick={() => onGenerate()}
            >
              <Wand2 size={14} />
              生成镜头分析
            </button>
          ) : (
            <>
              <button
                type="button"
                className="pyqt-btn-secondary inline-flex !min-h-9 items-center gap-1.5 !px-2.5 text-xs sm:!min-h-[28px]"
                disabled={generating || !canRegenerate}
                title={regenerateBlockedReason ?? undefined}
                onClick={() => onGenerate({ regenerateExisting: true })}
              >
                <RefreshCcw size={14} />
                重新生成
              </button>
              <button
                type="button"
                className="pyqt-btn-secondary inline-flex !min-h-9 items-center gap-1.5 !border-error/50 !px-2.5 text-xs !text-error hover:!border-error/70 hover:!bg-error/10 sm:!min-h-[28px]"
                disabled={generating || chapterGenerating}
                title={
                  segmentPlanPrompt
                    ? "已存在后续片段规划；删除时需要选择如何处理下游"
                    : regenerateBlockedReason ?? undefined
                }
                onClick={() => onDeletePrompt(prompt)}
              >
                <Trash2 size={14} />
                删除
              </button>
            </>
          )}
        </div>
      </div>

      {!prompt ? (
        <div className="flex min-h-0 min-w-0 flex-1 items-center justify-center rounded border border-dashed border-book-border/70 bg-book-bg-paper/40 p-6 text-sm text-book-text-muted">
          当前章节还没有镜头语言分析。
        </div>
      ) : (
        <div className="min-h-0 min-w-0 flex-1 overflow-hidden rounded border border-book-border/70 bg-book-bg-paper/55">
          <div className="h-full min-w-0 overflow-y-auto overflow-x-hidden custom-scrollbar">
            <div className="divide-y divide-book-border/60">
              {shots.map((shot, index) => {
                const shotObject = asRecord(shot) ?? {};
                const shotId = numberValue(shotObject.shot_id) ?? index + 1;
                const beats = arrayValue(shotObject.action_beats);
                const breakdown = arrayValue(shotObject.segment_breakdown);
                const riskFlags = arrayValue(shotObject.risk_flags);
                const sourceSpanIds = arrayValue(shotObject.source_span_ids);
                const requiredSegmentCount =
                  numberValue(shotObject.required_segment_count) ??
                  numberValue(shotObject.recommended_segment_count) ??
                  breakdown.length;
                const currentShotSegmentIndex: ShotSegmentIndex | undefined =
                  segmentIndexByShotId.get(shotId);
                return (
                  <section key={`${shotId}:${index}`} className="min-w-0 p-3 sm:p-4">
                    <div className="min-w-0">
                      <div className="flex min-w-0 flex-wrap items-center gap-2">
                        <span className="shrink-0 font-mono text-xs text-book-text-muted">
                          #{shotId}
                        </span>
                        <h3 className="min-w-[8rem] flex-1 truncate text-sm font-medium text-book-text-main">
                          {textValue(shotObject.title) ?? `镜头 ${shotId}`}
                        </h3>
                        <span className="pyqt-pill shrink-0 !text-[10px]">
                          {requiredSegmentCount} 段
                        </span>
                        <span className="pyqt-pill shrink-0 !text-[10px]">
                          估算 {numberValue(shotObject.estimated_natural_seconds) ?? "-"}s
                        </span>
                        {beats.length > 0 ? (
                          <span className="pyqt-pill shrink-0 !text-[10px]">
                            节拍 {beats.length}
                          </span>
                        ) : null}
                        <span className="pyqt-pill shrink-0 !text-[10px]">
                          {textValue(shotObject.action_load) ?? "load"}
                        </span>
                        {riskFlags.length > 0 ? (
                          <span className="pyqt-pill shrink-0 !border-warning/50 !text-[10px] !text-warning">
                            风险 {riskFlags.length}
                          </span>
                        ) : null}
                      </div>
                      <p className="mt-1 break-words text-xs leading-relaxed text-book-text-muted">
                        {textValue(shotObject.scene_intent) ?? "未提供镜头意图"}
                      </p>
                      <div className="mt-2 flex min-w-0 items-start gap-2">
                        <span className="mt-0.5 shrink-0 text-[10px] font-medium text-book-text-muted">
                          来源
                        </span>
                        <div className="flex max-h-14 min-w-0 flex-1 flex-wrap gap-1 overflow-y-auto overflow-x-hidden text-[10px] text-book-text-muted custom-scrollbar">
                          {sourceSpanIds.map((id, idIndex) => (
                          <span
                            key={`${idIndex}:${String(id)}`}
                            className="pyqt-pill shrink-0 !text-[10px]"
                          >
                            {String(id)}
                          </span>
                          ))}
                        </div>
                      </div>
                    </div>

                    <div className="mt-3 grid gap-2 lg:grid-cols-3">
                      <InfoBlock title="状态">
                        {compactJson(shotObject.start_state)}
                        {"\n→\n"}
                        {compactJson(shotObject.end_state)}
                      </InfoBlock>
                      <InfoBlock title="走位">
                        {compactJson(shotObject.blocking)}
                      </InfoBlock>
                      <InfoBlock title="运镜">
                        {compactJson(shotObject.camera_intent)}
                      </InfoBlock>
                      <InfoBlock title="转场">
                        {textValue(shotObject.transition_in) ?? "无"} →{" "}
                        {textValue(shotObject.transition_out) ?? "无"}
                      </InfoBlock>
                    </div>

                    {riskFlags.length > 0 ? (
                      <div className="mt-3 flex flex-col gap-1 rounded border border-warning/40 bg-warning/10 px-2 py-1.5 text-xs text-warning">
                        {riskFlags.map((risk, riskIndex) => {
                          const riskObject = asRecord(risk) ?? {};
                          return (
                            <div key={`${shotId}:risk:${riskIndex}`} className="break-words">
                              {textValue(riskObject.message) ??
                                textValue(riskObject.code) ??
                                "镜头规划风险"}
                            </div>
                          );
                        })}
                      </div>
                    ) : null}

                    {beats.length > 0 ? (
                      <div className="mt-3 min-w-0 overflow-x-auto rounded border border-book-border/60 custom-scrollbar">
                        <div className="grid min-w-[34rem] grid-cols-[52px_72px_minmax(0,1fr)_minmax(8rem,14rem)] gap-2 bg-book-bg-muted/50 px-2 py-1 text-[11px] font-medium text-book-text-muted">
                          <span>节拍</span>
                          <span>耗时</span>
                          <span>动作</span>
                          <span>来源</span>
                        </div>
                        {beats.map((item, itemIndex) => {
                          const row = asRecord(item) ?? {};
                          const beatId = numberValue(row.beat_id) ?? itemIndex + 1;
                          const beatSourceSpanIds = arrayValue(row.source_span_ids);
                          const mapped = currentShotSegmentIndex?.segmentByBeatId.get(beatId);
                          const mappedObject = asRecord(mapped);
                          const required =
                            booleanValue(row.required) ?? booleanValue(row.must_show) ?? false;
                          return (
                            <div
                              key={`${shotId}:beat:${beatId}:${itemIndex}`}
                              className="grid min-w-[34rem] grid-cols-[52px_72px_minmax(0,1fr)_minmax(8rem,14rem)] gap-2 border-t border-book-border/50 px-2 py-2 text-xs leading-relaxed text-book-text-main"
                            >
                              <span className="min-w-0 break-words font-mono text-book-text-muted">
                                {mappedObject ? `#${mappedObject.segment_number}` : `B${beatId}`}
                              </span>
                              <span className="min-w-0 break-words text-book-text-muted">
                                {numberValue(row.natural_seconds) ?? "-"}s
                                {required ? " 必需" : ""}
                              </span>
                              <span className="min-w-0 break-words">
                                {textValue(row.visible_action) ?? "未写动作"}
                              </span>
                              <span className="flex min-w-0 flex-wrap gap-1 text-book-text-sub">
                                {beatSourceSpanIds.slice(0, 3).map((id, idIndex) => (
                                  <span
                                    key={`${shotId}:beat:${beatId}:span:${idIndex}`}
                                    className="pyqt-pill shrink-0 !text-[10px]"
                                  >
                                    {String(id)}
                                  </span>
                                ))}
                                {beatSourceSpanIds.length > 3 ? (
                                  <span className="pyqt-pill shrink-0 !text-[10px]">
                                    +{beatSourceSpanIds.length - 3}
                                  </span>
                                ) : null}
                              </span>
                            </div>
                          );
                        })}
                      </div>
                    ) : (
                      <div className="mt-3 min-w-0 overflow-x-auto rounded border border-book-border/60 custom-scrollbar">
                        <div className="grid min-w-[34rem] grid-cols-[48px_minmax(0,1fr)_minmax(0,1fr)_minmax(0,1fr)] gap-2 bg-book-bg-muted/50 px-2 py-1 text-[11px] font-medium text-book-text-muted">
                          <span>段</span>
                          <span>目标</span>
                          <span>动作 / 运镜</span>
                          <span>交接</span>
                        </div>
                        {breakdown.map((item, itemIndex) => {
                          const row = asRecord(item) ?? {};
                          const segmentIndexInShot =
                            numberValue(row.segment_index_in_shot) ?? itemIndex + 1;
                          const mapped =
                            currentShotSegmentIndex?.segmentByIndexInShot.get(segmentIndexInShot);
                          const mappedObject = asRecord(mapped);
                          return (
                            <div
                              key={`${shotId}:${segmentIndexInShot}:${itemIndex}`}
                              className="grid min-w-[34rem] grid-cols-[48px_minmax(0,1fr)_minmax(0,1fr)_minmax(0,1fr)] gap-2 border-t border-book-border/50 px-2 py-2 text-xs leading-relaxed text-book-text-main"
                            >
                              <span className="min-w-0 break-words font-mono text-book-text-muted">
                                {mappedObject
                                  ? `#${mappedObject.segment_number}`
                                  : `${segmentIndexInShot}`}
                              </span>
                              <span className="min-w-0 break-words">
                                {textValue(row.segment_goal) ?? "未写目标"}
                              </span>
                              <span className="min-w-0 break-words">
                                {textValue(row.main_visible_action) ?? "未写动作"}
                                <span className="mx-1 text-book-text-muted">/</span>
                                {textValue(row.camera_movement) ?? "未写运镜"}
                              </span>
                              <span className="min-w-0 break-words">
                                {textValue(row.handoff_to_next_segment) ?? "未写交接"}
                              </span>
                            </div>
                          );
                        })}
                      </div>
                    )}
                  </section>
                );
              })}
            </div>
            <details className="border-t border-book-border/60 p-3">
              <summary className="cursor-pointer text-xs font-medium text-book-text-muted">
                原始 JSON
              </summary>
              <pre className="mt-2 max-h-80 overflow-auto rounded bg-book-bg-muted/60 p-3 text-[11px] leading-relaxed text-book-text-main">
                {JSON.stringify(
                  metadata?.cinematic_shots ?? metadata ?? prompt.metadata_json,
                  null,
                  2,
                )}
              </pre>
            </details>
          </div>
        </div>
      )}
    </div>
  );
}

function InfoBlock({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="min-w-0 rounded border border-book-border/60 bg-book-bg-paper/70 p-2">
      <div className="mb-1 text-[11px] font-medium text-book-text-muted">{title}</div>
      <pre className="whitespace-pre-wrap break-words text-xs leading-relaxed text-book-text-main">
        {children}
      </pre>
    </div>
  );
}
