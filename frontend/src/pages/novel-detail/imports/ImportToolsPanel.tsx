import { ThemedSelect } from "../../../components/ui/ThemedSelect";
import React from "react";
import { Check, ChevronDown, CopyCheck, Eye, GitBranch } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { BookButton } from "../../../components/ui/BookButton";
import { Dropdown } from "../../../components/ui/Dropdown";
import type {
  DedupResult,
  ReverseEngineeringCandidate,
  ReverseEngineeringTarget,
} from "../../../api/import";
import { useI18n, type MessageKey, type MessageTranslator } from "../../../i18n";

interface ImportToolsPanelProps {
  dedupText: string;
  dedupResult: DedupResult | null;
  dedupBusy: boolean;
  reverseType: ReverseEngineeringTarget;
  reverseSample: string;
  reverseAutoContext: boolean;
  reverseResult: ReverseEngineeringCandidate | null;
  reverseGenerating: boolean;
  reverseConfirmBusy: boolean;
  externalBusy: boolean;
  onDedupTextChange: (value: string) => void;
  onDedup: () => void | Promise<void>;
  onReverseTypeChange: (value: ReverseEngineeringTarget) => void;
  onReverseSampleChange: (value: string) => void;
  onReverseAutoContextChange: (value: boolean) => void;
  onPreviewReverseRequest: () => void | Promise<void>;
  onReverseEngineer: () => void | Promise<void>;
  onConfirmReverse: () => void | Promise<void>;
}

export const ImportToolsPanel: React.FC<ImportToolsPanelProps> = ({
  dedupText,
  dedupResult,
  dedupBusy,
  reverseType,
  reverseSample,
  reverseAutoContext,
  reverseResult,
  reverseGenerating,
  reverseConfirmBusy,
  externalBusy,
  onDedupTextChange,
  onDedup,
  onReverseTypeChange,
  onReverseSampleChange,
  onReverseAutoContextChange,
  onPreviewReverseRequest,
  onReverseEngineer,
  onConfirmReverse,
}) => {
  const { t } = useI18n();
  const toolsBusy = externalBusy || dedupBusy || reverseGenerating || reverseConfirmBusy;
  const dedupReady = dedupText.trim().length > 0;
  const reverseReady = reverseAutoContext || reverseSample.trim().length > 0;
  const reverseCandidate = reverseResult;
  const reverseLabel =
    reverseType === "blueprint"
      ? t("novel.importReview.blueprint")
      : reverseType === "outline"
        ? t("novel.importReview.chapterOutline")
        : t("novel.importReview.partOutline");

  return (
    <section aria-labelledby="import-tools-title" className="space-y-3 text-xs">
      <div>
        <h3 id="import-tools-title" className="text-sm font-semibold text-book-text-main">
          {t("novel.importReview.tools.title")}
        </h3>
        <p className="mt-0.5 text-small text-book-text-muted">
          {t("novel.importReview.tools.description")}
        </p>
      </div>
      <details className="group a4n-flat-surface overflow-hidden">
        <summary className="grid cursor-pointer list-none grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-2.5 px-3.5 py-3 text-book-text-main [&::-webkit-details-marker]:hidden">
          <CopyCheck size={15} className="text-book-text-muted" />
          <span className="min-w-0">
            <span className="block text-xs font-semibold">{t("novel.importReview.tools.dedupTitle")}</span>
            <span className="mt-0.5 block truncate text-small font-normal text-book-text-muted">
              {t("novel.importReview.tools.dedupDescription")}
            </span>
          </span>
          <ChevronDown
            size={14}
            className="text-book-text-muted transition-transform group-open:rotate-180"
          />
        </summary>
        <div className="space-y-2 border-t border-book-border/45 p-4">
          <textarea
            value={dedupText}
            onChange={(e) => onDedupTextChange(e.target.value)}
            disabled={toolsBusy}
            placeholder={t("novel.importReview.tools.dedupPlaceholder")}
            className="pyqt-input h-20 w-full resize-y"
          />
          <div className="flex justify-end">
            <BookButton
              variant="secondary"
              size="sm"
              onClick={() => void onDedup()}
              disabled={toolsBusy || !dedupReady}
              className="!min-h-8 !text-xs"
            >
              {dedupBusy ? (
                <span className="inline-flex items-center gap-1.5">
                  <LoadingSpinner size="sm" /> {t("novel.importReview.tools.checking")}
                </span>
              ) : (
                t("novel.importReview.tools.check")
              )}
            </BookButton>
          </div>
          {dedupResult != null && (
            <details className="border-t border-book-border/40 pt-2">
              <summary className="cursor-pointer py-1 text-small text-book-text-sub">
                {t("novel.importReview.tools.viewDedup")}
              </summary>
              <div className="pt-2">
                <ImportToolResult value={dedupResult} />
              </div>
            </details>
          )}
        </div>
      </details>

      <details className="group a4n-flat-surface overflow-hidden">
        <summary className="grid cursor-pointer list-none grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-2.5 px-3.5 py-3 text-book-text-main [&::-webkit-details-marker]:hidden">
          <GitBranch size={15} className="text-book-text-muted" />
          <span className="min-w-0">
            <span className="block text-xs font-semibold">{t("novel.importReview.tools.reverseTitle")}</span>
            <span className="mt-0.5 block truncate text-small font-normal text-book-text-muted">
              {t("novel.importReview.tools.reverseDescription")}
            </span>
          </span>
          <ChevronDown
            size={14}
            className="text-book-text-muted transition-transform group-open:rotate-180"
          />
        </summary>
        <div className="space-y-2 border-t border-book-border/45 p-4">
          <p className="text-small leading-relaxed text-book-text-muted">
            {reverseType === "blueprint"
              ? t("novel.importReview.tools.blueprintDescription")
              : t("novel.importReview.tools.outlineDescription")}
          </p>
          <div className="flex flex-wrap items-center gap-2">
            <ThemedSelect
              value={reverseType}
              disabled={toolsBusy}
              onChange={(selectedValue) =>
                onReverseTypeChange(selectedValue as ReverseEngineeringTarget)
              }
              className="pyqt-input !px-2 !py-1 text-xs"
            >
              <option value="blueprint">{t("novel.importReview.blueprint")}</option>
              <option value="outline">{t("novel.importReview.chapterOutline")}</option>
              <option value="part_outline">{t("novel.importReview.partOutline")}</option>
            </ThemedSelect>
            <button
              type="button"
              role="switch"
              aria-checked={reverseAutoContext}
              disabled={toolsBusy}
              onClick={() => onReverseAutoContextChange(!reverseAutoContext)}
              className="inline-flex items-center gap-1.5 text-book-text-muted disabled:opacity-50"
              title={reverseAutoContext ? t("novel.importReview.tools.closeContext") : t("novel.importReview.tools.useContext")}
            >
              <span
                className={`relative inline-flex h-[18px] w-8 shrink-0 rounded-full transition-colors ${
                  reverseAutoContext ? "bg-book-primary" : "bg-book-text-muted/25"
                }`}
              >
                <span
                  className={`absolute top-[2px] h-3.5 w-3.5 rounded-full bg-white shadow-sm transition-transform ${
                    reverseAutoContext ? "translate-x-4" : "translate-x-[2px]"
                  }`}
                />
              </span>
              {t("novel.importReview.tools.currentMaterials")}
            </button>
          </div>
          <textarea
            value={reverseSample}
            disabled={toolsBusy}
            onChange={(e) => onReverseSampleChange(e.target.value)}
            placeholder={
              reverseAutoContext
                ? t("novel.importReview.tools.contextPlaceholder")
                : t("novel.importReview.tools.samplePlaceholder")
            }
            className="pyqt-input h-28 w-full resize-y"
          />
          <div className="flex items-center justify-end gap-2">
            {reverseGenerating ? (
              <span className="inline-flex items-center gap-1.5 text-small text-book-text-muted">
                <LoadingSpinner size="sm" /> {t("novel.importReview.tools.generatingResult")}
              </span>
            ) : null}
            {!toolsBusy && reverseReady ? (
              <Dropdown
                items={[
                  {
                    label: t("novel.importReview.tools.viewRequest"),
                    icon: <Eye size={11} />,
                    onClick: () => void onPreviewReverseRequest(),
                  },
                ]}
              />
            ) : null}
            <BookButton
              variant="primary"
              onClick={() => void onReverseEngineer()}
              disabled={toolsBusy || !reverseReady}
            >
              {reverseGenerating ? (
                <span className="inline-flex items-center gap-1.5">
                  <LoadingSpinner size="sm" /> {t("novel.importReview.tools.generating")}
                </span>
              ) : (
                reverseType === "blueprint"
                  ? t("novel.importReview.tools.generateBlueprint")
                  : t("novel.importReview.tools.generateCandidate", { label: reverseLabel })
              )}
            </BookButton>
          </div>
          {reverseResult != null && (
            <div className="border-t border-book-border/40 pt-2">
              <div className="flex flex-wrap items-center justify-between gap-2 py-1">
                <span className="text-small text-book-text-sub">
                  {reverseType === "blueprint"
                    ? t("novel.importReview.tools.formalUpdate")
                    : reverseCandidate?.state === "proposal"
                      ? t("novel.importReview.tools.candidatePending", { label: reverseLabel })
                      : reverseCandidate
                        ? t("novel.importReview.tools.candidateApplied", { label: reverseLabel })
                        : t("novel.importReview.tools.candidate", { label: reverseLabel })}
                </span>
                {reverseType === "blueprint" && reverseCandidate ? (
                  <span className="inline-flex items-center gap-1 text-small text-emerald-700">
                    <Check size={13} /> {t("novel.importReview.tools.written")}
                  </span>
                ) : reverseCandidate?.state === "proposal" ? (
                  <BookButton
                    variant="secondary"
                    size="sm"
                    disabled={toolsBusy}
                    onClick={() => void onConfirmReverse()}
                    className="!min-h-8 !text-xs"
                  >
                    {reverseConfirmBusy ? (
                      <span className="inline-flex items-center gap-1.5">
                        <LoadingSpinner size="sm" /> {t("novel.importReview.tools.confirming")}
                      </span>
                    ) : (
                      t("novel.importReview.tools.confirmUpdate", { label: reverseLabel })
                    )}
                  </BookButton>
                ) : reverseCandidate ? (
                  <span className="inline-flex items-center gap-1 text-small text-emerald-700">
                    <Check size={13} /> {t("novel.importReview.tools.confirmed")}
                  </span>
                ) : null}
              </div>
              {reverseCandidate?.applied != null ? (
                <details open className="border-t border-book-border/40 pt-2">
                  <summary className="cursor-pointer py-1 text-small text-book-text-sub">
                    {t("novel.importReview.tools.viewApplied")}
                  </summary>
                  <div className="pt-2">
                    <ImportToolResult value={reverseCandidate.applied} />
                  </div>
                </details>
              ) : null}
              <details>
                <summary className="cursor-pointer py-1 text-small text-book-text-muted">
                  {reverseType === "blueprint"
                    ? t("novel.importReview.tools.viewBlueprintContent")
                    : t("novel.importReview.tools.viewCandidateContent")}
                </summary>
                <div className="pt-2">
                  <ImportToolResult value={reverseResult.result} />
                </div>
              </details>
            </div>
          )}
        </div>
      </details>
    </section>
  );
};

const RESULT_LABEL_KEYS: Record<string, MessageKey> = {
  original_count: "novel.importReview.result.originalCount",
  unique_count: "novel.importReview.result.uniqueCount",
  duplicate_count: "novel.importReview.result.duplicateCount",
  removed_count: "novel.importReview.result.removedCount",
  duplicates: "novel.importReview.result.duplicates",
  result: "novel.importReview.result.result",
  summary: "novel.importReview.result.summary",
  blueprint: "novel.importReview.blueprint",
  outline: "novel.importReview.chapterOutline",
  part_outline: "novel.importReview.partOutline",
  title: "novel.importReview.result.title",
  genre: "novel.importReview.result.genre",
  tone: "novel.importReview.result.tone",
  style: "novel.importReview.result.style",
  full_synopsis: "novel.importReview.result.fullSynopsis",
  target_audience: "novel.importReview.result.targetAudience",
  one_sentence_summary: "novel.importReview.result.oneSentenceSummary",
  total_chapters: "novel.importReview.result.totalChapters",
  needs_part_outlines: "novel.importReview.result.needsPartOutlines",
  chapters_per_part: "novel.importReview.result.chaptersPerPart",
  world_setting: "novel.importReview.result.worldSetting",
  characters: "novel.importReview.result.characters",
  relationships: "novel.importReview.result.relationships",
  updated_fields: "novel.importReview.result.updatedFields",
  blueprint_applied: "novel.importReview.result.blueprintApplied",
  outline_count: "novel.importReview.result.outlineCount",
  character_count: "novel.importReview.result.characterCount",
  relationship_count: "novel.importReview.result.relationshipCount",
  character_inserted: "novel.importReview.result.characterInserted",
  character_updated: "novel.importReview.result.characterUpdated",
  character_preserved_conflicts: "novel.importReview.result.characterPreservedConflicts",
  relationship_inserted: "novel.importReview.result.relationshipInserted",
  relationship_updated: "novel.importReview.result.relationshipUpdated",
  relationship_preserved_conflicts: "novel.importReview.result.relationshipPreservedConflicts",
  part_outline_count: "novel.importReview.result.partOutlineCount",
  inserted_count: "novel.importReview.result.insertedCount",
  updated_count: "novel.importReview.result.updatedCount",
  reused_count: "novel.importReview.result.reusedCount",
};

const ImportToolResult: React.FC<{ value: unknown }> = ({ value }) => {
  const { t } = useI18n();
  const entries = isPlainRecord(value) ? Object.entries(value) : [];
  if (entries.length === 0) {
    return <div className="whitespace-pre-wrap break-words text-small leading-relaxed text-book-text-sub">{formatResultValue(value, t)}</div>;
  }
  return (
    <div className="space-y-2">
      <dl className="divide-y divide-book-border/40 border-y border-book-border/40">
        {entries.slice(0, 16).map(([key, entryValue], index) => (
          <div key={key} className="grid gap-1 px-2.5 py-2 text-small sm:grid-cols-[88px_minmax(0,1fr)] sm:gap-3">
            <dt className="font-medium text-book-text-muted">
              {RESULT_LABEL_KEYS[key] ? t(RESULT_LABEL_KEYS[key]) : t("novel.importReview.result.unknown", { index: index + 1 })}
            </dt>
            <dd className="min-w-0 whitespace-pre-wrap break-words leading-relaxed text-book-text-sub">
              {formatResultValue(
                key === "updated_fields" && Array.isArray(entryValue)
                  ? entryValue.length > 0
                    ? entryValue.map((field) =>
                        typeof field === "string" && RESULT_LABEL_KEYS[field]
                          ? t(RESULT_LABEL_KEYS[field])
                          : field,
                      )
                    : t("novel.importReview.result.noTopLevelChanges")
                  : entryValue,
                t,
              )}
            </dd>
          </div>
        ))}
      </dl>
      <details className="text-caption text-book-text-muted">
        <summary className="w-fit cursor-pointer hover:text-book-text-main">{t("novel.importReview.result.diagnostics")}</summary>
        <pre className="mt-1 whitespace-pre-wrap break-words rounded-xl bg-book-bg-tertiary/45 p-2 font-mono leading-relaxed text-book-text-sub">
          {JSON.stringify(value, null, 2)}
        </pre>
      </details>
    </div>
  );
};

function isPlainRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function formatResultValue(value: unknown, t: MessageTranslator, depth = 0): string {
  if (value == null) return t("novel.importReview.result.noRecord");
  if (typeof value === "string") return value.trim() || t("novel.importReview.result.noRecord");
  if (typeof value === "number") return String(value);
  if (typeof value === "boolean") return value ? t("novel.importReview.result.yes") : t("novel.importReview.result.no");
  if (Array.isArray(value)) {
    const items = value.slice(0, 8).map((item) => formatResultValue(item, t, depth + 1));
    const suffix = value.length > items.length ? t("novel.importReview.result.moreItems", { count: value.length - items.length }) : "";
    return `${items.join(t("common.summarySeparator"))}${suffix}` || t("novel.importReview.result.noRecord");
  }
  if (isPlainRecord(value)) {
    return Object.entries(value)
      .slice(0, depth > 0 ? 6 : 10)
      .map(
        ([key, item], index) =>
          `${RESULT_LABEL_KEYS[key] ? t(RESULT_LABEL_KEYS[key]) : t("novel.importReview.result.content", { index: index + 1 })}：${formatResultValue(item, t, depth + 1)}`,
      )
      .join(t("common.summarySeparator")) || t("novel.importReview.result.noRecord");
  }
  return String(value);
}
