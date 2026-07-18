import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { ChapterData } from "../../api/chapter";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import type { ToastType } from "../../components/feedback/Toast";
import {
  getProjectWritingDraftSet,
  getWritingDraftKey,
  readWritingDraft,
  removeWritingDraft,
  writeWritingDraft,
} from "../../utils/writingDraft";
import { formatUiDateTime } from "../../utils/dateTime";

type AddToast = (message: unknown, type: ToastType) => void;

const chapterContentForEditor = (chapter: ChapterData | null): string => {
  if (chapter == null) return "";
  return chapter.content;
};

interface UseWritingDeskDraftsParams {
  projectIdKey: string;
  activeNumber: number | null;
  activeChapter: ChapterData | null;
  addToast: AddToast;
}

export function useWritingDeskDrafts({
  projectIdKey,
  activeNumber,
  activeChapter,
  addToast,
}: UseWritingDeskDraftsParams) {
  const [editorContent, setEditorContent] = useState("");
  const [dirty, setDirty] = useState(false);
  const [draftRevision, setDraftRevision] = useState(0);
  const editorContentRef = useRef("");
  const dirtyRef = useRef(false);
  const activeNumberRef = useRef<number | null>(null);
  const draftPromptedRef = useRef<Set<string>>(new Set());
  const draftSaveWarnedRef = useRef(false);
  const editorChapterKeyRef = useRef<string | null>(null);
  const editorActiveNumberRef = useRef<number | null>(null);

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

  const saveLocalDraftForCurrent = useCallback(() => {
    const chapterNumber = activeNumberRef.current;
    if (!projectIdKey || !chapterNumber || !dirtyRef.current) return true;

    const ok = writeWritingDraft(
      getWritingDraftKey(projectIdKey, chapterNumber),
      editorContentRef.current,
    );
    if (ok) {
      setDraftRevision((value) => value + 1);
      return true;
    }

    if (!draftSaveWarnedRef.current) {
      draftSaveWarnedRef.current = true;
      addToast(
        "本地草稿保存失败（浏览器存储不可用或容量不足），请尽快手动保存",
        "info",
      );
    }
    return false;
  }, [addToast, projectIdKey]);

  const getCurrentEditorContent = useCallback(
    () => editorContentRef.current,
    [],
  );

  const confirmCurrentEdit = useCallback(
    async (actionText: string) => {
      if (!dirtyRef.current) return true;
      const draftOk = saveLocalDraftForCurrent();
      const ok = await confirmDialog({
        title: "未保存修改",
        message: draftOk
          ? `当前章节有未保存修改。\n${actionText}前已自动保存本地草稿，可随时恢复。\n是否继续？`
          : `当前章节有未保存修改。\n本地草稿保存失败，${actionText}可能丢失未保存内容。\n是否继续？`,
        confirmText: "继续",
        cancelText: "留在当前",
        dialogType: "warning",
      });
      return ok;
    },
    [saveLocalDraftForCurrent],
  );

  useEffect(() => {
    const onBeforeUnload = (event: BeforeUnloadEvent) => {
      if (!dirtyRef.current) return;
      saveLocalDraftForCurrent();
      event.preventDefault();
      event.returnValue = "";
    };

    window.addEventListener("beforeunload", onBeforeUnload);
    return () => window.removeEventListener("beforeunload", onBeforeUnload);
  }, [saveLocalDraftForCurrent]);

  useEffect(() => {
    let cancelled = false;
    const baseContent = chapterContentForEditor(activeChapter);
    const chapterKey =
      activeChapter != null
        ? `chapter:${activeChapter.id}`
        : activeNumber != null
          ? `outline:${activeNumber}`
          : null;
    const activeNumberChanged = editorActiveNumberRef.current !== activeNumber;
    const chapterChanged = editorChapterKeyRef.current !== chapterKey;
    if (
      activeNumberChanged ||
      !dirtyRef.current ||
      editorContentRef.current === baseContent
    ) {
      setEditorState(baseContent, false);
    }
    editorChapterKeyRef.current = chapterKey;
    editorActiveNumberRef.current = activeNumber;

    if (chapterChanged && projectIdKey && activeNumber != null) {
      const draftKey = getWritingDraftKey(projectIdKey, activeNumber);
      if (!draftPromptedRef.current.has(draftKey)) {
        draftPromptedRef.current.add(draftKey);
        const draft = readWritingDraft(draftKey);
        if (draft && draft.content !== baseContent) {
          const ts = formatUiDateTime(draft.updatedAt, "时间未知");
          void confirmDialog({
            title: "恢复草稿",
            message: `检测到本地草稿（${ts}）。\n是否恢复到编辑器？`,
            confirmText: "恢复草稿",
            cancelText: "保持当前",
            dialogType: "warning",
          }).then((ok) => {
            if (!ok || cancelled || activeNumberRef.current !== activeNumber)
              return;
            setEditorState(draft.content, true);
            addToast("已恢复本地草稿（未保存）", "success");
          });
        }
      }
    }

    return () => {
      cancelled = true;
    };
  }, [
    activeNumber,
    activeChapter?.id,
    activeChapter?.content,
    addToast,
    projectIdKey,
    setEditorState,
  ]);

  useEffect(() => {
    if (!projectIdKey || activeNumber == null || !dirty) return;

    const chapterNumber = activeNumber;
    const content = editorContent;
    const baseContent = chapterContentForEditor(activeChapter);
    const timer = window.setTimeout(() => {
      if (activeNumberRef.current !== chapterNumber || !dirtyRef.current)
        return;

      if (content === baseContent) {
        const key = getWritingDraftKey(projectIdKey, chapterNumber);
        removeWritingDraft(key);
        setDraftRevision((value) => value + 1);
        setEditorState(content, false);
        return;
      }

      const ok = writeWritingDraft(
        getWritingDraftKey(projectIdKey, chapterNumber),
        content,
      );
      if (ok) {
        setDraftRevision((value) => value + 1);
        return;
      }
      if (!draftSaveWarnedRef.current) {
        draftSaveWarnedRef.current = true;
        addToast(
          "本地草稿保存失败（浏览器存储不可用或容量不足），请尽快手动保存",
          "info",
        );
      }
    }, 800);

    return () => window.clearTimeout(timer);
  }, [
    activeChapter?.content,
    activeNumber,
    addToast,
    dirty,
    editorContent,
    projectIdKey,
    setEditorState,
  ]);

  const removeDraftForChapter = useCallback(
    (chapterNumber: number) => {
      if (!projectIdKey) return;
      removeWritingDraft(getWritingDraftKey(projectIdKey, chapterNumber));
      setDraftRevision((value) => value + 1);
    },
    [projectIdKey],
  );

  const clearProjectDrafts = useCallback(() => {
    if (!projectIdKey) return;
    for (const chapterNumber of Array.from(
      getProjectWritingDraftSet(projectIdKey),
    )) {
      removeWritingDraft(getWritingDraftKey(projectIdKey, chapterNumber));
    }
    setDraftRevision((value) => value + 1);
  }, [projectIdKey]);

  const resetDraftTracking = useCallback(() => {
    editorChapterKeyRef.current = null;
    editorActiveNumberRef.current = null;
    draftPromptedRef.current.clear();
  }, []);

  return {
    editorContent,
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
