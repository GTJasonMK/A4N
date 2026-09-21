import {
  useCallback,
  type Dispatch,
  type MutableRefObject,
  type SetStateAction,
} from "react";
import { extractErrorMessage } from "../../api";
import {
  chapterContentCommitToastType,
  importChapter,
  updateChapterContent,
  type ChapterSummary,
} from "../../api/chapter";
import type { ChapterOutline } from "../../api/outline";
import type { MessageTranslator } from "../../i18n";
import { formatNumber } from "../../i18n";

interface UseWritingDeskSaveContentParams {
  projectId: number;
  locale: string;
  lifecycleBlockedReason: string | null;
  activeNumber: number | null;
  activeChapter: ChapterSummary | null;
  activeOutline: ChapterOutline | null;
  activeChapterBusy: boolean;
  activeChapterGeneratingMessage: string;
  editorContent: string;
  saveContentBusyRef: MutableRefObject<boolean>;
  activeNumberRef: MutableRefObject<number | null>;
  addToast: (message: string, type: "success" | "error" | "info") => void;
  t: MessageTranslator;
  setSavingContent: (saving: boolean) => void;
  setChapters: Dispatch<SetStateAction<ChapterSummary[]>>;
  reloadChapters: (options?: { force?: boolean }) => Promise<ChapterSummary[] | null>;
  getCurrentEditorContent: () => string;
  removeDraftForChapter: (chapterNumber: number) => void;
  setEditorState: (content: string, dirty: boolean) => void;
  saveLocalDraftForCurrent: () => void;
}

export const useWritingDeskSaveContent = ({
  projectId,
  locale,
  lifecycleBlockedReason,
  activeNumber,
  activeChapter,
  activeOutline,
  activeChapterBusy,
  activeChapterGeneratingMessage,
  editorContent,
  saveContentBusyRef,
  activeNumberRef,
  addToast,
  t,
  setSavingContent,
  setChapters,
  reloadChapters,
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
    if (activeChapter) {
      const updated = await updateChapterContent({
        chapter_id: activeChapter.id,
        content: contentToSave,
        title: activeChapter.title,
      });
      savedContent = updated.content;
      savedChapter = updated;
      setChapters((prev) =>
        prev.map((chapter) => (chapter.id === updated.id ? updated : chapter)),
      );
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
      const nextChapters = await reloadChapters({ force: true });
      const created = nextChapters?.find(
        (chapter) => chapter.chapter_number === chapterNumber,
      );
      savedChapter = created ?? imported;
      savedContent = savedChapter.content;
    }
    const saveStillMatchesEditor =
      activeNumberRef.current === chapterNumber &&
      getCurrentEditorContent() === contentToSave;
    if (saveStillMatchesEditor) {
      removeDraftForChapter(chapterNumber);
      setEditorState(savedContent, false);
      if (savedChapter?.derived_sync_status === "failed") {
        addToast(
          t("writing.savedBodySyncFailed"),
          chapterContentCommitToastType(savedChapter.derived_sync_status),
        );
      } else if (savedChapter?.derived_sync_status === "running") {
        addToast(t("writing.savedBodySyncRunning"), "info");
      } else {
        addToast(t("writing.savedBodyAndMaterials"), "success");
      }
      return;
    }

    saveLocalDraftForCurrent();
    addToast(t("writing.savedBodyWithUnsavedEdits"), "info");
  } catch (err) {
    addToast(extractErrorMessage(err, t("errors.saveFailed")), "error");
  } finally {
    saveContentBusyRef.current = false;
    setSavingContent(false);
  }
}, [
  activeChapter,
  activeChapterBusy,
  activeChapterGeneratingMessage,
  activeNumber,
  activeNumberRef,
  activeOutline,
  addToast,
  editorContent,
  getCurrentEditorContent,
  lifecycleBlockedReason,
  locale,
  projectId,
  reloadChapters,
  removeDraftForChapter,
  saveContentBusyRef,
  saveLocalDraftForCurrent,
  setChapters,
  setEditorState,
  setSavingContent,
  t,
]);
