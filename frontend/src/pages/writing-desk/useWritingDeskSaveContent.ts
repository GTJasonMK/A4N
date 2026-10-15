import {
  useCallback,
  type MutableRefObject,
} from "react";
import { extractErrorMessage } from "../../api";
import {
  chapterContentCommitToastType,
  importChapter,
  updateChapterContent,
  type ChapterSummary,
} from "../../api/chapter";
import type { ChapterOutlineListEntry } from "../../api/outline";
import type { Locale, MessageTranslator } from "../../i18n";
import { formatNumber } from "../../i18n";

interface UseWritingDeskSaveContentParams {
  projectId: number;
  locale: Locale;
  lifecycleBlockedReason: string | null;
  activeNumber: number | null;
  activeChapter: ChapterSummary | null;
  activeOutline: ChapterOutlineListEntry | null;
  activeChapterBusy: boolean;
  derivedSyncBusy: boolean;
  activeChapterGeneratingMessage: string;
  editorContent: string;
  editorRevisionRef: MutableRefObject<string | null>;
  saveContentBusyRef: MutableRefObject<boolean>;
  contentDiffActionRef: MutableRefObject<boolean>;
  resetGeneratedDataBusyRef: MutableRefObject<boolean>;
  activeNumberRef: MutableRefObject<number | null>;
  addToast: (message: string, type: "success" | "error" | "info") => void;
  t: MessageTranslator;
  setSavingContent: (saving: boolean) => void;
  applyChapter: (chapter: ChapterSummary) => void;
  getCurrentEditorContent: () => string;
  removeDraftForChapter: (chapterNumber: number) => Promise<void>;
  setEditorState: (content: string, dirty: boolean) => void;
  saveLocalDraftForCurrent: () => Promise<boolean>;
}

export const useWritingDeskSaveContent = ({
  projectId,
  locale,
  lifecycleBlockedReason,
  activeNumber,
  activeChapter,
  activeOutline,
  activeChapterBusy,
  derivedSyncBusy,
  activeChapterGeneratingMessage,
  editorContent,
  editorRevisionRef,
  saveContentBusyRef,
  contentDiffActionRef,
  resetGeneratedDataBusyRef,
  activeNumberRef,
  addToast,
  t,
  setSavingContent,
  applyChapter,
  getCurrentEditorContent,
  removeDraftForChapter,
  setEditorState,
  saveLocalDraftForCurrent,
}: UseWritingDeskSaveContentParams) => useCallback(async () => {
  if (lifecycleBlockedReason) {
    addToast(lifecycleBlockedReason, "info");
    return;
  }
  if (saveContentBusyRef.current) {
    addToast(t("writing.savingChapterBody"), "error");
    return;
  }
  if (derivedSyncBusy) {
    addToast(t("common.operationInProgress"), "info");
    return;
  }
  if (resetGeneratedDataBusyRef.current) {
    addToast(t("common.operationInProgress"), "info");
    return;
  }
  if (contentDiffActionRef.current) {
    addToast(t("common.operationInProgress"), "info");
    return;
  }
  saveContentBusyRef.current = true;
  setSavingContent(true);
  try {
    const chapterNumber = activeNumber;
    if (!chapterNumber) {
      addToast(t("writing.chooseChapterFirst"), "error");
      return;
    }
    if (activeChapterBusy) {
      addToast(activeChapterGeneratingMessage, "error");
      return;
    }
    const contentToSave = editorContent;
    if (!contentToSave.trim()) {
      addToast(t("writing.savingEmptyBody"), "error");
      return;
    }
    let savedContent = contentToSave;
    let savedChapter: ChapterSummary | null = null;
    const isCurrentChapter = () => activeNumberRef.current === chapterNumber;
    if (activeChapter) {
      const expectedRevision = editorRevisionRef.current;
      if (!expectedRevision) throw new Error(t("writing.editorRevisionUnavailable"));
      const updated = await updateChapterContent({
        chapter_id: activeChapter.id,
        expected_revision: expectedRevision,
        content: contentToSave,
        title: activeChapter.title,
      });
      savedContent = updated.content;
      savedChapter = updated;
      applyChapter(updated);
    } else {
      const title =
        activeOutline?.title?.trim() ||
        t("writing.chapterLabel", { number: formatNumber(chapterNumber, locale) });
      const imported = await importChapter({
        project_id: projectId,
        chapter_number: chapterNumber,
        content: contentToSave,
        title,
      });
      savedChapter = imported;
      savedContent = imported.content;
      applyChapter(imported);
    }
    if (isCurrentChapter()) {
      editorRevisionRef.current = savedChapter.edit_revision;
    }
    if (!isCurrentChapter()) return;
    const saveStillMatchesEditor =
      isCurrentChapter() &&
      getCurrentEditorContent() === contentToSave;
    if (saveStillMatchesEditor) {
      let draftCleanupError: string | null = null;
      try {
        await removeDraftForChapter(chapterNumber);
      } catch (error) {
        draftCleanupError = extractErrorMessage(error, t("writing.localDraftCleanupFailed"));
      }
      if (activeNumberRef.current === chapterNumber && getCurrentEditorContent() === contentToSave) {
        setEditorState(savedContent, false);
      } else {
        await saveLocalDraftForCurrent();
        addToast(t("writing.savedBodyWithUnsavedEdits"), "info");
        return;
      }
      let savedMessage: string;
      let savedType: "success" | "error" | "info" = "success";
      if (savedChapter?.derived_sync_status === "failed") {
        savedMessage = t("writing.savedBodySyncFailed");
        savedType = chapterContentCommitToastType(savedChapter.derived_sync_status);
      } else if (savedChapter?.derived_sync_status === "running") {
        savedMessage = t("writing.savedBodySyncRunning");
        savedType = "info";
      } else {
        savedMessage = t("writing.savedBodyAndMaterials");
      }
      if (draftCleanupError) {
        savedMessage = `${savedMessage}${t("common.summarySeparator")}${t("writing.savedBodyDraftCleanupFailed", { error: draftCleanupError })}`;
        if (savedType === "success") savedType = "info";
      }
      addToast(savedMessage, savedType);
      return;
    }

    await saveLocalDraftForCurrent();
    addToast(t("writing.savedBodyWithUnsavedEdits"), "info");
  } catch (err) {
    if (activeNumberRef.current === activeNumber) {
      addToast(extractErrorMessage(err, t("errors.saveFailed")), "error");
    }
  } finally {
    saveContentBusyRef.current = false;
    setSavingContent(false);
  }
}, [
  activeChapter,
  activeChapterBusy,
  derivedSyncBusy,
  activeChapterGeneratingMessage,
  activeNumber,
  activeNumberRef,
  activeOutline,
  addToast,
  editorContent,
  editorRevisionRef,
  getCurrentEditorContent,
  lifecycleBlockedReason,
  locale,
  projectId,
  removeDraftForChapter,
  saveContentBusyRef,
  contentDiffActionRef,
  resetGeneratedDataBusyRef,
  saveLocalDraftForCurrent,
  applyChapter,
  setEditorState,
  setSavingContent,
  t,
]);
