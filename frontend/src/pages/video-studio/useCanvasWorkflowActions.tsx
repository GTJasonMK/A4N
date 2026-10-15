import React, { useCallback, useEffect } from "react";
import { Eye, Play, Trash2, Upload } from "lucide-react";
import { extractErrorMessage } from "../../api";
import type { ToastType } from "../../components/feedback/Toast";
import {
  previewCanvasNodeRequest,
  runCanvasGraph,
  runCanvasGraphToNode,
  runCanvasNode,
  type CanvasGraphDocument,
} from "../../api/canvas";
import {
  cancelWorkflow,
  getWorkflowRunDetail,
  getWorkflowRunProgressEvents,
  listWorkflowRuns,
  resumeWorkflowRun,
  type WorkflowRunDetailSummary,
  type WorkflowRunSummary,
} from "../../api/streaming";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import type { ContextMenuEntry } from "../../components/ui/ContextMenu";
import type { ModelRequestPreview } from "../../api/requestPreview";
import type { ProgressEventRecord } from "../../api/progress";
import type { MessageTranslator } from "../../i18n";
import {
  CANVAS_WORKFLOW_VISIBLE_PROGRESS_EVENT_COUNT,
  canvasWorkflowRuntimeRefreshSignature,
  computeRunToNodeScope,
  findSubmittedCanvasWorkflowRun,
  runToNodeConfirmationMessage,
  workflowRunGraphVersion,
} from "./canvasWorkflowState";
import { nodeHasProviderRequest } from "./canvasNodeKinds";
import {
  nodeDisplayTitle,
  } from "./StudioCanvasNode";
import {
  studioNodeContract,
  type StudioNodeContractMap,
} from "./nodeContracts";
import { numericId } from "./canvasGraphMapping";
import type { StudioEdge, StudioNode } from "./types";

export const CANVAS_WORKFLOW_KIND = "video_canvas_graph";

export interface CanvasWorkflowSubmissionLabel {
  key: Parameters<MessageTranslator>[0];
  params?: Parameters<MessageTranslator>[1];
}

export interface CanvasWorkflowActionsOptions {
  projectId: number;
  project: { source_mode?: string } | null;
  document: CanvasGraphDocument | null;
  nodes: StudioNode[];
  edges: StudioEdge[];
  nodeContractsByKind: StudioNodeContractMap;
  canResumeCurrentCanvasRun: boolean;
  canvasMutationDisabled: boolean;
  dirty: boolean;
  workflowSubmitting: boolean;
  resumingWorkflow: boolean;
  cancellingWorkflow: boolean;
  latestWorkflowRunId: number | null;
  workflowDetail: WorkflowRunDetailSummary | null;
  contextMenu: { open: (x: number, y: number, items: ContextMenuEntry[]) => void };
  workflowDiscoveryTimerRef: React.MutableRefObject<number | null>;
  workflowDiscoveryInFlightRef: React.MutableRefObject<boolean>;
  workflowRuntimeRefreshSignatureRef: React.MutableRefObject<string | null>;
  t: MessageTranslator;
  addToast: (message: string, tone: ToastType) => void;
  setLatestWorkflowRunId: React.Dispatch<React.SetStateAction<number | null>>;
  setWorkflowDetail: React.Dispatch<React.SetStateAction<WorkflowRunDetailSummary | null>>;
  setWorkflowEvents: React.Dispatch<React.SetStateAction<ProgressEventRecord[]>>;
  setWorkflowStatusError: React.Dispatch<React.SetStateAction<string | null>>;
  setWorkflowEventsError: React.Dispatch<React.SetStateAction<string | null>>;
  setWorkflowLoading: React.Dispatch<React.SetStateAction<boolean>>;
  setWorkflowSubmissionLabel: React.Dispatch<React.SetStateAction<CanvasWorkflowSubmissionLabel | null>>;
  setCancellingWorkflow: React.Dispatch<React.SetStateAction<boolean>>;
  setResumingWorkflow: React.Dispatch<React.SetStateAction<boolean>>;
  setPreviewDialogOpen: React.Dispatch<React.SetStateAction<boolean>>;
  setPreviewLoading: React.Dispatch<React.SetStateAction<boolean>>;
  setPreviewData: React.Dispatch<React.SetStateAction<ModelRequestPreview | null>>;
  setContentPreviewNode: React.Dispatch<React.SetStateAction<StudioNode | null>>;
  refreshCanvasRuntimeResults: (graphId: number) => Promise<void>;
  selectCanvasNode: (nodeId: string) => void;
  selectCanvasEdge: (edgeId: string) => void;
  deleteCanvasNode: (node: StudioNode) => void;
  deleteCanvasEdge: (edge: StudioEdge) => void;
  importMedia: (mediaKind: "image" | "video" | "audio") => Promise<void>;
}

export function useCanvasWorkflowActions({
  projectId,
  project,
  document,
  nodes,
  edges,
  nodeContractsByKind,
  canResumeCurrentCanvasRun,
  canvasMutationDisabled,
  dirty,
  workflowSubmitting,
  resumingWorkflow,
  cancellingWorkflow,
  latestWorkflowRunId,
  workflowDetail,
  contextMenu,
  workflowDiscoveryTimerRef,
  workflowDiscoveryInFlightRef,
  workflowRuntimeRefreshSignatureRef,
  t,
  addToast,
  setLatestWorkflowRunId,
  setWorkflowDetail,
  setWorkflowEvents,
  setWorkflowStatusError,
  setWorkflowEventsError,
  setWorkflowLoading,
  setWorkflowSubmissionLabel,
  setCancellingWorkflow,
  setResumingWorkflow,
  setPreviewDialogOpen,
  setPreviewLoading,
  setPreviewData,
  setContentPreviewNode,
  refreshCanvasRuntimeResults,
  selectCanvasNode,
  selectCanvasEdge,
  deleteCanvasNode,
  deleteCanvasEdge,
  importMedia,
}: CanvasWorkflowActionsOptions) {
  const translatorRef = React.useRef(t);
  translatorRef.current = t;
  const currentT = useCallback(() => translatorRef.current, []);
  const workflowStatusRequestRef = React.useRef(0);
  const workflowStatusInFlightRef = React.useRef(false);
  const workflowProgressCursorRef = React.useRef<{ runId: number; eventId: string } | null>(null);
  const latestWorkflowRunIdRef = React.useRef(latestWorkflowRunId);
  latestWorkflowRunIdRef.current = latestWorkflowRunId;
  const canvasScopeRef = React.useRef("");
  canvasScopeRef.current = `${projectId}:${document?.graph.id ?? ""}:${document?.graph.version ?? ""}`;
  const previewRequestRef = React.useRef(0);
  useEffect(() => () => {
    previewRequestRef.current += 1;
  }, []);

  const loadWorkflowStatus = useCallback(
    async (workflowRunId: number, options: { silent?: boolean; expectedRunId?: number } = {}): Promise<boolean> => {
      if (options.expectedRunId != null && latestWorkflowRunIdRef.current !== options.expectedRunId) return false;
      workflowStatusInFlightRef.current = true;
      const requestId = workflowStatusRequestRef.current + 1;
      workflowStatusRequestRef.current = requestId;
      latestWorkflowRunIdRef.current = workflowRunId;
      setLatestWorkflowRunId(workflowRunId);
      setWorkflowLoading(true);
      const previousEventCursor = workflowProgressCursorRef.current;
      const afterEventId = previousEventCursor?.runId === workflowRunId
        ? previousEventCursor.eventId
        : undefined;
      try {
        const [detailResult, eventsResult] = await Promise.allSettled([
          getWorkflowRunDetail(workflowRunId, false, true, false),
          getWorkflowRunProgressEvents(
            workflowRunId,
            CANVAS_WORKFLOW_VISIBLE_PROGRESS_EVENT_COUNT,
            afterEventId,
          ),
        ]);
        if (workflowStatusRequestRef.current !== requestId) return false;
        const detail = detailResult.status === "fulfilled" ? detailResult.value : null;
        if (detailResult.status === "fulfilled") {
          setWorkflowDetail(detailResult.value);
          setWorkflowStatusError(null);
        } else {
          setWorkflowDetail(null);
          setWorkflowStatusError(extractErrorMessage(detailResult.reason, currentT()("video.loadCanvasRunsFailed")));
        }
        if (eventsResult.status === "fulfilled") {
          const nextEvents = eventsResult.value;
          const latestEvent = nextEvents.at(-1);
          if (latestEvent) {
            workflowProgressCursorRef.current = { runId: workflowRunId, eventId: latestEvent.id };
          }
          setWorkflowEvents((current) => afterEventId == null
            ? nextEvents
            : [...current, ...nextEvents].slice(-CANVAS_WORKFLOW_VISIBLE_PROGRESS_EVENT_COUNT));
          setWorkflowEventsError(null);
        } else {
          workflowProgressCursorRef.current = null;
          setWorkflowEvents([]);
          setWorkflowEventsError(extractErrorMessage(eventsResult.reason, currentT()("video.loadCanvasEventsFailed")));
        }
        if (!detail) return false;
        const refreshSignature = canvasWorkflowRuntimeRefreshSignature(detail);
        const graphId =
          detail.run.scope_table === "canvas_graph"
            ? detail.run.scope_id
            : null;
        if (
          refreshSignature &&
          graphId &&
          workflowRuntimeRefreshSignatureRef.current !== refreshSignature
        ) {
          try {
            await refreshCanvasRuntimeResults(graphId);
            if (workflowStatusRequestRef.current !== requestId) return false;
            workflowRuntimeRefreshSignatureRef.current = refreshSignature;
          } catch (err) {
            if (workflowStatusRequestRef.current === requestId) {
              const message = extractErrorMessage(err, currentT()("video.syncCanvasRuntimeFailed"));
              setWorkflowStatusError(message);
              if (!options.silent) addToast(message, "error");
            }
          }
        }
        return detailResult.status === "fulfilled";
      } finally {
        if (workflowStatusRequestRef.current === requestId) {
          workflowStatusInFlightRef.current = false;
          setWorkflowLoading(false);
        }
      }
    },
    [
      addToast,
      refreshCanvasRuntimeResults,
      setLatestWorkflowRunId,
      setWorkflowDetail,
      setWorkflowEvents,
      setWorkflowStatusError,
      setWorkflowEventsError,
      setWorkflowLoading,
      currentT,
    ],
  );

  const clearWorkflowStatus = useCallback(() => {
    workflowStatusRequestRef.current += 1;
    workflowStatusInFlightRef.current = false;
    latestWorkflowRunIdRef.current = null;
    setLatestWorkflowRunId(null);
    setWorkflowDetail(null);
    setWorkflowEvents([]);
    workflowProgressCursorRef.current = null;
    setWorkflowStatusError(null);
    setWorkflowEventsError(null);
    setWorkflowLoading(false);
    workflowRuntimeRefreshSignatureRef.current = null;
  }, [
    setLatestWorkflowRunId,
    setWorkflowDetail,
    setWorkflowEvents,
    setWorkflowEventsError,
    setWorkflowLoading,
    setWorkflowStatusError,
  ]);

  const stopWorkflowDiscovery = useCallback(() => {
    if (workflowDiscoveryTimerRef.current !== null) {
      window.clearInterval(workflowDiscoveryTimerRef.current);
      workflowDiscoveryTimerRef.current = null;
    }
    workflowDiscoveryInFlightRef.current = false;
  }, []);

  const findSubmittedCanvasWorkflowRunForGraph = useCallback(
    async (
      graphId: number,
      graphVersion: number,
      submittedAtMs: number,
    ): Promise<WorkflowRunSummary | null> => {
      const runs = await listWorkflowRuns({
        project_id: projectId,
        workflow_kind: CANVAS_WORKFLOW_KIND,
        resumable_only: false,
        limit: 50,
      });
      return findSubmittedCanvasWorkflowRun(runs, graphId, graphVersion, submittedAtMs);
    },
    [projectId],
  );

  const startWorkflowDiscovery = useCallback(
    (graphId: number, graphVersion: number, submittedAtMs: number) => {
      stopWorkflowDiscovery();
      const tick = async () => {
        if (workflowDiscoveryInFlightRef.current) return;
        workflowDiscoveryInFlightRef.current = true;
        try {
          const run = await findSubmittedCanvasWorkflowRunForGraph(graphId, graphVersion, submittedAtMs);
          if (!run) return;
          await loadWorkflowStatus(run.id, { silent: true });
          stopWorkflowDiscovery();
        } catch (err) {
          setWorkflowStatusError(extractErrorMessage(err, currentT()("video.loadCanvasRunsFailed")));
        } finally {
          workflowDiscoveryInFlightRef.current = false;
        }
      };
      workflowDiscoveryTimerRef.current = window.setInterval(() => {
        void tick();
      }, 700);
      void tick();
    },
    [findSubmittedCanvasWorkflowRunForGraph, loadWorkflowStatus, setWorkflowStatusError, stopWorkflowDiscovery, currentT],
  );

  useEffect(() => stopWorkflowDiscovery, [stopWorkflowDiscovery]);

  useEffect(() => {
    if (!latestWorkflowRunId) return;
    const status = workflowDetail?.run.status;
    if (status !== "running" && status !== "pending") return;
    const timer = window.setInterval(() => {
      if (workflowStatusInFlightRef.current) return;
      void loadWorkflowStatus(latestWorkflowRunId, { silent: true, expectedRunId: latestWorkflowRunId });
    }, 1000);
    return () => window.clearInterval(timer);
  }, [latestWorkflowRunId, loadWorkflowStatus, workflowDetail?.run.status]);

  useEffect(() => {
    const graph = document?.graph;
    if (!graph) return;
    let cancelled = false;
    clearWorkflowStatus();
    const restoreLatestRun = async () => {
      try {
        const runs = await listWorkflowRuns({
          project_id: projectId,
          workflow_kind: CANVAS_WORKFLOW_KIND,
          resumable_only: false,
          limit: 50,
        });
        if (cancelled) return;
        const latest = runs.find(
          (run) =>
            run.scope_table === "canvas_graph" &&
            run.scope_id === graph.id &&
            workflowRunGraphVersion(run) === graph.version,
        );
        workflowRuntimeRefreshSignatureRef.current = null;
        if (!latest) {
          return;
        }
        await loadWorkflowStatus(latest.id, { silent: true });
      } catch (err) {
        if (cancelled) return;
        clearWorkflowStatus();
        setWorkflowStatusError(extractErrorMessage(err, currentT()("video.loadCanvasRunsFailed")));
      }
    };
    void restoreLatestRun();
    return () => {
      cancelled = true;
    };
  }, [
    clearWorkflowStatus,
    document?.graph.id,
    document?.graph.version,
    loadWorkflowStatus,
    projectId,
    setWorkflowStatusError,
    currentT,
  ]);

  const previewNode = async (node: StudioNode) => {
    if (!nodeHasProviderRequest(node)) {
      setContentPreviewNode(node);
      return;
    }
    if (!document) return;
    if (dirty) {
      addToast(t("video.saveBeforePreview"), "info");
      return;
    }
    const nodeId = numericId(node.id);
    if (!nodeId) {
      addToast(t("video.saveNodeBeforeRequest"), "info");
      return;
    }
    const requestScope = canvasScopeRef.current;
    const requestId = previewRequestRef.current + 1;
    previewRequestRef.current = requestId;
    setPreviewDialogOpen(true);
    setPreviewLoading(true);
    setPreviewData(null);
    try {
      const preview = await previewCanvasNodeRequest(
        projectId,
        document.graph.id,
        nodeId,
      );
      if (
        canvasScopeRef.current !== requestScope ||
        previewRequestRef.current !== requestId
      ) return;
      setPreviewData(preview);
    } catch (err) {
      if (
        canvasScopeRef.current !== requestScope ||
        previewRequestRef.current !== requestId
      ) return;
      setPreviewDialogOpen(false);
      addToast(extractErrorMessage(err, currentT()("video.previewRequestFailed")), "error");
    } finally {
      if (
        canvasScopeRef.current === requestScope &&
        previewRequestRef.current === requestId
      ) {
        setPreviewLoading(false);
      }
    }
  };

  const closePreview = () => {
    previewRequestRef.current += 1;
    setPreviewDialogOpen(false);
    setPreviewLoading(false);
    setPreviewData(null);
  };

  const runNode = async (node: StudioNode) => {
    if (!document || canvasMutationDisabled || workflowSubmitting) return;
    if (dirty) {
      addToast(t("video.saveBeforeRun"), "info");
      return;
    }
    const contract = studioNodeContract(
      nodeContractsByKind,
      node.data.nodeKind,
    );
    if (!contract?.runnable) {
      addToast(t("video.inputNodeCannotRun"), "info");
      return;
    }
    const nodeId = numericId(node.id);
    if (!nodeId) {
      addToast(t("video.saveNodeBeforeRun"), "info");
      return;
    }
    const requestScope = canvasScopeRef.current;
    const confirmation = [
      t("video.runNodeMessage"),
      "",
      t("video.runNodeTarget", { name: nodeDisplayTitle(node.data, t) }),
      t("video.runNodeConfirmDescription"),
      t("video.runNodeInputHint"),
    ].join("\n");
    const confirmed = await confirmDialog({
      title: t("video.runNodeTitle"),
      message: confirmation,
      confirmText: t("video.run"),
      cancelText: t("common.cancel"),
      dialogType: "warning",
    });
    if (!confirmed || canvasScopeRef.current !== requestScope) {
      return;
    }
    setWorkflowSubmissionLabel({
      key: "video.runNodeSubmitting",
      params: { name: nodeDisplayTitle(node.data, currentT()) },
    });
    clearWorkflowStatus();
    const graphId = document.graph.id;
    startWorkflowDiscovery(graphId, document.graph.version, Date.now());
    try {
      const result = await runCanvasNode(projectId, graphId, nodeId);
      if (canvasScopeRef.current !== requestScope) return;
      addToast(
        result.error_message ?? currentT()("video.runOnlyNodeSubmitted"),
        result.status === "failed" ? "info" : "success",
      );
      void loadWorkflowStatus(result.workflow_run_id, { silent: true }).then((refreshed) => {
        if (!refreshed) addToast(currentT()("video.loadCanvasRunsFailed"), "error");
      });
    } catch (err) {
      addToast(extractErrorMessage(err, currentT()("video.runOnlyNodeError")), "error");
    } finally {
      setWorkflowSubmissionLabel(null);
      stopWorkflowDiscovery();
    }
  };

  const runToNode = async (node: StudioNode) => {
    if (!document || canvasMutationDisabled || workflowSubmitting) return;
    if (dirty) {
      addToast(t("video.saveBeforeRun"), "info");
      return;
    }
    if (
      studioNodeContract(nodeContractsByKind, node.data.nodeKind)?.category !==
      "output"
    ) {
      addToast(
        t("video.runToOutputOnly"),
        "info",
      );
      return;
    }
    const nodeId = numericId(node.id);
    if (!nodeId) {
      addToast(t("video.saveNodeBeforeRun"), "info");
      return;
    }
    const requestScope = canvasScopeRef.current;
    const scope = computeRunToNodeScope(
      node,
      nodes,
      edges,
      nodeContractsByKind,
    );
    const confirmation = runToNodeConfirmationMessage(scope, node, t);
    const confirmed = await confirmDialog({
      title: t("video.runToNodeTitle"),
      message: confirmation,
      confirmText: t("video.run"),
      cancelText: t("common.cancel"),
      dialogType: "warning",
    });
    if (!confirmed || canvasScopeRef.current !== requestScope) {
      return;
    }
    setWorkflowSubmissionLabel({
      key: "video.runToNodeSubmitting",
      params: { name: nodeDisplayTitle(node.data, currentT()) },
    });
    clearWorkflowStatus();
    const graphId = document.graph.id;
    startWorkflowDiscovery(graphId, document.graph.version, Date.now());
    try {
      const result = await runCanvasGraphToNode(projectId, graphId, nodeId);
      if (canvasScopeRef.current !== requestScope) return;
      addToast(
        result.error_message ?? currentT()("video.runToNodeSubmitted"),
        result.status === "failed" ? "info" : "success",
      );
      void loadWorkflowStatus(result.workflow_run_id, { silent: true }).then((refreshed) => {
        if (!refreshed) addToast(currentT()("video.loadCanvasRunsFailed"), "error");
      });
    } catch (err) {
      addToast(extractErrorMessage(err, currentT()("video.runToNodeError")), "error");
    } finally {
      setWorkflowSubmissionLabel(null);
      stopWorkflowDiscovery();
    }
  };

  const runGraph = async () => {
    if (
      !document ||
      canvasMutationDisabled ||
      workflowSubmitting ||
      resumingWorkflow
    )
      return;
    if (dirty) {
      addToast(t("video.saveBeforeRun"), "info");
      return;
    }
    if (!nodes.some((node) => studioNodeContract(nodeContractsByKind, node.data.nodeKind)?.runnable)) {
      addToast(t("video.noRunnableCanvasNodes"), "info");
      return;
    }
    if (canResumeCurrentCanvasRun) {
      await resumeActiveWorkflow();
      return;
    }
    setWorkflowSubmissionLabel({ key: "video.runCanvasSubmitting" });
    const requestScope = canvasScopeRef.current;
    clearWorkflowStatus();
    const graphId = document.graph.id;
    startWorkflowDiscovery(graphId, document.graph.version, Date.now());
    try {
      const result = await runCanvasGraph(projectId, graphId);
      if (canvasScopeRef.current !== requestScope) return;
      addToast(
        result.error_message ?? currentT()("video.submittedRunCanvas"),
        result.status === "failed" ? "info" : "success",
      );
      void loadWorkflowStatus(result.workflow_run_id, { silent: true }).then((refreshed) => {
        if (!refreshed) addToast(currentT()("video.loadCanvasRunsFailed"), "error");
      });
    } catch (err) {
      addToast(extractErrorMessage(err, currentT()("video.runCanvasError")), "error");
    } finally {
      setWorkflowSubmissionLabel(null);
      stopWorkflowDiscovery();
    }
  };

  const nodeContextMenuItems = useCallback(
    (node: StudioNode): ContextMenuEntry[] => {
      const contract = studioNodeContract(
        nodeContractsByKind,
        node.data.nodeKind,
      );
      const isOutputNode = contract?.category === "output";
      const isRunnableNode = Boolean(contract?.runnable);
      const hasProviderRequest = nodeHasProviderRequest(node);
      const items: ContextMenuEntry[] = [];
      if (!hasProviderRequest || document) {
        items.push({
          label: hasProviderRequest ? t("video.viewModelRequest") : t("video.viewNodeContent"),
          icon: <Eye size={13} />,
          onClick: () => void previewNode(node),
        });
      }
      if (isRunnableNode) {
        items.push({
          label: t("video.runOnlyNode"),
          icon: <Play size={13} />,
          disabled: canvasMutationDisabled || workflowSubmitting,
          onClick: () => void runNode(node),
        });
      }
      if (isOutputNode) {
        items.push({
          label: t("video.runToNode"),
          icon: <Play size={13} />,
          disabled: canvasMutationDisabled || workflowSubmitting,
          onClick: () => void runToNode(node),
        });
      }
      if (!canvasMutationDisabled) {
        if (items.length > 0) items.push({ divider: true });
        items.push({
          label: t("video.deleteNode"),
          icon: <Trash2 size={13} />,
          danger: true,
          shortcut: "Del",
          onClick: () => deleteCanvasNode(node),
        });
      }
      return items;
    },
    [
      canvasMutationDisabled,
      deleteCanvasNode,
      document,
      nodeContractsByKind,
      previewNode,
      runNode,
      runToNode,
      workflowSubmitting,
      t,
    ],
  );

  const edgeContextMenuItems = useCallback(
    (edge: StudioEdge): ContextMenuEntry[] =>
      !canvasMutationDisabled
        ? [
            {
              label: t("video.deleteEdge"),
              icon: <Trash2 size={13} />,
              danger: true,
              shortcut: "Del",
              onClick: () => deleteCanvasEdge(edge),
            },
          ]
        : [],
    [canvasMutationDisabled, deleteCanvasEdge, t],
  );

  const mediaImportMenuItems = useCallback(
    (): ContextMenuEntry[] => [
      {
        label: t("video.importImage"),
        icon: <Upload size={13} />,
        disabled: !document || canvasMutationDisabled,
        onClick: () => void importMedia("image"),
      },
      {
        label: t("video.importVideo"),
        icon: <Upload size={13} />,
        disabled: !document || canvasMutationDisabled,
        onClick: () => void importMedia("video"),
      },
      {
        label: t("video.importAudio"),
        icon: <Upload size={13} />,
        disabled: !document || canvasMutationDisabled || project?.source_mode !== "freeform",
        onClick: () => void importMedia("audio"),
      },
    ],
    [
      canvasMutationDisabled,
      document,
      importMedia,
      project?.source_mode,
      t,
    ],
  );

  const paneContextMenuItems = useCallback(
    (): ContextMenuEntry[] => [
      {
        label: t("video.runCanvasTitle"),
        icon: <Play size={13} />,
        disabled: !document || canvasMutationDisabled || workflowSubmitting || !nodes.some((node) => studioNodeContract(nodeContractsByKind, node.data.nodeKind)?.runnable),
        onClick: () => void runGraph(),
      },
      { divider: true },
      ...mediaImportMenuItems(),
    ],
    [
      canvasMutationDisabled,
      document,
      mediaImportMenuItems,
      nodeContractsByKind,
      nodes,
      runGraph,
      workflowSubmitting,
      t,
    ],
  );

  const onNodeContextMenu = useCallback(
    (event: React.MouseEvent, node: StudioNode) => {
      event.preventDefault();
      event.stopPropagation();
      selectCanvasNode(node.id);
      contextMenu.open(
        event.clientX,
        event.clientY,
        nodeContextMenuItems(node),
      );
    },
    [contextMenu, nodeContextMenuItems, selectCanvasNode],
  );

  const onEdgeContextMenu = useCallback(
    (event: React.MouseEvent, edge: StudioEdge) => {
      event.preventDefault();
      event.stopPropagation();
      selectCanvasEdge(edge.id);
      contextMenu.open(
        event.clientX,
        event.clientY,
        edgeContextMenuItems(edge),
      );
    },
    [contextMenu, edgeContextMenuItems, selectCanvasEdge],
  );

  const onPaneContextMenu = useCallback(
    (event: React.MouseEvent | MouseEvent) => {
      event.preventDefault();
      event.stopPropagation();
      contextMenu.open(event.clientX, event.clientY, paneContextMenuItems());
    },
    [contextMenu, paneContextMenuItems],
  );

  const cancelActiveWorkflow = async () => {
    const taskId = workflowDetail?.run.active_task_id;
    const workflowRunId = workflowDetail?.run.id ?? latestWorkflowRunId;
    if ((!taskId && !workflowRunId) || cancellingWorkflow) return;
    const requestScope = canvasScopeRef.current;
    setCancellingWorkflow(true);
    try {
      const cancelled = await cancelWorkflow({
        task_id: taskId,
        workflow_run_id: workflowRunId,
      });
      if (canvasScopeRef.current !== requestScope) return;
      addToast(
        currentT()(cancelled ? "video.cancelRunRequested" : "video.cancelRunAlreadyFinished"),
        "info",
      );
      if (workflowRunId) {
        void loadWorkflowStatus(workflowRunId, { silent: true }).then((refreshed) => {
          if (!refreshed) addToast(currentT()("video.loadCanvasRunsFailed"), "error");
        });
      }
    } catch (err) {
      addToast(extractErrorMessage(err, currentT()("video.cancelRunFailed")), "error");
    } finally {
      setCancellingWorkflow(false);
    }
  };

  const resumeActiveWorkflow = async () => {
    const workflowRunId = workflowDetail?.run.id ?? latestWorkflowRunId;
    if (!workflowRunId || resumingWorkflow) return;
    const requestScope = canvasScopeRef.current;
    setResumingWorkflow(true);
    try {
      const run = await resumeWorkflowRun(workflowRunId);
      if (canvasScopeRef.current !== requestScope) return;
      workflowRuntimeRefreshSignatureRef.current = null;
      setLatestWorkflowRunId(run.id);
      setWorkflowStatusError(null);
      setWorkflowDetail((current) =>
        current?.run.id === run.id ? { ...current, run } : current,
      );
      addToast(currentT()("video.resumeRunSuccess"), "success");
      void loadWorkflowStatus(run.id, { silent: true }).then((refreshed) => {
        if (!refreshed) addToast(currentT()("video.loadCanvasRunsFailed"), "error");
      });
    } catch (err) {
      addToast(extractErrorMessage(err, currentT()("video.resumeRunFailed")), "error");
    } finally {
      setResumingWorkflow(false);
    }
  };


  return {
    loadWorkflowStatus,
    previewNode,
    closePreview,
    runNode,
    runToNode,
    runGraph,
    onNodeContextMenu,
    onEdgeContextMenu,
    onPaneContextMenu,
    cancelActiveWorkflow,
    resumeActiveWorkflow,
  };
}
