import React from "react";
import { Check, Info, Sparkles } from "lucide-react";
import { BookButton } from "../../../components/ui/BookButton";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import {
  formatImportCandidateSourceScope,
  parseImportBlueprintCandidate,
} from "./importTabUtils";
import { RESULT_LABELS } from "./ImportToolsPanel";

interface ImportBlueprintCandidatePanelProps {
  proposal: unknown;
  projectId: number;
  busy: boolean;
  disabled?: boolean;
  onApply: () => void | Promise<void>;
}

export const ImportBlueprintCandidatePanel: React.FC<ImportBlueprintCandidatePanelProps> = ({
  proposal,
  projectId,
  busy,
  disabled = false,
  onApply,
}) => {
  const candidate = parseImportBlueprintCandidate(proposal, projectId);
  if (!candidate) return null;

  const confirmed = candidate.state === "confirmed";
  const updatedFields = confirmed ? candidate.apply_result.updated_fields : undefined;
  return (
    <section
      aria-labelledby="import-blueprint-candidate-title"
      className="a4n-flat-surface space-y-3 p-3 text-xs"
    >
      <div className="flex items-start gap-2">
        <div className="mt-0.5 rounded-lg bg-book-primary/10 p-1.5 text-book-primary">
          {confirmed ? <Check size={14} /> : <Sparkles size={14} />}
        </div>
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <h3
              id="import-blueprint-candidate-title"
              className="font-semibold text-book-text-main"
            >
              蓝图候选
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
                正式版本 {candidate.apply_result.version}
              </span>
            ) : null}
          </div>
          <p className="mt-1 text-small leading-relaxed text-book-text-muted">
            这是根据当前导入章节整理的蓝图候选，不是正式蓝图；确认后才会写入正式版本。
          </p>
          {updatedFields ? (
            <p className="mt-1 text-caption leading-relaxed text-book-text-muted">
              实际更新：
              {updatedFields.length > 0
                ? updatedFields.map((field) => RESULT_LABELS[field] ?? field).join("、")
                : "蓝图顶层字段无变化"}
            </p>
          ) : null}
        </div>
      </div>

      <dl className="divide-y divide-book-border/40 border-y border-book-border/40">
        <CandidateRow label="标题" value={candidate.title} />
        <CandidateRow label="题材" value={candidate.genre} />
        <CandidateRow label="一句话概要" value={candidate.one_sentence_summary} />
        <CandidateRow
          label="章节范围"
          value={formatImportCandidateSourceScope(candidate.source_scope)}
        />
      </dl>

      <details className="border-b border-book-border/40 pb-2">
        <summary className="cursor-pointer py-1 text-small text-book-text-sub">
          查看完整梗概
        </summary>
        <p className="whitespace-pre-wrap break-words pt-2 leading-relaxed text-book-text-sub">
          {candidate.full_synopsis}
        </p>
      </details>

      <details>
        <summary className="cursor-pointer py-1 text-small text-book-text-sub">
          查看世界观整理
        </summary>
        <pre className="mt-2 max-h-48 overflow-auto whitespace-pre-wrap break-words rounded-lg bg-book-bg-tertiary/45 p-2 font-mono text-caption leading-relaxed text-book-text-sub">
          {JSON.stringify(candidate.world_setting, null, 2)}
        </pre>
      </details>

      <div className="flex items-start gap-1.5 text-caption leading-relaxed text-book-text-muted">
        <Info size={12} className="mt-0.5 shrink-0" />
        <span>正文或章节发生变化后候选会失效，需要重新分析；确认不会删除已有角色和人物关系。</span>
      </div>

      <div className="flex justify-end">
        {confirmed ? (
          <span className="inline-flex items-center gap-1.5 text-small text-emerald-700">
            <Check size={13} /> 已应用到正式蓝图
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
              "确认应用到正式蓝图"
            )}
          </BookButton>
        )}
      </div>
    </section>
  );
};

const CandidateRow: React.FC<{ label: string; value: string }> = ({ label, value }) => (
  <div className="grid gap-1 px-2 py-2 sm:grid-cols-[72px_minmax(0,1fr)] sm:gap-3">
    <dt className="font-medium text-book-text-muted">{label}</dt>
    <dd className="min-w-0 whitespace-pre-wrap break-words leading-relaxed text-book-text-sub">
      {value.trim()}
    </dd>
  </div>
);
