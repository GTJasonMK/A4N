import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type SyntheticEvent,
} from "react";
import {
  loadVideoPlaybackData,
  loadVideoScrubPlaybackData,
} from "../../../api/video";
import type { GeneratedVideo, VideoPlaybackData } from "../../../api/video";
import { base64ToBytes } from "../../../utils/base64";
import { videoPlaybackDataToObjectUrl } from "../../../utils/videoPlayback";
import { useI18n, type MessageTranslator } from "../../../i18n";

const VIDEO_ELEMENT_LOAD_TIMEOUT_MS = 20_000;
const SCRUB_PROXY_LOAD_TIMEOUT_MS = 120_000;

class PlaybackLoadTimeoutError extends Error {
  constructor() {
    super("Playback load timed out");
    this.name = "PlaybackLoadTimeoutError";
  }
}
class ScrubFrameDecodeError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "ScrubFrameDecodeError";
  }
}

export interface VideoScrubFrameSequence {
  bytes: Uint8Array;
  framesPerSecond: number;
  ranges: Array<{ start: number; end: number }>;
}

const decodeScrubFrames = (
  data: VideoPlaybackData["scrub_frames"],
  t: MessageTranslator,
): VideoScrubFrameSequence => {
  if (!data || data.mime_type !== "image/jpeg" || data.frames_per_second <= 0) {
    throw new ScrubFrameDecodeError(t("video.chapter.scrubFramesInvalid"));
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
    throw new ScrubFrameDecodeError(t("video.chapter.scrubFramesCorrupted"));
  }
  return {
    bytes,
    framesPerSecond: data.frames_per_second,
    ranges,
  };
};

const loadPlaybackDataWithTimeout = async (
  loader: (videoId: number) => Promise<VideoPlaybackData>,
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
  mediaKind: "preview" | "scrub";
}

export const useVideoPlaybackSource = ({
  selectedVideo,
  mediaKind,
}: UseVideoPlaybackSourceParams) => {
  const { t } = useI18n();
  const selectedVideoId = selectedVideo?.id;
  const selectedVideoPath = selectedVideo?.file_path?.trim();
  const [playbackSource, setPlaybackSource] = useState<{
    videoId: number;
    videoPath: string;
    mediaKind: UseVideoPlaybackSourceParams["mediaKind"];
    url: string;
    scrubFrames: VideoScrubFrameSequence | null;
    sourceFallback: boolean;
  } | null>(null);
  const [sourceFallbackKey, setSourceFallbackKey] = useState<string | null>(
    null,
  );
  const playbackKey = `${selectedVideoId ?? ""}:${selectedVideoPath ?? ""}:${mediaKind}`;
  // Scrub proxies optimize random seeks; WebKit can still reject a specific proxy.
  const sourceFallbackActive =
    mediaKind === "scrub" && sourceFallbackKey === playbackKey;
  const videoPlaybackSrc =
    playbackSource &&
    playbackSource.videoId === selectedVideoId &&
    playbackSource.videoPath === selectedVideoPath &&
    playbackSource.mediaKind === mediaKind &&
    playbackSource.sourceFallback === sourceFallbackActive
      ? playbackSource.url
      : "";
  const videoScrubFrames = videoPlaybackSrc
    ? (playbackSource?.scrubFrames ?? null)
    : null;
  const [videoLoadError, setVideoLoadError] = useState<string | null>(null);
  const [videoLoading, setVideoLoading] = useState(false);
  const [playbackRequestVersion, setPlaybackRequestVersion] = useState(0);
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
    const useSourceFallback =
      mediaKind === "scrub" && sourceFallbackKey === playbackKey;

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
    const loadPlaybackData = useSourceFallback
      ? loadVideoPlaybackData
      : (videoId: number) =>
          loadVideoScrubPlaybackData(videoId, mediaKind === "preview");
    void loadPlaybackDataWithTimeout(
      loadPlaybackData,
      selectedVideoId,
      useSourceFallback ? VIDEO_ELEMENT_LOAD_TIMEOUT_MS : SCRUB_PROXY_LOAD_TIMEOUT_MS,
    )
      .then((data) => {
        if (cancelled || generation !== playbackGenerationRef.current) return;
        const scrubFrames =
          mediaKind === "scrub" && !useSourceFallback
            ? decodeScrubFrames(data.scrub_frames, t)
            : null;
        objectUrl = videoPlaybackDataToObjectUrl(data);
        objectUrlRef.current = objectUrl;
        setPlaybackSource({
          videoId: selectedVideoId,
          videoPath: selectedVideoPath,
          mediaKind,
          url: objectUrl,
          scrubFrames,
          sourceFallback: useSourceFallback,
        });
      })
      .catch((err) => {
        if (cancelled || generation !== playbackGenerationRef.current) return;
        if (mediaKind === "scrub" && !useSourceFallback) {
          setSourceFallbackKey(playbackKey);
          return;
        }
        if (err instanceof PlaybackLoadTimeoutError) {
          // Promise.race cannot cancel the host call; invalidate its late result.
          playbackGenerationRef.current += 1;
        }
        setVideoLoading(false);
        setVideoLoadError(
          err instanceof ScrubFrameDecodeError
            ? err.message
            : err instanceof PlaybackLoadTimeoutError
            ? t("video.chapter.playbackLoadTimeout")
            : t("video.chapter.playbackLoadFailed"),
        );
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
    playbackRequestVersion,
    playbackKey,
    selectedVideo?.file_path,
    selectedVideo?.id,
    sourceFallbackKey,
    t,
  ]);

  const retryVideoLoad = useCallback(() => {
    setPlaybackRequestVersion((version) => version + 1);
  }, []);

  const handleVideoLoadStart = useCallback(() => {
    clearVideoLoadTimeout();
    setVideoLoading(true);
    setVideoLoadError(null);
    videoLoadTimeoutRef.current = setTimeout(() => {
      videoLoadTimeoutRef.current = null;
      setVideoLoading(false);
      setVideoLoadError(t("video.chapter.playbackPlayerTimeout"));
    }, VIDEO_ELEMENT_LOAD_TIMEOUT_MS);
  }, [clearVideoLoadTimeout, t]);

  const handleVideoLoaded = useCallback(() => {
    clearVideoLoadTimeout();
    setVideoLoading(false);
  }, [clearVideoLoadTimeout]);

  const handleVideoError = useCallback(
    (event: SyntheticEvent<HTMLVideoElement>): boolean => {
      const eventSource =
        event.currentTarget.currentSrc || event.currentTarget.src;
      if (eventSource !== objectUrlRef.current) return false;
      if (mediaKind === "scrub" && !sourceFallbackActive) {
        clearVideoLoadTimeout();
        setVideoLoading(true);
        setVideoLoadError(null);
        setSourceFallbackKey(playbackKey);
        return true;
      }
      // WebKitGTK 在快速 seek 时可能发出 error 事件但 media error 为空。
      // 这类假错误不应清掉当前可用的视频源。
      if (!event.currentTarget.error) return false;
      clearVideoLoadTimeout();
      setVideoLoading(false);
      const detail = event.currentTarget.error?.message?.trim();
      setVideoLoadError(
        detail
          ? t("video.chapter.playbackCannotPlay", { detail })
          : t("video.chapter.playbackCannotDecode"),
      );
      return true;
    },
    [clearVideoLoadTimeout, mediaKind, playbackKey, sourceFallbackActive, t],
  );

  return {
    videoPlaybackSrc,
    videoScrubFrames,
    videoLoadError,
    videoLoading,
    retryVideoLoad,
    handleVideoLoadStart,
    handleVideoLoaded,
    handleVideoError,
  };
};
