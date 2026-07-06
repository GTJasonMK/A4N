import {
  useCallback,
  useEffect,
  useState,
  type SyntheticEvent,
} from "react";
import { extractErrorMessage } from "../../../api";
import { loadVideoPlaybackData } from "../../../api/video";
import type { GeneratedVideo } from "../../../api/video";
import { videoPlaybackDataToObjectUrl } from "../../../utils/videoPlayback";

interface UseVideoPlaybackSourceParams {
  selectedVideo: GeneratedVideo | null;
  selectedVideoPath: string;
}

export const useVideoPlaybackSource = ({
  selectedVideo,
  selectedVideoPath,
}: UseVideoPlaybackSourceParams) => {
  const [videoPlaybackSrc, setVideoPlaybackSrc] = useState("");
  const [videoLoadError, setVideoLoadError] = useState<string | null>(null);
  const [videoLoading, setVideoLoading] = useState(false);

  useEffect(() => {
    let cancelled = false;
    let objectUrl: string | null = null;
    const selectedVideoId = selectedVideo?.id;

    setVideoLoadError(null);
    setVideoPlaybackSrc("");
    if (!selectedVideoId || !selectedVideoPath) {
      setVideoLoading(false);
      return () => {};
    }

    setVideoLoading(true);
    void loadVideoPlaybackData(selectedVideoId)
      .then((data) => {
        if (cancelled) return;
        objectUrl = videoPlaybackDataToObjectUrl(data);
        setVideoPlaybackSrc(objectUrl);
      })
      .catch((err) => {
        if (cancelled) return;
        setVideoLoading(false);
        setVideoLoadError(extractErrorMessage(err, "无法读取本地视频文件"));
      });

    return () => {
      cancelled = true;
      if (objectUrl) URL.revokeObjectURL(objectUrl);
    };
  }, [selectedVideo?.id, selectedVideoPath]);

  const handleVideoLoadStart = useCallback(() => {
    setVideoLoading(true);
    setVideoLoadError(null);
  }, []);

  const handleVideoLoaded = useCallback(() => {
    setVideoLoading(false);
  }, []);

  const handleVideoError = useCallback((event: SyntheticEvent<HTMLVideoElement>) => {
    setVideoLoading(false);
    const detail = event.currentTarget.error?.message?.trim();
    setVideoLoadError(
      detail
        ? `内置播放器无法播放该视频：${detail}`
        : "内置播放器无法解码该视频，文件已保存，可用本地播放器进一步检查。",
    );
  }, []);

  return {
    videoPlaybackSrc,
    videoLoadError,
    videoLoading,
    handleVideoLoadStart,
    handleVideoLoaded,
    handleVideoError,
  };
};
