import React from "react";
import { Check, Info, Layers3 } from "lucide-react";
import { BookButton } from "../../../components/ui/BookButton";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import type { ImportPartOutlineCandidateItem } from "../../../api/import";
import {
  formatImportCandidateSourceScope,
  parseImportPartOutlineCandidate,
} from "./importTabUtils";

interface ImportPartOutlineCandidatePanelProps {
  proposal: unknown;
  projectId: number;
  busy: boolean;
  disabled?: boolean;
  onApply: () => void | Promise<void>;
}

export const ImportPartOutlineCandidatePanel: React.FC<
  ImportPartOutlineCandidatePanelProps
> = ({ proposal, projectId, busy, disabled = false, onApply }) => {
  const candidate = parseImportPartOutlineCandidate(proposal, projectId);
  if (!candidate) return null;

  const confirmed = candidate.state === "confirmed";
  return (
    <section
      aria-labelledby="import-part-outline-candidate-title"
      className="a4n-flat-surface space-y-3 p-3 text-xs"
    >
      <div className="flex items-start gap-2">
        <div className="mt-0.5 rounded-lg bg-book-primary/10 p-1.5 text-book-primary">
          {confirmed ? <Check size={14} /> : <Layers3 size={14} />}
        </div>
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <h3
              id="import-part-outline-candidate-title"
              className="font-semibold text-book-text-main"
            >
              分部大纲候选
            </h3>
            <span
              className={`rounded-full px-2 py-0.5 text-caption ${
                confirmed
                  ? "bg-emerald-500/10 text-emerald-700"
                  : "bg-amber-500/10 text-amber-700"
              }`}
            >
              {confirmed ? "已应用" : "待确认"}
            </span>
            {confirmed ? (
              <span className="text-caption text-book-text-muted">
                新增 {candidate.apply_result.inserted_count} · 更新{" "}
                {candidate.apply_result.updated_count ?? 0} · 未变化{" "}
                {candidate.apply_result.reused_count}
              </span>
            ) : null}
          </div>
          <p className="mt-1 text-small leading-relaxed text-book-text-muted">
            根据已写正文整理，确认前不会参与后续章纲或正文生成。
          </p>
        </div>
      </div>

      <div className="divide-y divide-book-border/40 border-y border-book-border/40">
        {candidate.parts.map((part) => (
          <PartCandidateRow key={part.part_number} part={part} />
        ))}
      </div>

      <div className="flex items-start gap-1.5 text-caption leading-relaxed text-book-text-muted">
        <Info size={12} className="mt-0.5 shrink-0" />
        <span>
          规划范围可能包含尚未写作的章节，实际证据范围已单独标出；确认后会按分部编号更新候选覆盖的分部，未覆盖分部保留，范围重叠会明确拒绝。
        </span>
      </div>

      <div className="flex justify-end">
        {confirmed ? (
          <span className="inline-flex items-center gap-1.5 text-small text-emerald-700">
            <Check size={13} /> 已应用到正式分部大纲
          </span>
        ) : (
          <BookButton
            variant="primary"
            size="sm"
            disabled={disabled || busy}
            onClick={() => void onApply()}
            className="!min-h-8 !text-xs"
          >
            {busy ? (
              <span className="inline-flex items-center gap-1.5">
                <LoadingSpinner size="sm" /> 应用中…
              </span>
            ) : (
              "确认应用到正式分部大纲"
            )}
          </BookButton>
        )}
      </div>
    </section>
  );
};

const PartCandidateRow: React.FC<{ part: ImportPartOutlineCandidateItem }> = ({ part }) => (
  <details className="px-2 py-2">
    <summary className="cursor-pointer list-none">
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="font-medium text-book-text-main">
            第 {part.part_number} 部 · {part.title}
          </div>
          <div className="mt-0.5 text-caption text-book-text-muted">
            规划第 {part.start_chapter}–{part.end_chapter} 章 · 证据
            {formatImportCandidateSourceScope(part.source_scope)}
          </div>
        </div>
        <span className="shrink-0 text-caption text-book-text-muted">查看摘要</span>
      </div>
    </summary>
    <p className="mt-2 whitespace-pre-wrap break-words leading-relaxed text-book-text-sub">
      {part.summary}
    </p>
  </details>
);
