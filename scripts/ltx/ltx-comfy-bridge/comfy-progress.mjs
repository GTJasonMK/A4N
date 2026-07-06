import { Buffer } from "node:buffer";
import { randomBytes, randomUUID } from "node:crypto";
import http from "node:http";
import https from "node:https";
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
  const secure = wsUrl.protocol === "wss:";
  if (!secure && wsUrl.protocol !== "ws:") {
    throw new Error(`unsupported websocket protocol: ${wsUrl.protocol}`);
  }

  const requestModule = secure ? https : http;
  const key = randomBytes(16).toString("base64");
  let socket = null;
  let closed = false;

  const request = requestModule.request({
    protocol: secure ? "https:" : "http:",
    hostname: wsUrl.hostname,
    port: wsUrl.port || (secure ? 443 : 80),
    path: `${wsUrl.pathname}${wsUrl.search}`,
    method: "GET",
    headers: {
      Connection: "Upgrade",
      Upgrade: "websocket",
      "Sec-WebSocket-Key": key,
      "Sec-WebSocket-Version": "13",
    },
  });

  request.on("upgrade", (response, upgradedSocket, head) => {
    if (response.statusCode !== 101) {
      handlers.onError?.(new Error(`ComfyUI websocket upgrade failed: HTTP ${response.statusCode}`));
      upgradedSocket.destroy();
      return;
    }
    socket = upgradedSocket;
    const parser = createWebSocketFrameParser({
      onMessage: handlers.onMessage,
      onPing: (payload) => {
        if (socket && !socket.destroyed) sendWebSocketFrame(socket, 0x0a, payload);
      },
      onClose: handlers.onClose,
      onError: handlers.onError,
    });
    socket.on("data", parser.push);
    socket.on("error", (err) => {
      if (!closed) handlers.onError?.(err);
    });
    socket.on("close", () => {
      closed = true;
      handlers.onClose?.();
    });
    if (head?.length) parser.push(head);
    handlers.onOpen?.();
  });

  request.on("response", (response) => {
    handlers.onError?.(new Error(`ComfyUI websocket endpoint returned HTTP ${response.statusCode}`));
    response.resume();
  });
  request.on("error", (err) => {
    if (!closed) handlers.onError?.(err);
  });
  request.end();

  return {
    close: () => {
      closed = true;
      if (socket && !socket.destroyed) {
        sendWebSocketFrame(socket, 0x08, Buffer.alloc(0));
        socket.end();
        return;
      }
      request.destroy();
    },
  };
}

function createWebSocketFrameParser(handlers = {}) {
  let buffer = Buffer.alloc(0);
  let textFragments = [];

  const emitText = (payload) => {
    const text = payload.toString("utf8");
    try {
      handlers.onMessage?.(JSON.parse(text));
    } catch {
      // Binary previews and non-JSON extension messages are irrelevant for bridge progress logs.
    }
  };

  const push = (chunk) => {
    buffer = buffer.length ? Buffer.concat([buffer, chunk]) : chunk;
    while (buffer.length >= 2) {
      const first = buffer[0];
      const second = buffer[1];
      const fin = Boolean(first & 0x80);
      const opcode = first & 0x0f;
      const masked = Boolean(second & 0x80);
      let length = second & 0x7f;
      let offset = 2;

      if (length === 126) {
        if (buffer.length < offset + 2) return;
        length = buffer.readUInt16BE(offset);
        offset += 2;
      } else if (length === 127) {
        if (buffer.length < offset + 8) return;
        const bigLength = buffer.readBigUInt64BE(offset);
        if (bigLength > BigInt(Number.MAX_SAFE_INTEGER)) {
          handlers.onError?.(new Error("ComfyUI websocket frame is too large"));
          return;
        }
        length = Number(bigLength);
        offset += 8;
      }

      const maskOffset = offset;
      if (masked) offset += 4;
      if (buffer.length < offset + length) return;

      let payload = buffer.subarray(offset, offset + length);
      if (masked) {
        const mask = buffer.subarray(maskOffset, maskOffset + 4);
        payload = Buffer.from(payload);
        for (let i = 0; i < payload.length; i += 1) {
          payload[i] ^= mask[i % 4];
        }
      }
      buffer = buffer.subarray(offset + length);

      if (opcode === 0x01) {
        if (fin) {
          emitText(payload);
        } else {
          textFragments = [payload];
        }
      } else if (opcode === 0x00 && textFragments.length > 0) {
        textFragments.push(payload);
        if (fin) {
          emitText(Buffer.concat(textFragments));
          textFragments = [];
        }
      } else if (opcode === 0x08) {
        handlers.onClose?.();
        return;
      } else if (opcode === 0x09) {
        handlers.onPing?.(payload);
      }
    }
  };

  return { push };
}

function sendWebSocketFrame(socket, opcode, payload) {
  const bytes = Buffer.isBuffer(payload) ? payload : Buffer.from(payload || "");
  const mask = randomBytes(4);
  let header;
  if (bytes.length < 126) {
    header = Buffer.alloc(2);
    header[1] = 0x80 | bytes.length;
  } else if (bytes.length <= 0xffff) {
    header = Buffer.alloc(4);
    header[1] = 0x80 | 126;
    header.writeUInt16BE(bytes.length, 2);
  } else {
    header = Buffer.alloc(10);
    header[1] = 0x80 | 127;
    header.writeBigUInt64BE(BigInt(bytes.length), 2);
  }
  header[0] = 0x80 | opcode;
  const masked = Buffer.from(bytes);
  for (let i = 0; i < masked.length; i += 1) {
    masked[i] ^= mask[i % 4];
  }
  socket.write(Buffer.concat([header, mask, masked]));
}
