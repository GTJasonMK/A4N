import { ThemedSelect } from "../../components/ui/ThemedSelect";
import React from "react";
import { Pencil, Sparkles } from "lucide-react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { BookButton } from "../../components/ui/BookButton";
import { Dropdown } from "../../components/ui/Dropdown";
import { useI18n, type MessageTranslator } from "../../i18n";

/**
 * NovelDetail 各 Tab 共用的辅助组件与类型。
 */

export interface ChapterRow {
  id: number;
  chapter_number: number;
  title?: string | null;
  content?: string | null;
  status?: string | null;
  summary?: string | null;
  analysis_data?: unknown | null;
}

export function isChapterGenerating(chapter: Pick<ChapterRow, "status"> | null | undefined): boolean {
  return chapter?.status === "generating";
}

export function chapterGeneratingMessage(
  chapter: Pick<ChapterRow, "chapter_number" | "status"> | null | undefined,
  operation: string,
  t: MessageTranslator,
): string {
  return chapter?.chapter_number != null
    ? t("novel.chapterGeneratingWithNumber", {
        number: chapter.chapter_number,
        operation,
      })
    : t("novel.chapterGeneratingCurrent", { operation });
}

export const EmptyHint: React.FC<{
  text: string;
  actionText?: string;
  onAction?: () => void;
}> = ({ text, actionText, onAction }) => (
  <div className="a4n-empty-state a4n-empty-state--compact">
    <p className="max-w-lg text-sm leading-relaxed text-book-text-muted">{text}</p>
    {actionText && onAction && (
      <BookButton variant="secondary" size="sm" onClick={onAction}>
        {actionText}
      </BookButton>
    )}
  </div>
);

export function DetailRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="px-3 py-2.5 sm:grid sm:grid-cols-[96px_minmax(0,1fr)] sm:gap-3">
      <dt className="text-small font-medium text-book-text-muted">{label}</dt>
      <dd className="mt-1 whitespace-pre-wrap leading-relaxed text-book-text-sub sm:mt-0">
        {value}
      </dd>
    </div>
  );
}

export function ProfileEvidence({ items }: { items: string[] }) {
  const { t } = useI18n();
  if (items.length === 0) return null;
  return (
    <details className="a4n-flat-surface overflow-hidden">
      <summary className="cursor-pointer px-3 py-2.5 text-small font-medium text-book-text-main">
        {t("novel.evidenceCount", { count: items.length })}
      </summary>
      <ul className="space-y-1 border-t border-book-border/40 px-3 py-2.5">
        {items.map((item, index) => (
          <li key={`${item}-${index}`} className="border-l border-book-border pl-2">
            {item}
          </li>
        ))}
      </ul>
    </details>
  );
}

export function ProfileDetailActions({
  busy,
  generating,
  generateDisabled = false,
  onEdit,
  onDelete,
  onGenerate,
}: {
  busy: boolean;
  generating: boolean;
  generateDisabled?: boolean;
  onEdit: () => void;
  onDelete: () => void;
  onGenerate: () => void;
}) {
  const { t } = useI18n();
  return (
    <div className="flex items-center justify-end gap-2 pt-1">
      {!busy ? (
        <Dropdown
          items={[
            { label: t("novel.editProfile"), icon: <Pencil size={12} />, onClick: onEdit },
            { type: "divider" },
            { label: t("novel.deleteProfile"), onClick: onDelete, danger: true },
          ]}
        />
      ) : null}
      <BookButton
        variant="primary"
        size="sm"
        onClick={onGenerate}
        disabled={busy || generateDisabled}
        className="min-w-0 flex-1 !text-xs sm:flex-none"
      >
        <span className="inline-flex items-center gap-1">
          {generating ? <LoadingSpinner size="sm" /> : <Sparkles size={12} />}
          {t("novel.generateMaterialImage")}
        </span>
      </BookButton>
    </div>
  );
}

export function stringList(value: unknown): string[] {
  return Array.isArray(value)
    ? value
        .map((item) => (typeof item === "string" ? item.trim() : ""))
        .filter(Boolean)
    : [];
}

export function splitLines(value: string): string[] {
  return value
    .split(/\r?\n|,/)
    .map((item) => item.trim())
    .filter(Boolean);
}

export function chapterRangeText(
  first: number,
  last: number,
  t: MessageTranslator,
): string {
  return first === last
    ? t("novel.chapterRangeSingle", { number: first })
    : t("novel.chapterRange", { start: first, end: last });
}

export function profileChapterRangeText(
  first: number,
  last: number,
  evidence: unknown,
  ownershipKind: string,
  t: MessageTranslator,
): string {
  if (ownershipKind === "user" && stringList(evidence).length === 0) {
    return t("novel.appearancePendingConfirmation");
  }
  return chapterRangeText(first, last, t);
}

export function profileOwnershipLabel(
  value: string,
  t: MessageTranslator,
): string {
  if (value === "user") return t("novel.profileSourceUser");
  if (value === "ai_sync") return t("novel.profileSourceAiSync");
  if (value === "legacy_unknown") return t("novel.profileSourceLegacy");
  return t("novel.profileSourceUnknown");
}

export function TextField({
  label,
  value,
  onChange,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <label className="grid gap-1 text-small text-book-text-muted">
      {label}
      <input
        value={value}
        onChange={(event) => onChange(event.target.value)}
        className="pyqt-input !px-2 !py-1 text-xs"
      />
    </label>
  );
}

export function NumberField({
  label,
  value,
  min = 1,
  max,
  onChange,
}: {
  label: string;
  value: number;
  min?: number;
  max?: number;
  onChange: (value: number) => void;
}) {
  return (
    <label className="grid gap-1 text-small text-book-text-muted">
      {label}
      <input
        type="number"
        min={min}
        max={max}
        value={value}
        onChange={(event) => onChange(Number(event.target.value) || min)}
        className="pyqt-input !px-2 !py-1 text-xs"
      />
    </label>
  );
}

export function SelectField({
  label,
  value,
  options,
  labels,
  onChange,
}: {
  label: string;
  value: string;
  options: string[];
  labels: Record<string, string>;
  onChange: (value: string) => void;
}) {
  return (
    <label className="grid gap-1 text-small text-book-text-muted">
      {label}
      <ThemedSelect
        value={value}
        onChange={onChange}
        className="pyqt-input !px-2 !py-1 text-xs"
      >
        {options.map((option) => (
          <option key={option} value={option}>
            {labels[option] ?? option}
          </option>
        ))}
      </ThemedSelect>
    </label>
  );
}

export function TextAreaField({
  label,
  value,
  onChange,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <label className="grid gap-1 text-small text-book-text-muted">
      {label}
      <textarea
        value={value}
        onChange={(event) => onChange(event.target.value)}
        className="pyqt-input min-h-[82px] resize-y !px-2 !py-1 text-xs"
      />
    </label>
  );
}
