import React, { useCallback, useEffect } from "react";
import { Eye, Play, Trash2, Upload } from "lucide-react";
import { extractErrorMessage } from "../../api";
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
  canvasWorkflowRuntimeRefreshSignature,
  computeRunToNodeScope,
  parseWorkflowTimestampMs,
  runToNodeConfirmationMessage,
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

export interface CanvasWorkflowActionsOptions {
  projectId: number;
  project: { source_mode?: string } | null;
  document: CanvasGraphDocument | null;
  nodes: StudioNode[];
  edges: StudioEdge[];
  nodeContractsByKind: StudioNodeContractMap;
  selectedNode: StudioNode | null;
  selectedNodeRunToTarget: boolean;
  selectedNodeRunnable: boolean;
  canEdit: boolean;
  canvasWriteBusy: boolean;
  canvasMutationDisabled: boolean;
  dirty: boolean;
  workflowSubmitting: boolean;
  resumingWorkflow: boolean;
  latestWorkflowRunId: number | null;
  workflowDetail: WorkflowRunDetailSummary | null;
  contextMenu: { open: (x: number, y: number, items: ContextMenuEntry[]) => void };
  workflowDiscoveryTimerRef: React.MutableRefObject<number | null>;
  workflowDiscoveryInFlightRef: React.MutableRefObject<boolean>;
  workflowRuntimeRefreshSignatureRef: React.MutableRefObject<string | null>;
  t: MessageTranslator;
  addToast: (message: string, tone: "success" | "error" | "info" | "warning") => void;
  setLatestWorkflowRunId: React.Dispatch<React.SetStateAction<number | null>>;
  setWorkflowDetail: React.Dispatch<React.SetStateAction<WorkflowRunDetailSummary | null>>;
  setWorkflowEvents: React.Dispatch<React.SetStateAction<ProgressEventRecord[]>>;
  setWorkflowLoading: React.Dispatch<React.SetStateAction<boolean>>;
  setWorkflowSubmissionLabel: React.Dispatch<React.SetStateAction<string | null>>;
  setCancellingWorkflow: React.Dispatch<React.SetStateAction<boolean>>;
  setResumingWorkflow: React.Dispatch<React.SetStateAction<boolean>>;
  setPreviewDialogOpen: React.Dispatch<React.SetStateAction<boolean>>;
  setPreviewLoading: React.Dispatch<React.SetStateAction<boolean>>;
  setPreviewData: React.Dispatch<React.SetStateAction<ModelRequestPreview | null>>;
  setContentPreviewNode: React.Dispatch<React.SetStateAction<StudioNode | null>>;
  refreshCanvasRuntimeResults: (graphId: number) => Promise<void>;
  load: () => Promise<void>;
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
  selectedNode,
  selectedNodeRunToTarget,
  selectedNodeRunnable,
  canEdit,
  canvasWriteBusy,
  canvasMutationDisabled,
  dirty,
  workflowSubmitting,
  resumingWorkflow,
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
  setWorkflowLoading,
  setWorkflowSubmissionLabel,
  setCancellingWorkflow,
  setResumingWorkflow,
  setPreviewDialogOpen,
  setPreviewLoading,
  setPreviewData,
  setContentPreviewNode,
  refreshCanvasRuntimeResults,
  load,
  selectCanvasNode,
  selectCanvasEdge,
  deleteCanvasNode,
  deleteCanvasEdge,
  importMedia,
}: CanvasWorkflowActionsOptions) {
  const loadWorkflowStatus = useCallback(
    async (workflowRunId: number, options: { silent?: boolean } = {}) => {
      setLatestWorkflowRunId(workflowRunId);
      setWorkflowLoading(true);
      try {
        const [detail, events] = await Promise.all([
          getWorkflowRunDetail(workflowRunId, true),
          getWorkflowRunProgressEvents(workflowRunId, 20),
        ]);
        setWorkflowDetail(detail);
        setWorkflowEvents(events);
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
          workflowRuntimeRefreshSignatureRef.current = refreshSignature;
          try {
            await refreshCanvasRuntimeResults(graphId);
          } catch (err) {
            if (!options.silent) {
              addToast(
                extractErrorMessage(err, t("video.syncCanvasRuntimeFailed")),
                "error",
              );
            }
          }
        }
      } catch (err) {
        if (!options.silent) {
          addToast(extractErrorMessage(err, t("video.loadCanvasRunsFailed")), "error");
        }
      } finally {
        setWorkflowLoading(false);
      }
    },
    [addToast, refreshCanvasRuntimeResults, t],
  );

  const stopWorkflowDiscovery = useCallback(() => {
    if (workflowDiscoveryTimerRef.current !== null) {
      window.clearInterval(workflowDiscoveryTimerRef.current);
      workflowDiscoveryTimerRef.current = null;
    }
    workflowDiscoveryInFlightRef.current = false;
  }, []);

  const findActiveCanvasWorkflowRun = useCallback(
    async (
      graphId: number,
      submittedAtMs: number,
    ): Promise<WorkflowRunSummary | null> => {
      const runs = await listWorkflowRuns({
        project_id: projectId,
        workflow_kind: CANVAS_WORKFLOW_KIND,
        resumable_only: false,
        limit: 50,
      });
      const submittedFloorMs = submittedAtMs - 5000;
      return (
        runs.find((run) => {
          if (run.scope_table !== "canvas_graph" || run.scope_id !== graphId)
            return false;
          if (run.status === "running" || run.status === "pending") return true;
          const updatedAtMs = parseWorkflowTimestampMs(run.updated_at);
          const createdAtMs = parseWorkflowTimestampMs(run.created_at);
          return (
            (updatedAtMs !== null && updatedAtMs >= submittedFloorMs) ||
            (createdAtMs !== null && createdAtMs >= submittedFloorMs)
          );
        }) ?? null
      );
    },
    [projectId],
  );

  const startWorkflowDiscovery = useCallback(
    (graphId: number, submittedAtMs: number) => {
      stopWorkflowDiscovery();
      const tick = async () => {
        if (workflowDiscoveryInFlightRef.current) return;
        workflowDiscoveryInFlightRef.current = true;
        try {
          const run = await findActiveCanvasWorkflowRun(graphId, submittedAtMs);
          if (!run) return;
          await loadWorkflowStatus(run.id, { silent: true });
          stopWorkflowDiscovery();
        } catch (_err) {
          // Discovery is best-effort while the run submission request is still in flight.
        } finally {
          workflowDiscoveryInFlightRef.current = false;
        }
      };
      workflowDiscoveryTimerRef.current = window.setInterval(() => {
        void tick();
      }, 700);
      void tick();
    },
    [findActiveCanvasWorkflowRun, loadWorkflowStatus, stopWorkflowDiscovery],
  );

  useEffect(() => stopWorkflowDiscovery, [stopWorkflowDiscovery]);

  useEffect(() => {
    if (!latestWorkflowRunId) return;
    const status = workflowDetail?.run.status;
    if (status !== "running" && status !== "pending") return;
    const timer = window.setInterval(() => {
      void loadWorkflowStatus(latestWorkflowRunId, { silent: true });
    }, 1000);
    return () => window.clearInterval(timer);
  }, [latestWorkflowRunId, loadWorkflowStatus, workflowDetail?.run.status]);

  useEffect(() => {
    const graph = document?.graph;
    if (!graph) return;
    let cancelled = false;
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
          setLatestWorkflowRunId(null);
          setWorkflowDetail(null);
          setWorkflowEvents([]);
          return;
        }
        await loadWorkflowStatus(latest.id, { silent: true });
      } catch {
        if (cancelled) return;
        setLatestWorkflowRunId(null);
        setWorkflowDetail(null);
        setWorkflowEvents([]);
      }
    };
    void restoreLatestRun();
    return () => {
      cancelled = true;
    };
  }, [document?.graph.id, document?.graph.version, loadWorkflowStatus, projectId]);

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
    setPreviewDialogOpen(true);
    setPreviewLoading(true);
    setPreviewData(null);
    try {
      const preview = await previewCanvasNodeRequest(
        projectId,
        document.graph.id,
        nodeId,
      );
      setPreviewData(preview);
    } catch (err) {
      setPreviewDialogOpen(false);
      addToast(extractErrorMessage(err, t("video.previewRequestFailed")), "error");
    } finally {
      setPreviewLoading(false);
    }
  };

  const runNode = async (node: StudioNode) => {
    if (!document || canvasWriteBusy || !canEdit || workflowSubmitting) return;
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
    if (!confirmed) {
      return;
    }
    setWorkflowSubmissionLabel(t("video.runNodeSubmitting", { name: nodeDisplayTitle(node.data, t) }));
    setLatestWorkflowRunId(null);
    setWorkflowDetail(null);
    setWorkflowEvents([]);
    workflowRuntimeRefreshSignatureRef.current = null;
    const graphId = document.graph.id;
    startWorkflowDiscovery(graphId, Date.now());
    try {
      const result = await runCanvasNode(projectId, graphId, nodeId);
      addToast(
        result.error_message ?? t("video.runOnlyNodeSubmitted"),
        result.status === "failed" ? "info" : "success",
      );
      await loadWorkflowStatus(result.workflow_run_id);
      await load();
    } catch (err) {
      addToast(extractErrorMessage(err, t("video.runOnlyNodeError")), "error");
    } finally {
      setWorkflowSubmissionLabel(null);
      stopWorkflowDiscovery();
    }
  };

  const runToNode = async (node: StudioNode) => {
    if (!document || canvasWriteBusy || !canEdit || workflowSubmitting) return;
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
    if (!confirmed) {
      return;
    }
    setWorkflowSubmissionLabel(t("video.runToNodeSubmitting", { name: nodeDisplayTitle(node.data, t) }));
    setLatestWorkflowRunId(null);
    setWorkflowDetail(null);
    setWorkflowEvents([]);
    workflowRuntimeRefreshSignatureRef.current = null;
    const graphId = document.graph.id;
    startWorkflowDiscovery(graphId, Date.now());
    try {
      const result = await runCanvasGraphToNode(projectId, graphId, nodeId);
      addToast(
        result.error_message ?? t("video.runToNodeSubmitted"),
        result.status === "failed" ? "info" : "success",
      );
      await loadWorkflowStatus(result.workflow_run_id);
      await load();
    } catch (err) {
      addToast(extractErrorMessage(err, t("video.runToNodeError")), "error");
    } finally {
      setWorkflowSubmissionLabel(null);
      stopWorkflowDiscovery();
    }
  };

  const runGraph = async () => {
    if (
      !document ||
      canvasWriteBusy ||
      !canEdit ||
      workflowSubmitting ||
      resumingWorkflow
    )
      return;
    if (dirty) {
      addToast(t("video.saveBeforeRun"), "info");
      return;
    }
    if (canResumeCurrentCanvasRun) {
      await resumeActiveWorkflow();
      return;
    }
    setWorkflowSubmissionLabel(t("video.runCanvasSubmitting"));
    setLatestWorkflowRunId(null);
    setWorkflowDetail(null);
    setWorkflowEvents([]);
    workflowRuntimeRefreshSignatureRef.current = null;
    const graphId = document.graph.id;
    startWorkflowDiscovery(graphId, Date.now());
    try {
      const result = await runCanvasGraph(projectId, graphId);
      addToast(
        result.error_message ?? t("video.submittedRunCanvas"),
        result.status === "failed" ? "info" : "success",
      );
      await loadWorkflowStatus(result.workflow_run_id);
      await load();
    } catch (err) {
      addToast(extractErrorMessage(err, t("video.runCanvasError")), "error");
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
        disabled: !document || canvasWriteBusy || !canEdit,
        onClick: () => void importMedia("image"),
      },
      {
        label: t("video.importVideo"),
        icon: <Upload size={13} />,
        disabled: !document || canvasWriteBusy || !canEdit,
        onClick: () => void importMedia("video"),
      },
      {
        label: t("video.importAudio"),
        icon: <Upload size={13} />,
        disabled:
          !document ||
          canvasWriteBusy ||
          !canEdit ||
          project?.source_mode !== "freeform",
        onClick: () => void importMedia("audio"),
      },
    ],
    [
      canEdit,
      document,
      importMedia,
      canvasWriteBusy,
      project?.source_mode,
      t,
    ],
  );

  const paneContextMenuItems = useCallback(
    (): ContextMenuEntry[] => [
      {
        label: t("video.runCanvasTitle"),
        icon: <Play size={13} />,
        disabled:
          !document || canvasWriteBusy || !canEdit || workflowSubmitting,
        onClick: () => void runGraph(),
      },
      { divider: true },
      ...mediaImportMenuItems(),
    ],
    [
      canEdit,
      canvasWriteBusy,
      document,
      mediaImportMenuItems,
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
    setCancellingWorkflow(true);
    try {
      await cancelWorkflow({
        task_id: taskId,
        workflow_run_id: workflowRunId,
      });
      addToast(t("video.cancelRunRequested"), "info");
      if (workflowRunId) {
        await loadWorkflowStatus(workflowRunId);
      }
    } catch (err) {
      addToast(extractErrorMessage(err, t("video.cancelRunFailed")), "error");
    } finally {
      setCancellingWorkflow(false);
    }
  };

  const resumeActiveWorkflow = async () => {
    const workflowRunId = workflowDetail?.run.id ?? latestWorkflowRunId;
    if (!workflowRunId || resumingWorkflow) return;
    setResumingWorkflow(true);
    try {
      const run = await resumeWorkflowRun(workflowRunId);
      addToast(t("video.resumeRunSuccess"), "success");
      workflowRuntimeRefreshSignatureRef.current = null;
      await loadWorkflowStatus(run.id);
    } catch (err) {
      addToast(extractErrorMessage(err, t("video.resumeRunFailed")), "error");
    } finally {
      setResumingWorkflow(false);
    }
  };


  return {
    loadWorkflowStatus,
    previewNode,
    runNode,
    runToNode,
    nodeContextMenuItems,
    edgeContextMenuItems,
    mediaImportMenuItems,
    paneContextMenuItems,
    onNodeContextMenu,
    onEdgeContextMenu,
    onPaneContextMenu,
    cancelActiveWorkflow,
    resumeActiveWorkflow,
  };
}

function workflowRunGraphVersion(run: WorkflowRunSummary): number | null {
  const fingerprint = run.input_fingerprint_json;
  if (
    typeof fingerprint !== "object" ||
    fingerprint === null ||
    Array.isArray(fingerprint) ||
    typeof (fingerprint as Record<string, unknown>).graph_version !== "number"
  ) {
    return null;
  }
  return (fingerprint as Record<string, number>).graph_version;
}
