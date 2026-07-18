import type { ReactNode } from "react";
import { Eye, MoreHorizontal, Pencil, RefreshCw, Trash2 } from "lucide-react";
import { Modal } from "../../../components/ui/Modal";
import { nonEmptyText } from "../../../utils/text";
import {
  chapterOutlineHeading,
  deleteChapterOutlinesFromAction,
  partHeading,
} from "./outlineTabUtils";
import type { ChapterOutline, PartOutline } from "../../../api/outline";

export function PartDetailDialog({
  part,
  onClose,
}: {
  part: PartOutline;
  onClose: () => void;
}) {
  return (
    <Modal
      isOpen
      onClose={onClose}
      title={partHeading(part)}
      subtitle="分部大纲详情"
      maxWidthClassName="max-w-3xl"
      bodyClassName="px-5 py-5"
    >
      {nonEmptyText(part.summary) ? (
        <div className="whitespace-pre-wrap text-sm leading-7 text-book-text-sub">
          {nonEmptyText(part.summary)}
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
      subtitle="章节大纲详情"
      maxWidthClassName="max-w-3xl"
      bodyClassName="px-5 py-5"
    >
      {nonEmptyText(outline.summary) ? (
        <div className="whitespace-pre-wrap text-sm leading-7 text-book-text-sub">
          {nonEmptyText(outline.summary)}
        </div>
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
          description="修改本章标题和摘要。"
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
        <span className="mt-1 block text-[11px] leading-relaxed text-book-text-muted">
          {description}
        </span>
      </span>
    </button>
  );
}
