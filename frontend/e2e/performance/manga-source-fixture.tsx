import React from "react";
import { createRoot } from "react-dom/client";
import { flushSync } from "react-dom";
import { DirtyRegistryProvider } from "../../src/components/navigation/dirtyRegistry";
import { ProgressTaskProvider } from "../../src/hooks/useProgressTask";
import { I18nProvider, translateMessage } from "../../src/i18n";
import { getProject, getSourceSnapshotChapter, listSourceSnapshotChapterIndex, listSourceSnapshotChapters } from "../../src/api/project";
import type { SourceSnapshotChapterIndexEntry, SourceSnapshotChapterSummary } from "../../src/api/project";
import { writeCurrentTokens } from "../../src/api/authStorage";
import { MangaTab } from "../../src/pages/novel-detail/MangaTab";
import "../../src/index.css";

const chapterCount = 1000;
const snapshotId = 23;
const chapterBody = `${"章节性能内容".repeat(512)}。`;
const chapterContent = (chapterNumber: number) => `${chapterBody}${"章".repeat(chapterNumber)}`;
const analysis = {
  key_events: Array.from({ length: 5 }, (_, index) => ({
    description: `事件${index}：${"灯火照亮河岸，林舟追寻来客留下的足迹。".repeat(6)}`,
    type: "discovery",
    importance: "medium",
    involved_characters: ["林舟"],
  })),
};
const variant = new URLSearchParams(location.search).get("variant") === "before" ? "before" : "after";
const requestLog: Array<{
  command: string;
  bytes: number;
  wire_bytes: number;
  duration_ms: number;
  complete: boolean;
  scope_chapter_id?: number | null;
}> = [];
const detailChapterIds: number[] = [];
const unexpectedRequests: string[] = [];
let pendingRequests = 0;
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

let activeRoot: ReturnType<typeof createRoot> | null = null;
let nextWireAvailableAt = 0;

function check(condition: unknown, message: string): asserts condition {
  if (!condition) throw new Error(message);
}

function fullChapter(chapterNumber: number): SourceSnapshotChapterSummary {
  return {
    id: chapterNumber + 1000,
    snapshot_id: snapshotId,
    root_project_id: 1,
    source_chapter_id: chapterNumber,
    chapter_number: chapterNumber,
    title: `性能章节 ${chapterNumber}`,
    summary: "章节性能摘要。".repeat(32),
    content: chapterContent(chapterNumber),
    analysis_data_json: analysis,
    content_hash: `content-${chapterNumber}`,
    analysis_hash: `analysis-${chapterNumber}`,
  };
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
  const source = fullChapter(chapterNumber);
  return {
    chapter_number: chapterNumber,
    title: source.title,
    source: {
      content_sha256: source.content_hash,
      root_project_id: source.root_project_id,
      source_chapter_id: source.source_chapter_id,
      source_snapshot_chapter_id: source.id,
      source_snapshot_id: source.snapshot_id,
    },
    source_spans: [
      { id: `ch${chapterNumber}:s1:f1`, text: chapterBody.slice(0, -1) },
      { id: `ch${chapterNumber}:s2:f1`, text: "章".repeat(chapterNumber) },
    ],
  };
}

function authoringWorkspace(chapterNumber?: number) {
  const chapterNumbers = chapterNumber == null
    ? Array.from({ length: chapterCount }, (_, index) => index + 1)
    : [chapterNumber];
  return {
    artifacts: [],
    chapters: chapterNumbers.map(authoringChapter),
    documents: [],
    source_snapshot_id: snapshotId,
    target: "manga",
  };
}

let fullListResponse: SerializedJson | null = null;
let indexResponse: SerializedJson | null = null;
let allChapterWorkspaceResponse: SerializedJson | null = null;
let selectedChapterWorkspaceResponse: SerializedJson | null = null;

async function preparePayloads() {
  if (variant === "before") {
    fullListResponse = await serializeJson(
      Array.from({ length: chapterCount }, (_, index) => fullChapter(index + 1)),
    );
    allChapterWorkspaceResponse = await serializeJson(authoringWorkspace());
  } else {
    indexResponse = await serializeJson(
      Array.from({ length: chapterCount }, (_, index) => chapterIndex(index + 1)),
    );
  }
  selectedChapterWorkspaceResponse = await serializeJson(authoringWorkspace(1));
}

function responseFor(command: string, payload: Record<string, unknown> | undefined): unknown {
  switch (command) {
    case "get_project":
      return {
        id: 1,
        title: "性能测试漫画项目",
        project_kind: "comic",
        source_mode: "derived",
        root_project_id: 1,
      };
    case "list_source_snapshot_chapters": {
      const response = fullListResponse;
      check(response, "Full chapter fixture was already consumed");
      fullListResponse = null;
      return response;
    }
    case "list_source_snapshot_chapter_index": {
      const response = indexResponse;
      check(response, "Chapter index fixture was already consumed");
      indexResponse = null;
      return response;
    }
    case "get_source_snapshot_chapter": {
      const input = payload?.input as Record<string, unknown> | undefined;
      check(Number(input?.snapshot_id) === snapshotId, "Selected chapter must match the loaded snapshot");
      return fullChapter(Number(input?.source_chapter_id));
    }
    case "list_manga_pipeline_statuses":
    case "list_manga_page_prompts":
    case "list_manga_pipeline_artifacts":
    case "list_profiles":
    case "list_portraits_by_project":
    case "list_images_by_project":
    case "list_image_configs":
    case "get_manga_pipeline_status":
      return [];
    case "get_media_authoring_workspace": {
      const input = payload?.input as Record<string, unknown> | undefined;
      const chapterId = Number(input?.chapter_id);
      if (!Number.isSafeInteger(chapterId) || chapterId <= 0) {
        const response = allChapterWorkspaceResponse;
        check(response, "Unscoped media authoring fixture was already consumed");
        allChapterWorkspaceResponse = null;
        return response;
      }
      if (chapterId === 1) {
        const response = selectedChapterWorkspaceResponse;
        check(response, "Selected media authoring fixture is unavailable");
        selectedChapterWorkspaceResponse = null;
        return response;
      }
      return authoringWorkspace(chapterId);
    }
    default:
      unexpectedRequests.push(command);
      throw new Error(`Unexpected manga fixture request: ${command}`);
  }
}

window.fetch = async (input, init) => {
  pendingRequests += 1;
  try {
    const url = new URL(input instanceof Request ? input.url : String(input), location.href);
    const command = url.pathname.split("/").pop()!;
    const payload = init?.body ? JSON.parse(String(init.body)) as Record<string, unknown> : undefined;
    const requestInput = payload?.input as Record<string, unknown> | undefined;
    const chapterId = requestInput?.chapter_id;
    const started = performance.now();
    const value = responseFor(command, payload);
    const serialized = value != null && typeof value === "object" && "body" in value && "wireBytes" in value
      ? value as SerializedJson
      : await serializeJson(value);
    const body = serialized.body;
    const bytes = serialized.bytes;
    const transferStartsAt = Math.max(performance.now() + 50, nextWireAvailableAt);
    const responseReadyAt = transferStartsAt + serialized.wireBytes / 500;
    nextWireAvailableAt = responseReadyAt;
    const requestRecord = {
      command,
      bytes,
      wire_bytes: serialized.wireBytes,
      duration_ms: 0,
      complete: false,
      ...(command === "get_media_authoring_workspace" ? { scope_chapter_id: typeof chapterId === "number" ? chapterId : null } : {}),
    };
    requestLog.push(requestRecord);
    await new Promise((resolve) => window.setTimeout(resolve, Math.max(0, responseReadyAt - performance.now())));
    if (command === "get_source_snapshot_chapter") {
      detailChapterIds.push(Number(requestInput?.source_chapter_id));
    }
    requestRecord.duration_ms = performance.now() - started;
    requestRecord.complete = true;
    return new Response(body, { headers: { "Content-Type": "application/json" } });
  } finally {
    pendingRequests -= 1;
  }
};

const rootElement = document.getElementById("root")!;
const bodyMeta = (chapterNumber: number) => translateMessage("zh-CN", "comic.chapter.bodyMeta", {
  count: chapterContent(chapterNumber).length,
});

async function measureMangaSource() {
  await writeCurrentTokens({
    session_id: "manga-source-performance",
    access_token: "synthetic-token",
    refresh_token: "synthetic-refresh",
    token_type: "Bearer",
    expires_in: 3600,
  });
  localStorage.setItem("a4n:comic:1:stage", "page_planning");
  requestLog.length = 0;
  nextWireAvailableAt = performance.now();
  const started = performance.now();
  const project = await getProject(1);
  check(project.id === 1 && project.source_mode === "derived", "Derived project must load");
  const chapters = variant === "before"
    ? await listSourceSnapshotChapters(1) as unknown as SourceSnapshotChapterIndexEntry[]
    : await listSourceSnapshotChapterIndex(1);
  const listLoadedAt = performance.now();
  flushSync(() => {
    activeRoot = createRoot(rootElement);
    activeRoot.render(
      <I18nProvider initialLocale="zh-CN">
        <DirtyRegistryProvider>
          <ProgressTaskProvider scopeKey="manga-source-performance">
            <MangaTab
              projectId={1}
              canEditManga
              snapshotChapters={chapters}
              assetProjectId={1}
              imageProjectIds={[1]}
            />
          </ProgressTaskProvider>
        </DirtyRegistryProvider>
      </I18nProvider>,
    );
  });
  await new Promise<void>((resolve) => requestAnimationFrame(() => requestAnimationFrame(() => resolve())));
  const initialSelectedChapterReady = selectedChapterReady(1);
  if (variant === "after") {
    const firstChapterButton = document.querySelector<HTMLButtonElement>(
      ".a4n-media-sidebar-surface button",
    );
    check(firstChapterButton, "First chapter button must render before detail selection");
    firstChapterButton.focus();
    firstChapterButton.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
    const deadline = performance.now() + 30_000;
    while (!selectedChapterReady(1) && performance.now() < deadline) {
      await new Promise((resolve) => window.setTimeout(resolve, 20));
    }
  }
  check(selectedChapterReady(1), "Selected chapter detail must become visible");
  const selectedChapterReadyMs = performance.now() - started;
  await new Promise((resolve) => window.setTimeout(resolve, 0));
  if (typeof window.gc === "function") window.gc();
  return {
    variant,
    chapterCount,
    bodyBytesPerChapter: new TextEncoder().encode(chapterContent(1)).byteLength,
    projectAndIndexReadyMs: listLoadedAt - started,
    selectedChapterReady: document.body.textContent?.includes(bodyMeta(1)) === true,
    initialSelectedChapterReady,
    selectedChapterReadyMs,
    heapUsedBytes: performance.memory?.usedJSHeapSize ?? null,
    domNodeCount: document.querySelectorAll("*").length,
    chapterRowCount: document.querySelectorAll(".a4n-media-sidebar-surface button").length,
    documentWidth: document.documentElement.clientWidth,
    documentScrollWidth: document.documentElement.scrollWidth,
    pendingRequestsAtReady: pendingRequests,
    requests: requestLog.map((request) => ({ ...request })),
    requestedResponseBytes: requestLog.reduce((total, request) => total + request.bytes, 0),
    requestedWireBytes: requestLog.reduce((total, request) => total + request.wire_bytes, 0),
  };
}

function selectedChapterLabel(): string | null {
  return document.querySelector<HTMLButtonElement>(
    ".a4n-media-sidebar-surface button[aria-current='true']",
  )?.textContent?.trim() ?? null;
}

function selectedChapterReady(chapterNumber: number): boolean {
  return document.body.textContent?.includes(bodyMeta(chapterNumber)) === true;
}

Object.assign(window, {
  measureMangaSource,
  mangaPerformanceReady: () => selectedChapterReady(1),
  mangaSelectedChapterLabel: selectedChapterLabel,
  mangaSelectedChapterReady: selectedChapterReady,
  mangaDetailLoaded: (chapterId: number) => detailChapterIds.includes(chapterId),
  mangaUnexpectedRequests: () => [...unexpectedRequests],
  performanceReady: false,
});

void preparePayloads().then(() => {
  window.performanceReady = true;
}).catch((error) => {
  window.performanceError = String(error);
  window.performanceReady = true;
});

window.addEventListener("pagehide", () => activeRoot?.unmount(), { once: true });
