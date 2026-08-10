import { useState, type ReactNode } from "react";
import { Eye, MoreHorizontal, Pencil, RefreshCw, Trash2 } from "lucide-react";
import { BookButton } from "../../../components/ui/BookButton";
import { Modal } from "../../../components/ui/Modal";
import { nonEmptyText } from "../../../utils/text";
import {
  chapterOutlineHeading,
  deleteChapterOutlinesFromAction,
  partHeading,
} from "./outlineTabUtils";
import {
  chapterOutlineSourceLabel,
  isDerivedChapterOutline,
  type ChapterOutline,
  type PartOutline,
} from "../../../api/outline";

export interface ManualChapterOutlineForm {
  chapterNumber: number;
  title: string;
  summary: string;
}

export interface ManualPartOutlineForm {
  partNumber: number;
  startChapter: number;
  endChapter: number;
  title: string;
  summary: string;
  theme: string;
}

export function ManualChapterOutlineDialog({
  initial,
  saving,
  onClose,
  onSave,
}: {
  initial: ManualChapterOutlineForm;
  saving: boolean;
  onClose: () => void;
  onSave: (form: ManualChapterOutlineForm) => Promise<void>;
}) {
  const [form, setForm] = useState(initial);
  const formId = "manual-chapter-outline-form";

  return (
    <Modal
      isOpen
      onClose={() => {
        if (!saving) onClose();
      }}
      title="直接录入章节大纲"
      subtitle="保存后立即成为正式写作计划；不会先生成 AI 候选。"
      maxWidthClassName="max-w-2xl"
      closeOnBackdrop={!saving}
      closeOnEscape={!saving}
      showCloseButton={!saving}
      footer={
        <>
          <BookButton variant="secondary" disabled={saving} onClick={onClose}>
            取消
          </BookButton>
          <BookButton type="submit" form={formId} disabled={saving}>
            {saving ? "保存中…" : "保存正式章纲"}
          </BookButton>
        </>
      }
    >
      <form
        id={formId}
        className="space-y-4"
        onSubmit={(event) => {
          event.preventDefault();
          void onSave(form);
        }}
      >
        <label className="block text-small text-book-text-muted">
          章节号
          <input
            type="number"
            min={1}
            step={1}
            required
            value={form.chapterNumber}
            disabled={saving}
            onChange={(event) =>
              setForm({ ...form, chapterNumber: Number(event.target.value) })
            }
            className="pyqt-input mt-1 h-10 w-full"
          />
        </label>
        <label className="block text-small text-book-text-muted">
          章节标题（可稍后补充）
          <input
            type="text"
            value={form.title}
            disabled={saving}
            onChange={(event) => setForm({ ...form, title: event.target.value })}
            className="pyqt-input mt-1 h-10 w-full"
          />
        </label>
        <label className="block text-small text-book-text-muted">
          章节摘要（可稍后补充）
          <textarea
            value={form.summary}
            disabled={saving}
            onChange={(event) => setForm({ ...form, summary: event.target.value })}
            placeholder="本章发生什么、人物动机和结尾悬念"
            className="book-control mt-1 min-h-32 w-full resize-y p-3 text-sm leading-6"
          />
        </label>
        <p className="text-caption leading-relaxed text-book-text-muted">
          标题和摘要至少填写一项。若章节号落在已有分部范围内，系统会自动建立归属。
        </p>
      </form>
    </Modal>
  );
}

export function ManualPartOutlineDialog({
  part,
  initial,
  saving,
  onClose,
  onSave,
}: {
  part: PartOutline | null;
  initial: ManualPartOutlineForm;
  saving: boolean;
  onClose: () => void;
  onSave: (form: ManualPartOutlineForm) => Promise<void>;
}) {
  const [form, setForm] = useState(initial);
  const formId = "manual-part-outline-form";

  return (
    <Modal
      isOpen
      onClose={() => {
        if (!saving) onClose();
      }}
      title={part ? `编辑第 ${part.part_number} 部` : "直接录入分部大纲"}
      subtitle="保存后立即更新正式分部；章节空档允许保留，范围重叠会被拒绝。"
      maxWidthClassName="max-w-2xl"
      closeOnBackdrop={!saving}
      closeOnEscape={!saving}
      showCloseButton={!saving}
      footer={
        <>
          <BookButton variant="secondary" disabled={saving} onClick={onClose}>
            取消
          </BookButton>
          <BookButton type="submit" form={formId} disabled={saving}>
            {saving ? "保存中…" : "保存正式分部"}
          </BookButton>
        </>
      }
    >
      <form
        id={formId}
        className="space-y-4"
        onSubmit={(event) => {
          event.preventDefault();
          void onSave(form);
        }}
      >
        <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
          <label className="block text-small text-book-text-muted">
            分部编号
            <input
              type="number"
              min={1}
              step={1}
              required
              value={form.partNumber}
              disabled={saving}
              onChange={(event) =>
                setForm({ ...form, partNumber: Number(event.target.value) })
              }
              className="pyqt-input mt-1 h-10 w-full"
            />
          </label>
          <label className="block text-small text-book-text-muted">
            起始章节
            <input
              type="number"
              min={1}
              step={1}
              required
              value={form.startChapter}
              disabled={saving}
              onChange={(event) =>
                setForm({ ...form, startChapter: Number(event.target.value) })
              }
              className="pyqt-input mt-1 h-10 w-full"
            />
          </label>
          <label className="block text-small text-book-text-muted">
            结束章节
            <input
              type="number"
              min={1}
              step={1}
              required
              value={form.endChapter}
              disabled={saving}
              onChange={(event) =>
                setForm({ ...form, endChapter: Number(event.target.value) })
              }
              className="pyqt-input mt-1 h-10 w-full"
            />
          </label>
        </div>
        <label className="block text-small text-book-text-muted">
          分部标题（可稍后补充）
          <input
            type="text"
            value={form.title}
            disabled={saving}
            onChange={(event) => setForm({ ...form, title: event.target.value })}
            className="pyqt-input mt-1 h-10 w-full"
          />
        </label>
        <label className="block text-small text-book-text-muted">
          分部主题（可稍后补充）
          <input
            type="text"
            value={form.theme}
            disabled={saving}
            onChange={(event) => setForm({ ...form, theme: event.target.value })}
            className="pyqt-input mt-1 h-10 w-full"
          />
        </label>
        <label className="block text-small text-book-text-muted">
          分部摘要（可稍后补充）
          <textarea
            value={form.summary}
            disabled={saving}
            onChange={(event) => setForm({ ...form, summary: event.target.value })}
            placeholder="这一阶段的目标、转折和结束状态"
            className="book-control mt-1 min-h-32 w-full resize-y p-3 text-sm leading-6"
          />
        </label>
        <p className="text-caption leading-relaxed text-book-text-muted">
          调整范围只会重新计算章纲归属，不会删除已有章节大纲。
        </p>
      </form>
    </Modal>
  );
}

export function PartDetailDialog({
  part,
  onClose,
  onEdit,
}: {
  part: PartOutline;
  onClose: () => void;
  onEdit?: (part: PartOutline) => void;
}) {
  return (
    <Modal
      isOpen
      onClose={onClose}
      title={partHeading(part)}
      subtitle="分部大纲详情"
      maxWidthClassName="max-w-3xl"
      bodyClassName="px-5 py-5"
      footer={
        onEdit ? (
          <BookButton
            onClick={() => {
              onClose();
              onEdit(part);
            }}
          >
            编辑正式分部
          </BookButton>
        ) : undefined
      }
    >
      {nonEmptyText(part.summary) ? (
        <div className="whitespace-pre-wrap text-sm leading-7 text-book-text-sub">
          {nonEmptyText(part.summary)}
        </div>
      ) : null}
      {nonEmptyText(part.theme) ? (
        <div className="mt-4 text-small leading-6 text-book-text-muted">
          主题：{nonEmptyText(part.theme)}
        </div>
      ) : null}
    </Modal>
  );
}

export function ChapterOutlineDetailDialog({
  outline,
  onClose,
}: {
  outline: ChapterOutline;
  onClose: () => void;
}) {
  return (
    <Modal
      isOpen
      onClose={onClose}
      title={chapterOutlineHeading(outline)}
      subtitle={
        isDerivedChapterOutline(outline)
          ? `自动分析${chapterOutlineSourceLabel(outline)}`
          : "章节大纲详情"
      }
      maxWidthClassName="max-w-3xl"
      bodyClassName="px-5 py-5"
    >
      {nonEmptyText(outline.summary) ? (
        <div className="whitespace-pre-wrap text-sm leading-7 text-book-text-sub">
          {nonEmptyText(outline.summary)}
        </div>
      ) : null}
      {isDerivedChapterOutline(outline) ? (
        <p className="mt-3 text-small leading-relaxed text-book-text-muted">
          自动分析结果用于记录已发生内容；编辑后才会作为当前写作计划使用。
        </p>
      ) : null}
    </Modal>
  );
}

interface ChapterOutlineActionDialogProps {
  outline: ChapterOutline | null;
  regenId: number | null;
  delActing: string | null;
  disabled?: boolean;
  onClose: () => void;
  onEdit: (outline: ChapterOutline) => void;
  onRegenerate: (outline: ChapterOutline) => Promise<void>;
  onPreviewRegenerate: (outline: ChapterOutline) => void;
  onDelete: (outline: ChapterOutline) => Promise<void>;
}

export function ChapterOutlineActionDialog({
  outline,
  regenId,
  delActing,
  disabled = false,
  onClose,
  onEdit,
  onRegenerate,
  onPreviewRegenerate,
  onDelete,
}: ChapterOutlineActionDialogProps) {
  if (!outline) return null;

  const busy = disabled || regenId != null || delActing != null;
  const deleteOutlinesAction = deleteChapterOutlinesFromAction(outline.id);

  return (
    <Modal
      isOpen
      onClose={() => {
        if (!busy) onClose();
      }}
      title={`第 ${outline.chapter_number} 章操作`}
      subtitle={chapterOutlineHeading(outline)}
      icon={<MoreHorizontal size={15} />}
      maxWidthClassName="max-w-md"
      bodyClassName="px-4 py-4"
      closeOnBackdrop={!busy}
      closeOnEscape={!busy}
      showCloseButton={!busy}
    >
      <div className="a4n-flat-surface divide-y divide-book-border/40 overflow-hidden">
        <OutlineActionButton
          icon={<Pencil size={14} />}
          title="编辑章纲"
          description={
            isDerivedChapterOutline(outline)
              ? "修改后将接管为本章写作计划。"
              : "修改本章标题和摘要。"
          }
          disabled={busy}
          onClick={() => {
            onClose();
            onEdit(outline);
          }}
        />
        <OutlineActionButton
          icon={<Eye size={14} />}
          title="预览重生成提示词"
          description="直接查看本章纲重生成将发送给模型的完整请求。"
          disabled={busy}
          onClick={() => onPreviewRegenerate(outline)}
        />
        <OutlineActionButton
          icon={
            <RefreshCw
              size={14}
              className={regenId === outline.id ? "animate-spin" : undefined}
            />
          }
          title="重新生成本章纲"
          description="基于蓝图和前后文重写这一章的章节大纲。"
          disabled={busy}
          onClick={async () => {
            await onRegenerate(outline);
            onClose();
          }}
        />
        <OutlineActionButton
          icon={<Trash2 size={14} />}
          title="从本章删除到结尾"
          description="删除当前章及其之后所有章纲，用于从中途重新规划。"
          tone="danger"
          disabled={busy || delActing === deleteOutlinesAction}
          onClick={async () => {
            await onDelete(outline);
            onClose();
          }}
        />
      </div>
    </Modal>
  );
}

function OutlineActionButton({
  icon,
  title,
  description,
  disabled,
  tone = "normal",
  onClick,
}: {
  icon: ReactNode;
  title: string;
  description: string;
  disabled?: boolean;
  tone?: "normal" | "danger";
  onClick: () => void | Promise<void>;
}) {
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={() => void onClick()}
      className={`flex w-full items-start gap-3 px-3 py-3 text-left transition-colors disabled:cursor-not-allowed disabled:opacity-55 ${
        tone === "danger"
          ? "text-error hover:bg-error-bg/55"
          : "text-book-text-main hover:bg-book-bg-tertiary/45"
      }`}
    >
      <span
        className={`mt-0.5 inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-[10px] ${
          tone === "danger"
            ? "bg-error/10 text-error"
            : "bg-book-primary/10 text-book-primary"
        }`}
      >
        {icon}
      </span>
      <span className="min-w-0">
        <span className="block text-sm font-semibold">{title}</span>
        <span className="mt-1 block text-small leading-relaxed text-book-text-muted">
          {description}
        </span>
      </span>
    </button>
  );
}
