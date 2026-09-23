import React from "react";
import { createRoot } from "react-dom/client";
import { I18nProvider } from "../../src/i18n";
import { VideoPostProcessWorkspace } from "../../src/pages/novel-detail/video/VideoPostProcessWorkspace";
import { writeCurrentUserId } from "../../src/api/authStorage";
import { initializePortableStorage } from "../../src/utils/portableStorage";
import "../../src/index.css";

window.__A4N_VIDEO_SCRUB_E2E_ERROR__ = null;
window.addEventListener("error", (event) => {
  window.__A4N_VIDEO_SCRUB_E2E_ERROR__ = event.error?.stack ?? event.message;
});
window.addEventListener("unhandledrejection", (event) => {
  window.__A4N_VIDEO_SCRUB_E2E_ERROR__ =
    event.reason?.stack ?? String(event.reason);
});

const mountHarness = async () => {
  const nativeTauriRuntime = Boolean(window.__TAURI_INTERNALS__?.metadata);
  let fixtureBytes = new Uint8Array();
  let fixtureBase64 = "";
  let scrubFramesBase64 = "";
  if (!nativeTauriRuntime) {
    const [fixtureResponse, scrubFramesResponse] = await Promise.all([
      fetch("./scrub-fixture.mp4"),
      fetch("./scrub-fixture.mjpg"),
    ]);
    if (!fixtureResponse.ok || !scrubFramesResponse.ok) {
      throw new Error(
        `无法加载视频测试文件：${fixtureResponse.status}/${scrubFramesResponse.status}`,
      );
    }
    fixtureBytes = new Uint8Array(await fixtureResponse.arrayBuffer());
    let fixtureBinary = "";
    for (let offset = 0; offset < fixtureBytes.length; offset += 0x8000) {
      fixtureBinary += String.fromCharCode(
        ...fixtureBytes.subarray(offset, offset + 0x8000),
      );
    }
    fixtureBase64 = btoa(fixtureBinary);
    const scrubFrameBytes = new Uint8Array(
      await scrubFramesResponse.arrayBuffer(),
    );
    let scrubFramesBinary = "";
    for (let offset = 0; offset < scrubFrameBytes.length; offset += 0x8000) {
      scrubFramesBinary += String.fromCharCode(
        ...scrubFrameBytes.subarray(offset, offset + 0x8000),
      );
    }
    scrubFramesBase64 = btoa(scrubFramesBinary);
  }

  const video = {
    id: 9001,
    project_id: 1,
    chapter_id: 2,
    video_type: "assembly_push",
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
          case "load_video_scrub_playback_data":
            if (args?.includeAudio !== false) {
              throw new Error("scrub playback must omit audio");
            }
            return {
              mime_type: "video/mp4",
              data_base64: fixtureBase64,
              scrub_frames: {
                mime_type: "image/jpeg",
                data_base64: scrubFramesBase64,
                frames_per_second: 12,
              },
            };
          default:
            throw new Error(`unexpected command: ${command}`);
        }
      },
      convertFileSrc: (path) => path,
      transformCallback: () => nextCallbackId++,
      unregisterCallback: () => undefined,
    };
  }

  await initializePortableStorage();
  if (nativeTauriRuntime) writeCurrentUserId(1);

  createRoot(document.getElementById("root")).render(
    <div className="flex min-h-0 flex-col" style={{ height: "100vh", padding: 16 }}>
      <I18nProvider initialLocale="zh-CN">
        <VideoPostProcessWorkspace
          projectId={1}
          chapter={{
            id: 2,
            chapter_number: 1,
            title: "剪辑预览测试",
            status: "completed",
          }}
          videos={videos}
          onToast={() => undefined}
          onRefresh={() => undefined}
        />
      </I18nProvider>
    </div>,
  );
};

void mountHarness().catch((error) => {
  window.__A4N_VIDEO_SCRUB_E2E_ERROR__ = error?.stack ?? String(error);
});
