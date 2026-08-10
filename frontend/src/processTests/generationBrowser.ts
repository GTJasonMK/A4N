import {
  writeCurrentAccessToken,
  writeCurrentRefreshToken,
  writeCurrentSessionId,
  writeCurrentUserId,
} from "../api/authStorage";
import { createElement, Profiler } from "react";
import { createRoot } from "react-dom/client";
import {
  parseGenerationTaskHandle,
  parseGenerationTaskSnapshot,
  parseGenerationTaskSummary,
} from "../api/generationEvents";
import type { GenerationTaskHandle } from "../api/generated/generationContracts";
import { generationClient } from "../api/generationTasks";
import { useGenerationTask } from "../hooks/useGenerationTask";
import { getGenerationTaskViewState } from "../store/generationTasks";

const config = new URLSearchParams(location.hash.slice(1));
const backend = required("backend").replace(/\/$/, "");
const username = required("username");
const password = required("password");
const projectId = Number(required("project"));
const imageConfigId = optionalPositiveInteger("image_config");
let accessToken = "";
const streamOutput = document.createElement("pre");
streamOutput.id = "generation-stream-output";
document.body.append(streamOutput);

void run().catch((error) => publish("error", {
  message: error instanceof Error ? error.message : String(error),
}));

async function run(): Promise<void> {
  const login = await jsonRequest(`${backend}/api/auth/login`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ username, password }),
  }) as {
    user: { id: number };
    tokens: { access_token: string; refresh_token: string; session_id: string };
  };
  writeCurrentUserId(login.user.id);
  writeCurrentSessionId(login.tokens.session_id);
  writeCurrentAccessToken(login.tokens.access_token);
  writeCurrentRefreshToken(login.tokens.refresh_token);
  accessToken = login.tokens.access_token;

  const first = await runInspiration("browser reconnect process");
  const { handle, task } = first;
  document.documentElement.dataset.taskId = handle.task_id;
  const uiFirstFrameUnixMs = [first.uiFirstFrameUnixMs];
  for (let index = 1; index < 5; index += 1) {
    uiFirstFrameUnixMs.push(
      (await runInspiration(`browser UI latency sample ${index + 1}`)).uiFirstFrameUnixMs,
    );
  }
  const capacity = await runReactCapacity();
  const summary = await summaryFor(handle.task_id, handle.attempt);
  const snapshot = await snapshotFor(handle.task_id, handle.attempt);
  publish("result", {
    client_state: task.clientState,
    recovery_state: task.recoveryState,
    status: task.taskStatus,
    terminal: task.terminalReceived,
    sequence: task.nextSequence - 1,
    text_bytes: new TextEncoder().encode(task.streams.raw_json?.text ?? "").byteLength,
    usage_total: task.usage?.total_tokens ?? null,
    checkpoint_covers: snapshot.covers_through_sequence,
    checkpoint_status: snapshot.status,
    summary_status: summary.handle.status,
    summary_result_revision: summary.result_ref?.revision ?? null,
    summary_terminal_sequence: summary.terminal_event?.sequence ?? null,
    ui_first_frame_unix_ms: uiFirstFrameUnixMs,
    capacity,
    combination: imageConfigId == null ? null : await runEventCombination(imageConfigId),
  });
}

async function runReactCapacity(): Promise<Record<string, unknown>> {
  const handle = await startInspiration("capacity-10000-chunks");
  const container = document.createElement("div");
  document.body.append(container);
  const root = createRoot(container);
  let reactCommits = 0;
  root.render(createElement(
    Profiler,
    { id: "generation-capacity", onRender: () => { reactCommits += 1; } },
    createElement(GenerationStreamProbe, { handle }),
  ));
  const task = await generationClient.waitForTerminal(handle);
  await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
  const text = task.streams.raw_json?.text ?? "";
  const domText = container.textContent ?? "";
  root.unmount();
  container.remove();
  return {
    status: task.taskStatus,
    terminal: task.terminalReceived,
    published_events: task.nextSequence - 1,
    react_commits: reactCommits,
    text_bytes: new TextEncoder().encode(text).byteLength,
    dom_text_bytes: new TextEncoder().encode(domText).byteLength,
    text_sha256: await sha256(text),
  };
}

function GenerationStreamProbe({ handle }: { handle: GenerationTaskHandle }) {
  const task = useGenerationTask({ handle });
  return createElement("pre", null, task.text("raw_json"));
}

async function runInspiration(message: string) {
  const handle = await startInspiration(message);
  let resolveFirstFrame: (value: number) => void = () => {};
  const firstFrame = new Promise<number>((resolve) => {
    resolveFirstFrame = resolve;
  });
  let firstFrameScheduled = false;
  const task = await generationClient.waitForTerminal(handle, (current) => {
    if (!current) return;
    document.documentElement.dataset.sequence = String(current.nextSequence - 1);
    document.documentElement.dataset.terminal = String(current.terminalReceived);
    const text = current.streams.raw_json?.text ?? "";
    if (!firstFrameScheduled && text) {
      firstFrameScheduled = true;
      streamOutput.textContent = text;
      requestAnimationFrame(() => resolveFirstFrame(Date.now()));
    }
  });
  return { handle, task, uiFirstFrameUnixMs: await firstFrame };
}

async function runEventCombination(configId: number): Promise<Record<string, unknown>> {
  const imageHandle = parseGenerationTaskHandle(await authorizedJson(
    "/api/generation-tasks/image",
    {
      method: "POST",
      body: JSON.stringify({
        meta: { start_request_id: crypto.randomUUID(), retry_of_task_id: null },
        project_id: projectId,
        operation: {
          kind: "scene",
          prompt: "browser process image artifact",
          config_id: configId,
          chapter_id: null,
        },
      }),
    },
  ));
  const image = await generationClient.waitForTerminal(imageHandle);
  const imageSummary = await summaryFor(imageHandle.task_id, imageHandle.attempt);
  const imageSnapshot = await snapshotFor(imageHandle.task_id, imageHandle.attempt);
  const artifact = image.artifacts[0] ?? null;

  const cancellation = await runCancellationSamples(20);
  const { handle: cancelHandle, task: cancelled } = cancellation.last;
  const cancelSummary = await summaryFor(cancelHandle.task_id, cancelHandle.attempt);
  const cancelSnapshot = await snapshotFor(cancelHandle.task_id, cancelHandle.attempt);

  return {
    image_status: image.taskStatus,
    image_terminal: image.terminalReceived,
    image_sequence: image.nextSequence - 1,
    image_client_state: image.clientState,
    image_artifact_count: image.artifacts.length,
    image_artifact_kind: artifact?.artifact_kind ?? null,
    image_artifact_revision: artifact?.revision ?? null,
    image_terminal_result_revision: image.resultRef?.revision ?? null,
    image_result_revision: imageSummary.result_ref?.revision ?? null,
    image_checkpoint_covers: imageSnapshot.covers_through_sequence,
    image_summary_terminal_sequence: imageSummary.terminal_event?.sequence ?? null,
    cancel_status: cancelled.taskStatus,
    cancel_terminal: cancelled.terminalReceived,
    cancel_sequence: cancelled.nextSequence - 1,
    cancel_client_state: cancelled.clientState,
    cancel_summary_status: cancelSummary.handle.status,
    cancel_checkpoint_covers: cancelSnapshot.covers_through_sequence,
    cancel_summary_terminal_sequence: cancelSummary.terminal_event?.sequence ?? null,
    cancel_latency_ms: cancellation.latencyMs,
  };
}

async function runCancellationSamples(count: number) {
  const latencyMs: number[] = [];
  let last: Awaited<ReturnType<typeof runCancellation>> | null = null;
  for (let index = 0; index < count; index += 1) {
    last = await runCancellation(index + 1);
    latencyMs.push(last.latencyMs);
  }
  if (!last) throw new Error("cancellation sample count must be positive");
  return { last, latencyMs };
}

async function runCancellation(index: number) {
  const handle = await startInspiration(`browser cancellation latency sample ${index}`);
  let cancelStartedAt = 0;
  let cancelRequest: Promise<unknown> | undefined;
  const cancelTerminal = generationClient.waitForTerminal(handle, (current) => {
    if (cancelRequest || !(current?.streams.raw_json?.text ?? "")) return;
    cancelStartedAt = performance.now();
    cancelRequest = authorizedJson(
      `/api/generation-tasks/${encodeURIComponent(handle.task_id)}/cancel`,
      {
        method: "POST",
        body: JSON.stringify({ task_id: handle.task_id, attempt: handle.attempt }),
      },
    ).then(parseGenerationTaskHandle);
  });
  let task;
  try {
    task = await cancelTerminal;
  } catch (error) {
    task = getGenerationTaskViewState(handle.task_id, handle.attempt);
    if (!task?.terminalReceived || task.taskStatus !== "cancelled") throw error;
  }
  const request = cancelRequest;
  if (!request) throw new Error("cancel request was not sent after the first delta");
  await request;
  if (!task.terminalReceived || task.taskStatus !== "cancelled") {
    throw new Error(`cancel task ended as ${task.taskStatus}`);
  }
  return {
    handle,
    task,
    latencyMs: Math.ceil(performance.now() - cancelStartedAt),
  };
}

function startInspiration(message: string) {
  return authorizedJson("/api/generation-tasks/inspiration", {
    method: "POST",
    body: JSON.stringify({
      meta: { start_request_id: crypto.randomUUID(), retry_of_task_id: null },
      project_id: projectId,
      message,
      variables: {},
    }),
  }).then(parseGenerationTaskHandle);
}

function summaryFor(taskId: string, attempt: number) {
  return authorizedJson(
    `/api/generation-tasks/${encodeURIComponent(taskId)}?attempt=${attempt}`,
  ).then(parseGenerationTaskSummary);
}

function snapshotFor(taskId: string, attempt: number) {
  return authorizedJson(
    `/api/generation-tasks/${encodeURIComponent(taskId)}/snapshot?attempt=${attempt}`,
  ).then(parseGenerationTaskSnapshot);
}

function authorizedJson(path: string, init: RequestInit = {}): Promise<unknown> {
  return jsonRequest(`${backend}${path}`, {
    ...init,
    headers: {
      Authorization: `Bearer ${accessToken}`,
      "Content-Type": "application/json",
      ...init.headers,
    },
  });
}

async function jsonRequest(url: string, init?: RequestInit): Promise<unknown> {
  const response = await fetch(url, init);
  if (response.ok) return response.json();
  throw new Error(`${response.status} ${await response.text()}`);
}

function required(key: string): string {
  const value = config.get(key)?.trim();
  if (!value) throw new Error(`missing ${key}`);
  return value;
}

function optionalPositiveInteger(key: string): number | null {
  const value = config.get(key)?.trim();
  if (!value) return null;
  const parsed = Number(value);
  if (!Number.isSafeInteger(parsed) || parsed <= 0) throw new Error(`invalid ${key}`);
  return parsed;
}

async function sha256(value: string): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(value));
  return [...new Uint8Array(digest)]
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join("");
}

function publish(name: "result" | "error", value: unknown): void {
  const bytes = new TextEncoder().encode(JSON.stringify(value));
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  document.documentElement.dataset[`a4n${name[0].toUpperCase()}${name.slice(1)}`] = btoa(binary);
}
