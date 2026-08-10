import React from "react";
import { Pencil, Sparkles } from "lucide-react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { BookButton } from "../../components/ui/BookButton";
import { Dropdown } from "../../components/ui/Dropdown";

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
): string {
  const prefix = chapter?.chapter_number ? `第 ${chapter.chapter_number} 章` : "当前章节";
  return `${prefix}正在生成中，请等待完成后再${operation}`;
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
  if (items.length === 0) return null;
  return (
    <details className="a4n-flat-surface overflow-hidden">
      <summary className="cursor-pointer px-3 py-2.5 text-small font-medium text-book-text-main">
        依据 · {items.length} 条
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
  onEdit,
  onDelete,
  onGenerate,
}: {
  busy: boolean;
  generating: boolean;
  onEdit: () => void;
  onDelete: () => void;
  onGenerate: () => void;
}) {
  return (
    <div className="flex items-center justify-end gap-2 pt-1">
      {!busy ? (
        <Dropdown
          items={[
            { label: "编辑档案", icon: <Pencil size={12} />, onClick: onEdit },
            { type: "divider" },
            { label: "删除档案", onClick: onDelete, danger: true },
          ]}
        />
      ) : null}
      <BookButton
        variant="primary"
        size="sm"
        onClick={onGenerate}
        disabled={busy}
        className="min-w-0 flex-1 !text-xs sm:flex-none"
      >
        <span className="inline-flex items-center gap-1">
          {generating ? <LoadingSpinner size="sm" /> : <Sparkles size={12} />}
          生成素材图
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

export function chapterRangeText(first: number, last: number): string {
  return first === last ? `第 ${first} 章` : `第 ${first}-${last} 章`;
}

export function profileChapterRangeText(
  first: number,
  last: number,
  evidence: unknown,
  ownershipKind: string,
): string {
  if (ownershipKind === "user" && stringList(evidence).length === 0) {
    return "出场章节待正文确认";
  }
  return chapterRangeText(first, last);
}

export function profileOwnershipLabel(value: string): string {
  if (value === "user") return "用户资料";
  if (value === "ai_sync") return "AI 同步资料";
  if (value === "legacy_unknown") return "历史来源";
  return "来源异常";
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
  onChange,
}: {
  label: string;
  value: number;
  min?: number;
  onChange: (value: number) => void;
}) {
  return (
    <label className="grid gap-1 text-small text-book-text-muted">
      {label}
      <input
        type="number"
        min={min}
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
      <select
        value={value}
        onChange={(event) => onChange(event.target.value)}
        className="pyqt-input !px-2 !py-1 text-xs"
      >
        {options.map((option) => (
          <option key={option} value={option}>
            {labels[option] ?? option}
          </option>
        ))}
      </select>
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
