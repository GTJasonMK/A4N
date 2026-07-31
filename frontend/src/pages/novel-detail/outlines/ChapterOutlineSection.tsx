import { useState } from "react";
import { Eye, MoreHorizontal } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { BookButton, BookIconButton } from "../../../components/ui/BookButton";
import { nonEmptyText } from "../../../utils/text";
import { EmptyHint } from "../shared";
import { deleteChapterOutlinesFromAction } from "./outlineTabUtils";
import type { ChapterOutline } from "../../../api/outline";

export interface ChapterOutlineEditForm {
  title: string;
  summary: string;
}

interface ChapterOutlineSectionProps {
  outlines: ChapterOutline[];
  canEdit: boolean;
  chapterCount: number;
  generating: boolean;
  editingId: number | null;
  editForm: ChapterOutlineEditForm;
  savingId: number | null;
  regenId: number | null;
  delActing: string | null;
  disabled?: boolean;
  onChapterCountChange: (value: number) => void;
  onGenerate: () => void;
  onPreviewGenerate: () => void;
  onOpenDetail: (outline: ChapterOutline) => void;
  onCancelEdit: () => void;
  onEditFormChange: (form: ChapterOutlineEditForm) => void;
  onSave: (outline: ChapterOutline) => void;
  onOpenAction: (outline: ChapterOutline) => void;
}

export function ChapterOutlineSection({
  outlines,
  canEdit,
  chapterCount,
  generating,
  editingId,
  editForm,
  savingId,
  regenId,
  delActing,
  disabled = false,
  onChapterCountChange,
  onGenerate,
  onPreviewGenerate,
  onOpenDetail,
  onCancelEdit,
  onEditFormChange,
  onSave,
  onOpenAction,
}: ChapterOutlineSectionProps) {
  const [generationOpen, setGenerationOpen] = useState(outlines.length === 0);
  const hasOutlines = outlines.length > 0;
  const generationActionLabel = hasOutlines ? "追加章纲" : "生成章纲";

  return (
    <section className="space-y-2.5">
      <header className="px-1">
        <h2 className="text-sm font-semibold text-book-text-main">章节大纲</h2>
        <p className="mt-0.5 text-small leading-relaxed text-book-text-muted">
          {hasOutlines ? `共 ${outlines.length} 章，按顺序进入写作桌` : "把故事阶段拆成可写作的章节计划"}
        </p>
      </header>

      {canEdit ? (
        <details
          className="a4n-flat-surface group overflow-hidden px-4 py-2.5"
          open={generationOpen}
          onToggle={(event) => setGenerationOpen(event.currentTarget.open)}
        >
          <summary className="flex cursor-pointer items-center justify-between gap-3 py-1 text-xs font-medium text-book-text-sub">
            <span>{generationActionLabel}</span>
            <span className="text-caption font-normal text-book-text-muted">从下一章开始追加</span>
          </summary>
          <div className="mt-3 grid grid-cols-1 items-end gap-3 border-t border-book-border/35 pt-3 md:grid-cols-[minmax(12rem,20rem)_auto]">
            <label className="flex min-w-0 flex-col gap-1 text-small text-book-text-muted">
              本次追加章数
              <input
                type="number"
                min={1}
                max={50}
                value={chapterCount}
                disabled={disabled}
                onChange={(event) => onChapterCountChange(Number(event.target.value))}
                className="pyqt-input h-10 w-full"
              />
            </label>
            <div className="flex items-center justify-end gap-2 md:justify-start">
              {!generating ? (
                <BookIconButton
                  label={hasOutlines ? "查看追加请求" : "查看生成请求"}
                  variant="secondary"
                  size="md"
                  onClick={onPreviewGenerate}
                  disabled={disabled}
                  className="!h-10 !w-10"
                >
                  <Eye size={14} />
                </BookIconButton>
              ) : null}
              <BookButton
                variant="primary"
                size="sm"
                onClick={onGenerate}
                disabled={disabled || generating}
                className="h-10 min-w-0 flex-1 !min-h-10 !px-4 !text-xs md:flex-none"
              >
                {generating ? (
                  <span className="inline-flex items-center gap-1.5">
                    <LoadingSpinner size="sm" />
                    {hasOutlines ? "追加中…" : "生成中…"}
                  </span>
                ) : (
                  generationActionLabel
                )}
              </BookButton>
            </div>
          </div>
        </details>
      ) : null}

      {outlines.length === 0 ? (
        <EmptyHint text="尚未生成章节大纲。" />
      ) : (
        <ul className="a4n-flat-surface divide-y divide-book-border/35 overflow-hidden">
          {outlines.map((outline) => {
            const isEditing = editingId === outline.id;
            const deleting = delActing === deleteChapterOutlinesFromAction(outline.id);
            return (
              <li
                key={outline.id}
                role={isEditing ? undefined : "button"}
                tabIndex={isEditing ? undefined : 0}
                onClick={isEditing ? undefined : () => onOpenDetail(outline)}
                onKeyDown={
                  isEditing
                    ? undefined
                    : (event) => {
                        if (event.key === "Enter" || event.key === " ") {
                          event.preventDefault();
                          onOpenDetail(outline);
                        }
                      }
                }
                className={`flex min-h-14 items-start gap-3 px-4 py-3 transition-colors hover:bg-book-bg-tertiary/35 focus-visible:outline-2 focus-visible:outline-book-primary/35 ${
                  isEditing ? "" : "cursor-pointer"
                }`}
                title={isEditing ? undefined : "查看章节大纲详情"}
              >
                {isEditing ? (
                  <div className="flex flex-1 flex-col gap-2">
                    <input
                      type="text"
                      value={editForm.title}
                      onChange={(event) =>
                        onEditFormChange({ ...editForm, title: event.target.value })
                      }
                      disabled={disabled || savingId === outline.id}
                      placeholder="章节标题"
                      className="pyqt-input !py-1 text-sm"
                    />
                    <textarea
                      value={editForm.summary}
                      onChange={(event) =>
                        onEditFormChange({ ...editForm, summary: event.target.value })
                      }
                      disabled={disabled || savingId === outline.id}
                      placeholder="章节摘要：场景、动机、悬念"
                      className="book-control min-h-[80px] resize-y p-2 text-xs leading-relaxed"
                    />
                    <div className="flex flex-wrap items-center justify-end gap-1.5">
                      <button
                        type="button"
                        onClick={onCancelEdit}
                        disabled={disabled || savingId === outline.id}
                        className="pyqt-btn-secondary !min-h-9 !px-2.5 text-small sm:!min-h-[26px]"
                      >
                        取消
                      </button>
                      <button
                        type="button"
                        onClick={() => onSave(outline)}
                        disabled={disabled || savingId === outline.id}
                        className="pyqt-btn-primary !min-h-9 !px-2.5 text-small sm:!min-h-[26px]"
                      >
                        {savingId === outline.id ? (
                          <LoadingSpinner size="sm" />
                        ) : (
                          "保存"
                        )}
                      </button>
                    </div>
                  </div>
                ) : (
                  <>
                    <div className="min-w-0 flex-1">
                      <div className="truncate text-sm font-semibold text-book-text-main">
                        第 {outline.chapter_number} 章
                        {nonEmptyText(outline.title) ? ` · ${nonEmptyText(outline.title)}` : ""}
                      </div>
                      {nonEmptyText(outline.summary) ? (
                        <p className="mt-0.5 line-clamp-2 text-small leading-relaxed text-book-text-muted">
                          {nonEmptyText(outline.summary)}
                        </p>
                      ) : null}
                    </div>
                    {canEdit && (
                      <div className="flex shrink-0 items-center">
                        <button
                          type="button"
                          onClick={(event) => {
                            event.stopPropagation();
                            onOpenAction(outline);
                          }}
                          disabled={disabled || regenId === outline.id || deleting}
                          title="更多章纲操作"
                          className="pyqt-btn-icon !h-9 !w-9 sm:!h-8 sm:!w-8"
                        >
                          {regenId === outline.id || deleting ? (
                            <LoadingSpinner size="sm" />
                          ) : (
                            <MoreHorizontal size={13} />
                          )}
                        </button>
                      </div>
                    )}
                  </>
                )}
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}
