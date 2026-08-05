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
import { videoPlaybackDataToObjectUrl } from "../../../utils/videoPlayback";

const PLAYBACK_LOAD_TIMEOUT_MS = 20_000;
const SCRUB_PROXY_LOAD_TIMEOUT_MS = 120_000;
class PlaybackLoadTimeoutError extends Error {
  constructor() {
    super("读取视频文件超时，请稍后重试");
    this.name = "PlaybackLoadTimeoutError";
  }
}
type PlaybackLoader = (videoId: number) => Promise<VideoPlaybackData>;

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
  mediaKind: "source" | "scrub";
}

export const useVideoPlaybackSource = ({
  selectedVideo,
  mediaKind,
}: UseVideoPlaybackSourceParams) => {
  const [videoPlaybackSrc, setVideoPlaybackSrc] = useState("");
  const [videoLoadError, setVideoLoadError] = useState<string | null>(null);
  const [videoLoading, setVideoLoading] = useState(false);
  const objectUrlRef = useRef<string | null>(null);
  const playbackGenerationRef = useRef(0);
  const scrubFallbackTriedRef = useRef(false);
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
    const selectedVideoId = selectedVideo?.id;
    const selectedVideoPath = selectedVideo?.file_path?.trim();

    if (objectUrlRef.current) {
      URL.revokeObjectURL(objectUrlRef.current);
      objectUrlRef.current = null;
    }
    scrubFallbackTriedRef.current = false;
    setVideoLoadError(null);
    setVideoPlaybackSrc("");
    if (!selectedVideoId || !selectedVideoPath) {
      setVideoLoading(false);
      return;
    }

    setVideoLoading(true);
    const loadPlaybackData =
      mediaKind === "scrub"
        ? loadVideoScrubPlaybackData
        : loadVideoPlaybackData;
    const loadTimeoutMs =
      mediaKind === "scrub"
        ? SCRUB_PROXY_LOAD_TIMEOUT_MS
        : PLAYBACK_LOAD_TIMEOUT_MS;
    const load = async () => {
      let data: VideoPlaybackData;
      try {
        data = await loadPlaybackDataWithTimeout(
          loadPlaybackData,
          selectedVideoId,
          loadTimeoutMs,
        );
      } catch (error) {
        if (error instanceof PlaybackLoadTimeoutError) throw error;
        if (
          mediaKind !== "scrub" ||
          scrubFallbackTriedRef.current ||
          cancelled
        ) {
          throw error;
        }
        scrubFallbackTriedRef.current = true;
        data = await loadPlaybackDataWithTimeout(
          loadVideoPlaybackData,
          selectedVideoId,
          PLAYBACK_LOAD_TIMEOUT_MS,
        );
      }
      return data;
    };
    void load()
      .then((data) => {
        if (cancelled || generation !== playbackGenerationRef.current) return;
        objectUrl = videoPlaybackDataToObjectUrl(data);
        objectUrlRef.current = objectUrl;
        setVideoPlaybackSrc(objectUrl);
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
    (event: SyntheticEvent<HTMLVideoElement>) => {
      clearVideoLoadTimeout();
      const selectedVideoId = selectedVideo?.id;
      if (
        mediaKind === "scrub" &&
        selectedVideoId &&
        !scrubFallbackTriedRef.current
      ) {
        scrubFallbackTriedRef.current = true;
        const generation = playbackGenerationRef.current;
        setVideoLoading(true);
        setVideoLoadError(null);
        void loadPlaybackDataWithTimeout(
          loadVideoPlaybackData,
          selectedVideoId,
          PLAYBACK_LOAD_TIMEOUT_MS,
        )
          .then((data) => {
            if (generation !== playbackGenerationRef.current) return;
            const nextObjectUrl = videoPlaybackDataToObjectUrl(data);
            if (objectUrlRef.current) {
              URL.revokeObjectURL(objectUrlRef.current);
            }
            objectUrlRef.current = nextObjectUrl;
            setVideoPlaybackSrc(nextObjectUrl);
          })
          .catch((err) => {
            setVideoLoading(false);
            setVideoLoadError(
              extractErrorMessage(err, "无法读取原视频文件"),
            );
          });
        return;
      }
      setVideoLoading(false);
      const detail = event.currentTarget.error?.message?.trim();
      setVideoLoadError(
        detail
          ? `内置播放器无法播放该视频：${detail}`
          : "内置播放器无法解码该视频，文件已保存，可用本地播放器进一步检查。",
      );
    },
    [clearVideoLoadTimeout, mediaKind, selectedVideo?.id],
  );

  return {
    videoPlaybackSrc,
    videoLoadError,
    videoLoading,
    handleVideoLoadStart,
    handleVideoLoaded,
    handleVideoError,
  };
};
