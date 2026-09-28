import React, { useEffect, useRef, useState } from "react";
import { Pencil, Plus, RefreshCw, Trash2 } from "lucide-react";
import { extractErrorMessage } from "../../../api";
import {
  createForeshadowing,
  deleteForeshadowing,
  updateForeshadowing,
  type Foreshadowing,
  type ForeshadowingInput,
} from "../../../api/foreshadowing";
import type { ForeshadowingStatus } from "../../../api/generated/host";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { useToast } from "../../../components/feedback/Toast";
import { BookButton } from "../../../components/ui/BookButton";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { Modal } from "../../../components/ui/Modal";
import { EmptyHint, NumberField, SelectField, TextAreaField, splitLines } from "../shared";
import { useI18n, type MessageKey } from "../../../i18n";
import { useNovelDetailDirty } from "../novelDetailDirty";

const STATUS_LABELS: Record<ForeshadowingStatus, MessageKey> = {
  pending: "novel.foreshadowing.statusPending",
  partially_revealed: "novel.foreshadowing.statusPartial",
  resolved: "novel.foreshadowing.statusResolved",
  abandoned: "novel.foreshadowing.statusAbandoned",
};
const STATUS_OPTIONS = Object.keys(STATUS_LABELS) as ForeshadowingStatus[];

interface Draft {
  description: string;
  setupChapter: number;
  payoffChapter: number;
  status: ForeshadowingStatus;
  relatedCharacters: string;
  importance: number;
}

interface ForeshadowingIndexCardProps {
  projectId: number;
  records: Foreshadowing[];
  canEdit: boolean;
  loading: boolean;
  onReload: () => Promise<boolean | void>;
}

export const ForeshadowingIndexCard: React.FC<ForeshadowingIndexCardProps> = ({
  projectId,
  records,
  canEdit,
  loading,
  onReload,
}) => {
  const { addToast } = useToast();
  const { t } = useI18n();
  const [editing, setEditing] = useState<"new" | Foreshadowing | null>(null);
  const [draft, setDraft] = useState<Draft>(() => emptyDraft());
  const initialDraftRef = useRef<Draft>(emptyDraft());
  const [busyId, setBusyId] = useState<number | null>(null);
  const draftDirty = editing != null && JSON.stringify(draft) !== JSON.stringify(initialDraftRef.current);

  useNovelDetailDirty(
    `foreshadowing-editor:${projectId}`,
    draftDirty,
    () => {
      setEditing(null);
      setDraft(initialDraftRef.current);
    },
    () => busyId == null,
  );

  useEffect(() => {
    setEditing(null);
    setBusyId(null);
  }, [projectId]);

  const openEditor = (record: "new" | Foreshadowing) => {
    const nextDraft = record === "new" ? emptyDraft() : draftFromRecord(record);
    setEditing(record);
    initialDraftRef.current = nextDraft;
    setDraft(nextDraft);
  };

  const closeEditor = async () => {
    if (busyId != null) return;
    if (draftDirty && !(await confirmDialog({
      title: t("common.unsavedChangesTitle"),
      message: t("common.unsavedChangesMessage", { name: t("novel.foreshadowing.title"), action: t("common.close") }),
      confirmText: t("common.discardDraft"),
      dialogType: "warning",
    }))) return;
    setEditing(null);
  };

  const handleSave = async () => {
    if (!editing || !canEdit) return;
    const record = inputFromDraft(draft);
    if (!record.description) {
      addToast(t("novel.foreshadowing.descriptionRequired"), "error");
      return;
    }
    if (
      record.status === "resolved" &&
      (record.payoff_chapter == null || record.payoff_chapter <= record.setup_chapter)
    ) {
      addToast(t("novel.foreshadowing.payoffAfterSetup"), "error");
      return;
    }

    setBusyId(editing === "new" ? -1 : editing.id);
    try {
      const result =
        editing === "new"
          ? await createForeshadowing(projectId, record)
          : await updateForeshadowing(projectId, editing, record);
      await onReload();
      setEditing(null);
      addToast(
        result.warnings.length > 0
          ? t("novel.foreshadowing.savedWithWarnings", { warnings: result.warnings.join(t("common.listSeparator")) })
          : t("novel.foreshadowing.saved"),
        result.warnings.length > 0 ? "info" : "success",
      );
    } catch (error) {
      addToast(extractErrorMessage(error, t("novel.foreshadowing.saveFailed")), "error");
    } finally {
      setBusyId(null);
    }
  };

  const handleDelete = async (record: Foreshadowing) => {
    if (!canEdit) return;
    const confirmed = await confirmDialog({
      title: t("novel.foreshadowing.deleteTitle"),
      message: t("novel.foreshadowing.deleteMessage", { description: record.description }),
      confirmText: t("common.delete"),
      dialogType: "danger",
    });
    if (!confirmed) return;
    setBusyId(record.id);
    try {
      const result = await deleteForeshadowing(projectId, record);
      await onReload();
      addToast(
        result.warnings.length > 0
          ? t("novel.foreshadowing.deletedWithWarnings", { warnings: result.warnings.join(t("common.listSeparator")) })
          : t("novel.foreshadowing.deleted"),
        result.warnings.length > 0 ? "info" : "success",
      );
    } catch (error) {
      addToast(extractErrorMessage(error, t("novel.foreshadowing.deleteFailed")), "error");
    } finally {
      setBusyId(null);
    }
  };

  return (
    <section className="a4n-flat-surface overflow-hidden">
      <header className="flex flex-wrap items-start gap-3 border-b border-book-border/40 px-3 py-3">
        <div className="min-w-0 flex-1 basis-56">
          <h2 className="text-sm font-semibold text-book-text-main">{t("novel.foreshadowing.title")}</h2>
          <p className="mt-0.5 text-small leading-relaxed text-book-text-muted">
            {t("novel.foreshadowing.description")}
          </p>
        </div>
        <button
          type="button"
          onClick={() => void onReload()}
          disabled={loading || busyId != null}
          className="pyqt-btn-icon !h-8 !w-8"
          aria-label={t("novel.foreshadowing.refresh")}
          title={t("novel.foreshadowing.refresh")}
        >
          <RefreshCw size={12} className={loading ? "animate-spin" : undefined} />
        </button>
        {canEdit ? (
          <BookButton
            size="sm"
            onClick={() => openEditor("new")}
            disabled={busyId != null}
          >
            <Plus size={12} /> {t("novel.foreshadowing.add")}
          </BookButton>
        ) : null}
      </header>

      {records.length === 0 ? (
        <div className="p-3">
          <EmptyHint
            text={t("novel.foreshadowing.empty")}
            actionText={canEdit ? t("novel.foreshadowing.add") : undefined}
            onAction={canEdit ? () => openEditor("new") : undefined}
          />
        </div>
      ) : (
        <div className="divide-y divide-book-border/35">
          {records.map((record) => (
            <article key={record.id} className="grid gap-2 px-3 py-3 sm:grid-cols-[1fr_auto]">
              <div className="min-w-0">
                <div className="flex flex-wrap items-center gap-2">
                  <span className="text-sm font-medium text-book-text-main">
                    {record.description}
                  </span>
                  <span className="rounded-full bg-book-bg-tertiary px-2 py-0.5 text-small text-book-text-muted">
                    {t(STATUS_LABELS[record.status])}
                  </span>
                  <span className="text-small text-book-text-muted">
                    {t("novel.foreshadowing.importance", { value: record.importance })}
                  </span>
                </div>
                <p className="mt-1 text-small text-book-text-muted">
                  {t("novel.foreshadowing.setup", { chapter: record.setup_chapter })}
                  {record.payoff_chapter != null ? ` · ${t("novel.foreshadowing.payoff", { chapter: record.payoff_chapter })}` : ""}
                  {record.related_characters.length > 0
                    ? ` · ${t("novel.foreshadowing.relatedCharacters", { value: record.related_characters.join(t("common.listSeparator")) })}`
                    : ""}
                </p>
              </div>
              {canEdit ? (
                <div className="flex items-center justify-end gap-1">
                  <button
                    type="button"
                    onClick={() => openEditor(record)}
                    disabled={busyId != null}
                    className="pyqt-btn-icon !h-8 !w-8"
                    aria-label={t("novel.foreshadowing.editAria", { description: record.description })}
                    title={t("novel.foreshadowing.edit")}
                  >
                    <Pencil size={12} />
                  </button>
                  <button
                    type="button"
                    onClick={() => void handleDelete(record)}
                    disabled={busyId != null}
                    className="pyqt-btn-icon !h-8 !w-8 text-error"
                    aria-label={t("novel.foreshadowing.deleteAria", { description: record.description })}
                    title={t("novel.foreshadowing.delete")}
                  >
                    {busyId === record.id ? <LoadingSpinner size="sm" /> : <Trash2 size={12} />}
                  </button>
                </div>
              ) : null}
            </article>
          ))}
        </div>
      )}

      <Modal
        isOpen={editing != null}
        onClose={() => void closeEditor()}
        title={editing === "new" ? t("novel.foreshadowing.addTitle") : t("novel.foreshadowing.editTitle")}
        closeOnBackdrop={busyId == null}
        closeOnEscape={busyId == null}
        footer={
          <>
            <BookButton variant="secondary" onClick={() => void closeEditor()} disabled={busyId != null}>
              {t("common.cancel")}
            </BookButton>
            <BookButton onClick={() => void handleSave()} disabled={busyId != null}>
              {busyId != null ? <LoadingSpinner size="sm" /> : null}
              {t("common.save")}
            </BookButton>
          </>
        }
      >
        <fieldset disabled={busyId != null} className="grid gap-3">
          <TextAreaField
            label={t("novel.foreshadowing.descriptionLabel")}
            value={draft.description}
            onChange={(description) => setDraft((current) => ({ ...current, description }))}
          />
          <div className="grid gap-3 sm:grid-cols-3">
            <NumberField
              label={t("novel.foreshadowing.setupLabel")}
              value={draft.setupChapter}
              onChange={(setupChapter) => setDraft((current) => ({ ...current, setupChapter }))}
            />
            <SelectField
              label={t("novel.foreshadowing.statusLabel")}
              value={draft.status}
              options={STATUS_OPTIONS}
              labels={Object.fromEntries(STATUS_OPTIONS.map((status) => [status, t(STATUS_LABELS[status])]))}
              onChange={(status) =>
                setDraft((current) => ({
                  ...current,
                  status: status as ForeshadowingStatus,
                }))
              }
            />
            <NumberField
              label={t("novel.foreshadowing.importanceLabel")}
              value={draft.importance}
              min={1}
              max={5}
              onChange={(importance) => setDraft((current) => ({ ...current, importance }))}
            />
          </div>
          {draft.status === "resolved" ? (
            <NumberField
              label={t("novel.foreshadowing.payoffLabel")}
              value={draft.payoffChapter}
              min={draft.setupChapter + 1}
              onChange={(payoffChapter) => setDraft((current) => ({ ...current, payoffChapter }))}
            />
          ) : null}
          <TextAreaField
            label={t("novel.foreshadowing.relatedCharactersLabel")}
            value={draft.relatedCharacters}
            onChange={(relatedCharacters) =>
              setDraft((current) => ({ ...current, relatedCharacters }))
            }
          />
        </fieldset>
      </Modal>
    </section>
  );
};

function emptyDraft(): Draft {
  return {
    description: "",
    setupChapter: 1,
    payoffChapter: 2,
    status: "pending",
    relatedCharacters: "",
    importance: 3,
  };
}

function draftFromRecord(record: Foreshadowing): Draft {
  return {
    description: record.description,
    setupChapter: record.setup_chapter,
    payoffChapter: record.payoff_chapter ?? record.setup_chapter + 1,
    status: record.status,
    relatedCharacters: record.related_characters.join("\n"),
    importance: record.importance,
  };
}

function inputFromDraft(draft: Draft): ForeshadowingInput {
  return {
    description: draft.description.trim(),
    setup_chapter: draft.setupChapter,
    payoff_chapter: draft.status === "resolved" ? draft.payoffChapter : null,
    status: draft.status,
    related_characters: splitLines(draft.relatedCharacters),
    importance: draft.importance,
  };
}
