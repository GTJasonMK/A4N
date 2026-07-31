import { useCallback, useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { extractErrorMessage } from "../../api";
import {
  type Blueprint,
  generateBlueprint,
  getBlueprint,
  previewGenerateBlueprintRequest,
  previewRefineBlueprintRequest,
  refineBlueprint,
} from "../../api/blueprint";
import { generationClient } from "../../api/generationTasks";
import type { GenerationTaskHandle } from "../../api/generated/generationContracts";
import type { ModelRequestPreview } from "../../api/requestPreview";
import type { NovelProject } from "../../api/project";
import { useGenerationTask } from "../../hooks/useGenerationTask";
import { useGenerationTasksStore, generationTaskKey } from "../../store/generationTasks";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import type { ToastType } from "../../components/feedback/Toast";
import { logger } from "../../utils/logger";
import {
  generationTaskRecoveryIssue,
  resolveFormalResultRecovery,
} from "../../utils/generationEventReducer";
import {
  buildFallbackTranscript,
  defaultChapterCountForProject,
  defaultGenreForProject,
} from "./blueprintDefaults";
import {
  buildInspirationTranscript,
  createBlueprintProgressState,
  extractChapterCountFromText,
  findChapterCountInValue,
  firstString,
  inferChapterCountFromMessages,
  type BlueprintProgressState,
  type Stage,
} from "./inspirationChatUtils";
import type { Message } from "./shared";

type AddToast = (message: unknown, type: ToastType) => void;
type BlueprintOperation = "generation" | "refinement";

interface UseBlueprintWorkflowOptions {
  projectId: number;
  project: NovelProject | null;
  messages: Message[];
  userTurns: number;
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
  userTurns,
  conversationComplete,
  sending,
  latestConversationState,
  progressSummary,
  nextQuestion,
  addToast,
}: UseBlueprintWorkflowOptions) => {
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
    createBlueprintProgressState(),
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
    setBlueprintProgress(createBlueprintProgressState());
    setBlueprintCancelling(false);
    setRequestPreviewOpen(false);
    setRequestPreviewLoading(false);
    setRequestPreviewData(null);
    handledTerminalRef.current = null;
    generationHandleRef.current = null;
    generatingRef.current = false;
    requestPreviewSeqRef.current += 1;
    refineSeqRef.current += 1;
  }, [projectId]);

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
    }).catch(() => undefined);
    return () => {
      active = false;
    };
  }, [projectId]);

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
        const resultId = resultRef ? Number(resultRef.id) : NaN;
        const canResolveResult = resultRef?.kind === "novel_blueprint"
          && Number.isSafeInteger(resultId);
        const persisted = canResolveResult ? await getBlueprint(requestProjectId) : null;
        const consistent = Boolean(
          persisted
          && resultRef?.kind === "novel_blueprint"
          && Number.isSafeInteger(resultId)
          && persisted.id === resultId
          && persisted.project_id === requestProjectId
          && persisted.current_version === resultRef.revision,
        );
        const recovery = resolveFormalResultRecovery({
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
        setBlueprintConsistencyWarning(null);
        setBlueprint(persisted);
        if (currentOperation === "generation") {
          setStage("confirm");
          setBlueprintProgress((previous) => ({
            ...previous,
            status: "success",
            current: previous.total,
            message: "首版蓝图已生成",
            error: null,
          }));
          addToast("首版蓝图已生成，请确认", "success");
        } else {
          addToast("蓝图已优化", "success");
        }
        return;
      }
      if (generationTask.taskStatus === "cancelled" || generationTask.taskStatus === "interrupted") {
        setBlueprintProgress((previous) => ({
          ...previous,
          status: "cancelled",
          message: "蓝图生成已停止",
          error: null,
        }));
        return;
      }
      throw new Error(generationTask.error?.message ?? "蓝图生成失败");
    };
    void finish().catch((error) => {
      if (!isCurrentProject(requestProjectId)) return;
      const message = extractErrorMessage(error, "蓝图生成失败");
      logger.scoped("InspirationChat").error("blueprint generation failed", { error });
      setBlueprintProgress((previous) => ({
        ...previous,
        status: "error",
        message: currentOperation === "refinement" ? "蓝图优化失败" : "蓝图生成失败",
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
  ]);

  const canTryGenerate = !sending && !generating && stage === "conversation";
  const canGenerate = (conversationComplete || userTurns >= 2) && !sending && !generating;
  const canCancelBlueprintGeneration = Boolean(
    generationHandle && operation === "generation" && generating && !generationTask.terminalReceived,
  );

  const showBlueprintConfirmation = useCallback((value: Blueprint) => {
    setBlueprintConsistencyWarning(null);
    setBlueprint(value);
    setStage("confirm");
  }, []);

  const handleBackToConversation = useCallback(() => {
    setBlueprintConsistencyWarning(null);
    setStage("conversation");
  }, []);

  const buildBlueprintGenerateInput = useCallback(() => {
    if (!project) return null;
    const transcript = userTurns > 0 ? buildInspirationTranscript(messages) : buildFallbackTranscript(project);
    const requestedChapterCount =
      findChapterCountInValue(latestConversationState)
      ?? (progressSummary ? extractChapterCountFromText(progressSummary) : null)
      ?? inferChapterCountFromMessages(messages);
    const genreHint = firstString(
      latestConversationState?.genre,
      latestConversationState?.story_genre,
      latestConversationState?.type,
    );
    const resolvedChapterCount = requestedChapterCount ?? defaultChapterCountForProject(project);
    const resolvedGenre = genreHint ?? defaultGenreForProject(project);
    const defaultedFields = [
      requestedChapterCount ? null : `目标章节数按项目默认 ${resolvedChapterCount} 章`,
      genreHint ? null : `题材类型按项目类型「${resolvedGenre}」`,
    ].filter((item): item is string => Boolean(item));
    return {
      defaultedFields,
      input: {
        project_id: projectId,
        title: project.title,
        genre: resolvedGenre,
        target_length: resolvedChapterCount,
        variables: {
          inspiration_transcript: transcript,
          conversation_state: latestConversationState ?? null,
          progress_summary: progressSummary ?? null,
          next_question: nextQuestion ?? null,
          project: {
            id: project.id,
            title: project.title,
            status: project.status,
            project_type: project.project_type,
            description: project.description,
            target_chapters: project.target_chapters,
            defaulted_blueprint_fields: defaultedFields,
          },
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
    userTurns,
  ]);

  const handleGenerateBlueprint = useCallback(async () => {
    if (!project || sending || generatingRef.current) return;
    const requestProjectId = projectId;
    generatingRef.current = true;
    if (!canGenerate) {
      const ok = await confirmDialog({
        title: "试生成蓝图",
        message: "当前信息可能还不完整，是否继续试生成？",
        confirmText: "继续试生成",
        dialogType: "warning",
      });
      if (!ok || !isCurrentProject(requestProjectId)) {
        generatingRef.current = false;
        return;
      }
    }
    const plan = buildBlueprintGenerateInput();
    if (!plan) {
      generatingRef.current = false;
      return;
    }
    if (plan.defaultedFields.length) addToast(`已默认补足：${plan.defaultedFields.join("；")}`, "info");
    setGenerating(true);
    setBlueprintConsistencyWarning(null);
    setOperation("generation");
    setBlueprintCancelling(false);
    setBlueprintProgress({ ...createBlueprintProgressState(), status: "running", message: "启动蓝图生成…" });
    setBlueprintProgressOpen(true);
    try {
      const handle = await generateBlueprint(plan.input);
      if (!isCurrentProject(requestProjectId)) return;
      handledTerminalRef.current = null;
      setGenerationHandle(handle);
    } catch (error) {
      generatingRef.current = false;
      setGenerating(false);
      setOperation(null);
      addToast(extractErrorMessage(error, "蓝图生成启动失败"), "error");
    }
  }, [addToast, buildBlueprintGenerateInput, canGenerate, isCurrentProject, project, projectId, sending]);

  const handlePreviewGenerateBlueprint = useCallback(async () => {
    if (!project || sending || generating) return;
    const plan = buildBlueprintGenerateInput();
    if (!plan) return;
    const requestSeq = ++requestPreviewSeqRef.current;
    setRequestPreviewOpen(true);
    setRequestPreviewLoading(true);
    setRequestPreviewData(null);
    try {
      const preview = await previewGenerateBlueprintRequest(plan.input);
      if (requestPreviewSeqRef.current === requestSeq) setRequestPreviewData(preview);
    } catch (error) {
      if (requestPreviewSeqRef.current === requestSeq) {
        addToast(extractErrorMessage(error, "预览蓝图提示词失败"), "error");
        setRequestPreviewOpen(false);
      }
    } finally {
      if (requestPreviewSeqRef.current === requestSeq) setRequestPreviewLoading(false);
    }
  }, [addToast, buildBlueprintGenerateInput, generating, project, sending]);

  const handleCancelBlueprint = useCallback(async () => {
    if (!generationHandle || operation !== "generation" || blueprintCancelling) return;
    setBlueprintCancelling(true);
    setBlueprintProgress((previous) => ({ ...previous, message: "正在请求停止蓝图生成…" }));
    try {
      await generationClient.cancel(generationHandle);
      addToast("已请求停止蓝图生成", "info");
    } catch (error) {
      setBlueprintCancelling(false);
      addToast(extractErrorMessage(error, "停止蓝图生成失败"), "error");
    }
  }, [addToast, blueprintCancelling, generationHandle, operation]);

  const handleRefineBlueprint = useCallback(async (focus: string) => {
    if (!project || refining) return;
    const requestFocus = focus.trim();
    if (!requestFocus) {
      addToast("请先说明要调整的方向", "error");
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
      addToast(extractErrorMessage(error, "蓝图优化启动失败"), "error");
    }
  }, [addToast, isCurrentProject, project, projectId, refining]);

  const handlePreviewRefineBlueprint = useCallback(async (focus: string) => {
    if (!project) return;
    const requestFocus = focus.trim();
    if (!requestFocus) {
      addToast("请先说明要调整的方向", "error");
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
        addToast(extractErrorMessage(error, "预览蓝图优化提示词失败"), "error");
        setRequestPreviewOpen(false);
      }
    } finally {
      if (requestPreviewSeqRef.current === requestSeq) setRequestPreviewLoading(false);
    }
  }, [addToast, project, projectId]);

  const handleConfirmBlueprint = useCallback(() => {
    const requestProjectId = projectId;
    setStage("display");
    if (confirmTimeoutRef.current != null) window.clearTimeout(confirmTimeoutRef.current);
    confirmTimeoutRef.current = window.setTimeout(() => {
      confirmTimeoutRef.current = null;
      if (isCurrentProject(requestProjectId)) navigate(`/novel/${requestProjectId}`, { replace: true });
    }, 1200);
  }, [isCurrentProject, navigate, projectId]);

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
    canTryGenerate,
    canGenerate,
    canCancelBlueprintGeneration,
    showBlueprintConfirmation,
    handleBackToConversation,
    handleGenerateBlueprint,
    handlePreviewGenerateBlueprint,
    handleCancelBlueprint,
    handleRefineBlueprint,
    handlePreviewRefineBlueprint,
    handleConfirmBlueprint,
  };
};
