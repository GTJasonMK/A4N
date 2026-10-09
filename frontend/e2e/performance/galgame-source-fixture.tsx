import React from "react";
import { createRoot } from "react-dom/client";
import { flushSync } from "react-dom";
import { DirtyRegistryProvider } from "../../src/components/navigation/dirtyRegistry";
import { ProgressTaskProvider } from "../../src/hooks/useProgressTask";
import { I18nProvider } from "../../src/i18n";
import { GalgameTab } from "../../src/pages/novel-detail/GalgameTab";
import type { SourceSnapshotChapterIndexEntry } from "../../src/api/project";
import { writeCurrentTokens } from "../../src/api/authStorage";
import "../../src/index.css";

const chapterCount = 1000;
const snapshotId = 23;
const chapterBody = `${"章节性能内容".repeat(512)}。`;
const requestLog: Array<{
  command: string;
  bytes: number;
  wire_bytes: number;
  duration_ms: number;
  complete: boolean;
  scope_chapter_id?: number | null;
}> = [];
let pendingRequests = 0;
let nextWireAvailableAt = 0;
let activeRoot: ReturnType<typeof createRoot> | null = null;
const variant = new URLSearchParams(location.search).get("variant") === "before" ? "before" : "after";

interface SerializedJson {
  body: Uint8Array;
  bytes: number;
  wireBytes: number;
}

async function serializeJson(value: unknown): Promise<SerializedJson> {
  const body = new TextEncoder().encode(JSON.stringify(value));
  let wireBytes = body.byteLength;
  if (body.byteLength > 1024 && typeof CompressionStream !== "undefined") {
    const compressed = await new Response(
      new Blob([body]).stream().pipeThrough(new CompressionStream("gzip")),
    ).arrayBuffer();
    wireBytes = compressed.byteLength;
  }
  return { body, bytes: body.byteLength, wireBytes };
}

function check(condition: unknown, message: string): asserts condition {
  if (!condition) throw new Error(message);
}

function chapterIndex(chapterNumber: number): SourceSnapshotChapterIndexEntry {
  return {
    id: chapterNumber + 1000,
    snapshot_id: snapshotId,
    source_chapter_id: chapterNumber,
    chapter_number: chapterNumber,
    title: `性能章节 ${chapterNumber}`,
  };
}

function authoringChapter(chapterNumber: number) {
  return {
    chapter_number: chapterNumber,
    title: `性能章节 ${chapterNumber}`,
    source: {
      content_sha256: `content-${chapterNumber}`,
      root_project_id: 1,
      source_chapter_id: chapterNumber,
      source_snapshot_chapter_id: chapterNumber + 1000,
      source_snapshot_id: snapshotId,
    },
    source_spans: [{ id: `ch${chapterNumber}:s1:f1`, text: chapterBody }],
  };
}

function authoringWorkspace() {
  return {
    artifacts: [],
    chapters: Array.from({ length: chapterCount }, (_, index) => authoringChapter(index + 1)),
    documents: [],
    source_snapshot_id: snapshotId,
    target: "galgame",
  };
}

const sourceIndex = Array.from({ length: chapterCount }, (_, index) => chapterIndex(index + 1));
let workspacePayload: SerializedJson | null = null;

async function preparePayloads() {
  workspacePayload = await serializeJson(authoringWorkspace());
}

function responseFor(command: string): unknown {
  switch (command) {
    case "get_galgame_pipeline_status":
    case "get_galgame_script":
    case "get_galgame_asset_plan":
      return null;
    case "get_media_authoring_workspace": {
      const response = workspacePayload;
      check(response, "Media authoring fixture was not prepared");
      return response;
    }
    case "list_profiles":
    case "list_portraits_by_project":
    case "list_images_by_project":
    case "list_image_configs":
    case "list_video_configs":
      return [];
    default:
      return [];
  }
}

window.fetch = async (input, init) => {
  pendingRequests += 1;
  try {
    const url = new URL(input instanceof Request ? input.url : String(input), location.href);
    const command = url.pathname.split("/").pop()!;
    const payload = responseFor(command);
    const serialized = payload != null && typeof payload === "object" && "body" in payload && "wireBytes" in payload
      ? payload as SerializedJson
      : await serializeJson(payload);
    const started = performance.now();
    const transferStartsAt = Math.max(started + 50, nextWireAvailableAt);
    const responseReadyAt = transferStartsAt + serialized.wireBytes / 500;
    nextWireAvailableAt = responseReadyAt;
    const requestRecord = {
      command,
      bytes: serialized.bytes,
      wire_bytes: serialized.wireBytes,
      duration_ms: 0,
      complete: false,
      ...(command === "get_media_authoring_workspace" ? { scope_chapter_id: null } : {}),
    };
    requestLog.push(requestRecord);
    await new Promise((resolve) => window.setTimeout(resolve, Math.max(0, responseReadyAt - performance.now())));
    requestRecord.duration_ms = performance.now() - started;
    requestRecord.complete = true;
    return new Response(serialized.body, { headers: { "Content-Type": "application/json" } });
  } finally {
    pendingRequests -= 1;
  }
};

async function measureGalgameSource() {
  await writeCurrentTokens({
    session_id: "galgame-source-performance",
    access_token: "synthetic-token",
    refresh_token: "synthetic-refresh",
    token_type: "Bearer",
    expires_in: 3600,
  });
  requestLog.length = 0;
  nextWireAvailableAt = performance.now();
  const started = performance.now();
  flushSync(() => {
    activeRoot = createRoot(document.getElementById("root")!);
    activeRoot.render(
      <I18nProvider initialLocale="zh-CN">
        <DirtyRegistryProvider>
          <ProgressTaskProvider scopeKey="galgame-source-performance">
            <GalgameTab
              projectId={1}
              materialProjectId={1}
              canEditGalgame={false}
              snapshotChapters={sourceIndex}
            />
          </ProgressTaskProvider>
        </DirtyRegistryProvider>
      </I18nProvider>,
    );
  });
  for (let frame = 0; frame < 3; frame += 1) {
    await new Promise<void>((resolve) => requestAnimationFrame(() => window.setTimeout(resolve, 0)));
  }
  const deadline = performance.now() + 30_000;
  const requiredCommands = [
    "get_galgame_pipeline_status",
    "get_galgame_script",
    "get_galgame_asset_plan",
    "list_image_configs",
    "list_portraits_by_project",
    "list_profiles",
    ...(variant === "before" ? ["get_media_authoring_workspace"] : []),
  ];
  while (
    (pendingRequests > 0 || requiredCommands.some((command) =>
      !requestLog.some((request) => request.command === command && request.complete)))
    && performance.now() < deadline
  ) {
    await new Promise((resolve) => window.setTimeout(resolve, 20));
  }
  check(pendingRequests === 0, "Galgame source requests did not settle");
  await new Promise((resolve) => window.setTimeout(resolve, 0));
  if (typeof window.gc === "function") window.gc();
  return {
    variant,
    chapterCount,
    initialReadyMs: performance.now() - started,
    heapUsedBytes: performance.memory?.usedJSHeapSize ?? null,
    domNodeCount: document.querySelectorAll("*").length,
    chapterRowCount: sourceIndex.length,
    documentWidth: document.documentElement.clientWidth,
    documentScrollWidth: document.documentElement.scrollWidth,
    requestedResponseBytes: requestLog.reduce((total, request) => total + request.bytes, 0),
    requestedWireBytes: requestLog.reduce((total, request) => total + request.wire_bytes, 0),
    requests: requestLog.map((request) => ({ ...request })),
  };
}

Object.assign(window, {
  measureGalgameSource,
  galgameWorkspaceRequestCount: () => requestLog.filter((request) => request.command === "get_media_authoring_workspace").length,
  galgameWorkspaceCompleteCount: () => requestLog.filter((request) => request.command === "get_media_authoring_workspace" && request.complete).length,
  performanceReady: false,
});
void preparePayloads().then(() => { window.performanceReady = true; }).catch((error) => {
  window.performanceError = String(error);
  window.performanceReady = true;
});
window.addEventListener("pagehide", () => activeRoot?.unmount(), { once: true });
