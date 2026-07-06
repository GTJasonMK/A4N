import React from "react";
import { BookButton } from "../../components/ui/BookButton";

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

export interface ChapterAnalysisUpdatedPayload {
  project_id?: number;
  chapter_id?: number;
  chapter_number?: number;
  updated_at?: string | null;
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

export const Field: React.FC<{ label: string; value: string; multiline?: boolean }> = ({
  label,
  value,
  multiline,
}) => (
  <div className={multiline ? "" : "flex flex-col"}>
    <dt className="text-book-text-muted">{label}</dt>
    <dd
      className={`text-book-text-main ${
        multiline ? "mt-1 whitespace-pre-wrap leading-relaxed" : "truncate"
      }`}
    >
      {value || "—"}
    </dd>
  </div>
);

export const EmptyHint: React.FC<{
  text: string;
  actionText?: string;
  onAction?: () => void;
}> = ({ text, actionText, onAction }) => (
  <div className="flex flex-col items-center justify-center py-6 text-center">
    <p className="text-sm text-book-text-muted mb-3">{text}</p>
    {actionText && onAction && (
      <BookButton variant="secondary" onClick={onAction}>
        {actionText}
      </BookButton>
    )}
  </div>
);

export const Row: React.FC<{ label: string; children: React.ReactNode }> = ({
  label,
  children,
}) => (
  <div className="flex justify-between gap-2">
    <span className="text-book-text-muted shrink-0">{label}</span>
    <span className="text-book-text-main text-right truncate">{children}</span>
  </div>
);
