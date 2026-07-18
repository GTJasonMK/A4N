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
import {
  listenAppEvent,
  type UnlistenFn,
} from "../../api/runtime";
import { getBlueprint } from "../../api/blueprint";
import {
  converseInspiration,
  converseInspirationStream,
  getInspirationHistory,
  previewInspirationRequest,
} from "../../api/inspiration";
import type { ModelRequestPreview } from "../../api/requestPreview";
import { getProject, type NovelProject } from "../../api/project";
import { logger } from "../../utils/logger";
import { isImeCompositionActive } from "../../utils/ime";
import { getProjectHomeEntryRoute } from "../../utils/projectRouting";
import {
  clearBlueprintPending,
  DEFAULT_INPUT_PLACEHOLDER,
  eventMessage,
  eventMessageOrNull,
  firstString,
  msgId,
  normalizeHistoryMessage,
  readBlueprintPending,
  readChatCache,
  splitSummaryLines,
  taskId,
  welcomeMessage,
  writeChatCache,
  type WorkflowEvent,
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
  const [useStream, setUseStream] = useState(true);
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
  const requestPreviewSeqRef = useRef(0);

  activeProjectIdRef.current = projectId;

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      sendingRef.current = false;
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
  const progressSummary = firstString(
    latestAssistant?.progressSummary,
    latestConversationState?.progress_summary,
  );
  const nextQuestion = firstString(
    latestAssistant?.nextQuestion,
    latestConversationState?.next_question,
  );
  const progressSummaryLines = useMemo(() => splitSummaryLines(progressSummary), [progressSummary]);
  const userTurns = useMemo(() => messages.filter((m) => m.role === "user").length, [messages]);
  const conversationComplete = useMemo(
    () => messages.some((m) => m.role === "assistant" && m.isComplete),
    [messages],
  );

  const blueprintWorkflow = useBlueprintWorkflow({
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
  });
  const { showBlueprintConfirmation, stage } = blueprintWorkflow;

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
    setRequestPreviewOpen(false);
    setRequestPreviewLoading(false);
    setRequestPreviewData(null);
    sendSeqRef.current += 1;
    requestPreviewSeqRef.current += 1;

    const cached = readChatCache(projectId);
    if (cached) {
      setMessages(cached.messages);
      if (cached.inputPlaceholder) setInputPlaceholder(cached.inputPlaceholder);
    } else {
      setMessages([]);
      setInputPlaceholder(DEFAULT_INPUT_PLACEHOLDER);
    }

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

        const history = await getInspirationHistory(projectId, 80);
        if (cancelled) return;
        const restored = history
          .map(normalizeHistoryMessage)
          .filter((m): m is Message => m !== null);
        if (restored.length > 0) {
          setMessages(restored);
        } else if (!cached) {
          setMessages([welcomeMessage(p.title)]);
        }

        const pending = readBlueprintPending(projectId);
        if (pending) {
          const existingBlueprint = await getBlueprint(projectId);
          if (cancelled) return;
          if (existingBlueprint) {
            clearBlueprintPending(projectId);
            showBlueprintConfirmation(existingBlueprint);
            addToast("检测到上次蓝图已生成，已恢复到确认页", "success");
          } else {
            addToast("上次蓝图生成可能仍未完成；如果稍后进入项目详情看到蓝图，说明后端已完成。", "info");
          }
        }
      } catch (err) {
        if (cancelled) return;
        addToast(extractErrorMessage(err, "加载项目失败"), "error");
        if (!cached) navigate("/");
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [projectId, addToast, navigate, showBlueprintConfirmation]);

  useEffect(() => {
    if (!Number.isFinite(projectId) || projectId <= 0) return;
    writeChatCache(projectId, messages, inputPlaceholder);
  }, [inputPlaceholder, messages, projectId]);

  useEffect(() => {
    if (stage === "conversation") {
      messagesEndRef.current?.scrollIntoView({ behavior: "smooth", block: "end" });
    }
  }, [messages, stage]);

  const handleSend = async (text?: string) => {
    const value = (text ?? input).trim();
    if (!value || sendingRef.current) return;
    const requestProjectId = projectId;
    const requestSeq = sendSeqRef.current + 1;
    sendSeqRef.current = requestSeq;
    sendingRef.current = true;
    const requestUseStream = useStream;

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
      streaming: requestUseStream,
      createdAt: Date.now(),
    };

    setMessages((prev) =>
      requestUseStream ? [...prev, userMessage, assistantMessage] : [...prev, userMessage],
    );
    setInput("");
    setSending(true);

    let unlisten: UnlistenFn | null = null;
    let streamedText = "";
    let nextChunkIndex = 0;
    let streamTerminated = false;
    const pendingChunks = new Map<number, string>();

    const appendOrderedChunks = (): boolean => {
      let changed = false;
      while (pendingChunks.has(nextChunkIndex)) {
        const chunk = pendingChunks.get(nextChunkIndex);
        pendingChunks.delete(nextChunkIndex);
        if (chunk) {
          streamedText += chunk;
          changed = true;
        }
        nextChunkIndex += 1;
      }
      return changed;
    };

    try {
      if (requestUseStream) {
        const currentTaskId = taskId(requestProjectId);
        unlisten = await listenAppEvent<WorkflowEvent>(`workflow:${currentTaskId}`, (event) => {
          if (!isCurrentSend(requestProjectId, requestSeq)) return;
          const payload = event.payload;
          if (payload.type === "Chunk") {
            if (streamTerminated) return;
            const data = (payload as {
              type: "Chunk";
              data: { content?: unknown; index?: unknown };
            }).data;
            if (typeof data.content !== "string") {
              addToast("流式响应缺少内容片段", "error");
              return;
            }
            const index =
              typeof data.index === "number" ? data.index : Number(data.index);
            if (!Number.isInteger(index) || index < 0) {
              addToast("流式响应缺少有效片段序号", "error");
              return;
            }
            const delta = data.content;
            if (!delta) return;
            if (index < nextChunkIndex || pendingChunks.has(index)) return;
            pendingChunks.set(index, delta);
            if (!appendOrderedChunks()) return;
            const preview = getStreamingResponsePreview(streamedText);
            setMessages((prev) =>
              prev.map((m) =>
                m.id === assistantId
                  ? {
                      ...m,
                      content: preview.content,
                      rawContent: streamedText,
                      streaming: true,
                      streamingOptions: preview.options,
                      optionsPending: preview.optionsPending,
                    }
                  : m,
              ),
            );
          } else if (payload.type === "Complete") {
            streamTerminated = true;
            const streamedResponse = firstString(streamedText);
            const fullResponse = eventMessageOrNull(payload.data) ?? streamedResponse;
            if (fullResponse) {
              applyParsedAssistant(assistantId, fullResponse);
            } else {
              setMessages((prev) =>
                prev.map((m) => (m.id === assistantId ? { ...m, streaming: false } : m)),
              );
            }
          } else if (payload.type === "Error") {
            streamTerminated = true;
            const message = eventMessage(payload.data, "流式响应事件缺少错误信息");
            setMessages((prev) =>
              prev.map((m) =>
                m.id === assistantId
                  ? {
                      ...m,
                      content: `❌ ${message}`,
                      streaming: false,
                    }
                  : m,
              ),
            );
          }
        });

        const response = await converseInspirationStream({
          project_id: requestProjectId,
          message: value,
          task_id: currentTaskId,
          variables: null,
        });
        if (!isCurrentSend(requestProjectId, requestSeq)) return;
        const finalResponse = firstString(response, streamedText);
        if (!finalResponse) {
          throw new Error("AI 未返回内容");
        }
        applyParsedAssistant(assistantId, finalResponse);
      } else {
        const response = await converseInspiration({
          project_id: requestProjectId,
          user_input: value,
          variables: null,
        });
        if (!isCurrentSend(requestProjectId, requestSeq)) return;
        const finalResponse = response.trim();
        if (!finalResponse) {
          throw new Error("AI 未返回内容");
        }
        const parsed = parseInspirationResponse(finalResponse);
        if (parsed.placeholder) {
          setInputPlaceholder(parsed.placeholder);
        }
        setMessages((prev) => [
          ...prev,
          {
            id: msgId(),
            role: "assistant",
            content: parsed.content,
            rawContent: parsed.rawContent,
            options: parsed.options,
            isComplete: parsed.isComplete,
            conversationState: parsed.conversationState,
            progressSummary: parsed.progressSummary,
            nextQuestion: parsed.nextQuestion,
            createdAt: Date.now(),
          },
        ]);
      }
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
    } finally {
      unlisten?.();
      if (isCurrentSend(requestProjectId, requestSeq)) {
        sendingRef.current = false;
        setSending(false);
        inputRef.current?.focus();
      } else if (!mountedRef.current || activeProjectIdRef.current !== requestProjectId) {
        sendingRef.current = false;
      }
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
        user_input: value,
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
    refining: blueprintWorkflow.refining,
    useStream,
    setUseStream,
    inputPlaceholder,
    blueprintProgressOpen: blueprintWorkflow.blueprintProgressOpen,
    setBlueprintProgressOpen: blueprintWorkflow.setBlueprintProgressOpen,
    blueprintProgress: blueprintWorkflow.blueprintProgress,
    blueprintCancelling: blueprintWorkflow.blueprintCancelling,
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
    canTryGenerate: blueprintWorkflow.canTryGenerate,
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
