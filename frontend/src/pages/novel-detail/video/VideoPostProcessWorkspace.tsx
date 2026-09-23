import React, {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { Check, RefreshCw, RotateCcw, Scissors, Upload, X } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { extractErrorMessage } from "../../../api";
import {
  applyAssemblyPostprocessEdit,
  updateAssemblyPushStatus,
  VIDEO_TYPE_ASSEMBLY_POSTPROCESS_EDIT,
  VIDEO_TYPE_ASSEMBLY_PUSH,
  VIDEO_WORKFLOW_MODE_MIXED,
  type GeneratedVideo,
} from "../../../api/video";
import type { ToastType } from "../../../components/feedback/Toast";
import { BookButton } from "../../../components/ui/BookButton";
import { Dropdown } from "../../../components/ui/Dropdown";
import { asJsonRecord as asRecord } from "../../../utils/jsonRecord";
import { readFileAsBase64 } from "../../../utils/fileUpload";
import {
  pickVideoUploadFile,
  type VideoUploadFile,
} from "../../../utils/videoUpload";
import { EmptyHint, type ChapterRow } from "../shared";
import { useVideoPlaybackSource } from "./useVideoPlaybackSource";
import { formatNumber, useI18n, type Locale, type MessageKey, type MessageTranslator } from "../../../i18n";
import {
  assemblyPushIndex,
  assemblyPushSourceIds,
  assemblyPushStatus,
  latestNonRejectedAssemblyPush,
} from "./videoAssemblyUtils";
import { formatVideoDuration, formatVideoCreatedAt } from "./videoImageUtils";

import { VideoPostProcessTimeline } from "./VideoPostProcessTimeline";
interface VideoPostProcessWorkspaceProps {
  projectId: number;
  chapter: ChapterRow;
  videos: GeneratedVideo[];
  onToast: (message: string, type: ToastType) => void;
  onRefresh: () => Promise<void> | void;
}

const MARKER_MIN_GAP_SECS = 0.05;
const VIDEO_SEEK_TOLERANCE_SECS = 0.05;
const VIDEO_FRAME_PRESENTATION_TIMEOUT_MS = 750;

const formatPreciseSeconds = (value: number, locale: Locale, t: MessageTranslator): string =>
  t("video.postprocess.preciseSeconds", {
    value: formatNumber(value, locale, { minimumFractionDigits: 2, maximumFractionDigits: 2 }),
  });

const editPushId = (video: GeneratedVideo): number | null => {
  const value = asRecord(
    asRecord(video.metadata_json)?.postprocess_edit,
  )?.push_video_id;
  return Number.isInteger(value) && Number(value) > 0 ? Number(value) : null;
};

const editOperationLabel: Record<string, MessageKey> = {
  insert: "video.chapter.postprocessInsert",
  delete: "video.chapter.postprocessDelete",
  replace: "video.chapter.postprocessReplace",
};

const PUSH_STATUS_LABELS: Record<string, MessageKey> = {
  pending: "video.chapter.postprocessPending",
  accepted: "video.chapter.postprocessAccepted",
  rejected: "video.chapter.postprocessRejected",
};

export const VideoPostProcessWorkspace: React.FC<
  VideoPostProcessWorkspaceProps
> = ({ projectId, chapter, videos, onToast, onRefresh }) => {
  const { locale, t } = useI18n();
  const pushes = useMemo(
    () =>
      videos
        .filter(
          (video) =>
            video.video_type === VIDEO_TYPE_ASSEMBLY_PUSH &&
            video.workflow_mode === VIDEO_WORKFLOW_MODE_MIXED &&
            video.status === "completed",
        )
        .sort(
          (a, b) =>
            (assemblyPushIndex(a) ?? Number.MAX_SAFE_INTEGER) -
              (assemblyPushIndex(b) ?? Number.MAX_SAFE_INTEGER) || a.id - b.id,
        ),
    [videos],
  );
  const editsByPushId = useMemo(() => {
    const map = new Map<number, GeneratedVideo[]>();
    for (const video of videos) {
      if (
        video.video_type !== VIDEO_TYPE_ASSEMBLY_POSTPROCESS_EDIT ||
        video.workflow_mode !== VIDEO_WORKFLOW_MODE_MIXED ||
        video.status !== "completed"
      ) {
        continue;
      }
      const pushId = editPushId(video);
      if (pushId == null) continue;
      const rows = map.get(pushId) ?? [];
      rows.push(video);
      map.set(pushId, rows);
    }
    for (const rows of map.values()) {
      rows.sort((a, b) => {
        return a.created_at.localeCompare(b.created_at) || a.id - b.id;
      });
    }
    return map;
  }, [videos]);

  const pushById = useMemo(
    () => new Map(pushes.map((push) => [push.id, push] as const)),
    [pushes],
  );
  const latestPush = useMemo(() => latestNonRejectedAssemblyPush(pushes), [pushes]);
  const [selectedPushId, setSelectedPushId] = useState<number | null>(null);
  const selectedPush =
    selectedPushId == null
      ? null
      : (pushById.get(selectedPushId) ?? null);
  const versions = selectedPush
    ? (editsByPushId.get(selectedPush.id) ?? [])
    : [];
  const currentVideo = versions[versions.length - 1] ?? selectedPush;
  const {
    videoPlaybackSrc,
    videoScrubFrames,
    videoLoadError,
    videoLoading,
    retryVideoLoad,
    handleVideoLoadStart,
    handleVideoLoaded,
    handleVideoError,
  } = useVideoPlaybackSource({
    selectedVideo: currentVideo ?? null,
    mediaKind: "scrub",
  });

  const [acting, setActing] = useState<string | null>(null);
  const [decodedDurationSecs, setDecodedDurationSecs] = useState<number | null>(
    null,
  );
  const [markerValue, setMarkerValue] = useState(0);
  const [playbackTimeSecs, setPlaybackTimeSecs] = useState(0);
  const [videoPlaying, setVideoPlaying] = useState(false);
  const [markers, setMarkers] = useState<number[]>([]);
  const [selectedMarkerIndex, setSelectedMarkerIndex] = useState<number | null>(
    null,
  );
  const [draggingMarkerIndex, setDraggingMarkerIndex] = useState<number | null>(
    null,
  );
  const [scrubPreviewVisible, setScrubPreviewVisible] = useState(false);
  const playbackVideoRef = useRef<HTMLVideoElement | null>(null);
  const scrubFrameImageRef = useRef<HTMLImageElement | null>(null);
  const scrubFrameUrlsRef = useRef(new Map<number, string>());
  const desiredSeekTimeRef = useRef<number | null>(null);
  const seekRafRef = useRef<number | null>(null);
  const seekInFlightRef = useRef(false);
  const presentationRafRef = useRef<number | null>(null);
  const presentationVideoRef = useRef<HTMLVideoElement | null>(null);
  const presentationCallbackRef = useRef<number | null>(null);
  const presentationTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(
    null,
  );
  const presentationTokenRef = useRef(0);
  const scrubPreviewActiveRef = useRef(false);
  const draggingScrubRef = useRef(false);
  const pendingDragReleaseRef = useRef(false);
  const timelineRef = useRef<HTMLDivElement | null>(null);
  const scopeKey = `${projectId}:${chapter.id}`;
  const activeScopeRef = useRef(scopeKey);
  const mountedRef = useRef(false);
  const actionBusyRef = useRef(false);
  const clearMarkers = useCallback(() => {
    setMarkers([]);
    setSelectedMarkerIndex(null);
  }, []);
  const cancelPresentationWait = useCallback(() => {
    presentationTokenRef.current += 1;
    if (presentationRafRef.current != null) {
      cancelAnimationFrame(presentationRafRef.current);
      presentationRafRef.current = null;
    }
    if (
      presentationVideoRef.current &&
      presentationCallbackRef.current != null &&
      typeof presentationVideoRef.current.cancelVideoFrameCallback ===
        "function"
    ) {
      presentationVideoRef.current.cancelVideoFrameCallback(
        presentationCallbackRef.current,
      );
    }
    presentationVideoRef.current = null;
    presentationCallbackRef.current = null;
    if (presentationTimeoutRef.current != null) {
      clearTimeout(presentationTimeoutRef.current);
      presentationTimeoutRef.current = null;
    }
  }, []);
  const cancelSeekPump = useCallback(() => {
    if (seekRafRef.current != null) cancelAnimationFrame(seekRafRef.current);
    seekRafRef.current = null;
    setScrubPreviewVisible(false);
    cancelPresentationWait();
    desiredSeekTimeRef.current = null;
    seekInFlightRef.current = false;
    scrubPreviewActiveRef.current = false;
    draggingScrubRef.current = false;
    pendingDragReleaseRef.current = false;
    const video = playbackVideoRef.current;
    if (video) delete video.dataset.scrubSeekPending;
  }, [cancelPresentationWait]);
  const waitForPresentedFrame = useCallback(
    (video: HTMLVideoElement, targetTime: number, onPresented: () => void) => {
      cancelPresentationWait();
      const token = presentationTokenRef.current;
      let finished = false;
      const finish = () => {
        if (finished || presentationTokenRef.current !== token) return;
        finished = true;
        if (presentationRafRef.current != null) {
          cancelAnimationFrame(presentationRafRef.current);
          presentationRafRef.current = null;
        }
        presentationVideoRef.current = null;
        presentationCallbackRef.current = null;
        if (presentationTimeoutRef.current != null) {
          clearTimeout(presentationTimeoutRef.current);
          presentationTimeoutRef.current = null;
        }
        onPresented();
      };
      if (typeof video.requestVideoFrameCallback === "function") {
        const waitForTargetFrame = () => {
          presentationVideoRef.current = video;
          presentationCallbackRef.current = video.requestVideoFrameCallback(
            (_now, metadata) => {
              presentationCallbackRef.current = null;
              if (finished || presentationTokenRef.current !== token) return;
              if (
                Math.abs(metadata.mediaTime - targetTime) <
                VIDEO_SEEK_TOLERANCE_SECS
              ) {
                finish();
              } else {
                waitForTargetFrame();
              }
            },
          );
        };
        waitForTargetFrame();
      } else {
        presentationRafRef.current = requestAnimationFrame(() => {
          presentationRafRef.current = requestAnimationFrame(finish);
        });
      }
      presentationTimeoutRef.current = setTimeout(
        finish,
        VIDEO_FRAME_PRESENTATION_TIMEOUT_MS,
      );
    },
    [cancelPresentationWait],
  );
  const showScrubFrame = useCallback(
    (time: number) => {
      // WebKit may defer rAF through a pointer burst; commit the JPEG now so
      // the visible frame always follows the latest drag position.
      const image = scrubFrameImageRef.current;
      if (!image || !videoScrubFrames) return;
      const index = Math.max(
        0,
        Math.min(
          videoScrubFrames.ranges.length - 1,
          Math.round(time * videoScrubFrames.framesPerSecond),
        ),
      );
      let url = scrubFrameUrlsRef.current.get(index);
      if (!url) {
        const range = videoScrubFrames.ranges[index];
        url = URL.createObjectURL(
          new Blob([videoScrubFrames.bytes.slice(range.start, range.end)], {
            type: "image/jpeg",
          }),
        );
        scrubFrameUrlsRef.current.set(index, url);
      }
      if (image.src !== url) image.src = url;
      image.dataset.scrubFrameIndex = String(index);
      setScrubPreviewVisible(true);
    },
    [videoScrubFrames],
  );
  useEffect(() => {
    const urls = scrubFrameUrlsRef.current;
    return () => {
      for (const url of urls.values()) URL.revokeObjectURL(url);
      urls.clear();
    };
  }, [videoScrubFrames]);
  useLayoutEffect(() => {
    activeScopeRef.current = scopeKey;
    actionBusyRef.current = false;
    setActing(null);
    clearMarkers();
    setDecodedDurationSecs(null);
    setMarkerValue(0);
    setPlaybackTimeSecs(0);
    setVideoPlaying(false);
    setDraggingMarkerIndex(null);
    cancelSeekPump();
  }, [cancelSeekPump, clearMarkers, currentVideo?.id, scopeKey]);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      actionBusyRef.current = false;
      cancelSeekPump();
    };
  }, [cancelSeekPump]);

  const isCurrentScope = (requestScope: string) =>
    mountedRef.current && activeScopeRef.current === requestScope;

  const beginAction = (): boolean => {
    if (actionBusyRef.current) return false;
    actionBusyRef.current = true;
    return true;
  };

  const finishAction = (requestScope: string) => {
    if (isCurrentScope(requestScope)) actionBusyRef.current = false;
  };

  useEffect(() => {
    setSelectedPushId((current) => {
      if (current != null && pushById.has(current))
        return current;
      return latestPush?.id ?? pushes[0]?.id ?? null;
    });
  }, [latestPush?.id, pushById, pushes]);

  const duration = Math.max(
    0.1,
    decodedDurationSecs ?? currentVideo?.duration_secs ?? 0,
  );
  // 解码时长可能短于记录时长；时长收敛后把已有竖线夹回真实片尾，
  // 避免把越界秒数提交给后端。
  useEffect(() => {
    setMarkers((prev) =>
      prev.some((marker) => marker > duration)
        ? prev.map((marker) => Math.min(marker, duration))
        : prev,
    );
    setMarkerValue((prev) => Math.min(prev, duration));
  }, [duration]);
  const selectedRange = useMemo(() => {
    if (markers.length < 2) return null;
    const sorted = [...markers].sort((a, b) => a - b);
    if (sorted[1] - sorted[0] < MARKER_MIN_GAP_SECS) return null;
    return { start: sorted[0], end: sorted[1] };
  }, [markers]);
  const singleMarker = markers.length === 1 ? markers[0] : null;
  const editableLatest =
    selectedPush != null && latestPush?.id === selectedPush.id;
  const selectedPushIndex = selectedPush
    ? assemblyPushIndex(selectedPush)
    : null;
  const selectedPushStatus = selectedPush
    ? assemblyPushStatus(selectedPush)
    : null;
  const selectedPushSourceIds = selectedPush
    ? assemblyPushSourceIds(selectedPush)
    : null;
  const editDisabledReason = !selectedPush
    ? t("video.postprocess.noEditablePush")
    : selectedPushIndex == null ||
        selectedPushStatus == null ||
        selectedPushSourceIds == null
      ? t("video.postprocess.invalidPush")
      : selectedPushStatus === "rejected"
        ? t("video.postprocess.rejectedCannotEdit")
        : !editableLatest
          ? t("video.postprocess.latestOnly")
          : null;

  const appendMarker = (rawValue: number) => {
    const value = Math.max(0, Math.min(duration, rawValue));
    setMarkers((prev) => {
      if (
        prev.length >= 2 ||
        prev.some((marker) => Math.abs(marker - value) <= MARKER_MIN_GAP_SECS)
      ) {
        return prev;
      }
      return [...prev, value];
    });
  };
  const constrainMarkerValue = useCallback(
    (index: number, rawValue: number, sourceMarkers = markers): number => {
      const value = Math.max(0, Math.min(duration, rawValue));
      const currentValue = sourceMarkers[index];
      if (currentValue == null) return value;
      let lowerBound = 0;
      let upperBound = duration;
      for (const [itemIndex, item] of sourceMarkers.entries()) {
        if (itemIndex === index) continue;
        if (item < currentValue) {
          lowerBound = Math.max(lowerBound, item + MARKER_MIN_GAP_SECS);
        } else {
          upperBound = Math.min(upperBound, item - MARKER_MIN_GAP_SECS);
        }
      }
      return Math.max(lowerBound, Math.min(upperBound, value));
    },
    [duration, markers],
  );
  const applyPendingSeek = useCallback(() => {
    seekRafRef.current = null;
    const video = playbackVideoRef.current;
    const requestedTime = desiredSeekTimeRef.current;
    if (
      !video ||
      requestedTime == null ||
      video.readyState < HTMLMediaElement.HAVE_METADATA
    ) {
      return;
    }
    // `video.seeking` can remain true for WebKit's initial media seek even
    // though no scrub request is in flight. `seekInFlightRef` is the state we
    // own, so use it as the guard and allow the first requested target through.
    if (seekInFlightRef.current) return;
    const target = Math.max(0, Math.min(requestedTime, duration));
    video.pause();
    if (Math.abs(video.currentTime - target) >= VIDEO_SEEK_TOLERANCE_SECS) {
      seekInFlightRef.current = true;
      video.dataset.scrubSeekPending = "true";
      video.currentTime = target;
    } else {
      desiredSeekTimeRef.current = null;
      video.dataset.scrubSeekPending = "false";
      if (draggingScrubRef.current && !pendingDragReleaseRef.current) {
        showScrubFrame(target);
      }
      if (pendingDragReleaseRef.current) {
        draggingScrubRef.current = false;
        pendingDragReleaseRef.current = false;
        scrubPreviewActiveRef.current = false;
        setScrubPreviewVisible(false);
      } else if (!draggingScrubRef.current) {
        scrubPreviewActiveRef.current = false;
        setScrubPreviewVisible(false);
      }
    }
  }, [duration, showScrubFrame]);
  const schedulePendingSeek = useCallback(() => {
    if (seekRafRef.current != null) return;
    seekRafRef.current = requestAnimationFrame(applyPendingSeek);
  }, [applyPendingSeek]);
  const handleScrubSeeked = useCallback(() => {
    const video = playbackVideoRef.current;
    const desiredTime = desiredSeekTimeRef.current;
    if (!video) return;
    const seekVideoElement = video;
    const finish = () => {
      seekInFlightRef.current = false;
      const latestVideo = playbackVideoRef.current;
      if (!latestVideo) return;
      if (draggingScrubRef.current && !pendingDragReleaseRef.current) {
        // 拖动期间只显示 JPEG 帧。旧 seek 若此时结束，不要继续追赶
        // 最新指针位置，否则 WebKitGTK 又会退化为高频连续 seek。
        setPlaybackTimeSecs(latestVideo.currentTime);
        latestVideo.dataset.scrubSeekPending = "false";
        return;
      }
      const latestDesiredTime = desiredSeekTimeRef.current;
      if (
        latestDesiredTime != null &&
        Math.abs(latestVideo.currentTime - latestDesiredTime) >=
          VIDEO_SEEK_TOLERANCE_SECS
      ) {
        setPlaybackTimeSecs(latestVideo.currentTime);
        schedulePendingSeek();
        return;
      }
      if (latestDesiredTime != null) {
        desiredSeekTimeRef.current = null;
        if (pendingDragReleaseRef.current) {
          draggingScrubRef.current = false;
          pendingDragReleaseRef.current = false;
          scrubPreviewActiveRef.current = false;
        }
      }
      if (!draggingScrubRef.current && !pendingDragReleaseRef.current) {
        scrubPreviewActiveRef.current = false;
      }
      if (!scrubPreviewActiveRef.current) {
        // 原生控件 seek（非 scrub 请求）：把竖线基准同步到画面位置，
        // 否则读数与后续添加的竖线会停在旧位置，与画面脱节。
        setPlaybackTimeSecs(latestVideo.currentTime);
        setMarkerValue(latestVideo.currentTime);
        latestVideo.dataset.scrubSeekPending = "false";
        setScrubPreviewVisible(false);
        return;
      }
      setPlaybackTimeSecs(latestVideo.currentTime);
      latestVideo.dataset.scrubSeekPending = "false";
      if (!draggingScrubRef.current) setScrubPreviewVisible(false);
    };
    const reachedDesiredTime =
      desiredTime != null &&
      Math.abs(seekVideoElement.currentTime - desiredTime) <
        VIDEO_SEEK_TOLERANCE_SECS;
    if (
      reachedDesiredTime &&
      scrubPreviewActiveRef.current &&
      (!draggingScrubRef.current || pendingDragReleaseRef.current)
    ) {
      waitForPresentedFrame(seekVideoElement, desiredTime, finish);
    } else {
      finish();
    }
  }, [schedulePendingSeek, waitForPresentedFrame]);
  const seekVideo = useCallback(
    (value: number) => {
      const target = Math.max(0, Math.min(value, duration));
      playbackVideoRef.current?.pause();
      setMarkerValue(target);
      showScrubFrame(target);
      desiredSeekTimeRef.current = target;
      if (!draggingScrubRef.current || !videoScrubFrames) {
        schedulePendingSeek();
      }
    },
    [duration, schedulePendingSeek, showScrubFrame],
  );
  const finishScrub = useCallback(() => {
    if (seekRafRef.current != null) cancelAnimationFrame(seekRafRef.current);
    applyPendingSeek();
  }, [applyPendingSeek]);
  const beginScrub = useCallback(() => {
    cancelPresentationWait();
    // WebKitGTK 偶尔不会为失败的 seek 派发 seeked。新一次用户拖动
    // 必须能覆盖旧请求，不能被我们自己的 in-flight 标记永久锁死。
    seekInFlightRef.current = false;
    scrubPreviewActiveRef.current = true;
    pendingDragReleaseRef.current = false;
    const video = playbackVideoRef.current;
    video?.pause();
    showScrubFrame(video?.currentTime ?? markerValue);
  }, [cancelPresentationWait, markerValue, showScrubFrame]);
  const addMarker = () => {
    appendMarker(markerValue);
  };

  const markerValueFromClientX = (clientX: number): number | null => {
    const rect = timelineRef.current?.getBoundingClientRect();
    if (!rect || rect.width <= 0) return null;
    const ratio = Math.max(0, Math.min(1, (clientX - rect.left) / rect.width));
    return ratio * duration;
  };

  const handleTimelineClick = (event: React.MouseEvent<HTMLDivElement>) => {
    if (
      event.target !== event.currentTarget ||
      !currentVideo ||
      editDisabledReason ||
      acting?.startsWith("edit-") ||
      markers.length >= 2 ||
      draggingMarkerIndex != null
    ) {
      return;
    }
    const value = markerValueFromClientX(event.clientX);
    if (value == null) return;
    beginScrub();
    seekVideo(value);
    finishScrub();
    appendMarker(value);
  };

  useEffect(() => {
    if (selectedMarkerIndex == null) return;
    const handleMarkerDelete = (event: KeyboardEvent) => {
      if (
        event.repeat ||
        (event.key !== "Delete" && event.key !== "Backspace")
      ) {
        return;
      }
      const target = event.target;
      if (
        target instanceof HTMLInputElement ||
        target instanceof HTMLTextAreaElement ||
        target instanceof HTMLSelectElement ||
        (target instanceof HTMLElement && target.isContentEditable)
      ) {
        return;
      }
      event.preventDefault();
      setMarkers((prev) =>
        prev.filter((_, itemIndex) => itemIndex !== selectedMarkerIndex),
      );
      setSelectedMarkerIndex(null);
    };
    window.addEventListener("keydown", handleMarkerDelete);
    return () => window.removeEventListener("keydown", handleMarkerDelete);
  }, [selectedMarkerIndex]);

  const handlePushStatus = async (action: "accept" | "reject" | "undo") => {
    if (!selectedPush) return;
    const requestScope = scopeKey;
    if (!beginAction()) return;
    setActing(`status-${action}`);
    try {
      await updateAssemblyPushStatus(selectedPush.id, action);
      if (!isCurrentScope(requestScope)) return;
      await onRefresh();
      if (!isCurrentScope(requestScope)) return;
      onToast(
        action === "accept"
          ? t("video.postprocess.acceptedLatest")
          : action === "reject"
            ? t("video.postprocess.rejectedLatest")
            : t("video.postprocess.undoAcceptedLatest"),
        "success",
      );
    } catch (err) {
      if (!isCurrentScope(requestScope)) return;
      onToast(extractErrorMessage(err, t("video.postprocess.updateStatusFailed")), "error");
    } finally {
      finishAction(requestScope);
      if (isCurrentScope(requestScope)) setActing(null);
    }
  };

  const applyUploadEdit = async (operation: "insert" | "replace") => {
    if (!selectedPush || editDisabledReason) return;
    const requestScope = scopeKey;
    if (operation === "insert" && singleMarker == null) {
      onToast(t("video.postprocess.insertNeedMarker"), "error");
      return;
    }
    if (operation === "replace" && !selectedRange) {
      onToast(t("video.postprocess.replaceNeedMarkers"), "error");
      return;
    }
    if (!beginAction()) return;
    setActing(`edit-${operation}`);
    let upload: VideoUploadFile | null = null;
    try {
      upload = await pickVideoUploadFile(t);
    } catch (err) {
      if (!isCurrentScope(requestScope)) return;
      onToast(extractErrorMessage(err, t("video.postprocess.filePickerFailed")), "error");
      return;
    } finally {
      if (!upload) {
        finishAction(requestScope);
        if (isCurrentScope(requestScope)) setActing(null);
      }
    }
    if (!upload) return;
    const { file, mimeType } = upload;
    if (!isCurrentScope(requestScope)) {
      finishAction(requestScope);
      return;
    }
    try {
      const dataBase64 = await readFileAsBase64(file);
      if (!isCurrentScope(requestScope)) return;
      await applyAssemblyPostprocessEdit(selectedPush.id, {
        operation,
        at_secs: operation === "insert" ? singleMarker : null,
        start_secs: operation === "replace" ? selectedRange?.start : null,
        end_secs: operation === "replace" ? selectedRange?.end : null,
        source_file_name: file.name,
        source_mime_type: mimeType,
        source_data_base64: dataBase64,
      });
      if (!isCurrentScope(requestScope)) return;
      clearMarkers();
      await onRefresh();
      if (!isCurrentScope(requestScope)) return;
      onToast(t("video.postprocess.editUploaded", { operation: t(editOperationLabel[operation]) }), "success");
    } catch (err) {
      if (!isCurrentScope(requestScope)) return;
      onToast(extractErrorMessage(err, t("video.postprocess.applyEditFailed")), "error");
    } finally {
      finishAction(requestScope);
      if (isCurrentScope(requestScope)) setActing(null);
    }
  };

  const applyDelete = async () => {
    if (!selectedPush || editDisabledReason || !selectedRange) {
      onToast(t("video.postprocess.deleteNeedMarkers"), "error");
      return;
    }
    const requestScope = scopeKey;
    if (!beginAction()) return;
    setActing("edit-delete");
    try {
      await applyAssemblyPostprocessEdit(selectedPush.id, {
        operation: "delete",
        start_secs: selectedRange.start,
        end_secs: selectedRange.end,
      });
      if (!isCurrentScope(requestScope)) return;
      clearMarkers();
      await onRefresh();
      if (!isCurrentScope(requestScope)) return;
      onToast(t("video.postprocess.deletedSelection"), "success");
    } catch (err) {
      if (!isCurrentScope(requestScope)) return;
      onToast(extractErrorMessage(err, t("video.postprocess.deleteFailed")), "error");
    } finally {
      finishAction(requestScope);
      if (isCurrentScope(requestScope)) setActing(null);
    }
  };

  const statusActionItems: React.ComponentProps<typeof Dropdown>["items"] =
    editableLatest && selectedPushStatus === "pending"
      ? [
          {
            label: t("video.postprocess.rejectPush"),
            icon: <X size={12} />,
            danger: true,
            onClick: () => void handlePushStatus("reject"),
          },
        ]
      : editableLatest && selectedPushStatus === "accepted"
        ? [
            {
              label: t("video.postprocess.undoAccept"),
              icon: <RotateCcw size={12} />,
              onClick: () => void handlePushStatus("undo"),
            },
          ]
        : [];
  const editActionItems: React.ComponentProps<typeof Dropdown>["items"] = [
    ...(markers.length > 0
      ? [
          {
            label: t("video.postprocess.clearMarkers"),
            icon: <RotateCcw size={12} />,
            onClick: clearMarkers,
          },
        ]
      : []),
    ...(selectedRange
      ? [
          { type: "divider" as const },
          {
            label: t("video.postprocess.deleteSelection"),
            icon: <Scissors size={12} />,
            danger: true,
            onClick: () => void applyDelete(),
          },
        ]
      : []),
  ];
  const statusActing = acting?.startsWith("status-") ?? false;
  const editActing = acting?.startsWith("edit-") ?? false;
  const markerSummary =
    markers.length === 0
      ? t("video.postprocess.markerHint")
      : singleMarker != null
        ? t("video.postprocess.insertPosition", { time: formatPreciseSeconds(singleMarker, locale, t) })
        : selectedRange
          ? t("video.postprocess.selectedRange", {
              start: formatPreciseSeconds(selectedRange.start, locale, t),
              end: formatPreciseSeconds(selectedRange.end, locale, t),
              duration: formatPreciseSeconds(selectedRange.end - selectedRange.start, locale, t),
            })
          : t("video.postprocess.markerGapRequired", {
              gap: formatPreciseSeconds(MARKER_MIN_GAP_SECS, locale, t),
            });
  const timelineReadout = videoPlaying ? playbackTimeSecs : markerValue;

  if (pushes.length === 0) {
    return (
      <section className="a4n-flat-surface w-full self-start">
        <div className="a4n-empty-state min-h-[180px] px-4 py-6">
          <div className="flex h-9 w-9 items-center justify-center rounded-full bg-book-bg-tertiary text-book-primary">
            <Scissors size={16} />
          </div>
          <div className="max-w-sm">
            <h3 className="text-sm font-medium text-book-text-main">
              {t("video.postprocess.emptyTitle")}
            </h3>
            <p className="mt-1 text-xs leading-relaxed text-book-text-muted">
              {t("video.postprocess.emptyDescription")}
            </p>
          </div>
        </div>
      </section>
    );
  }

  return (
    <div className="grid min-h-0 flex-1 gap-3 xl:grid-cols-[220px_minmax(0,1fr)]">
      <section className="a4n-flat-surface flex min-h-0 flex-col overflow-hidden">
        <div className="flex shrink-0 items-baseline justify-between gap-3 border-b border-book-border/35 px-3 py-2.5">
          <h3 className="text-xs font-medium text-book-text-main">{t("video.postprocess.pushBatches")}</h3>
          <span className="text-caption text-book-text-muted">
            {t("video.postprocess.batchCount", { count: pushes.length })}
          </span>
        </div>
        <ul className="flex min-h-0 divide-x divide-book-border/35 overflow-x-auto custom-scrollbar xl:flex-1 xl:flex-col xl:divide-x-0 xl:divide-y xl:overflow-x-hidden xl:overflow-y-auto">
          {pushes.map((push) => {
            const active = selectedPush?.id === push.id;
            const status = assemblyPushStatus(push);
            const pushIndex = assemblyPushIndex(push);
            const sourceIds = assemblyPushSourceIds(push);
            const statusTone =
              status === "accepted"
                ? "text-success"
                : status === "rejected"
                  ? "text-error"
                  : status === "pending"
                    ? "text-warning"
                    : "text-book-text-muted";
            return (
              <li
                key={push.id}
                className="min-w-[148px] shrink-0 xl:min-w-0 xl:w-full"
              >
                <button
                  type="button"
                  onClick={() => setSelectedPushId(push.id)}
                  disabled={acting != null}
                  data-video-push-id={push.id}
                  data-active={active ? "1" : undefined}
                  aria-current={active ? "true" : undefined}
                  className={`w-full border-b-2 px-3 py-2.5 text-left transition-colors disabled:cursor-not-allowed disabled:opacity-60 xl:border-b-0 xl:border-l-2 ${
                    active
                      ? "border-book-primary bg-book-primary/[0.07]"
                      : "border-transparent hover:bg-book-bg-tertiary/35"
                  }`}
                >
                  <div className="text-xs font-medium text-book-text-main">
                    {pushIndex == null ? t("video.postprocess.invalidBatch") : t("video.chapter.timelineItem", { number: pushIndex })}
                  </div>
                  <div className="mt-1 text-caption text-book-text-muted">
                    {sourceIds == null
                      ? t("video.postprocess.invalidSource")
                      : t("video.postprocess.segmentCount", { count: sourceIds.length })}
                    <span className="mx-1">·</span>
                    <span className={statusTone}>
                      {status == null ? t("video.postprocess.invalidStatus") : t(PUSH_STATUS_LABELS[status])}
                    </span>
                  </div>
                </button>
              </li>
            );
          })}
        </ul>
      </section>

      <div className="min-h-0 space-y-3 overflow-y-auto pb-6 pr-1 custom-scrollbar xl:grid xl:grid-rows-[minmax(0,1fr)_auto] xl:gap-3 xl:space-y-0 xl:overflow-hidden xl:pb-0">
        <section className="a4n-flat-surface flex min-h-[clamp(380px,58vh,640px)] flex-col p-3 xl:min-h-0">
          <div className="mb-2 flex min-w-0 flex-wrap items-center gap-2 border-b border-book-border/35 pb-2">
            <div className="min-w-0 flex-1">
              <h3 className="text-sm font-medium text-book-text-main">
                {t("video.postprocess.previewTitle")}
              </h3>
              {currentVideo && selectedPush ? (
                <p className="mt-1 text-caption leading-relaxed text-book-text-muted">
                  {selectedPushIndex == null
                    ? t("video.postprocess.invalidBatch")
                    : t("video.chapter.timelineItem", { number: selectedPushIndex })}{" "}
                  ·{" "}
                  {selectedPushStatus == null
                    ? t("video.postprocess.invalidStatus")
                    : t(PUSH_STATUS_LABELS[selectedPushStatus])}{" "}
                  · {formatVideoDuration(currentVideo.duration_secs, locale, t)} · {t("video.postprocess.version", { number: versions.length + 1 })}
                  {` · ${formatVideoCreatedAt(currentVideo.updated_at, locale, t)}`}
                </p>
              ) : null}
            </div>
            {statusActing ? (
              <span
                role="status"
                className="inline-flex h-8 w-8 items-center justify-center text-book-text-muted"
              >
                <LoadingSpinner size="sm" />
              </span>
            ) : (
              <>
                {acting == null && statusActionItems.length > 0 ? (
                  <Dropdown items={statusActionItems} />
                ) : null}
                {editableLatest && selectedPushStatus === "pending" ? (
                  <BookButton
                    variant="primary"
                    size="sm"
                    onClick={() => void handlePushStatus("accept")}
                    disabled={acting != null}
                    className="!min-h-8 !px-3"
                  >
                    <Check size={12} />
                    {t("video.postprocess.acceptPush")}
                  </BookButton>
                ) : null}
              </>
            )}
          </div>

          <div className="relative min-h-0 flex-1 overflow-hidden rounded-2xl border border-book-border/50 bg-black">
            {videoPlaybackSrc ? (
              <video
                key={videoPlaybackSrc}
                ref={playbackVideoRef}
                data-video-id={currentVideo?.id}
                controls
                playsInline
                preload="auto"
                src={videoPlaybackSrc}
                className="h-full min-h-[280px] w-full bg-black object-contain"
                onLoadStart={() => {
                  // 换源/重载后旧的 in-flight seek 永远不会有 seeked 回调，
                  // 必须重置，否则 seek 泵会永久死锁；desiredSeekTime 保留，
                  // 元数据就绪后恢复到原位置。
                  seekInFlightRef.current = false;
                  scrubPreviewActiveRef.current = false;
                  draggingScrubRef.current = false;
                  pendingDragReleaseRef.current = false;
                  setDraggingMarkerIndex(null);
                  cancelPresentationWait();
                  setScrubPreviewVisible(false);
                  handleVideoLoadStart();
                }}
                onLoadedMetadata={(event) => {
                  const mediaDuration = event.currentTarget.duration;
                  setPlaybackTimeSecs(event.currentTarget.currentTime);
                  setDecodedDurationSecs(
                    Number.isFinite(mediaDuration) && mediaDuration > 0
                      ? mediaDuration
                      : null,
                  );
                  handleVideoLoaded();
                  if (desiredSeekTimeRef.current != null) schedulePendingSeek();
                }}
                onLoadedData={handleVideoLoaded}
                onCanPlay={handleVideoLoaded}
                onTimeUpdate={(event) =>
                  setPlaybackTimeSecs(event.currentTarget.currentTime)
                }
                onSeeked={handleScrubSeeked}
                onPlay={() => {
                  if (
                    scrubPreviewActiveRef.current &&
                    (draggingScrubRef.current ||
                      desiredSeekTimeRef.current != null ||
                      pendingDragReleaseRef.current)
                  ) {
                    playbackVideoRef.current?.pause();
                    return;
                  }
                  cancelSeekPump();
                  setVideoPlaying(true);
                }}
                onPause={(event) => {
                  setVideoPlaying(false);
                  const time = event.currentTarget.currentTime;
                  setPlaybackTimeSecs(time);
                  // scrub 过程中的暂停由 seekVideo 触发，markerValue 已是目标值；
                  // 只在用户经原生控件暂停时同步，保证竖线基准跟随画面。
                  if (!scrubPreviewActiveRef.current) setMarkerValue(time);
                }}
                onError={(event) => {
                  if (!handleVideoError(event)) return;
                  seekInFlightRef.current = false;
                  cancelPresentationWait();
                  setScrubPreviewVisible(false);
                }}
              />
            ) : (
              <div className="flex h-full min-h-[280px] items-center justify-center bg-book-bg-tertiary">
                <EmptyHint
                  text={
                    currentVideo ? t("video.postprocess.loadingVideo") : t("video.postprocess.noPushVideo")
                  }
                />
              </div>
            )}
            {videoPlaybackSrc ? (
              <img
                ref={scrubFrameImageRef}
                alt=""
                aria-hidden="true"
                data-scrub-preview-visible={
                  scrubPreviewVisible ? "true" : "false"
                }
                className={`pointer-events-none absolute inset-0 z-[2] h-full min-h-[280px] w-full bg-black object-contain transition-none ${
                  scrubPreviewVisible ? "opacity-100" : "opacity-0"
                }`}
              />
            ) : null}
            {videoLoading && (
              <div className="pointer-events-none absolute inset-0 z-20 flex items-center justify-center bg-black/35 text-xs text-white">
                <span className="inline-flex items-center gap-1.5 rounded-xl bg-black/55 px-2.5 py-1.5 backdrop-blur-sm">
                  <LoadingSpinner size="sm" />
                  {t("video.postprocess.loadingVideo")}
                </span>
              </div>
            )}
          </div>
          {videoLoadError && (
            <div
              role="alert"
              className="mt-2 flex flex-wrap items-center gap-2 rounded-xl border border-error/40 bg-error-bg px-2.5 py-2 text-small text-error"
            >
              <span className="min-w-0 flex-1">{videoLoadError}</span>
              <BookButton
                variant="secondary"
                size="sm"
                onClick={retryVideoLoad}
              >
                <RefreshCw size={12} />
                {t("video.chapter.retryVideoLoad")}
              </BookButton>
            </div>
          )}
        </section>

        <section className="a4n-flat-surface shrink-0 px-3 py-2">
          <div className="mb-1.5 flex min-h-10 min-w-0 flex-wrap items-center gap-x-3 gap-y-1.5 border-b border-book-border/35 pb-1.5 xl:flex-nowrap">
            <div className="min-w-0">
              <h3 className="text-xs font-medium text-book-text-main">
                {t("video.postprocess.editBar")}
              </h3>
              {editDisabledReason ? (
                <p className="mt-0.5 text-caption text-book-text-muted">
                  {editDisabledReason}
                </p>
              ) : null}
            </div>
            {!editActing && !editDisabledReason && currentVideo ? (
              <div className="ml-auto flex min-h-8 min-w-0 flex-wrap items-center justify-end gap-2 xl:flex-nowrap">
                <p
                  className="min-w-0 text-caption leading-none text-book-text-muted xl:whitespace-nowrap"
                  title={markerSummary}
                >
                  {markerSummary}
                </p>
                {acting == null && editActionItems.length > 0 ? (
                  <Dropdown items={editActionItems} />
                ) : null}
                {singleMarker != null ? (
                  <BookButton
                    variant="primary"
                    size="sm"
                    onClick={() => void applyUploadEdit("insert")}
                    disabled={acting != null}
                    className="!min-h-8 !px-3 text-small"
                  >
                    <Upload size={12} />
                    {t("video.postprocess.uploadInsert")}
                  </BookButton>
                ) : selectedRange ? (
                  <BookButton
                    variant="primary"
                    size="sm"
                    onClick={() => void applyUploadEdit("replace")}
                    disabled={acting != null}
                    className="!min-h-8 !px-3 text-small"
                  >
                    <Upload size={12} />
                    {t("video.postprocess.uploadReplace")}
                  </BookButton>
                ) : null}
              </div>
            ) : null}
          </div>

          {editActing ? (
            <div
              role="status"
              className="flex min-h-24 items-center justify-center gap-2 text-xs text-book-text-muted"
            >
              <LoadingSpinner size="sm" />
              {t("video.postprocess.applyingEdit")}
            </div>
          ) : editDisabledReason || !currentVideo ? (
            <div className="flex min-h-20 items-center justify-center text-xs text-book-text-muted">
              {t("video.postprocess.currentBatchNotEditable")}
            </div>
          ) : (
            <VideoPostProcessTimeline
              locale={locale}
              t={t}
              duration={duration}
              timelineReadout={timelineReadout}
              formatPreciseSeconds={formatPreciseSeconds}
              timelineRef={timelineRef}
              markers={markers}
              videoPlaying={videoPlaying}
              playbackTimeSecs={playbackTimeSecs}
              selectedMarkerIndex={selectedMarkerIndex}
              draggingMarkerIndex={draggingMarkerIndex}
              selectedRange={selectedRange}
              markerValueFromClientX={markerValueFromClientX}
              addMarker={addMarker}
              constrainMarkerValue={constrainMarkerValue}
              beginScrub={beginScrub}
              seekVideo={seekVideo}
              finishScrub={finishScrub}
              handleTimelineClick={handleTimelineClick}
              setMarkers={setMarkers}
              setSelectedMarkerIndex={setSelectedMarkerIndex}
              setDraggingMarkerIndex={setDraggingMarkerIndex}
              draggingScrubRef={draggingScrubRef}
              pendingDragReleaseRef={pendingDragReleaseRef}
              desiredSeekTimeRef={desiredSeekTimeRef}
            />
          )}
        </section>
      </div>
    </div>
  );
};
