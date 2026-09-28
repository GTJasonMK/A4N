import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { ChapterSummary } from "../../api/chapter";
import { extractErrorMessage } from "../../api";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import type { ToastType } from "../../components/feedback/Toast";
import {
  getProjectWritingDraftSet,
  getWritingDraftKey,
  getWritingDraftProjectPrefix,
  readWritingDraft,
  removeWritingDraft,
  writeWritingDraft,
} from "../../utils/writingDraft";
import { formatDateTime, useI18n } from "../../i18n";
import { useDirty } from "../../components/navigation/dirtyRegistry";

type AddToast = (message: string, type: ToastType) => void;

const chapterContentForEditor = (chapter: ChapterSummary | null): string => {
  if (chapter == null) return "";
  return chapter.content;
};

interface UseWritingDeskDraftsParams {
  projectIdKey: string;
  activeNumber: number | null;
  activeChapter: ChapterSummary | null;
  chapterReady: boolean;
  canDiscard?: () => boolean;
  addToast: AddToast;
}

export function useWritingDeskDrafts({
  projectIdKey,
  activeNumber,
  activeChapter,
  chapterReady,
  canDiscard = () => true,
  addToast,
}: UseWritingDeskDraftsParams) {
  const { locale, t } = useI18n();
  const [editorContent, setEditorContent] = useState("");
  const [dirty, setDirty] = useState(false);
  const [draftRevision, setDraftRevision] = useState(0);
  const editorContentRef = useRef("");
  const editorRevisionRef = useRef<string | null>(null);
  const dirtyRef = useRef(false);
  const activeNumberRef = useRef<number | null>(null);
  const draftPromptedRef = useRef<Map<string, string>>(new Map());
  const draftPromptOpenRef = useRef(false);
  const mountedRef = useRef(true);
  const projectIdKeyRef = useRef(projectIdKey);
  const draftSaveWarnedRef = useRef(false);
  const editorActiveNumberRef = useRef<number | null>(null);
  projectIdKeyRef.current = projectIdKey;

  useEffect(() => {
    mountedRef.current = true;
    return () => { mountedRef.current = false; };
  }, []);

  useEffect(() => {
    const onStorage = (event: StorageEvent) => {
      if (event.storageArea !== window.localStorage) return;
      if (event.key !== null && !event.key.startsWith(getWritingDraftProjectPrefix(projectIdKey))) return;
      setDraftRevision((value) => value + 1);
    };
    window.addEventListener("storage", onStorage);
    return () => window.removeEventListener("storage", onStorage);
  }, [projectIdKey]);

  const setEditorState = useCallback((content: string, isDirty: boolean) => {
    editorContentRef.current = content;
    dirtyRef.current = isDirty;
    setEditorContent(content);
    setDirty(isDirty);
  }, []);

  useEffect(() => {
    editorContentRef.current = editorContent;
  }, [editorContent]);

  useEffect(() => {
    dirtyRef.current = dirty;
  }, [dirty]);

  useEffect(() => {
    activeNumberRef.current = activeNumber;
  }, [activeNumber]);

  const draftSet = useMemo(
    () =>
      projectIdKey
        ? getProjectWritingDraftSet(projectIdKey)
        : new Set<number>(),
    [projectIdKey, draftRevision],
  );

  const refreshDraftSet = useCallback(
    () => setDraftRevision((value) => value + 1),
    [],
  );

  const reportDraftError = useCallback((error: unknown) => {
    if (!draftSaveWarnedRef.current) {
      draftSaveWarnedRef.current = true;
      addToast(extractErrorMessage(error, t("writing.localDraftSaveFailed")), "error");
    }
  }, [addToast, t]);

  const saveLocalDraftForCurrent = useCallback(async () => {
    const chapterNumber = editorActiveNumberRef.current;
    if (!projectIdKey || !chapterNumber || !dirtyRef.current) return true;

    try {
      const ok = await writeWritingDraft(
        getWritingDraftKey(projectIdKey, chapterNumber),
        editorContentRef.current,
      );
      if (!ok) throw new Error(t("writing.localDraftSaveFailed"));
      draftSaveWarnedRef.current = false;
      setDraftRevision((value) => value + 1);
      return true;
    } catch (error) {
      reportDraftError(error);
      return false;
    }
  }, [projectIdKey, reportDraftError, t]);

  const getCurrentEditorContent = useCallback(
    () => editorContentRef.current,
    [],
  );

  useEffect(() => () => {
    void saveLocalDraftForCurrent();
  }, [saveLocalDraftForCurrent]);

  const confirmCurrentEdit = useCallback(
    async (actionText: string) => {
      if (!dirtyRef.current) return true;
      const draftOk = await saveLocalDraftForCurrent();
      const ok = await confirmDialog({
        title: t("writing.unsavedEditTitle"),
        message: draftOk
          ? t("writing.unsavedEditMessage", { action: actionText })
          : t("writing.unsavedEditMessageNoDraft", { action: actionText }),
        confirmText: t("writing.continueAction"),
        cancelText: t("writing.stayCurrent"),
        dialogType: "warning",
      });
      return ok;
    },
    [saveLocalDraftForCurrent],
  );

  useEffect(() => {
    const onBeforeUnload = (event: BeforeUnloadEvent) => {
      if (!dirtyRef.current) return;
      void saveLocalDraftForCurrent();
      event.preventDefault();
      event.returnValue = "";
    };

    window.addEventListener("beforeunload", onBeforeUnload);
    return () => window.removeEventListener("beforeunload", onBeforeUnload);
  }, [saveLocalDraftForCurrent]);

  useEffect(() => {
    if (!chapterReady) return;
    const baseContent = chapterContentForEditor(activeChapter);
    const activeNumberChanged = editorActiveNumberRef.current !== activeNumber;
    if (
      activeNumberChanged ||
      !dirtyRef.current ||
      editorContentRef.current === baseContent
    ) {
      setEditorState(baseContent, false);
      editorRevisionRef.current = activeChapter?.edit_revision ?? null;
    }
    editorActiveNumberRef.current = activeNumber;

    if (!dirtyRef.current && !draftPromptOpenRef.current && projectIdKey && activeNumber != null) {
      const draftKey = getWritingDraftKey(projectIdKey, activeNumber);
      const draft = readWritingDraft(draftKey);
      const draftVersion = JSON.stringify(draft);
      if (draftPromptedRef.current.get(draftKey) !== draftVersion) {
        if (draft && draft.content !== baseContent) {
          draftPromptedRef.current.set(draftKey, draftVersion);
          draftPromptOpenRef.current = true;
          const ts = formatDateTime(draft.updatedAt, locale, t("common.unknown"));
          void confirmDialog({
            title: t("writing.restoreDraftTitle"),
            message: t("writing.restoreDraftMessage", { time: ts }),
            confirmText: t("writing.restoreDraft"),
            cancelText: t("writing.keepCurrent"),
            dialogType: "warning",
          }).then((ok) => {
            if (!ok || !mountedRef.current || projectIdKeyRef.current !== projectIdKey
              || activeNumberRef.current !== activeNumber || dirtyRef.current)
              return;
            const currentDraft = readWritingDraft(draftKey);
            if (!currentDraft) {
              addToast(t("writing.draftRemovedElsewhere"), "info");
              return;
            }
            draftPromptedRef.current.set(draftKey, JSON.stringify(currentDraft));
            setEditorState(currentDraft.content, true);
            addToast(t("writing.draftRestored"), "success");
          }).catch(reportDraftError).finally(() => {
            draftPromptOpenRef.current = false;
          });
        }
      }
    }

  }, [
    chapterReady,
    activeNumber,
    activeChapter?.id,
    activeChapter?.content,
    activeChapter?.edit_revision,
    addToast,
    draftRevision,
    locale,
    projectIdKey,
    reportDraftError,
    setEditorState,
    t,
  ]);

  useEffect(() => {
    if (!chapterReady || !projectIdKey || activeNumber == null || !dirty) return;

    const chapterNumber = activeNumber;
    const content = editorContent;
    const baseContent = chapterContentForEditor(activeChapter);
    let cancelled = false;
    const save = async () => {
      if (activeNumberRef.current !== chapterNumber || !dirtyRef.current)
        return;
      try {
        if (content === baseContent) {
          await removeWritingDraft(getWritingDraftKey(projectIdKey, chapterNumber));
          if (!cancelled && editorContentRef.current === content) setEditorState(content, false);
        } else {
          const ok = await writeWritingDraft(getWritingDraftKey(projectIdKey, chapterNumber), content);
          if (!ok) throw new Error(t("writing.localDraftSaveFailed"));
        }
        draftSaveWarnedRef.current = false;
        if (!cancelled) setDraftRevision((value) => value + 1);
      } catch (error) {
        reportDraftError(error);
      }
    };
    const timer = window.setTimeout(() => void save(), 800);

    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [
    chapterReady,
    activeChapter?.content,
    activeNumber,
    dirty,
    editorContent,
    projectIdKey,
    reportDraftError,
    setEditorState,
    t,
  ]);

  const removeDraftForChapter = useCallback(
    async (chapterNumber: number) => {
      if (!projectIdKey) return;
      await removeWritingDraft(getWritingDraftKey(projectIdKey, chapterNumber));
      setDraftRevision((value) => value + 1);
    },
    [projectIdKey],
  );

  const discardCurrentEdit = useCallback(async () => {
    if (!canDiscard()) return false;
    const chapterNumber = activeNumberRef.current;
    try {
      if (chapterNumber != null) await removeDraftForChapter(chapterNumber);
      setEditorState(chapterContentForEditor(activeChapter), false);
      return true;
    } catch (error) {
      reportDraftError(error);
      return false;
    }
  }, [activeChapter, canDiscard, removeDraftForChapter, reportDraftError, setEditorState]);

  useDirty(`writing-desk:${projectIdKey || "unknown"}`, dirty, discardCurrentEdit, canDiscard);

  const clearProjectDrafts = useCallback(async () => {
    if (!projectIdKey) return;
    for (const chapterNumber of Array.from(
      getProjectWritingDraftSet(projectIdKey),
    )) {
      await removeWritingDraft(getWritingDraftKey(projectIdKey, chapterNumber));
    }
    setDraftRevision((value) => value + 1);
  }, [projectIdKey]);

  const resetDraftTracking = useCallback(() => {
    editorActiveNumberRef.current = null;
    draftPromptedRef.current.clear();
  }, []);

  return {
    editorContent,
    editorRevisionRef,
    dirty,
    dirtyRef,
    activeNumberRef,
    draftSet,
    setEditorState,
    getCurrentEditorContent,
    saveLocalDraftForCurrent,
    confirmCurrentEdit,
    refreshDraftSet,
    removeDraftForChapter,
    clearProjectDrafts,
    resetDraftTracking,
  };
}
