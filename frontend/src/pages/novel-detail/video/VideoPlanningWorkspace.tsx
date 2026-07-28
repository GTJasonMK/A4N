import React, { useMemo } from "react";
import { ListChecks, Loader2, RotateCcw, Trash2, Wand2 } from "lucide-react";
import type { ChapterVideoPrompt } from "../../../api/video";
import { BookButton } from "../../../components/ui/BookButton";
import { asArray, asJsonRecord } from "../../../utils/jsonRecord";
import { getOperationStatusLabel } from "../../../utils/workflowPresentation";
import { EmptyHint } from "../shared";

interface VideoPlanningWorkspaceProps {
  prompt: ChapterVideoPrompt | null;
  acting: string | null;
  chapterGenerating: boolean;
  onGenerate: (regenerateExisting: boolean) => void | Promise<void>;
  onDelete: (prompt: ChapterVideoPrompt) => void | Promise<void>;
}

const textValue = (value: unknown): string | null =>
  typeof value === "string" && value.trim() ? value.trim() : null;

const numberValue = (value: unknown): number | null =>
  typeof value === "number" && Number.isFinite(value) ? value : null;

export const VideoPlanningWorkspace: React.FC<VideoPlanningWorkspaceProps> = ({
  prompt,
  acting,
  chapterGenerating,
  onGenerate,
  onDelete,
}) => {
  const segments = useMemo(
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
  const busy = acting === "generate-segment-plan" || chapterGenerating;

  return (
    <section className="flex h-full min-h-0 flex-col" aria-label="视频片段规划">
      <div className="mb-3 flex min-w-0 flex-wrap items-center gap-2 border-b border-book-border/50 pb-3">
        <div className="min-w-0 flex-1">
          <div className="inline-flex items-center gap-1.5 text-sm font-medium text-book-text-main">
            <ListChecks size={14} className="text-book-primary" />
            片段规划
          </div>
          <div className="mt-1 text-[11px] text-book-text-muted">
            {prompt
              ? `${getOperationStatusLabel(prompt.status)} · ${segments.length} 个片段`
              : "根据章节内容建立共享片段顺序，之后再生成各片段提示词。"}
          </div>
        </div>
        <BookButton
          variant="primary"
          size="sm"
          onClick={() => void onGenerate(Boolean(prompt))}
          disabled={busy}
        >
          {busy ? <Loader2 size={12} className="animate-spin" /> : prompt ? <RotateCcw size={12} /> : <Wand2 size={12} />}
          {prompt ? "重新规划" : "生成片段规划"}
        </BookButton>
        {prompt ? (
          <BookButton
            variant="secondary"
            size="sm"
            onClick={() => void onDelete(prompt)}
            disabled={busy}
            title="删除片段规划及其受影响的下游记录"
          >
            <Trash2 size={12} />
          </BookButton>
        ) : null}
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto custom-scrollbar pr-1">
        {!prompt ? (
          <EmptyHint text="当前章节还没有片段规划。" />
        ) : segments.length === 0 ? (
          <EmptyHint text="片段规划中没有可展示的片段。" />
        ) : (
          <ol className="divide-y divide-book-border/40 border-y border-book-border/50">
            {segments.map((segment, index) => {
              const number = numberValue(segment.segment_number) ?? index + 1;
              const title = textValue(segment.title) ?? `片段 ${number}`;
              const duration = numberValue(segment.duration_secs);
              const summary = textValue(segment.scene_summary);
              const blocking = textValue(segment.blocking_summary);
              return (
                <li key={`${number}:${index}`} className="grid gap-2 py-3 sm:grid-cols-[64px_minmax(0,1fr)_auto]">
                  <div className="text-xs font-medium text-book-primary">片段 {number}</div>
                  <div className="min-w-0">
                    <div className="text-xs font-medium text-book-text-main">{title}</div>
                    {summary ? <p className="mt-1 text-[11px] leading-relaxed text-book-text-sub">{summary}</p> : null}
                    {blocking ? <p className="mt-1 text-[10px] leading-relaxed text-book-text-muted">走位：{blocking}</p> : null}
                  </div>
                  <div className="text-[10px] text-book-text-muted">{duration ? `${duration} 秒` : ""}</div>
                </li>
              );
            })}
          </ol>
        )}
      </div>
    </section>
  );
};
