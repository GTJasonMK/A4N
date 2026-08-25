import { useCallback, useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { extractErrorMessage } from "../../api";
import {
  type Blueprint,
  generateBlueprint,
  getBlueprint,
  matchesBlueprintTaskResult,
  previewGenerateBlueprintRequest,
  previewRefineBlueprintRequest,
  refineBlueprint,
  syncBlueprintMaterials,
} from "../../api/blueprint";
import { generationClient } from "../../api/generationTasks";
import type { GenerationTaskHandle } from "../../api/generated/generationContracts";
import type { ModelRequestPreview } from "../../api/requestPreview";
import type { NovelProject } from "../../api/project";
import { useGenerationTask } from "../../hooks/useGenerationTask";
import { useGenerationTasksStore, generationTaskKey } from "../../store/generationTasks";
import type { ToastType } from "../../components/feedback/Toast";
import { logger } from "../../utils/logger";
import {
  generationTaskRecoveryIssue,
  resolveFormalResultRecovery,
} from "../../utils/generationEventReducer";
import { getProjectPrimaryRoute } from "../../utils/projectRouting";
import {
  buildInspirationTranscript,
  createBlueprintProgressState,
  type BlueprintProgressState,
  type Stage,
} from "./inspirationChatUtils";
import type { Message } from "./shared";
import { useI18n } from "../../i18n";

type AddToast = (message: string, type: ToastType) => void;
type BlueprintOperation = "generation" | "refinement";

interface UseBlueprintWorkflowOptions {
  projectId: number;
  project: NovelProject | null;
  messages: Message[];
  conversationComplete: boolean;
  sending: boolean;
  latestConversationState: Record<string, unknown> | undefined;
  progressSummary: string | undefined;
  nextQuestion: string | undefined;
  addToast: AddToast;
}

export const useBlueprintWorkflow = ({
  projectId,
  project,
  messages,
  conversationComplete,
  sending,
  latestConversationState,
  progressSummary,
  nextQuestion,
  addToast,
}: UseBlueprintWorkflowOptions) => {
  const { t } = useI18n();
  const navigate = useNavigate();
  const [stage, setStage] = useState<Stage>("conversation");
  const [blueprint, setBlueprint] = useState<Blueprint | null>(null);
  const [blueprintConsistencyWarning, setBlueprintConsistencyWarning] = useState<string | null>(null);
  const [generationHandle, setGenerationHandle] = useState<GenerationTaskHandle | null>(null);
  const [operation, setOperation] = useState<BlueprintOperation | null>(null);
  const [generating, setGenerating] = useState(false);
  const [refining, setRefining] = useState(false);
  const [blueprintProgressOpen, setBlueprintProgressOpen] = useState(false);
  const [blueprintProgress, setBlueprintProgress] = useState<BlueprintProgressState>(() =>
    createBlueprintProgressState(t("inspiration.progressPreparing")),
  );
  const [blueprintCancelling, setBlueprintCancelling] = useState(false);
  const [requestPreviewOpen, setRequestPreviewOpen] = useState(false);
  const [requestPreviewLoading, setRequestPreviewLoading] = useState(false);
  const [requestPreviewData, setRequestPreviewData] = useState<ModelRequestPreview | null>(null);

  const generationHandleRef = useRef<GenerationTaskHandle | null>(null);
  const handledTerminalRef = useRef<string | null>(null);
  const activeProjectIdRef = useRef(projectId);
  const mountedRef = useRef(true);
  const generatingRef = useRef(false);
  const requestPreviewSeqRef = useRef(0);
  const refineSeqRef = useRef(0);
  const confirmTimeoutRef = useRef<number | null>(null);

  activeProjectIdRef.current = projectId;
  generationHandleRef.current = generationHandle;

  const generationTask = useGenerationTask({ handle: generationHandle });
  const taskKey = generationHandle
    ? generationTaskKey(generationHandle.task_id, generationHandle.attempt)
    : null;
  const streamText = generationTask.text("raw_json");

  const isCurrentProject = useCallback((requestProjectId: number) => {
    return mountedRef.current && activeProjectIdRef.current === requestProjectId;
  }, []);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      generatingRef.current = false;
      if (confirmTimeoutRef.current != null) window.clearTimeout(confirmTimeoutRef.current);
    };
  }, []);

  useEffect(() => {
    setStage("conversation");
    setBlueprint(null);
    setBlueprintConsistencyWarning(null);
    setGenerating(false);
    setRefining(false);
    setGenerationHandle(null);
    setOperation(null);
    setBlueprintProgressOpen(false);
    setBlueprintProgress(createBlueprintProgressState(t("inspiration.progressPreparing")));
    setBlueprintCancelling(false);
    setRequestPreviewOpen(false);
    setRequestPreviewLoading(false);
    setRequestPreviewData(null);
    handledTerminalRef.current = null;
    generationHandleRef.current = null;
    generatingRef.current = false;
    requestPreviewSeqRef.current += 1;
    refineSeqRef.current += 1;
  }, [projectId, t]);

  // Reattach a still-active task after a page remount; the persisted target is the owner.
  useEffect(() => {
    let active = true;
    void generationClient.listActive(projectId).then((items) => {
      if (!active || generationHandleRef.current) return;
      const item = items.find((candidate) =>
        candidate.started.project_id === projectId
        && (candidate.started.target?.kind === "blueprint_generation"
          || candidate.started.target?.kind === "blueprint_refinement"),
      );
      if (!item) return;
      const nextOperation: BlueprintOperation =
        item.started.target?.kind === "blueprint_refinement" ? "refinement" : "generation";
      setGenerationHandle(item.handle);
      setOperation(nextOperation);
      setGenerating(nextOperation === "generation");
      setRefining(nextOperation === "refinement");
      setBlueprintProgressOpen(nextOperation === "generation");
    }).catch((error) => {
      if (active) addToast(extractErrorMessage(error, t("inspiration.restoreBlueprintFailed")), "error");
    });
    return () => {
      active = false;
    };
  }, [addToast, projectId, t]);

  useEffect(() => {
    if (!generationHandle || !generationTask.progress) return;
    const progress = generationTask.progress;
    setBlueprintProgress((previous) => ({
      ...previous,
      status: generationTask.terminalReceived ? previous.status : "running",
      current: Number(progress.current ?? previous.current),
      total: Number(progress.total ?? previous.total),
      message: progress.message,
      error: null,
    }));
  }, [generationHandle, generationTask.progress, generationTask.terminalReceived]);

  useEffect(() => {
    const recoveryIssue = generationTaskRecoveryIssue(generationTask);
    if (
      !generationHandle
      || !operation
      || !taskKey
      || (!generationTask.terminalReceived
        && !recoveryIssue)
    ) return;
    if (handledTerminalRef.current === taskKey) return;
    handledTerminalRef.current = taskKey;
    const requestProjectId = projectId;
    const currentOperation = operation;
    const finish = async () => {
      if (generationTask.taskStatus === "completed" || recoveryIssue) {
        const resultRef = generationTask.resultRef;
        const canResolveResult = resultRef?.kind === "novel_blueprint"
          && Number.isSafeInteger(Number(resultRef.id));
        const persisted = canResolveResult ? await getBlueprint(requestProjectId) : null;
        const consistent = matchesBlueprintTaskResult(persisted, requestProjectId, resultRef);
        const recovery = resolveFormalResultRecovery({
          t,
          recoveryIssue,
          consistent,
          hasPersisted: Boolean(persisted),
          hasResultRef: Boolean(resultRef),
          domain: "blueprint",
        });
        if (recovery.kind !== "ok") {
          if (recovery.shouldMarkConsistencyError) {
            useGenerationTasksStore.getState().dispatch(taskKey, {
              type: "client_state",
              state: "consistency_error",
            });
          }
          if (recovery.kind === "readonly" && persisted) {
            setBlueprint(persisted);
            setStage("confirm");
            setBlueprintConsistencyWarning(recovery.warning);
          } else {
            setBlueprintConsistencyWarning(null);
          }
          throw new Error(recovery.message);
        }
        if (!isCurrentProject(requestProjectId) || !persisted) return;
        const syncWarnings = await syncBlueprintMaterials(requestProjectId);
        if (!isCurrentProject(requestProjectId)) return;
        setBlueprintConsistencyWarning(null);
        setBlueprint(persisted);
        if (currentOperation === "generation") {
          setStage("confirm");
          setBlueprintProgress((previous) => ({
            ...previous,
            status: "success",
            current: previous.total,
            message: t("inspiration.blueprintGenerated"),
            error: null,
          }));
          addToast(
            syncWarnings.length > 0
              ? `${t("inspiration.blueprintGeneratedConfirm")}${t("common.summarySeparator")}${syncWarnings.join(t("common.summarySeparator"))}`
              : t("inspiration.blueprintGeneratedConfirm"),
            syncWarnings.length > 0 ? "info" : "success",
          );
        } else {
          addToast(
            syncWarnings.length > 0
              ? `${t("inspiration.blueprintOptimized")}${t("common.summarySeparator")}${syncWarnings.join(t("common.summarySeparator"))}`
              : t("inspiration.blueprintOptimized"),
            syncWarnings.length > 0 ? "info" : "success",
          );
        }
        return;
      }
      if (generationTask.taskStatus === "cancelled" || generationTask.taskStatus === "interrupted") {
        setBlueprintProgress((previous) => ({
          ...previous,
          status: "cancelled",
          message: t("inspiration.blueprintStopped"),
          error: null,
        }));
        return;
      }
      throw new Error(generationTask.error?.message ?? t("inspiration.blueprintFailed"));
    };
    void finish().catch((error) => {
      if (!isCurrentProject(requestProjectId)) return;
      const message = extractErrorMessage(error, t("inspiration.blueprintFailed"));
      logger.scoped("InspirationChat").error("blueprint generation failed", { error });
      setBlueprintProgress((previous) => ({
        ...previous,
        status: "error",
        message: currentOperation === "refinement" ? t("inspiration.blueprintRefineFailed") : t("inspiration.blueprintFailed"),
        error: message,
      }));
      addToast(message, "error");
    }).finally(() => {
      if (!isCurrentProject(requestProjectId)) return;
      setGenerating(false);
      setRefining(false);
      generatingRef.current = false;
      setBlueprintCancelling(false);
      setGenerationHandle(null);
      setOperation(null);
    });
  }, [
    addToast,
    generationHandle,
    generationTask.error,
    generationTask.clientState,
    generationTask.recoveryState,
    generationTask.resultRef,
    generationTask.taskStatus,
    generationTask.terminalReceived,
    isCurrentProject,
    operation,
    projectId,
    taskKey,
    t,
  ]);

  const canGenerate = conversationComplete
    && !sending
    && !generating
    && stage === "conversation";
  const canCancelBlueprintGeneration = Boolean(
    generationHandle && operation === "generation" && generating && !generationTask.terminalReceived,
  );

  const handleBackToConversation = useCallback(() => {
    setBlueprintConsistencyWarning(null);
    setStage("conversation");
  }, []);

  const buildBlueprintGenerateInput = useCallback(() => {
    if (!project || !latestConversationState) return null;
    const targetLength = latestConversationState.chapter_count;
    if (
      typeof targetLength !== "number"
      || !Number.isInteger(targetLength)
      || targetLength < 5
      || targetLength > 10000
    ) {
      throw new Error("完成态 conversation_state.chapter_count 必须是 5-10000 的整数");
    }
    const genre = typeof latestConversationState.genre === "string"
      ? latestConversationState.genre.trim()
      : "";
    if (!genre) throw new Error("完成态 conversation_state.genre 必须是非空字符串");

    return {
      project_id: projectId,
      title: project.title,
      genre,
      target_length: targetLength,
      variables: {
        inspiration_transcript: buildInspirationTranscript(messages),
        conversation_state: latestConversationState,
        progress_summary: progressSummary ?? null,
        next_question: nextQuestion ?? null,
        project: {
          id: project.id,
          title: project.title,
          status: project.status,
          project_type: project.project_type,
          description: project.description,
          target_chapters: project.target_chapters,
        },
      },
    };
  }, [
    latestConversationState,
    messages,
    nextQuestion,
    progressSummary,
    project,
    projectId,
  ]);

  const handleGenerateBlueprint = useCallback(async () => {
    if (!project || !canGenerate || generatingRef.current) return;
    const requestProjectId = projectId;
    let input: ReturnType<typeof buildBlueprintGenerateInput>;
    try {
      input = buildBlueprintGenerateInput();
    } catch (error) {
      addToast(extractErrorMessage(error, t("inspiration.invalidConversation")), "error");
      return;
    }
    if (!input) return;
    generatingRef.current = true;
    setGenerating(true);
    setBlueprintConsistencyWarning(null);
    setOperation("generation");
    setBlueprintCancelling(false);
    setBlueprintProgress({ ...createBlueprintProgressState(t("inspiration.progressPreparing")), status: "running", message: t("inspiration.startBlueprint") });
    setBlueprintProgressOpen(true);
    try {
      const handle = await generateBlueprint(input);
      if (!isCurrentProject(requestProjectId)) return;
      handledTerminalRef.current = null;
      setGenerationHandle(handle);
    } catch (error) {
      generatingRef.current = false;
      setGenerating(false);
      setOperation(null);
      addToast(extractErrorMessage(error, t("inspiration.startBlueprintFailed")), "error");
    }
  }, [addToast, buildBlueprintGenerateInput, canGenerate, isCurrentProject, project, projectId, t]);

  const handlePreviewGenerateBlueprint = useCallback(async () => {
    if (!project || !canGenerate) return;
    const requestSeq = ++requestPreviewSeqRef.current;
    setRequestPreviewOpen(true);
    setRequestPreviewLoading(true);
    setRequestPreviewData(null);
    try {
      const input = buildBlueprintGenerateInput();
      if (!input) return;
      const preview = await previewGenerateBlueprintRequest(input);
      if (requestPreviewSeqRef.current === requestSeq) setRequestPreviewData(preview);
    } catch (error) {
      if (requestPreviewSeqRef.current === requestSeq) {
        addToast(extractErrorMessage(error, t("inspiration.previewBlueprintFailed")), "error");
        setRequestPreviewOpen(false);
      }
    } finally {
      if (requestPreviewSeqRef.current === requestSeq) setRequestPreviewLoading(false);
    }
  }, [addToast, buildBlueprintGenerateInput, canGenerate, project, t]);

  const handleCancelBlueprint = useCallback(async () => {
    if (!generationHandle || operation !== "generation" || blueprintCancelling) return;
    setBlueprintCancelling(true);
    setBlueprintProgress((previous) => ({ ...previous, message: t("inspiration.stopBlueprintRequest") }));
    try {
      await generationClient.cancel(generationHandle);
      addToast(t("inspiration.stopBlueprintRequested"), "info");
    } catch (error) {
      setBlueprintCancelling(false);
      addToast(extractErrorMessage(error, t("inspiration.stopBlueprintFailed")), "error");
    }
  }, [addToast, blueprintCancelling, generationHandle, operation, t]);

  const handleRefineBlueprint = useCallback(async (focus: string) => {
    if (!project || refining) return;
    const requestFocus = focus.trim();
    if (!requestFocus) {
      addToast(t("inspiration.missingRefineFocus"), "error");
      return;
    }
    const requestSeq = ++refineSeqRef.current;
    setRefining(true);
    setBlueprintConsistencyWarning(null);
    setOperation("refinement");
    try {
      const handle = await refineBlueprint({ project_id: projectId, focus: requestFocus, variables: null });
      if (refineSeqRef.current !== requestSeq || !isCurrentProject(projectId)) return;
      handledTerminalRef.current = null;
      setGenerationHandle(handle);
    } catch (error) {
      setRefining(false);
      setOperation(null);
      addToast(extractErrorMessage(error, t("inspiration.startRefineFailed")), "error");
    }
  }, [addToast, isCurrentProject, project, projectId, refining, t]);

  const handlePreviewRefineBlueprint = useCallback(async (focus: string) => {
    if (!project) return;
    const requestFocus = focus.trim();
    if (!requestFocus) {
      addToast(t("inspiration.missingRefineFocus"), "error");
      return;
    }
    const requestSeq = ++requestPreviewSeqRef.current;
    setRequestPreviewOpen(true);
    setRequestPreviewLoading(true);
    setRequestPreviewData(null);
    try {
      const preview = await previewRefineBlueprintRequest({ project_id: projectId, focus: requestFocus, variables: null });
      if (requestPreviewSeqRef.current === requestSeq) setRequestPreviewData(preview);
    } catch (error) {
      if (requestPreviewSeqRef.current === requestSeq) {
        addToast(extractErrorMessage(error, t("inspiration.previewRefineFailed")), "error");
        setRequestPreviewOpen(false);
      }
    } finally {
      if (requestPreviewSeqRef.current === requestSeq) setRequestPreviewLoading(false);
    }
  }, [addToast, project, projectId, t]);

  const handleConfirmBlueprint = useCallback(() => {
    if (!project) return;
    const requestProjectId = projectId;
    const targetRoute = getProjectPrimaryRoute(project);
    setStage("display");
    if (confirmTimeoutRef.current != null) window.clearTimeout(confirmTimeoutRef.current);
    confirmTimeoutRef.current = window.setTimeout(() => {
      confirmTimeoutRef.current = null;
      if (isCurrentProject(requestProjectId)) navigate(targetRoute, { replace: true });
    }, 1200);
  }, [isCurrentProject, navigate, project, projectId]);

  return {
    stage,
    blueprint,
    blueprintConsistencyWarning,
    generating,
    refining,
    blueprintProgressOpen,
    setBlueprintProgressOpen,
    blueprintProgress,
    blueprintCancelling,
    blueprintStreamText: streamText,
    requestPreviewOpen,
    requestPreviewLoading,
    requestPreviewData,
    setRequestPreviewOpen,
    canGenerate,
    canCancelBlueprintGeneration,
    handleBackToConversation,
    handleGenerateBlueprint,
    handlePreviewGenerateBlueprint,
    handleCancelBlueprint,
    handleRefineBlueprint,
    handlePreviewRefineBlueprint,
    handleConfirmBlueprint,
  };
};
