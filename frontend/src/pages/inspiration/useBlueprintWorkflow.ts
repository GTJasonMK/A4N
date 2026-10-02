import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
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
const DEFAULT_BLUEPRINT_CHAPTERS = 100;
const FREE_BLUEPRINT_GENRE = "自由创作";
const FREE_BLUEPRINT_INSTRUCTION =
  "用户尚未提供具体灵感，请自由随机生成一个完整、自洽、可写作的小说蓝图，并自行决定题材、人物、世界观与核心冲突。";

const validChapterCount = (value: unknown): value is number =>
  typeof value === "number"
  && Number.isInteger(value)
  && value >= 5
  && value <= 10000;

interface UseBlueprintWorkflowOptions {
  projectId: number;
  project: NovelProject | null;
  messages: Message[];
  sending: boolean;
  latestConversationState: Record<string, unknown> | undefined;
  progressSummary: string | undefined;
  nextQuestion: string | undefined;
  addToast: AddToast;
}

export const buildBlueprintGenerationInput = ({
  project,
  messages,
  latestConversationState,
  progressSummary,
  nextQuestion,
}: Pick<
  UseBlueprintWorkflowOptions,
  "messages" | "latestConversationState" | "progressSummary" | "nextQuestion"
> & {
  project: Pick<
    NovelProject,
    "id" | "title" | "status" | "project_type" | "description" | "target_chapters"
  >;
}) => {
  const conversationState = latestConversationState ?? {};
  const targetLength = validChapterCount(conversationState.chapter_count)
    ? conversationState.chapter_count
    : validChapterCount(project.target_chapters)
      ? project.target_chapters
      : DEFAULT_BLUEPRINT_CHAPTERS;
  const genre = typeof conversationState.genre === "string"
    ? conversationState.genre.trim()
    : "";
  const inspirationTranscript = messages.some((message) => message.role === "user")
    ? buildInspirationTranscript(messages)
    : "";

  return {
    project_id: project.id,
    title: project.title,
    genre: genre || FREE_BLUEPRINT_GENRE,
    target_length: targetLength,
    variables: {
      inspiration_transcript: inspirationTranscript,
      ...(inspirationTranscript ? {} : { generation_instruction: FREE_BLUEPRINT_INSTRUCTION }),
      conversation_state: conversationState,
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
};

export const useBlueprintWorkflow = ({
  projectId,
  project,
  messages,
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
  const [blueprintStreamText, setBlueprintStreamText] = useState("");
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
  const streamText = [
    generationTask.text("reasoning"),
    generationTask.text("raw_json"),
  ].filter(Boolean).join("\n");

  useEffect(() => {
    if (generationHandle && streamText) setBlueprintStreamText(streamText);
  }, [generationHandle, streamText]);

  const isCurrentProject = useCallback((requestProjectId: number) => {
    return mountedRef.current && activeProjectIdRef.current === requestProjectId;
  }, []);

  useLayoutEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      generatingRef.current = false;
      requestPreviewSeqRef.current += 1;
      if (confirmTimeoutRef.current != null) window.clearTimeout(confirmTimeoutRef.current);
    };
  }, []);

  useLayoutEffect(() => {
    setStage("conversation");
    setBlueprint(null);
    setBlueprintConsistencyWarning(null);
    setGenerating(false);
    setRefining(false);
    setGenerationHandle(null);
    setOperation(null);
    setBlueprintProgressOpen(false);
    setBlueprintProgress(createBlueprintProgressState(t("inspiration.progressPreparing")));
    setBlueprintStreamText("");
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
  useLayoutEffect(() => {
    if (!project || project.id !== projectId || !Number.isFinite(projectId) || projectId <= 0) return;
    let active = true;
    void generationClient.listActive(projectId).then((items) => {
      if (!active || !isCurrentProject(projectId) || generationHandleRef.current) return;
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
      setBlueprintProgress({
        ...createBlueprintProgressState(
          nextOperation === "refinement"
            ? t("inspiration.refining")
            : t("inspiration.startBlueprint"),
        ),
        status: "running",
      });
      setBlueprintProgressOpen(true);
    }).catch((error) => {
      if (active && isCurrentProject(projectId)) {
        addToast(extractErrorMessage(error, t("inspiration.restoreBlueprintFailed")), "error");
      }
    });
    return () => {
      active = false;
    };
  }, [addToast, project, projectId, t]);

  useEffect(() => {
    const current = generationHandleRef.current;
    if (
      !generationHandle
      || !generationTask.progress
      || !current
      || generationTaskKey(current.task_id, current.attempt) !== taskKey
      || !isCurrentProject(projectId)
    ) return;
    const progress = generationTask.progress;
    const incomingCurrent = Number(progress.current ?? 0);
    const incomingTotal = Number(progress.total ?? 0);
    setBlueprintProgress((previous) => ({
      ...previous,
      status: generationTask.terminalReceived ? previous.status : "running",
      current: Math.max(previous.current, incomingCurrent),
      total: Math.max(previous.total, incomingTotal),
      message: progress.message,
      error: null,
    }));
  }, [generationHandle, generationTask.progress, generationTask.terminalReceived, isCurrentProject, projectId, taskKey]);

  useEffect(() => {
    const recoveryIssue = generationTaskRecoveryIssue(generationTask);
    if (
      !generationHandle
      || !operation
      || !taskKey
      || (!generationTask.terminalReceived
        && !recoveryIssue)
    ) return;
    if (!isCurrentProject(projectId) || handledTerminalRef.current === taskKey) return;
    handledTerminalRef.current = taskKey;
    const requestProjectId = projectId;
    const currentOperation = operation;
    let formalResultReadFailed = false;
    const finish = async () => {
      if (generationTask.taskStatus === "completed" || recoveryIssue) {
        const resultRef = generationTask.resultRef;
        const canResolveResult = resultRef?.kind === "novel_blueprint"
          && Number.isSafeInteger(Number(resultRef.id));
        let persisted: Blueprint | null = null;
        if (canResolveResult) {
          try {
            persisted = await getBlueprint(requestProjectId);
          } catch (error) {
            formalResultReadFailed = true;
            throw error;
          }
        }
        const consistent = matchesBlueprintTaskResult(persisted, requestProjectId, resultRef);
        if (!isCurrentProject(requestProjectId)) return;
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
        if (!persisted) return;
        const syncWarnings = await syncBlueprintMaterials(requestProjectId);
        if (!isCurrentProject(requestProjectId)) return;
        setBlueprintConsistencyWarning(null);
        setBlueprint(persisted);
        setBlueprintProgress((previous) => ({
          ...previous,
          status: "success",
          current: Math.max(previous.current, previous.total),
          message: currentOperation === "refinement"
            ? t("inspiration.blueprintOptimized")
            : t("inspiration.blueprintGenerated"),
          error: null,
        }));
        if (currentOperation === "generation") setStage("confirm");
        if (syncWarnings.length > 0) {
          addToast(
            `${t(currentOperation === "generation" ? "inspiration.blueprintGeneratedConfirm" : "inspiration.blueprintOptimized")}${t("common.summarySeparator")}${syncWarnings.join(t("common.summarySeparator"))}`,
            "info",
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
      if (formalResultReadFailed) {
        const recovery = resolveFormalResultRecovery({
          t,
          recoveryIssue,
          consistent: false,
          hasPersisted: false,
          hasResultRef: Boolean(generationTask.resultRef),
          domain: "blueprint",
        });
        if (recovery.shouldMarkConsistencyError) {
          useGenerationTasksStore.getState().dispatch(taskKey, {
            type: "client_state",
            state: "consistency_error",
          });
        }
        logger.scoped("InspirationChat").error("formal blueprint readback failed", { error });
        setBlueprintProgress((previous) => ({
          ...previous,
          status: "error",
          message: recovery.message,
          error: recovery.message,
        }));
        addToast(recovery.message, "error");
        return;
      }
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

  const canGenerate = stage === "conversation" && !sending && !generating && !refining;
  const canCancelBlueprintOperation = Boolean(
    generationHandle
    && (operation === "generation" || operation === "refinement")
    && (generating || refining)
    && !generationTask.terminalReceived,
  );

  const handleBackToConversation = useCallback(() => {
    setBlueprintConsistencyWarning(null);
    setStage("conversation");
  }, []);

  const buildCurrentBlueprintGenerationInput = useCallback((currentProject: NovelProject) => {
    return buildBlueprintGenerationInput({
      project: currentProject,
      messages,
      latestConversationState,
      progressSummary,
      nextQuestion,
    });
  }, [
    latestConversationState,
    messages,
    nextQuestion,
    progressSummary,
  ]);

  const handleGenerateBlueprint = useCallback(async () => {
    if (!project || !canGenerate || generatingRef.current) return;
    const requestProjectId = projectId;
    let input: ReturnType<typeof buildCurrentBlueprintGenerationInput>;
    try {
      input = buildCurrentBlueprintGenerationInput(project);
    } catch (error) {
      addToast(extractErrorMessage(error, t("errors.requestFailed")), "error");
      return;
    }
    generatingRef.current = true;
    setGenerating(true);
    setBlueprintConsistencyWarning(null);
    setOperation("generation");
    setBlueprintStreamText("");
    setBlueprintCancelling(false);
    setBlueprintProgress({ ...createBlueprintProgressState(t("inspiration.progressPreparing")), status: "running", message: t("inspiration.startBlueprint") });
    setBlueprintProgressOpen(true);
    try {
      const handle = await generateBlueprint(input);
      if (!isCurrentProject(requestProjectId)) return;
      handledTerminalRef.current = null;
      setGenerationHandle(handle);
    } catch (error) {
      if (!isCurrentProject(requestProjectId)) return;
      generatingRef.current = false;
      setGenerating(false);
      const message = extractErrorMessage(error, t("inspiration.startBlueprintFailed"));
      setBlueprintProgress((previous) => ({
        ...previous,
        status: "error",
        message: t("inspiration.blueprintFailed"),
        error: message,
      }));
      addToast(message, "error");
    }
  }, [addToast, buildCurrentBlueprintGenerationInput, canGenerate, isCurrentProject, project, projectId, t]);

  const closeRequestPreview = useCallback(() => {
    requestPreviewSeqRef.current += 1;
    setRequestPreviewOpen(false);
    setRequestPreviewLoading(false);
    setRequestPreviewData(null);
  }, []);

  const handlePreviewGenerateBlueprint = useCallback(async () => {
    if (!project || !canGenerate) return;
    const requestProjectId = projectId;
    const requestSeq = ++requestPreviewSeqRef.current;
    setRequestPreviewOpen(true);
    setRequestPreviewLoading(true);
    setRequestPreviewData(null);
    try {
      const input = buildCurrentBlueprintGenerationInput(project);
      const preview = await previewGenerateBlueprintRequest(input);
      if (isCurrentProject(requestProjectId) && requestPreviewSeqRef.current === requestSeq) setRequestPreviewData(preview);
    } catch (error) {
      if (isCurrentProject(requestProjectId) && requestPreviewSeqRef.current === requestSeq) {
        addToast(extractErrorMessage(error, t("inspiration.previewBlueprintFailed")), "error");
        setRequestPreviewOpen(false);
      }
    } finally {
      if (isCurrentProject(requestProjectId) && requestPreviewSeqRef.current === requestSeq) setRequestPreviewLoading(false);
    }
  }, [addToast, buildCurrentBlueprintGenerationInput, canGenerate, isCurrentProject, project, projectId, t]);

  const handleCancelBlueprint = useCallback(async () => {
    if (
      !generationHandle
      || (operation !== "generation" && operation !== "refinement")
      || blueprintCancelling
    ) return;
    const requestProjectId = projectId;
    setBlueprintCancelling(true);
    setBlueprintProgress((previous) => ({ ...previous, message: t("inspiration.stopBlueprintRequest") }));
    try {
      await generationClient.cancel(generationHandle);
      if (isCurrentProject(requestProjectId)) {
        addToast(t("inspiration.stopBlueprintRequested"), "info");
      }
    } catch (error) {
      if (!isCurrentProject(requestProjectId)) return;
      setBlueprintCancelling(false);
      addToast(extractErrorMessage(error, t("inspiration.stopBlueprintFailed")), "error");
    }
  }, [addToast, blueprintCancelling, generationHandle, isCurrentProject, operation, projectId, t]);

  const handleRefineBlueprint = useCallback(async (focus: string) => {
    if (!project || refining) return;
    const requestFocus = focus.trim();
    if (!requestFocus) {
      addToast(t("inspiration.missingRefineFocus"), "error");
      return;
    }
    const requestProjectId = projectId;
    const requestSeq = ++refineSeqRef.current;
    setRefining(true);
    setBlueprintConsistencyWarning(null);
    setOperation("refinement");
    setBlueprintStreamText("");
    setBlueprintCancelling(false);
    setBlueprintProgress({
      ...createBlueprintProgressState(t("inspiration.refining")),
      status: "running",
      message: t("inspiration.refining"),
    });
    setBlueprintProgressOpen(true);
    try {
      const handle = await refineBlueprint({ project_id: requestProjectId, focus: requestFocus, variables: null });
      if (refineSeqRef.current !== requestSeq || !isCurrentProject(requestProjectId)) return;
      handledTerminalRef.current = null;
      setGenerationHandle(handle);
    } catch (error) {
      if (refineSeqRef.current !== requestSeq || !isCurrentProject(requestProjectId)) return;
      setRefining(false);
      const message = extractErrorMessage(error, t("inspiration.startRefineFailed"));
      setBlueprintProgress((previous) => ({
        ...previous,
        status: "error",
        message: t("inspiration.blueprintRefineFailed"),
        error: message,
      }));
      addToast(message, "error");
    }
  }, [addToast, isCurrentProject, project, projectId, refining, t]);

  const handlePreviewRefineBlueprint = useCallback(async (focus: string) => {
    if (!project) return;
    const requestProjectId = projectId;
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
      if (isCurrentProject(requestProjectId) && requestPreviewSeqRef.current === requestSeq) setRequestPreviewData(preview);
    } catch (error) {
      if (isCurrentProject(requestProjectId) && requestPreviewSeqRef.current === requestSeq) {
        addToast(extractErrorMessage(error, t("inspiration.previewRefineFailed")), "error");
        setRequestPreviewOpen(false);
      }
    } finally {
      if (isCurrentProject(requestProjectId) && requestPreviewSeqRef.current === requestSeq) setRequestPreviewLoading(false);
    }
  }, [addToast, isCurrentProject, project, projectId, t]);

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

  const closeBlueprintProgress = useCallback(() => {
    setBlueprintProgressOpen(false);
    setOperation(null);
  }, []);

  return {
    stage,
    blueprint,
    blueprintConsistencyWarning,
    generating,
    refining,
    blueprintProgressOpen,
    blueprintOperation: operation,
    closeBlueprintProgress,
    blueprintProgress,
    blueprintCancelling,
    blueprintStreamText,
    requestPreviewOpen,
    requestPreviewLoading,
    requestPreviewData,
    closeRequestPreview,
    canGenerate,
    canCancelBlueprintOperation,
    handleBackToConversation,
    handleGenerateBlueprint,
    handlePreviewGenerateBlueprint,
    handleCancelBlueprint,
    handleRefineBlueprint,
    handlePreviewRefineBlueprint,
    handleConfirmBlueprint,
  };
};
