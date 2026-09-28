import React, { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { Modal } from "../components/ui/Modal";
import { useToast } from "../components/feedback/Toast";
import { confirmDialog } from "../components/feedback/ConfirmDialog";
import { RequestPreviewModal } from "../components/feedback/RequestPreviewModal";
import { extractErrorMessage } from "../api";
import {
  deleteChapterScene,
  generateChapterScene,
  listChapterScenes,
  previewChapterSceneRequest,
  reorderChapterScenes,
  updateChapterScene,
  CHAPTER_SCENE_PROGRESS_TOTAL,
  type ChapterSceneSummary,
} from "../api/chapter";
import type { ModelRequestPreview } from "../api/requestPreview";
import { cancelWorkflowTask } from "../api/streaming";
import { createWorkflowTaskId, useProgressTask } from "../hooks/useProgressTask";
import { isCancelledOperation } from "../utils/cancellation";
import { nonEmptyText } from "../utils/text";
import { formatNumber, useI18n } from "../i18n";
import { SceneGenerationPanel } from "./scenes-dialog/SceneGenerationPanel";
import { SceneList } from "./scenes-dialog/SceneList";
import { useDirty } from "../components/navigation/dirtyRegistry";
import {
  createSceneEditForm,
  DEFAULT_SCENE_GENERATION_PROMPT,
  getNextSceneNumber,
  sceneEditFormChanged,
  sceneEditInput,
  sortScenes,
  type SceneEditForm,
} from "./scenes-dialog/scenesDialogUtils";

interface ScenesDialogProps {
  open: boolean;
  onClose: () => void;
  chapterId: number | null;
  chapterNumber: number | null;
  aiActionBlockedReason: string | null;
}

export const ScenesDialog: React.FC<ScenesDialogProps> = ({
  open,
  onClose,
  chapterId,
  chapterNumber,
  aiActionBlockedReason,
}) => {
  const { addToast } = useToast();
  const { locale, t } = useI18n();
  const [scenes, setScenes] = useState<ChapterSceneSummary[]>([]);
  const [loadedScope, setLoadedScope] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [expanded, setExpanded] = useState<Set<number>>(new Set());
  const [editingId, setEditingId] = useState<number | null>(null);
  const [editForm, setEditForm] = useState<SceneEditForm>({});
  const [saving, setSaving] = useState(false);
  const [generating, setGenerating] = useState(false);
  const [requestPreviewOpen, setRequestPreviewOpen] = useState(false);
  const [requestPreviewLoading, setRequestPreviewLoading] = useState(false);
  const [requestPreviewData, setRequestPreviewData] =
    useState<ModelRequestPreview | null>(null);
  const { runProgressTask } = useProgressTask();
  const [genPrompt, setGenPrompt] = useState(DEFAULT_SCENE_GENERATION_PROMPT);
  const [actingKeys, setActingKeys] = useState<Set<string>>(() => new Set());
  const scopeKey = open && chapterId != null ? String(chapterId) : null;
  const chapterKey = chapterId != null ? String(chapterId) : null;
  const activeScopeRef = useRef<string | null>(scopeKey);
  const activeChapterIdRef = useRef<number | null>(chapterId);
  const mountedRef = useRef(true);
  const loadSeqRef = useRef(0);
  const previewSeqRef = useRef(0);
  const saveSeqRef = useRef(0);
  const generateSeqRef = useRef(0);
  const previewingRef = useRef(false);
  const savingRef = useRef(false);
  const generatingRef = useRef(false);
  const actingKeysRef = useRef<Set<string>>(new Set());
  const loadInFlightScopeRef = useRef<string | null>(null);

  const isMutationBusy = () =>
    savingRef.current || generatingRef.current || actingKeysRef.current.size > 0;

  activeChapterIdRef.current = chapterId;
  const sceneListReady = scopeKey != null && loadedScope === scopeKey;
  const canStartMutation = () =>
    sceneListReady && !loading && !isMutationBusy() && !previewingRef.current;
  const canUseSceneList = () => canStartMutation() && loadError == null;
  const canStartAction = () => canStartMutation();

  const isCurrentScope = (requestScope: string | null) =>
    mountedRef.current && activeScopeRef.current === requestScope;

  const isCurrentMutation = (
    requestChapterId: number,
    seqRef: React.MutableRefObject<number>,
    requestSeq: number,
  ) =>
    mountedRef.current &&
    activeChapterIdRef.current === requestChapterId &&
    seqRef.current === requestSeq;

  const isCurrentChapter = (requestChapterId: number) =>
    mountedRef.current && activeChapterIdRef.current === requestChapterId;

  const startAction = (key: string, requiresCurrentOrder = false) => {
    if (!canStartAction() || (requiresCurrentOrder && loadError != null)) return false;
    const next = new Set(actingKeysRef.current);
    next.add(key);
    actingKeysRef.current = next;
    setActingKeys(new Set(next));
    return true;
  };

  const finishAction = (key: string) => {
    const next = new Set(actingKeysRef.current);
    next.delete(key);
    actingKeysRef.current = next;
    setActingKeys(new Set(next));
  };

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      loadSeqRef.current += 1;
      previewSeqRef.current += 1;
      saveSeqRef.current += 1;
      generateSeqRef.current += 1;
      previewingRef.current = false;
      savingRef.current = false;
      generatingRef.current = false;
      actingKeysRef.current = new Set();
      loadInFlightScopeRef.current = null;
    };
  }, []);

  const load = useCallback(async (options?: { force?: boolean }): Promise<boolean> => {
    if (chapterId == null) return false;
    const requestScope = String(chapterId);
    if (!options?.force && loadInFlightScopeRef.current === requestScope) return false;
    loadInFlightScopeRef.current = requestScope;
    const requestSeq = loadSeqRef.current + 1;
    loadSeqRef.current = requestSeq;
    setLoading(true);
    setLoadError(null);
    try {
      const data = await listChapterScenes(chapterId);
      if (loadSeqRef.current !== requestSeq || !isCurrentScope(requestScope)) return false;
      setScenes(data);
      setLoadedScope(requestScope);
      setLoadError(null);
      return true;
    } catch (err) {
      if (loadSeqRef.current !== requestSeq || !isCurrentScope(requestScope)) return false;
      const message = extractErrorMessage(err, t("scenes.loadFailed"));
      setLoadError(message);
      return false;
    } finally {
      if (loadSeqRef.current === requestSeq && isCurrentScope(requestScope)) {
        setLoading(false);
      }
      if (loadInFlightScopeRef.current === requestScope) {
        loadInFlightScopeRef.current = null;
      }
    }
  }, [addToast, chapterId, t]);

  useLayoutEffect(() => {
    activeScopeRef.current = scopeKey;
    loadSeqRef.current += 1;
    previewSeqRef.current += 1;
    setScenes([]);
    setLoadedScope(null);
    setExpanded(new Set());
    setEditingId(null);
    setEditForm({});
    setLoading(scopeKey != null);
    setLoadError(null);
    loadInFlightScopeRef.current = null;
    setRequestPreviewOpen(false);
    setRequestPreviewLoading(false);
    setRequestPreviewData(null);
    previewingRef.current = false;
    setGenPrompt(DEFAULT_SCENE_GENERATION_PROMPT);
  }, [scopeKey]);

  useLayoutEffect(() => {
    saveSeqRef.current += 1;
    generateSeqRef.current += 1;
    savingRef.current = false;
    generatingRef.current = false;
    actingKeysRef.current = new Set();
    setSaving(false);
    setGenerating(false);
    setActingKeys(new Set());
  }, [chapterKey]);

  useEffect(() => {
    if (scopeKey != null) {
      void load();
    }
  }, [scopeKey, load]);

  useEffect(() => {
    const ids = new Set(scenes.map((scene) => scene.id));
    setExpanded((prev) => new Set([...prev].filter((id) => ids.has(id))));
    setEditingId((current) => {
      if (current == null || ids.has(current)) return current;
      setEditForm({});
      return null;
    });
  }, [scenes]);

  const editingScene = editingId == null
    ? null
    : scenes.find((scene) => scene.id === editingId) ?? null;
  const editDirty = editingScene != null && sceneEditFormChanged(editForm, editingScene);
  const discardNavigationDraft = useCallback(async () => {
    if (isMutationBusy()) return false;
    if (!isCurrentScope(scopeKey)) return false;
    setEditingId(null);
    setEditForm({});
    setGenPrompt(DEFAULT_SCENE_GENERATION_PROMPT);
    return true;
  }, [scopeKey, isCurrentScope]);
  useDirty(
    "scenes-dialog-workspace",
    editDirty || genPrompt !== DEFAULT_SCENE_GENERATION_PROMPT || saving || generating || actingKeys.size > 0,
    discardNavigationDraft,
    () => !isMutationBusy(),
  );

  const toggleExpand = (id: number) => {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const startEdit = (s: ChapterSceneSummary) => {
    setEditingId(s.id);
    setEditForm(createSceneEditForm(s));
    setExpanded((prev) => new Set(prev).add(s.id));
  };

  const cancelEdit = () => {
    setEditingId(null);
    setEditForm({});
  };

  const handleSave = async () => {
    if (editingId == null || chapterId == null || !editingScene || !editDirty || !canStartAction()) return;
    savingRef.current = true;
    const requestScope = scopeKey;
    const requestChapterId = chapterId;
    const requestSeq = saveSeqRef.current + 1;
    saveSeqRef.current = requestSeq;
    setSaving(true);
    try {
      const saved = await updateChapterScene(sceneEditInput(editingId, editForm, editingScene));
      if (!isCurrentMutation(requestChapterId, saveSeqRef, requestSeq)) return;
      if (isCurrentScope(requestScope)) {
        setScenes((current) => sortScenes(current.map((scene) => scene.id === saved.id ? saved : scene)));
        setEditingId(null);
        setEditForm({});
      }
      addToast(t("scenes.saved"), "success");
    } catch (err) {
      if (!isCurrentMutation(requestChapterId, saveSeqRef, requestSeq)) return;
      addToast(extractErrorMessage(err, t("scenes.saveFailed")), "error");
    } finally {
      if (isCurrentMutation(requestChapterId, saveSeqRef, requestSeq)) setSaving(false);
      if (saveSeqRef.current === requestSeq) {
        savingRef.current = false;
      }
    }
  };

  const handleGenerate = async () => {
    if (chapterId == null) return;
    if (editDirty) {
      addToast(t("scenes.saveBeforeGenerate"), "info");
      return;
    }
    if (aiActionBlockedReason) {
      addToast(aiActionBlockedReason, "error");
      return;
    }
    if (!canUseSceneList()) return;
    generatingRef.current = true;
    const requestScope = scopeKey;
    const requestChapterId = chapterId;
    const requestSeq = generateSeqRef.current + 1;
    generateSeqRef.current = requestSeq;
    const nextNumber = getNextSceneNumber(scenes);
    const taskId = createWorkflowTaskId("chapter-scene", chapterId);
    setGenerating(true);
    try {
      const s = await runProgressTask(
        {
          cancelTaskId: taskId,
          cancelTask: cancelWorkflowTask,
          pollTaskProgress: true,
          title: t("scenes.generateTitle", { number: formatNumber(nextNumber, locale) }),
          startMessage: t("scenes.preparing"),
          successMessage: t("scenes.generated"),
          errorMessage: t("scenes.generateFailed"),
          total: CHAPTER_SCENE_PROGRESS_TOTAL,
          cancelLabel: t("scenes.stopGeneration"),
          runningHint: t("scenes.stopHint"),
        },
        () =>
          generateChapterScene(
            {
              chapter_id: requestChapterId,
              scene_number: nextNumber,
              scene_prompt: genPrompt,
            },
            taskId,
          ),
      );
      if (!isCurrentMutation(requestChapterId, generateSeqRef, requestSeq)) return;
      if (isCurrentScope(requestScope)) {
        setScenes((current) => sortScenes([...current.filter((scene) => scene.id !== s.id), s]));
      }
      addToast(t("scenes.generatedNumber", { number: formatNumber(s.scene_number, locale) }), "success");
    } catch (err) {
      if (!isCurrentMutation(requestChapterId, generateSeqRef, requestSeq)) return;
      const message = extractErrorMessage(err, t("scenes.generateFailed"));
      addToast(message, isCancelledOperation(message) ? "info" : "error");
      if (isCurrentScope(requestScope)) await load({ force: true });
    } finally {
      if (isCurrentMutation(requestChapterId, generateSeqRef, requestSeq)) setGenerating(false);
      if (generateSeqRef.current === requestSeq) {
        generatingRef.current = false;
      }
    }
  };

  const handlePreviewGenerateRequest = async () => {
    if (chapterId == null) return;
    if (editDirty) {
      addToast(t("scenes.saveBeforeGenerate"), "info");
      return;
    }
    if (aiActionBlockedReason) {
      addToast(aiActionBlockedReason, "error");
      return;
    }
    if (!canUseSceneList()) return;
    previewingRef.current = true;
    const requestScope = scopeKey;
    const requestSeq = previewSeqRef.current + 1;
    previewSeqRef.current = requestSeq;
    const nextNumber = getNextSceneNumber(scenes);
    setRequestPreviewOpen(true);
    setRequestPreviewLoading(true);
    setRequestPreviewData(null);
    try {
      const preview = await previewChapterSceneRequest({
        chapter_id: chapterId,
        scene_number: nextNumber,
        scene_prompt: genPrompt,
      });
      if (previewSeqRef.current !== requestSeq || !isCurrentScope(requestScope)) return;
      setRequestPreviewData(preview);
    } catch (err) {
      if (previewSeqRef.current !== requestSeq || !isCurrentScope(requestScope)) return;
      addToast(extractErrorMessage(err, t("scenes.previewFailed")), "error");
      setRequestPreviewOpen(false);
    } finally {
      if (previewSeqRef.current === requestSeq && isCurrentScope(requestScope)) {
        setRequestPreviewLoading(false);
      }
      if (previewSeqRef.current === requestSeq) {
        previewingRef.current = false;
      }
    }
  };

  const handleDelete = async (s: ChapterSceneSummary) => {
    if (chapterId == null) return;
    const requestScope = scopeKey;
    const requestChapterId = chapterId;
    const actionKey = `del-${s.id}`;
    if (!startAction(actionKey)) return;
    const title = nonEmptyText(s.title);
    const ok = await confirmDialog({
      title: t("scenes.deleteTitle"),
      message: title
        ? t("scenes.deleteWithTitle", { number: formatNumber(s.scene_number, locale), title })
        : t("scenes.deleteWithoutTitle", { number: formatNumber(s.scene_number, locale) }),
      confirmText: t("common.delete"),
      dialogType: "danger",
    });
    if (!ok) {
      finishAction(actionKey);
      return;
    }
    if (!isCurrentChapter(requestChapterId)) {
      finishAction(actionKey);
      return;
    }
    try {
      await deleteChapterScene(s.id);
      if (!isCurrentChapter(requestChapterId)) return;
      if (isCurrentScope(requestScope)) {
        setScenes((current) => current.filter((scene) => scene.id !== s.id));
        setExpanded((current) => {
          if (!current.has(s.id)) return current;
          const next = new Set(current);
          next.delete(s.id);
          return next;
        });
      }
      addToast(t("scenes.deleted"), "success");
    } catch (err) {
      if (!isCurrentChapter(requestChapterId)) return;
      addToast(extractErrorMessage(err, t("scenes.deleteFailed")), "error");
    } finally {
      if (isCurrentChapter(requestChapterId)) {
        finishAction(actionKey);
      }
    }
  };

  const handleMove = async (s: ChapterSceneSummary, dir: -1 | 1) => {
    if (chapterId == null) return;
    const requestScope = scopeKey;
    const requestChapterId = chapterId;
    const sorted = sortScenes(scenes);
    const idx = sorted.findIndex((x) => x.id === s.id);
    if (idx < 0) return;
    const swapIdx = idx + dir;
    if (swapIdx < 0 || swapIdx >= sorted.length) return;
    const a = sorted[idx];
    const b = sorted[swapIdx];
    const actionKey = `move-${s.id}`;
    if (!isCurrentChapter(requestChapterId)) return;
    if (!startAction(actionKey, true)) return;
    try {
      const reordered = await reorderChapterScenes({
        order: [
          [a.id, b.scene_number],
          [b.id, a.scene_number],
        ],
      });
      if (!isCurrentChapter(requestChapterId)) return;
      if (isCurrentScope(requestScope)) {
        setScenes(sortScenes(reordered));
        setLoadedScope(requestScope);
        setLoadError(null);
      }
    } catch (err) {
      if (!isCurrentChapter(requestChapterId)) return;
      addToast(extractErrorMessage(err, t("scenes.reorderFailed")), "error");
    } finally {
      if (isCurrentChapter(requestChapterId)) {
        finishAction(actionKey);
      }
    }
  };

  const handleClose = async () => {
    if (isMutationBusy()) {
      addToast(t("common.operationInProgress"), "info");
      return;
    }
    const hasDraft = editDirty || genPrompt !== DEFAULT_SCENE_GENERATION_PROMPT;
    if (hasDraft && !(await confirmDialog({
      title: t("common.unsavedChangesTitle"),
      message: t("common.unsavedChangesMessage", {
        name: editDirty ? t("scenes.chapterScenes") : t("scenes.generateNext"),
        action: t("common.close"),
      }),
      confirmText: t("common.discardDraft"),
      dialogType: "warning",
    }))) return;
    if (!await discardNavigationDraft()) return;
    onClose();
  };

  const sortedScenes = useMemo(() => sortScenes(scenes), [scenes]);
  const operationBusy = saving || generating || actingKeys.size > 0 || requestPreviewLoading || loading;
  const nextSceneNumber = canUseSceneList() ? getNextSceneNumber(scenes) : null;

  return (
    <>
      <Modal
        isOpen={open}
        onClose={() => void handleClose()}
        title={t("scenes.chapterScenes")}
        subtitle={
          chapterNumber != null
            ? `${t("scenes.chapterSummary", { chapter: formatNumber(chapterNumber, locale) })}${scenes.length > 0 ? ` · ${t("scenes.sceneCount", { count: formatNumber(scenes.length, locale) })}` : ""}`
            : scenes.length > 0
              ? t("scenes.sceneCount", { count: formatNumber(scenes.length, locale) })
              : undefined
        }
        maxWidthClassName="max-w-3xl"
      >
        <div className="flex flex-col gap-3">
          <SceneGenerationPanel
            nextSceneNumber={nextSceneNumber}
            prompt={genPrompt}
            generating={generating}
            canGenerate={canUseSceneList() && !editDirty && !operationBusy}
            hasUnsavedSceneChanges={editDirty}
            previewing={requestPreviewLoading}
            aiActionBlockedReason={aiActionBlockedReason}
            onPromptChange={setGenPrompt}
            onPreviewRequest={() => void handlePreviewGenerateRequest()}
            onGenerate={() => void handleGenerate()}
          />

          <SceneList
            loading={loading}
            loadError={loadError}
            scenes={sortedScenes}
            expanded={expanded}
            editingId={editingId}
            editForm={editForm}
            saving={saving}
            mutationBusy={operationBusy}
            reorderDisabled={loadError != null}
            actingKeys={actingKeys}
            onToggleExpand={toggleExpand}
            onStartEdit={startEdit}
            onCancelEdit={cancelEdit}
            onEditFormChange={(patch) => setEditForm((prev) => ({ ...prev, ...patch }))}
            onSave={() => void handleSave()}
            onMove={(scene, direction) => void handleMove(scene, direction)}
            onDelete={(scene) => void handleDelete(scene)}
          />
        </div>
      </Modal>

      <RequestPreviewModal
        open={requestPreviewOpen}
        loading={requestPreviewLoading}
        data={requestPreviewData}
        fallbackTitle={
          nextSceneNumber == null
            ? t("scenes.chapterScenes")
            : t("scenes.generateTitle", { number: formatNumber(nextSceneNumber, locale) })
        }
        onClose={() => {
          previewSeqRef.current += 1;
          previewingRef.current = false;
          setRequestPreviewOpen(false);
          setRequestPreviewLoading(false);
        }}
      />
    </>
  );
};
