import React, { useEffect, useState } from "react";
import { Pencil, Plus, RefreshCw, Trash2 } from "lucide-react";
import { extractErrorMessage } from "../../../api";
import {
  createForeshadowing,
  deleteForeshadowing,
  updateForeshadowing,
  type Foreshadowing,
  type ForeshadowingInput,
} from "../../../api/foreshadowing";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { useToast } from "../../../components/feedback/Toast";
import { BookButton } from "../../../components/ui/BookButton";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { Modal } from "../../../components/ui/Modal";
import { EmptyHint, NumberField, SelectField, TextAreaField, splitLines } from "../shared";

const STATUS_LABELS: Record<string, string> = {
  pending: "待回收",
  partially_revealed: "部分揭示",
  resolved: "已回收",
  abandoned: "已放弃",
};
const STATUS_OPTIONS = Object.keys(STATUS_LABELS);

interface Draft {
  description: string;
  setupChapter: number;
  payoffChapter: number;
  status: string;
  relatedCharacters: string;
  importance: number;
}

interface ForeshadowingIndexCardProps {
  projectId: number;
  records: Foreshadowing[];
  canEdit: boolean;
  loading: boolean;
  onReload: () => Promise<void>;
}

export const ForeshadowingIndexCard: React.FC<ForeshadowingIndexCardProps> = ({
  projectId,
  records,
  canEdit,
  loading,
  onReload,
}) => {
  const { addToast } = useToast();
  const [editing, setEditing] = useState<"new" | Foreshadowing | null>(null);
  const [draft, setDraft] = useState<Draft>(() => emptyDraft());
  const [busyId, setBusyId] = useState<number | null>(null);

  useEffect(() => {
    setEditing(null);
    setBusyId(null);
  }, [projectId]);

  const openEditor = (record: "new" | Foreshadowing) => {
    setEditing(record);
    setDraft(record === "new" ? emptyDraft() : draftFromRecord(record));
  };

  const handleSave = async () => {
    if (!editing || !canEdit) return;
    const record = inputFromDraft(draft);
    if (!record.description) {
      addToast("伏笔描述不能为空", "error");
      return;
    }
    if (
      record.status === "resolved" &&
      (record.payoff_chapter == null || record.payoff_chapter <= record.setup_chapter)
    ) {
      addToast("回收章节必须晚于埋设章节", "error");
      return;
    }

    setBusyId(editing === "new" ? -1 : editing.id);
    try {
      const result =
        editing === "new"
          ? await createForeshadowing(projectId, record)
          : await updateForeshadowing(projectId, editing, record);
      setEditing(null);
      await onReload();
      addToast(
        result.warnings.length > 0
          ? `伏笔已保存；${result.warnings.join("；")}`
          : "伏笔已保存",
        result.warnings.length > 0 ? "info" : "success",
      );
    } catch (error) {
      addToast(extractErrorMessage(error, "保存伏笔失败"), "error");
    } finally {
      setBusyId(null);
    }
  };

  const handleDelete = async (record: Foreshadowing) => {
    if (!canEdit) return;
    const confirmed = await confirmDialog({
      title: "删除正式伏笔",
      message: `删除“${record.description}”？章节再次分析不会自动恢复这条记录，除非正文重新识别到同一伏笔。`,
      confirmText: "删除",
      dialogType: "danger",
    });
    if (!confirmed) return;
    setBusyId(record.id);
    try {
      const result = await deleteForeshadowing(projectId, record);
      await onReload();
      addToast(
        result.warnings.length > 0
          ? `伏笔已删除；${result.warnings.join("；")}`
          : "伏笔已删除",
        result.warnings.length > 0 ? "info" : "success",
      );
    } catch (error) {
      addToast(extractErrorMessage(error, "删除伏笔失败"), "error");
    } finally {
      setBusyId(null);
    }
  };

  return (
    <section className="a4n-flat-surface overflow-hidden">
      <header className="flex items-start gap-3 border-b border-book-border/40 px-3 py-3">
        <div className="min-w-0 flex-1">
          <h2 className="text-sm font-semibold text-book-text-main">正式伏笔索引</h2>
          <p className="mt-0.5 text-small leading-relaxed text-book-text-muted">
            章节分析可以补充和回收伏笔，但不会因本次遗漏而删除，也不会覆盖已有填写。
          </p>
        </div>
        <button
          type="button"
          onClick={() => void onReload()}
          disabled={loading || busyId != null}
          className="pyqt-btn-icon !h-8 !w-8"
          aria-label="刷新伏笔索引"
          title="刷新伏笔索引"
        >
          <RefreshCw size={12} className={loading ? "animate-spin" : undefined} />
        </button>
        {canEdit ? (
          <BookButton
            size="sm"
            onClick={() => openEditor("new")}
            disabled={busyId != null}
          >
            <Plus size={12} /> 新增伏笔
          </BookButton>
        ) : null}
      </header>

      {records.length === 0 ? (
        <div className="p-3">
          <EmptyHint
            text="还没有正式伏笔。可以手工建立，也可以在章节分析后自动补充。"
            actionText={canEdit ? "新增伏笔" : undefined}
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
                    {STATUS_LABELS[record.status] ?? record.status}
                  </span>
                  <span className="text-small text-book-text-muted">
                    重要度 {record.importance}/5
                  </span>
                </div>
                <p className="mt-1 text-small text-book-text-muted">
                  第 {record.setup_chapter} 章埋设
                  {record.payoff_chapter != null ? ` · 第 ${record.payoff_chapter} 章回收` : ""}
                  {record.related_characters.length > 0
                    ? ` · 关联 ${record.related_characters.join("、")}`
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
                    aria-label={`编辑伏笔：${record.description}`}
                    title="编辑伏笔"
                  >
                    <Pencil size={12} />
                  </button>
                  <button
                    type="button"
                    onClick={() => void handleDelete(record)}
                    disabled={busyId != null}
                    className="pyqt-btn-icon !h-8 !w-8 text-error"
                    aria-label={`删除伏笔：${record.description}`}
                    title="删除伏笔"
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
        onClose={() => (busyId == null ? setEditing(null) : undefined)}
        title={editing === "new" ? "新增正式伏笔" : "编辑正式伏笔"}
        closeOnBackdrop={busyId == null}
        closeOnEscape={busyId == null}
        footer={
          <>
            <BookButton variant="secondary" onClick={() => setEditing(null)} disabled={busyId != null}>
              取消
            </BookButton>
            <BookButton onClick={() => void handleSave()} disabled={busyId != null}>
              {busyId != null ? <LoadingSpinner size="sm" /> : null}
              保存
            </BookButton>
          </>
        }
      >
        <div className="grid gap-3">
          <TextAreaField
            label="伏笔描述"
            value={draft.description}
            onChange={(description) => setDraft((current) => ({ ...current, description }))}
          />
          <div className="grid gap-3 sm:grid-cols-3">
            <NumberField
              label="埋设章节"
              value={draft.setupChapter}
              onChange={(setupChapter) => setDraft((current) => ({ ...current, setupChapter }))}
            />
            <SelectField
              label="当前状态"
              value={draft.status}
              options={STATUS_OPTIONS}
              labels={STATUS_LABELS}
              onChange={(status) => setDraft((current) => ({ ...current, status }))}
            />
            <NumberField
              label="重要度（1-5）"
              value={draft.importance}
              min={1}
              onChange={(importance) =>
                setDraft((current) => ({ ...current, importance: Math.min(5, importance) }))
              }
            />
          </div>
          {draft.status === "resolved" ? (
            <NumberField
              label="回收章节"
              value={draft.payoffChapter}
              min={draft.setupChapter + 1}
              onChange={(payoffChapter) => setDraft((current) => ({ ...current, payoffChapter }))}
            />
          ) : null}
          <TextAreaField
            label="关联角色（每行一个）"
            value={draft.relatedCharacters}
            onChange={(relatedCharacters) =>
              setDraft((current) => ({ ...current, relatedCharacters }))
            }
          />
        </div>
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
