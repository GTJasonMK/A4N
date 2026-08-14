import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent,
} from "react";
import { useNavigate, useParams } from "react-router-dom";
import { useToast } from "../../components/feedback/Toast";
import { extractErrorMessage } from "../../api";
import { generationClient } from "../../api/generationTasks";
import {
  getInspirationHistory,
  inspirationGenerationProject,
  previewInspirationRequest,
  startInspirationGeneration,
} from "../../api/inspiration";
import type { GenerationTaskHandle } from "../../api/generated/generationContracts";
import type { ModelRequestPreview } from "../../api/requestPreview";
import { getProject, type NovelProject } from "../../api/project";
import { useGenerationTask } from "../../hooks/useGenerationTask";
import {
  generationTaskKey,
  useGenerationTasksStore,
} from "../../store/generationTasks";
import { sha256Hex } from "../../utils/contentHash";
import {
  generationTaskRecoveryIssue,
  resolveFormalResultRecovery,
} from "../../utils/generationEventReducer";
import { logger } from "../../utils/logger";
import { isImeCompositionActive } from "../../utils/ime";
import { getProjectHomeEntryRoute } from "../../utils/projectRouting";
import {
  DEFAULT_INPUT_PLACEHOLDER,
  msgId,
  normalizeHistoryMessage,
  splitSummaryLines,
  welcomeMessage,
} from "./inspirationChatUtils";
import {
  getStreamingResponsePreview,
  parseInspirationResponse,
} from "./responseParser";
import type { Message } from "./shared";
import { useBlueprintWorkflow } from "./useBlueprintWorkflow";

const canEditInspirationProject = (project: NovelProject): boolean =>
  project.my_role === "owner" ||
  project.my_role === "manager" ||
  project.my_role === "editor";

export const useInspirationChatController = () => {
  const { id } = useParams<{ id: string }>();
  const projectId = Number(id);
  const navigate = useNavigate();
  const { addToast } = useToast();

  const [project, setProject] = useState<NovelProject | null>(null);
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState("");
  const [sending, setSending] = useState(false);
  const [generationHandle, setGenerationHandle] = useState<GenerationTaskHandle | null>(null);
  const [inputPlaceholder, setInputPlaceholder] = useState(DEFAULT_INPUT_PLACEHOLDER);
  const [inputActive, setInputActive] = useState(false);
  const [requestPreviewOpen, setRequestPreviewOpen] = useState(false);
  const [requestPreviewLoading, setRequestPreviewLoading] = useState(false);
  const [requestPreviewData, setRequestPreviewData] = useState<ModelRequestPreview | null>(null);

  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const composingRef = useRef(false);
  const lastCompositionEndRef = useRef(0);
  const activeProjectIdRef = useRef(projectId);
  const mountedRef = useRef(true);
  const sendSeqRef = useRef(0);
  const sendingRef = useRef(false);
  const generationHandleRef = useRef<GenerationTaskHandle | null>(null);
  const generationAssistantIdRef = useRef<string | null>(null);
  const handledTerminalRef = useRef<string | null>(null);
  const requestPreviewSeqRef = useRef(0);

  const generationTask = useGenerationTask({
    handle: generationHandle,
    autoSubscribe: generationHandle != null,
  });

  activeProjectIdRef.current = projectId;

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      sendingRef.current = false;
      generationHandleRef.current = null;
      generationAssistantIdRef.current = null;
    };
  }, []);

  const isCurrentProject = useCallback((requestProjectId: number) => {
    return mountedRef.current && activeProjectIdRef.current === requestProjectId;
  }, []);

  const isCurrentSend = useCallback((requestProjectId: number, seq: number) => {
    return isCurrentProject(requestProjectId) && sendSeqRef.current === seq;
  }, [isCurrentProject]);

  const latestAssistant = useMemo(() => {
    for (let i = messages.length - 1; i >= 0; i -= 1) {
      if (messages[i].role === "assistant") return messages[i];
    }
    return null;
  }, [messages]);

  const latestConversationState = latestAssistant?.conversationState;
  const progressSummary = latestAssistant?.progressSummary;
  const nextQuestion = latestAssistant?.nextQuestion;
  const progressSummaryLines = useMemo(() => splitSummaryLines(progressSummary), [progressSummary]);
  const conversationComplete = latestAssistant?.isComplete === true;

  const blueprintWorkflow = useBlueprintWorkflow({
    projectId,
    project,
    messages,
    conversationComplete,
    sending,
    latestConversationState,
    progressSummary,
    nextQuestion,
    addToast,
  });
  const { stage } = blueprintWorkflow;

  const applyParsedAssistant = useCallback((assistantId: string, rawResponse: string) => {
    const parsed = parseInspirationResponse(rawResponse);
    if (parsed.placeholder) {
      setInputPlaceholder(parsed.placeholder);
    }
    setMessages((prev) =>
      prev.map((m) =>
        m.id === assistantId
          ? {
              ...m,
              content: parsed.content,
              rawContent: parsed.rawContent,
              options: parsed.options,
              isComplete: parsed.isComplete,
              conversationState: parsed.conversationState,
              progressSummary: parsed.progressSummary,
              nextQuestion: parsed.nextQuestion,
              streaming: false,
            }
          : m,
      ),
    );
  }, []);

  useEffect(() => {
    if (!Number.isFinite(projectId) || projectId <= 0) {
      addToast("无效的项目 ID", "error");
      navigate("/");
      return;
    }

    setProject(null);
    setInput("");
    setSending(false);
    sendingRef.current = false;
    generationHandleRef.current = null;
    generationAssistantIdRef.current = null;
    handledTerminalRef.current = null;
    setGenerationHandle(null);
    setRequestPreviewOpen(false);
    setRequestPreviewLoading(false);
    setRequestPreviewData(null);
    sendSeqRef.current += 1;
    requestPreviewSeqRef.current += 1;

    setMessages([]);
    setInputPlaceholder(DEFAULT_INPUT_PLACEHOLDER);

    let cancelled = false;
    (async () => {
      try {
        const p = await getProject(String(projectId));
        if (cancelled) return;
        if (p.project_kind !== "text") {
          navigate(getProjectHomeEntryRoute(p), { replace: true });
          return;
        }
        if (!canEditInspirationProject(p)) {
          addToast("当前账号没有编辑此项目的权限", "error");
          navigate(`/novel/${projectId}`, { replace: true });
          return;
        }
        setProject(p);

        const history = await getInspirationHistory(projectId);
        if (cancelled) return;
        const restored = history.map(normalizeHistoryMessage);
        if (restored.length > 0) {
          setMessages(restored);
        } else {
          setMessages([welcomeMessage(p.title)]);
        }

      } catch (err) {
        if (cancelled) return;
        addToast(extractErrorMessage(err, "加载项目失败"), "error");
        navigate("/");
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [projectId, addToast, navigate]);

  useEffect(() => {
    if (!project || project.id !== projectId || generationHandleRef.current) return;
    let cancelled = false;
    void generationClient.listActive(projectId).then((summaries) => {
      if (cancelled || !isCurrentProject(projectId) || generationHandleRef.current) return;
      const active = summaries.find(
        (summary) => inspirationGenerationProject(summary, projectId) === projectId,
      );
      if (!active) return;
      const assistantId = `generation-${active.handle.task_id}`;
      generationHandleRef.current = active.handle;
      generationAssistantIdRef.current = assistantId;
      handledTerminalRef.current = null;
      sendingRef.current = true;
      setGenerationHandle(active.handle);
      setSending(true);
      setMessages((prev) => prev.some((message) => message.id === assistantId)
        ? prev
        : [
            ...prev,
            {
              id: assistantId,
              role: "assistant",
              content: "",
              streaming: true,
              createdAt: Date.now(),
            },
          ]);
    }).catch((error) => {
      if (!cancelled && isCurrentProject(projectId)) {
        addToast(extractErrorMessage(error, "恢复灵感对话任务失败"), "error");
      }
    });
    return () => {
      cancelled = true;
    };
  }, [addToast, isCurrentProject, project, projectId]);

  useEffect(() => {
    if (stage === "conversation") {
      messagesEndRef.current?.scrollIntoView({ behavior: "smooth", block: "end" });
    }
  }, [messages, stage]);

  const streamedText = generationTask.text("raw_json");

  useEffect(() => {
    const assistantId = generationAssistantIdRef.current;
    if (!generationHandle || !assistantId || !streamedText) return;
    const preview = getStreamingResponsePreview(streamedText);
    setMessages((prev) => {
      const next: Message = {
        id: assistantId,
        role: "assistant",
        content: preview.content,
        rawContent: streamedText,
        streaming: true,
        streamingOptions: preview.options,
        optionsPending: preview.optionsPending,
        createdAt: Date.now(),
      };
      return prev.some((message) => message.id === assistantId)
        ? prev.map((message) => message.id === assistantId ? { ...message, ...next } : message)
        : [...prev, next];
    });
  }, [generationHandle, streamedText]);

  useEffect(() => {
    const recoveryIssue = generationTaskRecoveryIssue(generationTask);
    if (
      !generationHandle
      || (!generationTask.terminalReceived
        && !recoveryIssue)
    ) return;
    const key = generationTaskKey(generationHandle.task_id, generationHandle.attempt);
    const current = generationHandleRef.current;
    if (!current || generationTaskKey(current.task_id, current.attempt) !== key) return;
    if (handledTerminalRef.current === key) return;
    handledTerminalRef.current = key;
    const assistantId = generationAssistantIdRef.current ?? `generation-${generationHandle.task_id}`;
    const requestProjectId = projectId;

    const finish = async () => {
      if (generationTask.taskStatus === "completed" || recoveryIssue) {
        const resultRef = generationTask.resultRef;
        const resultId = resultRef ? Number(resultRef.id) : NaN;
        const canResolveResult = resultRef?.kind === "inspiration_message"
          && Number.isSafeInteger(resultId);
        const history = canResolveResult
          ? await getInspirationHistory(requestProjectId)
          : [];
        const persisted = canResolveResult
          ? history.find((message) => message.id === resultId && message.role === "assistant") ?? null
          : null;
        const revision = persisted ? await sha256Hex(persisted.content) : null;
        const latest = generationHandleRef.current;
        if (
          !latest
          || generationTaskKey(latest.task_id, latest.attempt) !== key
          || !isCurrentProject(requestProjectId)
        ) {
          return;
        }
        const consistent = Boolean(persisted && revision === resultRef?.revision);
        const recovery = resolveFormalResultRecovery({
          recoveryIssue,
          consistent,
          hasPersisted: Boolean(persisted),
          hasResultRef: Boolean(resultRef),
          domain: "inspiration_message",
        });
        if (recovery.shouldMarkConsistencyError) {
          useGenerationTasksStore.getState().dispatch(key, {
            type: "client_state",
            state: "consistency_error",
          });
        }
        setMessages((prev) => prev.some((message) => message.id === assistantId)
          ? prev
          : [
              ...prev,
              {
                id: assistantId,
                role: "assistant",
                content: "",
                streaming: true,
                createdAt: Date.now(),
              },
            ]);
        if (persisted) {
          applyParsedAssistant(assistantId, persisted.content);
          if (recovery.kind !== "ok") {
            const consistencyWarning = recovery.warning ?? recovery.message;
            setMessages((prev) => prev.map((message) => message.id === assistantId
              ? { ...message, consistencyWarning }
              : message));
            addToast(consistencyWarning, "error");
          }
        } else if (recovery.kind !== "ok") {
          const unavailableMessage = recovery.message;
          setMessages((prev) => prev.map((message) => message.id === assistantId
            ? {
                ...message,
                content: unavailableMessage,
                consistencyWarning: unavailableMessage,
                streaming: false,
              }
            : message));
          addToast(unavailableMessage, "error");
        }
      } else {
        const message = generationTask.taskStatus === "cancelled"
          ? "灵感对话已取消"
          : generationTask.taskStatus === "interrupted"
            ? "灵感对话已中断"
            : generationTask.error?.message?.trim() || "灵感对话失败";
        setMessages((prev) => {
          const failed: Message = {
            id: assistantId,
            role: "assistant",
            content: `❌ ${message}`,
            streaming: false,
            createdAt: Date.now(),
          };
          return prev.some((item) => item.id === assistantId)
            ? prev.map((item) => item.id === assistantId ? { ...item, ...failed } : item)
            : [...prev, failed];
        });
        if (generationTask.taskStatus !== "cancelled") addToast(message, "error");
      }

      const latest = generationHandleRef.current;
      if (!latest || generationTaskKey(latest.task_id, latest.attempt) !== key) return;
      generationHandleRef.current = null;
      generationAssistantIdRef.current = null;
      setGenerationHandle(null);
      sendingRef.current = false;
      setSending(false);
      inputRef.current?.focus();
    };

    void finish().catch((error) => {
      const latest = generationHandleRef.current;
      if (!latest || generationTaskKey(latest.task_id, latest.attempt) !== key) return;
      logger.scoped("InspirationChat").error("generation finalization failed", { error });
      const message = extractErrorMessage(error, "灵感对话结果校验失败");
      setMessages((prev) => prev.map((item) => item.id === assistantId
        ? { ...item, content: `❌ ${message}`, streaming: false }
        : item));
      addToast(message, "error");
      generationHandleRef.current = null;
      generationAssistantIdRef.current = null;
      setGenerationHandle(null);
      sendingRef.current = false;
      setSending(false);
      inputRef.current?.focus();
    });
  }, [
    addToast,
    applyParsedAssistant,
    generationHandle,
    generationTask.error,
    generationTask.clientState,
    generationTask.recoveryState,
    generationTask.resultRef,
    generationTask.taskStatus,
    generationTask.terminalReceived,
    isCurrentProject,
    projectId,
  ]);

  const handleSend = async (text?: string) => {
    const value = (text ?? input).trim();
    if (!value || sendingRef.current) return;
    const requestProjectId = projectId;
    const requestSeq = sendSeqRef.current + 1;
    sendSeqRef.current = requestSeq;
    sendingRef.current = true;

    const userMessage: Message = {
      id: msgId(),
      role: "user",
      content: value,
      createdAt: Date.now(),
    };
    const assistantId = msgId();
    const assistantMessage: Message = {
      id: assistantId,
      role: "assistant",
      content: "",
      streaming: true,
      createdAt: Date.now(),
    };

    generationAssistantIdRef.current = assistantId;
    handledTerminalRef.current = null;
    setMessages((prev) => [...prev, userMessage, assistantMessage]);
    setInput("");
    setSending(true);

    try {
      const handle = await generationClient.start((meta) =>
        startInspirationGeneration({
          meta,
          project_id: requestProjectId,
          message: value,
        }),
      );
      if (!isCurrentSend(requestProjectId, requestSeq)) return;
      generationHandleRef.current = handle;
      setGenerationHandle(handle);
    } catch (err) {
      if (!isCurrentSend(requestProjectId, requestSeq)) return;
      logger.scoped("InspirationChat").error("converse failed", { err });
      addToast(extractErrorMessage(err, "对话失败"), "error");
      setMessages((prev) => {
        const cleared = prev.map((m) =>
          m.id === assistantId ? { ...m, streaming: false } : m,
        );
        return [
          ...cleared,
          {
            id: msgId(),
            role: "system",
            content: `❌ ${extractErrorMessage(err)}`,
            createdAt: Date.now(),
          },
        ];
      });
      generationAssistantIdRef.current = null;
      sendingRef.current = false;
      setSending(false);
      inputRef.current?.focus();
    }
  };

  const handlePreviewInspirationRequest = useCallback(async () => {
    const value = input.trim();
    if (!value || sending) return;
    const requestProjectId = projectId;
    const requestSeq = requestPreviewSeqRef.current + 1;
    requestPreviewSeqRef.current = requestSeq;
    setRequestPreviewOpen(true);
    setRequestPreviewLoading(true);
    setRequestPreviewData(null);
    try {
      const preview = await previewInspirationRequest({
        project_id: requestProjectId,
        message: value,
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
      addToast(extractErrorMessage(err, "预览提示词失败"), "error");
      setRequestPreviewOpen(false);
    } finally {
      if (
        isCurrentProject(requestProjectId) &&
        requestPreviewSeqRef.current === requestSeq
      ) {
        setRequestPreviewLoading(false);
      }
    }
  }, [addToast, input, isCurrentProject, projectId, sending]);

  const handleKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (
      e.key === "Enter" &&
      !e.shiftKey &&
      !isImeCompositionActive(
        e.nativeEvent,
        composingRef.current,
        lastCompositionEndRef.current,
      )
    ) {
      e.preventDefault();
      void handleSend();
    }
  };

  const handleCompositionStart = () => {
    composingRef.current = true;
    setInputActive(true);
  };

  const handleCompositionEnd = () => {
    lastCompositionEndRef.current = Date.now();
    window.setTimeout(() => {
      composingRef.current = false;
    }, 0);
  };

  return {
    project,
    stage,
    messages,
    input,
    setInput,
    sending,
    generating: blueprintWorkflow.generating,
    blueprint: blueprintWorkflow.blueprint,
    blueprintConsistencyWarning: blueprintWorkflow.blueprintConsistencyWarning,
    refining: blueprintWorkflow.refining,
    inputPlaceholder,
    blueprintProgressOpen: blueprintWorkflow.blueprintProgressOpen,
    setBlueprintProgressOpen: blueprintWorkflow.setBlueprintProgressOpen,
    blueprintProgress: blueprintWorkflow.blueprintProgress,
    blueprintCancelling: blueprintWorkflow.blueprintCancelling,
    blueprintStreamText: blueprintWorkflow.blueprintStreamText,
    blueprintRequestPreviewOpen: blueprintWorkflow.requestPreviewOpen,
    blueprintRequestPreviewLoading: blueprintWorkflow.requestPreviewLoading,
    blueprintRequestPreviewData: blueprintWorkflow.requestPreviewData,
    setBlueprintRequestPreviewOpen: blueprintWorkflow.setRequestPreviewOpen,
    messagesEndRef,
    inputRef,
    inputActive,
    requestPreviewOpen,
    requestPreviewLoading,
    requestPreviewData,
    setRequestPreviewOpen,
    progressSummaryLines,
    nextQuestion,
    conversationComplete,
    canGenerate: blueprintWorkflow.canGenerate,
    canCancelBlueprintGeneration: blueprintWorkflow.canCancelBlueprintGeneration,
    handleSend,
    handlePreviewInspirationRequest,
    handleKeyDown,
    handleCompositionStart,
    handleCompositionEnd,
    handleGenerateBlueprint: blueprintWorkflow.handleGenerateBlueprint,
    handlePreviewGenerateBlueprint: blueprintWorkflow.handlePreviewGenerateBlueprint,
    handleCancelBlueprint: blueprintWorkflow.handleCancelBlueprint,
    handleRefineBlueprint: blueprintWorkflow.handleRefineBlueprint,
    handlePreviewRefineBlueprint: blueprintWorkflow.handlePreviewRefineBlueprint,
    handleConfirmBlueprint: blueprintWorkflow.handleConfirmBlueprint,
    handleConversationFocus: () => setInputActive(true),
    handleConversationBlur: () => setInputActive(false),
    handleBackToConversation: blueprintWorkflow.handleBackToConversation,
  };
};
