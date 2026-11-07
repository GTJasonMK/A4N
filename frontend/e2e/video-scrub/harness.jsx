import React from "react";
import { createRoot } from "react-dom/client";
import { I18nProvider } from "../../src/i18n";
import { ImageLightboxProvider } from "../../src/components/ui/ImageLightbox";
import { VideoPostProcessWorkspace } from "../../src/pages/novel-detail/video/VideoPostProcessWorkspace";
import { VideoAssemblyWorkspace } from "../../src/pages/novel-detail/video/VideoAssemblyWorkspace";
import { writeFixtureAuth } from "../fixtureAuth";
import { initializePortableStorage } from "../../src/utils/portableStorage";
import { installTauriEventMock } from "../tauriEventMock.js";
import "../../src/index.css";

window.__A4N_VIDEO_SCRUB_E2E_ERROR__ = null;
window.addEventListener("error", (event) => {
  window.__A4N_VIDEO_SCRUB_E2E_ERROR__ = event.error?.stack ?? event.message;
});
window.addEventListener("unhandledrejection", (event) => {
  window.__A4N_VIDEO_SCRUB_E2E_ERROR__ =
    event.reason?.stack ?? String(event.reason);
});

const bytesToBase64 = (bytes) => {
  let binary = "";
  for (let offset = 0; offset < bytes.length; offset += 0x8000) {
    binary += String.fromCharCode(...bytes.subarray(offset, offset + 0x8000));
  }
  return btoa(binary);
};

const mountHarness = async () => {
  const nativeTauriRuntime = Boolean(window.__TAURI_INTERNALS__?.metadata);
  const assemblyScenario = new URLSearchParams(window.location.search).get("scenario") === "assembly";
  const sourceFallbackScenario =
    new URLSearchParams(window.location.search).get("scenario") ===
    "source-fallback";
  let fixtureBytes = new Uint8Array();
  let fixtureBase64 = "";
  let playbackBase64 = "";
  let scrubFramesBase64 = "";
  if (!nativeTauriRuntime) {
    const [fixtureResponse, playbackResponse, scrubFramesResponse] = await Promise.all([
      fetch("./scrub-fixture.mp4"),
      fetch("./playback-fixture.mp4"),
      fetch("./scrub-fixture.mjpg"),
    ]);
    if (!fixtureResponse.ok || !playbackResponse.ok || !scrubFramesResponse.ok) {
      throw new Error(
        `无法加载视频测试文件：${fixtureResponse.status}/${playbackResponse.status}/${scrubFramesResponse.status}`,
      );
    }
    fixtureBytes = new Uint8Array(await fixtureResponse.arrayBuffer());
    fixtureBase64 = bytesToBase64(fixtureBytes);
    playbackBase64 = bytesToBase64(new Uint8Array(await playbackResponse.arrayBuffer()));
    const scrubFrameBytes = new Uint8Array(
      await scrubFramesResponse.arrayBuffer(),
    );
    scrubFramesBase64 = bytesToBase64(scrubFrameBytes);
  }

  const video = {
    id: 9001,
    project_id: 1,
    chapter_id: 2,
    video_type: assemblyScenario ? "assembly_insert" : "assembly_push",
    workflow_mode: "mixed",
    file_path: "/tmp/a4n-video-scrub-e2e.mp4",
    thumbnail_path: null,
    prompt: "浏览器剪辑预览测试",
    negative_prompt: null,
    model: "browser-test",
    width: 640,
    height: 360,
    duration_secs: 12,
    fps: 24,
    file_size: fixtureBytes.length,
    source_video_id: null,
    source_image_id: null,
    status: "completed",
    provider_job_id: null,
    error_message: null,
    metadata_json: {
      ...(assemblyScenario ? { assembly_insert: { sort_order: 1000 } } : {}),
      assembly_push: {
        push_index: 1,
        status: "pending",
        source_video_ids: [8001],
      },
    },
    is_favorite: false,
    created_at: "2026-08-04T00:00:00Z",
    updated_at: "2026-08-04T00:00:00Z",
  };
  const videos =
    new URLSearchParams(window.location.search).get("scenario") ===
    "multiple-pushes"
      ? [
          video,
          {
            ...video,
            id: 9002,
            metadata_json: {
              assembly_push: {
                ...video.metadata_json.assembly_push,
                push_index: 2,
              },
            },
            created_at: "2026-08-04T01:00:00Z",
            updated_at: "2026-08-04T01:00:00Z",
          },
        ]
      : [video];

  if (!nativeTauriRuntime) {
    let nextCallbackId = 1;
    window.__TAURI_INTERNALS__ = {
      invoke: async (command, args) => {
        if (command === "invoke_host_command") {
          if (args?.userId !== 1) throw new Error("unexpected fixture user");
          command = args.command;
          args = args.args;
        }
        switch (command) {
          case "get_all_frontend_state":
            return { "a4n:current_user_id": "1" };
          case "set_frontend_state_many":
          case "remove_frontend_state_many":
            return null;
          case "load_cached_assembled_chapter_video_preview":
          case "assemble_chapter_videos":
            if (!assemblyScenario) throw new Error("unexpected assembly fixture request");
            return {
              mime_type: "video/mp4",
              data_base64: playbackBase64,
              file_path: video.file_path,
              video_ids: [video.id],
              clips: [{
                video_id: video.id,
                video_type: video.video_type,
                shot_number: 1,
                sort_order: 1000,
                insert_after_video_id: null,
                duration_secs: 12,
                file_path: video.file_path,
              }],
              duration_secs: 12,
              file_size: fixtureBytes.length,
              updated_at: video.updated_at,
              cached: true,
            };
          case "load_video_scrub_playback_data":
            if (typeof args?.includeAudio !== "boolean") {
              throw new Error("includeAudio must be explicit");
            }
            if (sourceFallbackScenario) {
              throw new Error("scrub proxy fixture failure");
            }
            return {
              mime_type: "video/mp4",
              data_base64: args.includeAudio ? playbackBase64 : fixtureBase64,
              scrub_frames: args.includeAudio ? null : {
                mime_type: "image/jpeg",
                data_base64: scrubFramesBase64,
                frames_per_second: 12,
              },
            };
          case "load_video_playback_data":
            if (!sourceFallbackScenario) {
              throw new Error("unexpected source playback request");
            }
            return {
              mime_type: "video/mp4",
              data_base64: playbackBase64,
              scrub_frames: null,
            };
          default:
            throw new Error(`unexpected command: ${command}`);
        }
      },
      convertFileSrc: (path) => path,
      transformCallback: () => nextCallbackId++,
      unregisterCallback: () => undefined,
    };
    installTauriEventMock(window.__TAURI_INTERNALS__);
  }

  await initializePortableStorage();
  if (nativeTauriRuntime) await writeFixtureAuth("video-scrub-fixture");
  const Workspace = assemblyScenario ? VideoAssemblyWorkspace : VideoPostProcessWorkspace;

  createRoot(document.getElementById("root")).render(
    <div className="flex min-h-0 flex-col" style={{ height: "100vh", padding: 16 }}>
      <I18nProvider initialLocale="zh-CN">
        <ImageLightboxProvider>
          <Workspace
            projectId={1}
            chapter={{
              id: 2,
              chapter_number: 1,
              title: "剪辑预览测试",
              status: "completed",
            }}
            canEdit={true}
            readUnavailable={false}
            readUnavailableReason={null}
            videos={videos}
            timeline={null}
            adoptedVideoIds={new Set()}
            onToast={() => undefined}
            onRefresh={() => undefined}
            onVideoRecordUpdated={() => undefined}
          />
        </ImageLightboxProvider>
      </I18nProvider>
    </div>,
  );
};

void mountHarness().catch((error) => {
  window.__A4N_VIDEO_SCRUB_E2E_ERROR__ = error?.stack ?? String(error);
});
