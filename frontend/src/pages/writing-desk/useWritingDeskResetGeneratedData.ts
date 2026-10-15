import { useCallback, useMemo, type Dispatch, type MutableRefObject, type SetStateAction } from "react";
import { extractErrorMessage } from "../../api";
import { canEditProjectMaterials, getProject, resetProjectGeneratedData, type NovelProject } from "../../api/project";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import type { ChapterMetadata } from "../../api/chapter";
import type { MessageTranslator } from "../../i18n";
import { getProjectLifecycleBlockedReason } from "../../utils/projectStatus";

interface UseWritingDeskResetGeneratedDataParams {
  projectId: number;
  projectIdRef: MutableRefObject<number>;
  confirmPendingEdits: () => Promise<boolean>;
  project: NovelProject | null;
  chapters: ChapterMetadata[];
  lifecycleBlockedReason: string | null;
  resetGeneratedDataBusy: boolean;
  derivedSyncBusy: boolean;
  streamTaskId: string | null;
  streamBusy: boolean;
  resetGeneratedDataBusyRef: MutableRefObject<boolean>;
  setResetGeneratedDataBusy: (busy: boolean) => void;
  setProject: Dispatch<SetStateAction<NovelProject | null>>;
  clearResetProjectState: () => Promise<string | null>;
  addToast: (message: string, type: "success" | "error" | "info") => void;
  t: MessageTranslator;
}

export const useWritingDeskResetGeneratedData = ({
  projectId,
  projectIdRef,
  confirmPendingEdits,
  project,
  chapters,
  lifecycleBlockedReason,
  resetGeneratedDataBusy,
  derivedSyncBusy,
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
    if (derivedSyncBusy) return t("common.operationInProgress");
    if (streamTaskId) return t("writing.streamGenerationResetBlocked");
    if (streamBusy) return t("writing.streamStateResetBlocked");
    if (chapters.some((chapter) => chapter.status === "generating")) {
      return t("writing.chapterGenerationResetBlocked");
    }
    return null;
  }, [chapters, derivedSyncBusy, lifecycleBlockedReason, project, resetGeneratedDataBusy, streamBusy, streamTaskId, t]);

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
    if (!(await confirmPendingEdits())) {
      resetGeneratedDataBusyRef.current = false;
      return;
    }
    const ok = await confirmDialog({
      title: t("writing.resetProjectData"),
      message: t("writing.resetProjectDataMessage"),
      confirmText: t("common.reset"),
      cancelText: t("common.cancel"),
      dialogType: "danger",
    });
    if (!ok || projectIdRef.current !== projectId) {
      resetGeneratedDataBusyRef.current = false;
      return;
    }

    setResetGeneratedDataBusy(true);
    try {
      const latestProject = await getProject(String(projectId));
      if (projectIdRef.current !== projectId) return;
      if (latestProject.lifecycle_gate !== "active" || !canEditProjectMaterials(latestProject)) {
        addToast(getProjectLifecycleBlockedReason(latestProject, t) ?? t("novel.editPermissionDenied"), "error");
        return;
      }
      const response = await resetProjectGeneratedData(projectId);
      const warnings = [...response.warnings];
      const formalResetCommitted = response.database_committed;
      let nextProject = response.result?.project ?? null;
      let stillShowingRequestProject = projectIdRef.current === projectId;
      if (stillShowingRequestProject && !nextProject) {
        try {
          nextProject = await getProject(projectId);
        } catch (error) {
          warnings.push(extractErrorMessage(error, t("writing.refreshProjectStatusFailed")));
        }
      }
      stillShowingRequestProject = projectIdRef.current === projectId;
      if (stillShowingRequestProject && nextProject) {
        setProject(nextProject);
      } else if (stillShowingRequestProject && response.status === "pending") {
        setProject((current) =>
          current ? { ...current, lifecycle_gate: "resetting" } : current,
        );
      }
      if (formalResetCommitted && stillShowingRequestProject) {
        const draftCleanupError = await clearResetProjectState();
        if (draftCleanupError) {
          warnings.push(t("writing.resetDraftCleanupWarning", { error: draftCleanupError }));
        }
      }

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
    confirmPendingEdits,
    projectIdRef,
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
