import { useCallback, useMemo, type Dispatch, type MutableRefObject, type SetStateAction } from "react";
import { extractErrorMessage } from "../../api";
import { getProject, resetProjectGeneratedData, type NovelProject } from "../../api/project";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import type { ChapterMetadata } from "../../api/chapter";
import type { MessageTranslator } from "../../i18n";

interface UseWritingDeskResetGeneratedDataParams {
  projectId: number;
  project: NovelProject | null;
  chapters: ChapterMetadata[];
  lifecycleBlockedReason: string | null;
  resetGeneratedDataBusy: boolean;
  streamTaskId: string | null;
  streamBusy: boolean;
  resetGeneratedDataBusyRef: MutableRefObject<boolean>;
  setResetGeneratedDataBusy: (busy: boolean) => void;
  setProject: Dispatch<SetStateAction<NovelProject | null>>;
  clearResetProjectState: () => Promise<void>;
  addToast: (message: string, type: "success" | "error" | "info") => void;
  t: MessageTranslator;
}

export const useWritingDeskResetGeneratedData = ({
  projectId,
  project,
  chapters,
  lifecycleBlockedReason,
  resetGeneratedDataBusy,
  streamTaskId,
  streamBusy,
  resetGeneratedDataBusyRef,
  setResetGeneratedDataBusy,
  setProject,
  clearResetProjectState,
  addToast,
  t,
}: UseWritingDeskResetGeneratedDataParams) => {
  const resetGeneratedDataDisabledReason = useMemo(() => {
    if (!project) return t("writing.projectNotLoaded");
    if (lifecycleBlockedReason) return lifecycleBlockedReason;
    if (resetGeneratedDataBusy) return t("writing.resettingProjectData");
    if (streamTaskId) return t("writing.streamGenerationResetBlocked");
    if (streamBusy) return t("writing.streamStateResetBlocked");
    if (chapters.some((chapter) => chapter.status === "generating")) {
      return t("writing.chapterGenerationResetBlocked");
    }
    return null;
  }, [chapters, lifecycleBlockedReason, project, resetGeneratedDataBusy, streamBusy, streamTaskId, t]);

  const handleResetGeneratedData = useCallback(async () => {
    if (resetGeneratedDataBusyRef.current) {
      addToast(t("writing.resettingProjectData"), "error");
      return;
    }
    if (resetGeneratedDataDisabledReason) {
      addToast(resetGeneratedDataDisabledReason, "error");
      return;
    }
    resetGeneratedDataBusyRef.current = true;
    const ok = await confirmDialog({
      title: t("writing.resetProjectData"),
      message: t("writing.resetProjectDataMessage"),
      confirmText: t("common.reset"),
      cancelText: t("common.cancel"),
      dialogType: "danger",
    });
    if (!ok) {
      resetGeneratedDataBusyRef.current = false;
      return;
    }

    setResetGeneratedDataBusy(true);
    try {
      const response = await resetProjectGeneratedData(projectId);
      const warnings = [...response.warnings];
      const formalResetCommitted = response.database_committed;
      let nextProject = response.result?.project ?? null;
      if (!nextProject) {
        try {
          nextProject = await getProject(projectId);
        } catch (error) {
          warnings.push(extractErrorMessage(error, t("writing.refreshProjectStatusFailed")));
        }
      }
      if (nextProject) {
        setProject(nextProject);
      } else if (response.status === "pending") {
        setProject((current) =>
          current ? { ...current, lifecycle_gate: "resetting" } : current,
        );
      }
      if (formalResetCommitted) await clearResetProjectState();

      const warningText = warnings.length > 0
        ? `${t("common.summarySeparator")}${warnings.join(t("common.summarySeparator"))}`
        : "";
      if (response.status === "pending") {
        addToast(
          formalResetCommitted
            ? t("writing.resetProjectDataCommitted", { warnings: warningText })
            : t("writing.resetProjectDataAccepted", { warnings: warningText }),
          "info",
        );
      } else if (response.result) {
        addToast(
          t("writing.resetProjectDataSummary", {
            chapters: response.result.deleted_chapters,
            outlines: response.result.deleted_chapter_outlines,
            profiles: response.result.deleted_character_profiles,
            versions: response.result.preserved_blueprint_versions,
            warnings: warningText,
          }),
          warnings.length > 0 ? "info" : "success",
        );
      } else {
        addToast(
          t("writing.resetProjectDataDone", { warnings: warningText }),
          warnings.length > 0 ? "info" : "success",
        );
      }
    } catch (err) {
      addToast(extractErrorMessage(err, t("writing.resetProjectDataFailed")), "error");
    } finally {
      resetGeneratedDataBusyRef.current = false;
      setResetGeneratedDataBusy(false);
    }
  }, [
    addToast,
    clearResetProjectState,
    projectId,
    resetGeneratedDataBusyRef,
    resetGeneratedDataDisabledReason,
    setProject,
    setResetGeneratedDataBusy,
    t,
  ]);

  return {
    canResetGeneratedData: resetGeneratedDataDisabledReason == null,
    handleResetGeneratedData,
  };
};
