import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type SyntheticEvent,
} from "react";
import { extractErrorMessage } from "../../../api";
import {
  loadVideoPlaybackData,
  loadVideoScrubPlaybackData,
} from "../../../api/video";
import type { GeneratedVideo, VideoPlaybackData } from "../../../api/video";
import {
  base64ToBytes,
  videoPlaybackDataToObjectUrl,
} from "../../../utils/videoPlayback";

const PLAYBACK_LOAD_TIMEOUT_MS = 20_000;
const SCRUB_PROXY_LOAD_TIMEOUT_MS = 120_000;
class PlaybackLoadTimeoutError extends Error {
  constructor() {
    super("读取视频文件超时，请稍后重试");
    this.name = "PlaybackLoadTimeoutError";
  }
}
type PlaybackLoader = (videoId: number) => Promise<VideoPlaybackData>;

export interface VideoScrubFrameSequence {
  bytes: Uint8Array;
  framesPerSecond: number;
  ranges: Array<{ start: number; end: number }>;
}

const decodeScrubFrames = (
  data: VideoPlaybackData["scrub_frames"],
): VideoScrubFrameSequence => {
  if (!data || data.mime_type !== "image/jpeg" || data.frames_per_second <= 0) {
    throw new Error("剪辑拖动帧数据无效");
  }
  const bytes = base64ToBytes(data.data_base64);
  const ranges: VideoScrubFrameSequence["ranges"] = [];
  let start = -1;
  for (let index = 0; index + 1 < bytes.length; index += 1) {
    if (bytes[index] === 0xff && bytes[index + 1] === 0xd8) {
      start = index;
    } else if (
      start >= 0 &&
      bytes[index] === 0xff &&
      bytes[index + 1] === 0xd9
    ) {
      ranges.push({ start, end: index + 2 });
      start = -1;
      index += 1;
    }
  }
  if (start >= 0 || ranges.length === 0) {
    throw new Error("剪辑拖动帧 JPEG 序列损坏");
  }
  return {
    bytes,
    framesPerSecond: data.frames_per_second,
    ranges,
  };
};

const loadPlaybackDataWithTimeout = async (
  loader: PlaybackLoader,
  videoId: number,
  timeoutMs: number,
): Promise<VideoPlaybackData> => {
  let timeoutId: ReturnType<typeof setTimeout> | null = null;
  try {
    return await Promise.race([
      loader(videoId),
      new Promise<VideoPlaybackData>((_, reject) => {
        timeoutId = setTimeout(
          () => reject(new PlaybackLoadTimeoutError()),
          timeoutMs,
        );
      }),
    ]);
  } finally {
    if (timeoutId != null) clearTimeout(timeoutId);
  }
};

interface UseVideoPlaybackSourceParams {
  selectedVideo: GeneratedVideo | null;
  // preview 保留声音用于正常播放；scrub 去掉音轨，避免 WebKitGTK
  // 在暂停状态连续 seek 时触发 GStreamer data stream error。
  mediaKind: "source" | "preview" | "scrub";
}

export const useVideoPlaybackSource = ({
  selectedVideo,
  mediaKind,
}: UseVideoPlaybackSourceParams) => {
  const selectedVideoId = selectedVideo?.id;
  const selectedVideoPath = selectedVideo?.file_path?.trim();
  const [playbackSource, setPlaybackSource] = useState<{
    videoId: number;
    videoPath: string;
    mediaKind: UseVideoPlaybackSourceParams["mediaKind"];
    url: string;
    scrubFrames: VideoScrubFrameSequence | null;
  } | null>(null);
  const videoPlaybackSrc =
    playbackSource &&
    playbackSource.videoId === selectedVideoId &&
    playbackSource.videoPath === selectedVideoPath &&
    playbackSource.mediaKind === mediaKind
      ? playbackSource.url
      : "";
  const videoScrubFrames = videoPlaybackSrc
    ? (playbackSource?.scrubFrames ?? null)
    : null;
  const [videoLoadError, setVideoLoadError] = useState<string | null>(null);
  const [videoLoading, setVideoLoading] = useState(false);
  const objectUrlRef = useRef<string | null>(null);
  const playbackGenerationRef = useRef(0);
  const videoLoadTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(
    null,
  );

  const clearVideoLoadTimeout = useCallback(() => {
    if (videoLoadTimeoutRef.current != null) {
      clearTimeout(videoLoadTimeoutRef.current);
      videoLoadTimeoutRef.current = null;
    }
  }, []);

  useEffect(() => {
    let cancelled = false;
    let objectUrl: string | null = null;
    const generation = ++playbackGenerationRef.current;

    if (objectUrlRef.current) {
      URL.revokeObjectURL(objectUrlRef.current);
      objectUrlRef.current = null;
    }
    setVideoLoadError(null);
    setPlaybackSource(null);
    if (!selectedVideoId || !selectedVideoPath) {
      setVideoLoading(false);
      return;
    }

    setVideoLoading(true);
    const loadPlaybackData: PlaybackLoader =
      mediaKind === "source"
        ? loadVideoPlaybackData
        : (videoId) =>
            loadVideoScrubPlaybackData(videoId, mediaKind === "preview");
    const loadTimeoutMs =
      mediaKind === "source"
        ? PLAYBACK_LOAD_TIMEOUT_MS
        : SCRUB_PROXY_LOAD_TIMEOUT_MS;
    void loadPlaybackDataWithTimeout(
      loadPlaybackData,
      selectedVideoId,
      loadTimeoutMs,
    )
      .then((data) => {
        if (cancelled || generation !== playbackGenerationRef.current) return;
        const scrubFrames =
          mediaKind === "scrub" ? decodeScrubFrames(data.scrub_frames) : null;
        objectUrl = videoPlaybackDataToObjectUrl(data);
        objectUrlRef.current = objectUrl;
        setPlaybackSource({
          videoId: selectedVideoId,
          videoPath: selectedVideoPath,
          mediaKind,
          url: objectUrl,
          scrubFrames,
        });
      })
      .catch((err) => {
        if (cancelled) return;
        setVideoLoading(false);
        setVideoLoadError(extractErrorMessage(err, "无法读取本地视频文件"));
      });

    return () => {
      cancelled = true;
      clearVideoLoadTimeout();
      if (objectUrl && objectUrlRef.current === objectUrl) {
        URL.revokeObjectURL(objectUrl);
        objectUrlRef.current = null;
      }
    };
  }, [
    clearVideoLoadTimeout,
    mediaKind,
    selectedVideo?.file_path,
    selectedVideo?.id,
  ]);

  const handleVideoLoadStart = useCallback(() => {
    clearVideoLoadTimeout();
    setVideoLoading(true);
    setVideoLoadError(null);
    videoLoadTimeoutRef.current = setTimeout(() => {
      videoLoadTimeoutRef.current = null;
      setVideoLoading(false);
      setVideoLoadError("内置播放器加载视频超时，请重试或使用本地播放器打开。");
    }, PLAYBACK_LOAD_TIMEOUT_MS);
  }, [clearVideoLoadTimeout]);

  const handleVideoLoaded = useCallback(() => {
    clearVideoLoadTimeout();
    setVideoLoading(false);
  }, [clearVideoLoadTimeout]);

  const handleVideoError = useCallback(
    (event: SyntheticEvent<HTMLVideoElement>): boolean => {
      const eventSource =
        event.currentTarget.currentSrc || event.currentTarget.src;
      if (eventSource !== objectUrlRef.current) return false;
      // WebKitGTK 在快速 seek 时可能发出 error 事件但 media error 为空。
      // 这类假错误不应清掉当前可用的视频源。
      if (!event.currentTarget.error) return false;
      clearVideoLoadTimeout();
      setVideoLoading(false);
      const detail = event.currentTarget.error?.message?.trim();
      setVideoLoadError(
        detail
          ? `内置播放器无法播放该视频：${detail}`
          : "内置播放器无法解码该视频，文件已保存，可用本地播放器进一步检查。",
      );
      return true;
    },
    [clearVideoLoadTimeout],
  );

  return {
    videoPlaybackSrc,
    videoScrubFrames,
    videoLoadError,
    videoLoading,
    handleVideoLoadStart,
    handleVideoLoaded,
    handleVideoError,
  };
};
