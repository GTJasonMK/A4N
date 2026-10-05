import { useState, type ReactNode } from "react";
import { Eye, MoreHorizontal, Pencil, RefreshCw, Trash2, Wand2 } from "lucide-react";
import { BookButton } from "../../../components/ui/BookButton";
import { Modal } from "../../../components/ui/Modal";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { nonEmptyText } from "../../../utils/text";
import {
  chapterOutlineHeading,
  deleteChapterOutlinesFromAction,
  partHeading,
} from "./outlineTabUtils";
import {
  isDerivedChapterOutline,
  type ChapterOutline,
  type PartOutline,
} from "../../../api/outline";
import { useI18n } from "../../../i18n";
import { useNovelDetailDirty } from "../novelDetailDirty";

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

type ManualChapterOutlineDraft = Omit<ManualChapterOutlineForm, "chapterNumber"> & {
  chapterNumber: string;
};

type ManualPartOutlineDraft = Omit<ManualPartOutlineForm, "partNumber" | "startChapter" | "endChapter"> & {
  partNumber: string;
  startChapter: string;
  endChapter: string;
};

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
  const { t } = useI18n();
  const [form, setForm] = useState<ManualChapterOutlineDraft>(() => ({
    ...initial,
    chapterNumber: String(initial.chapterNumber),
  }));
  const initialDraft: ManualChapterOutlineDraft = {
    ...initial,
    chapterNumber: String(initial.chapterNumber),
  };
  const dirty = JSON.stringify(form) !== JSON.stringify(initialDraft);
  useNovelDetailDirty("outline-manual-chapter", dirty);
  const formId = "manual-chapter-outline-form";
  const close = async () => {
    if (saving) return;
    if (dirty && !(await confirmDialog({
      title: t("common.unsavedChangesTitle"),
      message: t("common.unsavedChangesMessage", { name: t("outline.dialogs.directChapterTitle"), action: t("common.close") }),
      confirmText: t("common.discardDraft"),
      dialogType: "warning",
    }))) return;
    onClose();
  };

  return (
    <Modal
      isOpen
      onClose={() => void close()}
      title={t("outline.dialogs.directChapterTitle")}
      maxWidthClassName="max-w-2xl"
      closeOnBackdrop={!saving}
      closeOnEscape={!saving}
      showCloseButton={!saving}
      footer={
        <>
          <BookButton variant="secondary" disabled={saving} onClick={() => void close()}>
            {t("outline.dialogs.cancel")}
          </BookButton>
          <BookButton type="submit" form={formId} disabled={saving}>
            {saving ? t("outline.dialogs.saving") : t("outline.dialogs.saveChapter")}
          </BookButton>
        </>
      }
    >
      <form
        id={formId}
        className="space-y-4"
        onSubmit={(event) => {
          event.preventDefault();
          void onSave({ ...form, chapterNumber: Number(form.chapterNumber) });
        }}
      >
        <label className="block text-small text-book-text-muted">
          {t("outline.dialogs.chapterNumber")}
          <input
            type="number"
            min={1}
            step={1}
            required
            value={form.chapterNumber}
            disabled={saving}
            onChange={(event) =>
              setForm({ ...form, chapterNumber: event.target.value })
            }
            className="pyqt-input mt-1 h-10 w-full"
          />
        </label>
        <label className="block text-small text-book-text-muted">
          {t("outline.dialogs.chapterTitle")}
          <input
            type="text"
            value={form.title}
            disabled={saving}
            onChange={(event) => setForm({ ...form, title: event.target.value })}
            className="pyqt-input mt-1 h-10 w-full"
          />
        </label>
        <label className="block text-small text-book-text-muted">
          {t("outline.dialogs.chapterSummary")}
          <textarea
            value={form.summary}
            disabled={saving}
            onChange={(event) => setForm({ ...form, summary: event.target.value })}
            placeholder={t("outline.dialogs.chapterSummaryPlaceholder")}
            className="book-control mt-1 min-h-32 w-full resize-y p-3 text-sm leading-6"
          />
        </label>
        <p className="text-caption leading-relaxed text-book-text-muted">
          {t("outline.dialogs.chapterRequiredHint")}
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
  const { t } = useI18n();
  const [form, setForm] = useState<ManualPartOutlineDraft>(() => ({
    ...initial,
    partNumber: String(initial.partNumber),
    startChapter: String(initial.startChapter),
    endChapter: String(initial.endChapter),
  }));
  const initialDraft: ManualPartOutlineDraft = {
    ...initial,
    partNumber: String(initial.partNumber),
    startChapter: String(initial.startChapter),
    endChapter: String(initial.endChapter),
  };
  const dirty = JSON.stringify(form) !== JSON.stringify(initialDraft);
  useNovelDetailDirty("outline-manual-part", dirty);
  const formId = "manual-part-outline-form";
  const close = async () => {
    if (saving) return;
    if (dirty && !(await confirmDialog({
      title: t("common.unsavedChangesTitle"),
      message: t("common.unsavedChangesMessage", { name: t("outline.dialogs.directPartTitle"), action: t("common.close") }),
      confirmText: t("common.discardDraft"),
      dialogType: "warning",
    }))) return;
    onClose();
  };

  return (
    <Modal
      isOpen
      onClose={() => void close()}
      title={part ? t("outline.dialogs.editPartTitle", { part: part.part_number }) : t("outline.dialogs.directPartTitle")}
      maxWidthClassName="max-w-2xl"
      closeOnBackdrop={!saving}
      closeOnEscape={!saving}
      showCloseButton={!saving}
      footer={
        <>
          <BookButton variant="secondary" disabled={saving} onClick={() => void close()}>
            {t("outline.dialogs.cancel")}
          </BookButton>
          <BookButton type="submit" form={formId} disabled={saving}>
            {saving ? t("outline.dialogs.saving") : t("outline.dialogs.savePart")}
          </BookButton>
        </>
      }
    >
      <form
        id={formId}
        className="space-y-4"
        onSubmit={(event) => {
          event.preventDefault();
          void onSave({
            ...form,
            partNumber: Number(form.partNumber),
            startChapter: Number(form.startChapter),
            endChapter: Number(form.endChapter),
          });
        }}
      >
        <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
          <label className="block text-small text-book-text-muted">
            {t("outline.dialogs.partNumber")}
            <input
              type="number"
              min={1}
              step={1}
              required
              value={form.partNumber}
              disabled={saving}
              onChange={(event) =>
                setForm({ ...form, partNumber: event.target.value })
              }
              className="pyqt-input mt-1 h-10 w-full"
            />
          </label>
          <label className="block text-small text-book-text-muted">
            {t("outline.dialogs.startChapter")}
            <input
              type="number"
              min={1}
              step={1}
              required
              value={form.startChapter}
              disabled={saving}
              onChange={(event) =>
                setForm({ ...form, startChapter: event.target.value })
              }
              className="pyqt-input mt-1 h-10 w-full"
            />
          </label>
          <label className="block text-small text-book-text-muted">
            {t("outline.dialogs.endChapter")}
            <input
              type="number"
              min={1}
              step={1}
              required
              value={form.endChapter}
              disabled={saving}
              onChange={(event) =>
                setForm({ ...form, endChapter: event.target.value })
              }
              className="pyqt-input mt-1 h-10 w-full"
            />
          </label>
        </div>
        <label className="block text-small text-book-text-muted">
          {t("outline.dialogs.partTitle")}
          <input
            type="text"
            value={form.title}
            disabled={saving}
            onChange={(event) => setForm({ ...form, title: event.target.value })}
            className="pyqt-input mt-1 h-10 w-full"
          />
        </label>
        <label className="block text-small text-book-text-muted">
          {t("outline.dialogs.partTheme")}
          <input
            type="text"
            value={form.theme}
            disabled={saving}
            onChange={(event) => setForm({ ...form, theme: event.target.value })}
            className="pyqt-input mt-1 h-10 w-full"
          />
        </label>
        <label className="block text-small text-book-text-muted">
          {t("outline.dialogs.partSummary")}
          <textarea
            value={form.summary}
            disabled={saving}
            onChange={(event) => setForm({ ...form, summary: event.target.value })}
            placeholder={t("outline.dialogs.partSummaryPlaceholder")}
            className="book-control mt-1 min-h-32 w-full resize-y p-3 text-sm leading-6"
          />
        </label>
        <p className="text-caption leading-relaxed text-book-text-muted">
          {t("outline.dialogs.partRangeHint")}
        </p>
      </form>
    </Modal>
  );
}

export function PartDetailDialog({
  part,
  onClose,
  onEdit,
  onRefine,
  disabled = false,
}: {
  part: PartOutline;
  onClose: () => void;
  onEdit?: (part: PartOutline) => void;
  onRefine?: (part: PartOutline) => void;
  disabled?: boolean;
}) {
  const { t } = useI18n();
  return (
    <Modal
      isOpen
      onClose={onClose}
      title={partHeading(part, t)}
      maxWidthClassName="max-w-3xl"
      bodyClassName="px-5 py-5"
      footer={
        onEdit || onRefine ? (
          <div className="flex flex-wrap justify-end gap-2">
            {onRefine ? (
              <BookButton
                variant="secondary"
                onClick={() => {
                  onClose();
                  onRefine(part);
                }}
                disabled={disabled}
              >
                <Wand2 size={13} /> {t("outline.dialogs.refinePart", { part: part.part_number })}
              </BookButton>
            ) : null}
            {onEdit ? (
              <BookButton
                onClick={() => {
                  onClose();
                  onEdit(part);
                }}
                disabled={disabled}
              >
                {t("outline.dialogs.editPart")}
              </BookButton>
            ) : null}
          </div>
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
          {t("outline.dialogs.theme")}{nonEmptyText(part.theme)}
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
  const { t } = useI18n();
  return (
    <Modal
      isOpen
      onClose={onClose}
      title={chapterOutlineHeading(outline, t)}
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
          {t("outline.dialogs.autoAnalysisSource")}
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
  const { t } = useI18n();
  if (!outline) return null;

  const busy = disabled || regenId != null || delActing != null;
  const deleteOutlinesAction = deleteChapterOutlinesFromAction(outline.id);

  return (
    <Modal
      isOpen
      onClose={() => {
        if (!busy) onClose();
      }}
      title={t("outline.dialogs.chapterActions", { chapter: outline.chapter_number })}
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
          title={t("outline.dialogs.editOutline")}
          description={
            isDerivedChapterOutline(outline)
              ? t("outline.dialogs.editOutlineOwnPlan")
              : t("outline.dialogs.editOutlineDescription")
          }
          disabled={busy}
          onClick={() => {
            onClose();
            onEdit(outline);
          }}
        />
        <OutlineActionButton
          icon={<Eye size={14} />}
          title={t("outline.dialogs.previewRegenerate")}
          description={t("outline.dialogs.previewRegenerateDescription")}
          disabled={busy}
          onClick={() => {
            onClose();
            onPreviewRegenerate(outline);
          }}
        />
        <OutlineActionButton
          icon={
            <RefreshCw
              size={14}
              className={regenId === outline.id ? "animate-spin" : undefined}
            />
          }
          title={t("outline.dialogs.regenerateOutline")}
          description={t("outline.dialogs.regenerateDescription")}
          disabled={busy}
          onClick={() => {
            onClose();
            void onRegenerate(outline);
          }}
        />
        <OutlineActionButton
          icon={<Trash2 size={14} />}
          title={t("outline.dialogs.deleteFollowing")}
          description={t("outline.dialogs.deleteFollowingDescription")}
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
