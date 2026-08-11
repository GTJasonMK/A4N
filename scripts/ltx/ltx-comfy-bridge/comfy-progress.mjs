import { randomUUID } from "node:crypto";
import {
  formatTimingStatus,
  numberOrNull,
  optionalString,
  toErrorMessage,
} from "./common.mjs";

export function createComfyProgressMonitor({ config }) {
  function startComfyProgressMonitor(promptId, timing = {}) {
    const state = {
      promptId,
      timing,
      active: false,
      connected: false,
      closed: false,
      warned: false,
      completed: false,
      phase: "queued",
      currentNode: "",
      progressValue: null,
      progressMax: null,
      lastLogMs: 0,
      lastLogKey: "",
      error: null,
    };

    let socketHandle = null;
    try {
      socketHandle = connectComfyWebSocket(comfyWebSocketUrl(timing.clientId || randomUUID()), {
        onOpen: () => {
          state.connected = true;
        },
        onMessage: (message) => {
          handleComfyProgressMessage(state, message);
        },
        onError: (err) => {
          if (state.closed || state.warned) return;
          state.warned = true;
          console.warn(
            `${new Date().toLocaleTimeString()} progress websocket unavailable for ${promptId}: ${toErrorMessage(err)}`,
          );
        },
      });
    } catch (err) {
      console.warn(
        `${new Date().toLocaleTimeString()} progress websocket unavailable for ${promptId}: ${toErrorMessage(err)}`,
      );
    }

    return {
      snapshot: () => progressSnapshotFromState(state),
      close: () => {
        state.closed = true;
        if (socketHandle) socketHandle.close();
      },
      get error() {
        return state.error;
      },
    };
  }

  function handleComfyProgressMessage(state, message) {
    if (!message || typeof message !== "object") return;
    const type = optionalString(message.type);
    const data = message.data && typeof message.data === "object" ? message.data : {};
    const messagePromptId = optionalString(data.prompt_id ?? data.promptId);
    if (messagePromptId && messagePromptId !== state.promptId) return;
    if (!messagePromptId && !state.active && type !== "status") return;

    if (type === "execution_start") {
      state.active = true;
      state.phase = "started";
      updateProgressState(state, { forceLog: true });
      return;
    }

    if (type === "executing") {
      state.active = true;
      const node = data.node == null ? "" : String(data.node);
      const changedNode = node !== state.currentNode;
      state.currentNode = node;
      state.phase = node ? "executing" : "finalizing";
      if (changedNode) {
        state.progressValue = null;
        state.progressMax = null;
      }
      updateProgressState(state, { forceLog: changedNode || !node });
      return;
    }

    if (type === "progress") {
      state.active = true;
      if (data.node != null) state.currentNode = String(data.node);
      state.phase = "sampling";
      state.progressValue = numberOrNull(data.value);
      state.progressMax = numberOrNull(data.max);
      updateProgressState(state);
      return;
    }

    if (type === "execution_success") {
      state.active = true;
      state.completed = true;
      state.phase = "success";
      updateProgressState(state, { forceLog: true });
      return;
    }

    if (type === "execution_error" || type === "execution_interrupted") {
      state.active = true;
      state.phase = type;
      const node = data.node_id ?? data.node;
      if (node != null) state.currentNode = String(node);
      state.error = new Error(formatComfyExecutionEventError(type, data));
      updateProgressState(state, { forceLog: true });
    }
  }

  function updateProgressState(state, { forceLog = false } = {}) {
    const snapshot = progressSnapshotFromState(state);
    if (!snapshot) return;
    if (typeof state.timing.onProgress === "function") {
      state.timing.onProgress(snapshot);
    }

    const now = Date.now();
    const logKey = progressLogKey(snapshot);
    const shouldLog =
      forceLog ||
      logKey !== state.lastLogKey ||
      now - state.lastLogMs >= config.progressLogIntervalMs;
    if (!shouldLog) return;

    console.log(
      `${new Date().toLocaleTimeString()} progress ${state.promptId}${formatTimingStatus(state.timing, now)}${formatProgressSnapshot(snapshot)}`,
    );
    state.lastLogMs = now;
    state.lastLogKey = logKey;
  }

  function progressSnapshotFromState(state) {
    const now = Date.now();
    const nodeLabel = state.currentNode
      ? workflowNodeLabel(state.timing.workflow, state.currentNode)
      : "";
    const percent =
      state.progressValue != null && state.progressMax != null && state.progressMax > 0
        ? Math.max(0, Math.min(100, Math.round((state.progressValue / state.progressMax) * 100)))
        : null;
    return {
      phase: state.phase,
      node: state.currentNode || null,
      node_label: nodeLabel || null,
      value: state.progressValue,
      max: state.progressMax,
      percent,
      elapsed_ms: state.timing.startedAtMs ? now - state.timing.startedAtMs : null,
      inference_ms: state.timing.submittedAtMs ? now - state.timing.submittedAtMs : null,
    };
  }

  function comfyWebSocketUrl(clientId) {
    const url = new URL(config.comfyBaseUrl);
    url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
    const basePath = url.pathname.replace(/\/+$/, "");
    url.pathname = `${basePath}/ws`;
    url.search = "";
    url.searchParams.set("clientId", clientId);
    return url;
  }

  return {
    startComfyProgressMonitor,
    formatProgressSnapshot,
  };
}

function progressLogKey(snapshot) {
  return [
    snapshot.phase,
    snapshot.node || "",
    snapshot.value ?? "",
    snapshot.max ?? "",
    snapshot.percent ?? "",
  ].join(":");
}

function formatProgressSnapshot(snapshot) {
  if (!snapshot) return "";
  const parts = [];
  if (snapshot.phase) parts.push(`phase=${snapshot.phase}`);
  if (snapshot.node) {
    const label = compactLogToken(snapshot.node_label || "node");
    parts.push(`node=${snapshot.node}:${label}`);
  }
  if (snapshot.value != null && snapshot.max != null) {
    const percent = snapshot.percent != null ? ` ${snapshot.percent}%` : "";
    parts.push(`step=${snapshot.value}/${snapshot.max}${percent}`);
  }
  return parts.length ? ` progress=[${parts.join(" ")}]` : "";
}

function workflowNodeLabel(workflow, nodeId) {
  const node = workflow?.[String(nodeId)];
  if (!node || typeof node !== "object") return "";
  return optionalString(node._meta?.title) || optionalString(node.class_type);
}

function compactLogToken(value) {
  return String(value || "")
    .trim()
    .replace(/\s+/g, "_")
    .replace(/[^\w.+:/|()[\]-]+/g, "")
    .slice(0, 80);
}

function formatComfyExecutionEventError(type, data) {
  const node = data.node_id ?? data.node ?? "unknown";
  const nodeType = data.node_type ?? data.class_type ?? "unknown";
  const message = data.exception_message ?? data.message ?? type;
  return `ComfyUI ${type}: node ${node} (${nodeType}): ${message}`;
}

function connectComfyWebSocket(wsUrl, handlers = {}) {
  const socket = new WebSocket(wsUrl);
  socket.addEventListener("open", () => handlers.onOpen?.());
  socket.addEventListener("message", ({ data }) => {
    if (typeof data !== "string") return;
    try {
      handlers.onMessage?.(JSON.parse(data));
    } catch {
      // Binary previews and non-JSON extension messages are irrelevant for bridge progress logs.
    }
  });
  socket.addEventListener("error", (event) => {
    handlers.onError?.(event.error ?? new Error("ComfyUI websocket connection failed"));
  });
  socket.addEventListener("close", () => handlers.onClose?.());

  return {
    close: () => socket.close(),
  };
}
