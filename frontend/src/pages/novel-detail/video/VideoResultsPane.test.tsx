import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { VIDEO_TYPE_CHAPTER, type GeneratedVideo } from "../../../api/video";
import { I18nProvider } from "../../../i18n";
import { VideoResultsPane } from "./VideoResultsPane";

vi.mock("./useVideoPlaybackSource", () => ({
  useVideoPlaybackSource: () => ({
    videoPlaybackSrc: "",
    videoLoadError: null,
    videoLoading: true,
    retryVideoLoad: () => undefined,
    handleVideoLoadStart: () => undefined,
    handleVideoLoaded: () => undefined,
    handleVideoError: () => false,
  }),
}));

const video: GeneratedVideo = {
  id: 1,
  project_id: 1,
  chapter_id: 1,
  video_type: VIDEO_TYPE_CHAPTER,
  workflow_mode: "chat_video",
  file_path: "/tmp/chapter.mp4",
  thumbnail_path: null,
  prompt: "测试视频",
  negative_prompt: null,
  model: "test",
  width: 1280,
  height: 720,
  duration_secs: 6,
  fps: 24,
  file_size: 1024,
  source_video_id: null,
  source_image_id: null,
  timeline_item_id: null,
  status: "completed",
  provider_job_id: null,
  error_message: null,
  metadata_json: { shot_number: 1 },
  is_favorite: false,
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
};

describe("VideoResultsPane", () => {
  it("shows only the file-loading indicator before the player source is ready", () => {
    const html = renderToStaticMarkup(
      <I18nProvider initialLocale="zh-CN">
        <VideoResultsPane
          selectedVideo={video}
          adoptedVideoId={null}
          selectedVideoPath={video.file_path!}
          trimStart="0"
          trimEnd="6"
          acting={null}
          onPollVideo={() => undefined}
          onPreviewVideoRequest={() => undefined}
          onToggleVideoFavorite={() => undefined}
          onUpdateVideo={() => undefined}
          onDeleteVideo={() => undefined}
          onTrimStartChange={() => undefined}
          onTrimEndChange={() => undefined}
          onTrim={() => undefined}
          onOpenSegmentConfig={() => undefined}
          onAdoptVideo={() => undefined}
        />
      </I18nProvider>,
    );

    expect(html.match(/animate-spin/g)).toHaveLength(1);
    expect(html).toContain("正在读取视频文件");
    expect(html).not.toContain("正在加载视频");
  });
});
