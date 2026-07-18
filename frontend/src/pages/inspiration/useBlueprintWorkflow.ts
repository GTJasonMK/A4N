import { useCallback, useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { extractErrorMessage } from "../../api";
import {
  listenAppEvent,
  type UnlistenFn,
} from "../../api/runtime";
import {
  type Blueprint,
  generateBlueprint,
  previewGenerateBlueprintRequest,
  previewRefineBlueprintRequest,
  refineBlueprint,
} from "../../api/blueprint";
import type { ModelRequestPreview } from "../../api/requestPreview";
import type { NovelProject } from "../../api/project";
import { cancelWorkflow } from "../../api/streaming";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import type { ToastType } from "../../components/feedback/Toast";
import { isCancelledOperation } from "../../utils/cancellation";
import { logger } from "../../utils/logger";
import {
  buildFallbackTranscript,
  defaultChapterCountForProject,
  defaultGenreForProject,
} from "./blueprintDefaults";
import {
  buildInspirationTranscript,
  clearBlueprintPending,
  createBlueprintProgressState,
  eventMessage,
  extractChapterCountFromText,
  findChapterCountInValue,
  firstString,
  inferChapterCountFromMessages,
  markBlueprintPending,
  parseBlueprintProgressEvent,
  type BlueprintProgressState,
  type Stage,
  type WorkflowEvent,
} from "./inspirationChatUtils";
import type { Message } from "./shared";

type AddToast = (message: unknown, type: ToastType) => void;

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

  const blueprintTaskIdRef = useRef<string | null>(null);
  const blueprintTaskProjectIdRef = useRef<number | null>(null);
  const blueprintCancelRequestedRef = useRef(false);
  const activeProjectIdRef = useRef(projectId);
  const mountedRef = useRef(true);
  const generatingRef = useRef(false);
  const requestPreviewSeqRef = useRef(0);
  const refineSeqRef = useRef(0);
  const confirmTimeoutRef = useRef<number | null>(null);

  activeProjectIdRef.current = projectId;

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      generatingRef.current = false;
      if (confirmTimeoutRef.current != null) {
        window.clearTimeout(confirmTimeoutRef.current);
      }
    };
  }, []);

  const isCurrentProject = useCallback((requestProjectId: number) => {
    return mountedRef.current && activeProjectIdRef.current === requestProjectId;
  }, []);

  useEffect(() => {
    if (confirmTimeoutRef.current != null) {
      window.clearTimeout(confirmTimeoutRef.current);
      confirmTimeoutRef.current = null;
    }
    setStage("conversation");
    setBlueprint(null);
    setGenerating(false);
    generatingRef.current = false;
    setRefining(false);
    setBlueprintProgressOpen(false);
    setBlueprintProgress(createBlueprintProgressState());
    setBlueprintCancelling(false);
    setRequestPreviewOpen(false);
    setRequestPreviewLoading(false);
    setRequestPreviewData(null);
    blueprintTaskIdRef.current = null;
    blueprintTaskProjectIdRef.current = null;
    blueprintCancelRequestedRef.current = false;
    requestPreviewSeqRef.current += 1;
    refineSeqRef.current += 1;
  }, [projectId]);

  const canTryGenerate = !sending && !generating && stage === "conversation";
  const canGenerate = (conversationComplete || userTurns >= 2) && !sending && !generating;
  const canCancelBlueprintGeneration =
    generating && blueprintProgress.status === "running" && blueprintProgress.current < 8;

  const showBlueprintConfirmation = useCallback((value: Blueprint) => {
    setBlueprint(value);
    setStage("confirm");
  }, []);

  const handleBackToConversation = useCallback(() => {
    setStage("conversation");
  }, []);

  const buildBlueprintGenerateInput = useCallback(() => {
    if (!project) return null;
    const transcript =
      userTurns > 0 ? buildInspirationTranscript(messages) : buildFallbackTranscript(project);
    const requestedChapterCount =
      findChapterCountInValue(latestConversationState) ??
      (progressSummary ? extractChapterCountFromText(progressSummary) : null) ??
      inferChapterCountFromMessages(messages);
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
        message:
          "当前信息可能还不完整。\n\n你可以继续对话，把世界观、人物动机与核心冲突说清楚；也可以先生成一版草稿蓝图，后面再回到项目详情里补充和优化。\n\n是否继续试生成？",
        confirmText: "继续试生成",
        dialogType: "warning",
      });
      if (!ok || !isCurrentProject(requestProjectId)) {
        generatingRef.current = false;
        return;
      }
    }

    setGenerating(true);
    const blueprintTaskId = `blueprint-${requestProjectId}-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
    blueprintTaskIdRef.current = blueprintTaskId;
    blueprintTaskProjectIdRef.current = requestProjectId;
    blueprintCancelRequestedRef.current = false;
    setBlueprintCancelling(false);
    let unlisten: UnlistenFn | null = null;
    setBlueprintProgress({
      ...createBlueprintProgressState(),
      status: "running",
      message: "启动蓝图生成…",
    });
    setBlueprintProgressOpen(true);
    markBlueprintPending(requestProjectId);
    try {
      unlisten = await listenAppEvent<WorkflowEvent>(`workflow:${blueprintTaskId}`, (event) => {
        if (!isCurrentProject(requestProjectId)) return;
        const payload = event.payload;
        if (payload.type === "Progress") {
          let progress: ReturnType<typeof parseBlueprintProgressEvent>;
          try {
            progress = parseBlueprintProgressEvent(payload.data);
          } catch (err) {
            const message = extractErrorMessage(err, "蓝图进度事件无效");
            setBlueprintProgress((prev) => ({
              ...prev,
              status: "error",
              message: "蓝图进度事件无效",
              error: message,
            }));
            addToast(message, "error");
            return;
          }
          setBlueprintProgress((prev) => ({
            ...prev,
            status: "running",
            current: progress.current,
            total: progress.total,
            message: progress.message,
            error: null,
          }));
        } else if (payload.type === "Complete") {
          const message = eventMessage(payload.data, "首版蓝图已生成");
          setBlueprintProgress((prev) => ({
            ...prev,
            status: "success",
            current: prev.total,
            message,
          }));
        } else if (payload.type === "Error") {
          const message = eventMessage(payload.data, "蓝图错误事件缺少错误信息");
          const code =
            payload.data && typeof payload.data === "object"
              ? firstString((payload.data as { code?: unknown }).code)
              : undefined;
          if (code === "cancelled") {
            clearBlueprintPending(requestProjectId);
            setBlueprintProgress((prev) => ({
              ...prev,
              status: "cancelled",
              message: "蓝图生成已停止",
              error: null,
            }));
            return;
          }
          setBlueprintProgress((prev) => ({
            ...prev,
            status: "error",
            message: "蓝图生成失败",
            error: message,
          }));
        }
      });

      const generatePlan = buildBlueprintGenerateInput();
      if (!generatePlan) return;
      if (!isCurrentProject(requestProjectId)) throw new Error("蓝图生成已停止");
      const { input: generateInput, defaultedFields } = generatePlan;
      if (defaultedFields.length > 0) {
        addToast(`已默认补足：${defaultedFields.join("；")}`, "info");
      }

      const generated = await generateBlueprint({
        ...generateInput,
        task_id: blueprintTaskId,
      });
      if (!isCurrentProject(requestProjectId)) throw new Error("蓝图生成已停止");
      showBlueprintConfirmation(generated);
      clearBlueprintPending(requestProjectId);
      setBlueprintProgress((prev) => ({
        ...prev,
        status: "success",
        current: prev.total,
        message: "首版蓝图已生成",
      }));
      addToast("首版蓝图已生成，请确认", "success");
    } catch (err) {
      clearBlueprintPending(requestProjectId);
      if (!isCurrentProject(requestProjectId)) return;
      const message = extractErrorMessage(err, "蓝图生成失败");
      const cancelled = blueprintCancelRequestedRef.current || isCancelledOperation(message);
      if (cancelled) {
        setBlueprintProgress((prev) => ({
          ...prev,
          status: "cancelled",
          message: "蓝图生成已停止",
          error: null,
        }));
        addToast("蓝图生成已停止", "info");
        return;
      }
      logger.scoped("InspirationChat").error("blueprint generate failed", { err });
      setBlueprintProgress((prev) => ({
        ...prev,
        status: "error",
        message: "蓝图生成失败",
        error: message,
      }));
      addToast(message, "error");
    } finally {
      unlisten?.();
      if (isCurrentProject(requestProjectId)) {
        blueprintTaskIdRef.current = null;
        blueprintTaskProjectIdRef.current = null;
        blueprintCancelRequestedRef.current = false;
        generatingRef.current = false;
        setBlueprintCancelling(false);
        setGenerating(false);
      } else if (!mountedRef.current || activeProjectIdRef.current !== requestProjectId) {
        generatingRef.current = false;
      }
    }
  }, [
    addToast,
    buildBlueprintGenerateInput,
    canGenerate,
    isCurrentProject,
    project,
    projectId,
    sending,
    showBlueprintConfirmation,
  ]);

  const handlePreviewGenerateBlueprint = useCallback(async () => {
    if (!project || sending || generating) return;
    const requestProjectId = projectId;
    if (!canGenerate) {
      const ok = await confirmDialog({
        title: "预览草稿蓝图提示词",
        message: "当前信息可能还不完整。是否按当前上下文构造首版蓝图生成请求预览？",
        confirmText: "预览",
        dialogType: "warning",
      });
      if (!ok) return;
      if (!isCurrentProject(requestProjectId)) return;
    }
    const generatePlan = buildBlueprintGenerateInput();
    if (!generatePlan) return;
    const requestSeq = requestPreviewSeqRef.current + 1;
    requestPreviewSeqRef.current = requestSeq;
    setRequestPreviewOpen(true);
    setRequestPreviewLoading(true);
    setRequestPreviewData(null);
    try {
      const preview = await previewGenerateBlueprintRequest({
        ...generatePlan.input,
        task_id: null,
      });
      if (
        !isCurrentProject(requestProjectId) ||
        requestPreviewSeqRef.current !== requestSeq
      ) {
        return;
      }
      setRequestPreviewData(preview);
    } catch (err) {
      if (
        !isCurrentProject(requestProjectId) ||
        requestPreviewSeqRef.current !== requestSeq
      ) {
        return;
      }
      addToast(extractErrorMessage(err, "预览蓝图提示词失败"), "error");
      setRequestPreviewOpen(false);
    } finally {
      if (
        isCurrentProject(requestProjectId) &&
        requestPreviewSeqRef.current === requestSeq
      ) {
        setRequestPreviewLoading(false);
      }
    }
  }, [
    addToast,
    buildBlueprintGenerateInput,
    canGenerate,
    generating,
    isCurrentProject,
    project,
    projectId,
    sending,
  ]);

  const handleCancelBlueprint = useCallback(async () => {
    const currentTaskId = blueprintTaskIdRef.current;
    const requestProjectId = blueprintTaskProjectIdRef.current;
    if (
      !currentTaskId ||
      requestProjectId == null ||
      !isCurrentProject(requestProjectId) ||
      !generating ||
      blueprintCancelling
    ) {
      return;
    }

    blueprintCancelRequestedRef.current = true;
    setBlueprintCancelling(true);
    setBlueprintProgress((prev) => ({
      ...prev,
      status: "running",
      message: "正在请求停止蓝图生成…",
      error: null,
    }));

    try {
      await cancelWorkflow(currentTaskId);
      if (!isCurrentProject(requestProjectId)) return;
      addToast("已请求停止蓝图生成", "info");
    } catch (err) {
      if (!isCurrentProject(requestProjectId)) return;
      blueprintCancelRequestedRef.current = false;
      setBlueprintCancelling(false);
      addToast(extractErrorMessage(err, "停止蓝图生成失败"), "error");
    }
  }, [addToast, blueprintCancelling, generating, isCurrentProject]);

  const handleRefineBlueprint = useCallback(
    async (focus: string) => {
      if (!project) return;
      const requestFocus = focus.trim();
      if (!requestFocus) {
        addToast("请先说明要调整的方向", "error");
        return;
      }
      const requestProjectId = projectId;
      const requestSeq = refineSeqRef.current + 1;
      refineSeqRef.current = requestSeq;
      setRefining(true);
      try {
        const refined = await refineBlueprint({
          project_id: requestProjectId,
          focus: requestFocus,
          variables: null,
        });
        if (!isCurrentProject(requestProjectId) || refineSeqRef.current !== requestSeq) return;
        setBlueprint(refined);
        addToast("蓝图已优化", "success");
      } catch (err) {
        if (!isCurrentProject(requestProjectId) || refineSeqRef.current !== requestSeq) return;
        addToast(extractErrorMessage(err, "蓝图优化失败"), "error");
      } finally {
        if (isCurrentProject(requestProjectId) && refineSeqRef.current === requestSeq) {
          setRefining(false);
        }
      }
    },
    [addToast, isCurrentProject, project, projectId],
  );

  const handlePreviewRefineBlueprint = useCallback(
    async (focus: string) => {
      if (!project) return;
      const requestFocus = focus.trim();
      if (!requestFocus) {
        addToast("请先说明要调整的方向", "error");
        return;
      }
      const requestProjectId = projectId;
      const requestSeq = requestPreviewSeqRef.current + 1;
      requestPreviewSeqRef.current = requestSeq;
      setRequestPreviewOpen(true);
      setRequestPreviewLoading(true);
      setRequestPreviewData(null);
      try {
        const preview = await previewRefineBlueprintRequest({
          project_id: requestProjectId,
          focus: requestFocus,
          variables: null,
        });
        if (
          !isCurrentProject(requestProjectId) ||
          requestPreviewSeqRef.current !== requestSeq
        ) {
          return;
        }
        setRequestPreviewData(preview);
      } catch (err) {
        if (
          !isCurrentProject(requestProjectId) ||
          requestPreviewSeqRef.current !== requestSeq
        ) {
          return;
        }
        addToast(extractErrorMessage(err, "预览蓝图优化提示词失败"), "error");
        setRequestPreviewOpen(false);
      } finally {
        if (
          isCurrentProject(requestProjectId) &&
          requestPreviewSeqRef.current === requestSeq
        ) {
          setRequestPreviewLoading(false);
        }
      }
    },
    [addToast, isCurrentProject, project, projectId],
  );

  const handleConfirmBlueprint = useCallback(() => {
    const requestProjectId = projectId;
    setStage("display");
    if (confirmTimeoutRef.current != null) {
      window.clearTimeout(confirmTimeoutRef.current);
    }
    confirmTimeoutRef.current = window.setTimeout(() => {
      confirmTimeoutRef.current = null;
      if (isCurrentProject(requestProjectId)) {
        navigate(`/novel/${requestProjectId}`, { replace: true });
      }
    }, 1200);
  }, [isCurrentProject, navigate, projectId]);

  return {
    stage,
    blueprint,
    generating,
    refining,
    blueprintProgressOpen,
    setBlueprintProgressOpen,
    blueprintProgress,
    blueprintCancelling,
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
