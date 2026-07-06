import { Edit2, Eye, Loader2, MoreHorizontal } from "lucide-react";
import { BookButton } from "../../../components/ui/BookButton";
import { BookCard } from "../../../components/ui/BookCard";
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
  onStartEdit: (outline: ChapterOutline) => void;
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
  onStartEdit,
  onCancelEdit,
  onEditFormChange,
  onSave,
  onOpenAction,
}: ChapterOutlineSectionProps) {
  return (
    <BookCard className="p-5">
      <div className="mb-3 flex flex-wrap items-center justify-between gap-3">
        <h3 className="font-serif text-sm text-book-text-main">
          章节大纲 <span className="text-xs text-book-text-muted">({outlines.length})</span>
        </h3>
        {canEdit && (
          <div className="flex items-center gap-2 text-xs">
            <label className="text-book-text-muted">追加</label>
            <input
              type="number"
              min={1}
              max={50}
              value={chapterCount}
              disabled={disabled}
              onChange={(event) => onChapterCountChange(Number(event.target.value))}
              className="pyqt-input !px-2 !py-1 w-16"
            />
            <span className="text-book-text-muted">章</span>
            <BookButton variant="primary" onClick={onGenerate} disabled={disabled || generating}>
              {generating ? (
                <span className="inline-flex items-center gap-2">
                  <Loader2 size={14} className="animate-spin" /> 生成中…
                </span>
              ) : outlines.length > 0 ? (
                "继续生成章纲"
              ) : (
                "AI 生成章纲"
              )}
            </BookButton>
            <BookButton variant="secondary" onClick={onPreviewGenerate} disabled={disabled || generating}>
              <span className="inline-flex items-center gap-1.5">
                <Eye size={12} /> 预览提示词
              </span>
            </BookButton>
          </div>
        )}
      </div>
      {outlines.length === 0 ? (
        <EmptyHint text="尚未生成章节大纲。" />
      ) : (
        <ul className="space-y-1.5">
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
                className={`flex items-start gap-3 rounded-sm border border-book-border bg-book-bg-paper px-3 py-2 transition-colors hover:border-book-primary/60 hover:bg-book-bg-tertiary/40 ${
                  isEditing ? "" : "cursor-pointer"
                }`}
                style={{ minHeight: "48px" }}
                title={isEditing ? undefined : "查看章节大纲详情"}
              >
                <span
                  className="flex h-9 w-9 shrink-0 items-center justify-center rounded-sm border border-book-primary/40 bg-book-primary/10 font-mono text-xs font-bold text-book-primary"
                  title={`第 ${outline.chapter_number} 章`}
                >
                  {outline.chapter_number}
                </span>
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
                      className="book-control min-h-[80px] resize-y rounded-sm p-2 text-xs leading-relaxed"
                    />
                    <div className="flex flex-wrap items-center justify-end gap-1.5">
                      <button
                        type="button"
                        onClick={onCancelEdit}
                        disabled={disabled || savingId === outline.id}
                        className="pyqt-btn-secondary !min-h-9 !px-2.5 text-[11px] sm:!min-h-[26px]"
                      >
                        取消
                      </button>
                      <button
                        type="button"
                        onClick={() => onSave(outline)}
                        disabled={disabled || savingId === outline.id}
                        className="pyqt-btn-primary !min-h-9 !px-2.5 text-[11px] sm:!min-h-[26px]"
                      >
                        {savingId === outline.id ? (
                          <Loader2 size={11} className="animate-spin" />
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
                        {nonEmptyText(outline.title) ?? `第 ${outline.chapter_number} 章`}
                      </div>
                      {nonEmptyText(outline.summary) ? (
                        <p className="mt-0.5 line-clamp-2 text-[11px] leading-relaxed text-book-text-muted">
                          {nonEmptyText(outline.summary)}
                        </p>
                      ) : null}
                    </div>
                    {canEdit && (
                      <div className="flex shrink-0 items-center gap-1">
                        <button
                          type="button"
                          onClick={(event) => {
                            event.stopPropagation();
                            onStartEdit(outline);
                          }}
                          disabled={disabled}
                          title="编辑章纲"
                          className="pyqt-btn-icon !h-9 !w-9 sm:!h-7 sm:!w-7"
                        >
                          <Edit2 size={12} />
                        </button>
                        <button
                          type="button"
                          onClick={(event) => {
                            event.stopPropagation();
                            onOpenAction(outline);
                          }}
                          disabled={disabled || regenId === outline.id || deleting}
                          title="更多章纲操作"
                          className="pyqt-btn-icon !h-9 !w-9 sm:!h-7 sm:!w-7"
                        >
                          {regenId === outline.id || deleting ? (
                            <Loader2 size={12} className="animate-spin" />
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
    </BookCard>
  );
}
