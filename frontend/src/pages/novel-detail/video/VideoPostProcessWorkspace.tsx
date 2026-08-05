import React, {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { Check, Play, RotateCcw, Scissors, Upload, X } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { extractErrorMessage } from "../../../api";
import {
  applyAssemblyPostprocessEdit,
  updateAssemblyPushStatus,
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
import {
  ASSEMBLY_WORKFLOW_MODE,
  assemblyPushIndex,
  assemblyPushSourceIds,
  assemblyPushStatus,
  formatVideoCreatedAt,
} from "./videoTabUtils";

interface VideoPostProcessWorkspaceProps {
  projectId: number;
  chapter: ChapterRow;
  videos: GeneratedVideo[];
  onToast: (message: unknown, type: ToastType) => void;
  onRefresh: () => Promise<void> | void;
}

const ASSEMBLY_PUSH_VIDEO_TYPE = "assembly_push";
const POSTPROCESS_EDIT_VIDEO_TYPE = "assembly_postprocess_edit";
const MARKER_MIN_GAP_SECS = 0.05;
const VIDEO_SEEK_TOLERANCE_SECS = 0.05;

const formatSeconds = (value: number): string => {
  if (!Number.isFinite(value) || value <= 0) return "0s";
  if (value < 60) return `${Math.round(value)}s`;
  const minutes = Math.floor(value / 60);
  const seconds = Math.round(value - minutes * 60);
  return `${minutes}m ${seconds}s`;
};

const editPushId = (video: GeneratedVideo): number | null => {
  const value = asRecord(
    asRecord(video.metadata_json)?.postprocess_edit,
  )?.push_video_id;
  return Number.isInteger(value) && Number(value) > 0 ? Number(value) : null;
};

const editOperationLabel: Record<string, string> = {
  insert: "插入",
  delete: "删除",
  replace: "替换",
};

const PUSH_STATUS_LABELS: Record<string, string> = {
  pending: "待确认",
  accepted: "已同意",
  rejected: "已拒绝",
};

export const VideoPostProcessWorkspace: React.FC<
  VideoPostProcessWorkspaceProps
> = ({ projectId, chapter, videos, onToast, onRefresh }) => {
  const pushes = useMemo(
    () =>
      videos
        .filter(
          (video) =>
            video.video_type === ASSEMBLY_PUSH_VIDEO_TYPE &&
            video.workflow_mode === ASSEMBLY_WORKFLOW_MODE &&
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
        video.video_type !== POSTPROCESS_EDIT_VIDEO_TYPE ||
        video.workflow_mode !== ASSEMBLY_WORKFLOW_MODE ||
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
        const left = a.created_at ?? "";
        const right = b.created_at ?? "";
        return left.localeCompare(right) || a.id - b.id;
      });
    }
    return map;
  }, [videos]);

  const latestPush = useMemo(
    () =>
      [...pushes]
        .reverse()
        .find((push) => assemblyPushStatus(push) !== "rejected") ?? null,
    [pushes],
  );
  const [selectedPushId, setSelectedPushId] = useState<number | null>(null);
  const selectedPush =
    selectedPushId == null
      ? null
      : (pushes.find((push) => push.id === selectedPushId) ?? null);
  const versions = selectedPush
    ? (editsByPushId.get(selectedPush.id) ?? [])
    : [];
  const currentVideo = versions[versions.length - 1] ?? selectedPush;
  const {
    videoPlaybackSrc,
    videoLoadError,
    videoLoading,
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
  const [scrubFrameReady, setScrubFrameReady] = useState(false);
  const playbackVideoRef = useRef<HTMLVideoElement | null>(null);
  const scrubCanvasRef = useRef<HTMLCanvasElement | null>(null);
  const scrubCanvasUsableRef = useRef<boolean | null>(null);
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
    cancelPresentationWait();
    desiredSeekTimeRef.current = null;
    seekInFlightRef.current = false;
    scrubPreviewActiveRef.current = false;
    draggingScrubRef.current = false;
    pendingDragReleaseRef.current = false;
  }, [cancelPresentationWait]);
  const waitForPresentedFrame = useCallback(
    (video: HTMLVideoElement, onPresented: () => void) => {
      cancelPresentationWait();
      const token = presentationTokenRef.current;
      let finished = false;
      const finish = () => {
        if (finished || presentationTokenRef.current !== token) return;
        finished = true;
        presentationRafRef.current = null;
        presentationVideoRef.current = null;
        presentationCallbackRef.current = null;
        if (presentationTimeoutRef.current != null) {
          clearTimeout(presentationTimeoutRef.current);
          presentationTimeoutRef.current = null;
        }
        onPresented();
      };
      // `seeked` only means that the media timeline moved. WebKitGTK can fire
      // it before the compositor has submitted the target frame. Prefer the
      // frame callback, which is defined to run after a frame is presented.
      if (typeof video.requestVideoFrameCallback === "function") {
        presentationVideoRef.current = video;
        presentationCallbackRef.current = video.requestVideoFrameCallback(
          finish,
        );
      } else {
        // Older WebKitGTK has no frame callback. Two animation frames allow
        // the paused native video layer to repaint without restarting it.
        presentationRafRef.current = requestAnimationFrame(() => {
          presentationRafRef.current = requestAnimationFrame(finish);
        });
      }
      // A paused video may not deliver a frame callback on older WebKitGTK.
      // Keep the seek state from being stuck forever, while retaining the
      // native frame instead of clearing the preview during the wait.
      presentationTimeoutRef.current = setTimeout(finish, 250);
    },
    [cancelPresentationWait],
  );
  const drawScrubFrame = useCallback((): boolean => {
    if (scrubCanvasUsableRef.current === false) return false;
    const video = playbackVideoRef.current;
    const canvas = scrubCanvasRef.current;
    if (
      !video ||
      !canvas ||
      video.readyState < HTMLMediaElement.HAVE_CURRENT_DATA ||
      video.videoWidth <= 0 ||
      video.videoHeight <= 0
    ) {
      return false;
    }
    if (canvas.width !== video.videoWidth) canvas.width = video.videoWidth;
    if (canvas.height !== video.videoHeight) canvas.height = video.videoHeight;
    const context = canvas.getContext("2d");
    if (!context) return false;
    context.drawImage(video, 0, 0, canvas.width, canvas.height);
    try {
      // WebKitGTK can expose a decoded video frame to the video element while
      // returning a fully transparent canvas surface. Do not put that surface
      // above the native video or it will look like a frozen/black preview.
      const alpha = context.getImageData(0, 0, 1, 1).data[3];
      scrubCanvasUsableRef.current = alpha > 0;
    } catch {
      scrubCanvasUsableRef.current = false;
    }
    if (scrubCanvasUsableRef.current === false) {
      setScrubPreviewVisible(false);
      setScrubFrameReady(false);
      return false;
    }
    setScrubPreviewVisible(true);
    setScrubFrameReady(true);
    return true;
  }, []);
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
    setScrubPreviewVisible(false);
    setScrubFrameReady(false);
    const userAgent =
      typeof navigator === "undefined" ? "" : navigator.userAgent;
    scrubCanvasUsableRef.current =
      !/AppleWebKit/i.test(userAgent) || /Chrome|Chromium/i.test(userAgent);
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
      if (current != null && pushes.some((push) => push.id === current))
        return current;
      return latestPush?.id ?? pushes[0]?.id ?? null;
    });
  }, [latestPush?.id, pushes]);

  const duration = Math.max(
    0.1,
    decodedDurationSecs ?? currentVideo?.duration_secs ?? 0,
  );
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
    ? "没有可编辑推送"
    : selectedPushIndex == null ||
        selectedPushStatus == null ||
        selectedPushSourceIds == null
      ? "推送记录无效，不能编辑"
      : selectedPushStatus === "rejected"
        ? "已拒绝推送不能编辑"
        : !editableLatest
          ? "只能编辑当前最新推送"
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
      video.currentTime = target;
    } else {
      desiredSeekTimeRef.current = null;
      if (pendingDragReleaseRef.current) {
        draggingScrubRef.current = false;
        pendingDragReleaseRef.current = false;
      }
      if (draggingScrubRef.current) {
        drawScrubFrame();
      }
    }
  }, [drawScrubFrame, duration]);
  const schedulePendingSeek = useCallback(() => {
    if (seekRafRef.current != null) return;
    seekRafRef.current = requestAnimationFrame(applyPendingSeek);
  }, [applyPendingSeek]);
  const handleScrubSeeked = useCallback(() => {
    seekInFlightRef.current = false;
    const video = playbackVideoRef.current;
    const desiredTime = desiredSeekTimeRef.current;
    if (!video) return;
    const seekVideoElement = video;
    if (scrubPreviewActiveRef.current && draggingScrubRef.current) {
      drawScrubFrame();
    }
    const finish = () => {
      const latestVideo = playbackVideoRef.current;
      if (!latestVideo) return;
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
        }
      }
      if (!scrubPreviewActiveRef.current) return;
      setPlaybackTimeSecs(latestVideo.currentTime);
      if (!draggingScrubRef.current) drawScrubFrame();
    };
    if (desiredTime != null && !draggingScrubRef.current) {
      waitForPresentedFrame(seekVideoElement, finish);
    } else {
      finish();
    }
  }, [drawScrubFrame, schedulePendingSeek, waitForPresentedFrame]);
  const seekVideo = useCallback(
    (value: number) => {
      const target = Math.max(0, Math.min(value, duration));
      playbackVideoRef.current?.pause();
      setMarkerValue(target);
      if (scrubPreviewActiveRef.current && !draggingScrubRef.current) {
        setScrubFrameReady(false);
      }
      desiredSeekTimeRef.current = target;
      schedulePendingSeek();
    },
    [duration, schedulePendingSeek],
  );
  const finishScrub = useCallback(() => {
    if (seekRafRef.current != null) cancelAnimationFrame(seekRafRef.current);
    applyPendingSeek();
  }, [applyPendingSeek]);
  const beginScrub = useCallback((preserveCurrentFrame = true) => {
    scrubPreviewActiveRef.current = true;
    playbackVideoRef.current?.pause();
    setScrubFrameReady(false);
    if (preserveCurrentFrame) drawScrubFrame();
  }, [drawScrubFrame]);
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
    beginScrub(false);
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
          ? "已同意最新推送"
          : action === "reject"
            ? "已拒绝最新推送"
            : "已撤销最新推送同意",
        "success",
      );
    } catch (err) {
      if (!isCurrentScope(requestScope)) return;
      onToast(extractErrorMessage(err, "更新推送状态失败"), "error");
    } finally {
      finishAction(requestScope);
      if (isCurrentScope(requestScope)) setActing(null);
    }
  };

  const applyUploadEdit = async (operation: "insert" | "replace") => {
    if (!selectedPush || editDisabledReason) return;
    const requestScope = scopeKey;
    if (operation === "insert" && singleMarker == null) {
      onToast("插入操作需要先添加一根时间线标记", "error");
      return;
    }
    if (operation === "replace" && !selectedRange) {
      onToast("替换操作需要先添加两根时间线标记", "error");
      return;
    }
    if (!beginAction()) return;
    setActing(`edit-${operation}`);
    let upload: VideoUploadFile | null = null;
    try {
      upload = await pickVideoUploadFile();
    } catch (err) {
      if (!isCurrentScope(requestScope)) return;
      onToast(extractErrorMessage(err, "打开文件选择器失败"), "error");
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
      onToast(`已${editOperationLabel[operation]}上传片段`, "success");
    } catch (err) {
      if (!isCurrentScope(requestScope)) return;
      onToast(extractErrorMessage(err, "应用后处理编辑失败"), "error");
    } finally {
      finishAction(requestScope);
      if (isCurrentScope(requestScope)) setActing(null);
    }
  };

  const applyDelete = async () => {
    if (!selectedPush || editDisabledReason || !selectedRange) {
      onToast("删除操作需要先在最新推送上添加两根时间线标记", "error");
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
      onToast("已删除选中片段", "success");
    } catch (err) {
      if (!isCurrentScope(requestScope)) return;
      onToast(extractErrorMessage(err, "删除后处理片段失败"), "error");
    } finally {
      finishAction(requestScope);
      if (isCurrentScope(requestScope)) setActing(null);
    }
  };

  const statusActionItems: React.ComponentProps<typeof Dropdown>["items"] =
    editableLatest && selectedPushStatus === "pending"
      ? [
          {
            label: "拒绝此推送",
            icon: <X size={12} />,
            danger: true,
            onClick: () => void handlePushStatus("reject"),
          },
        ]
      : editableLatest && selectedPushStatus === "accepted"
        ? [
            {
              label: "撤销同意",
              icon: <RotateCcw size={12} />,
              onClick: () => void handlePushStatus("undo"),
            },
          ]
        : [];
  const editActionItems: React.ComponentProps<typeof Dropdown>["items"] = [
    ...(markers.length > 0
      ? [
          {
            label: "清除竖线",
            icon: <RotateCcw size={12} />,
            onClick: clearMarkers,
          },
        ]
      : []),
    ...(selectedRange
      ? [
          { type: "divider" as const },
          {
            label: "删除选中区间",
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
      ? "点击剪辑条添加插入点；添加第二根竖线可选择区间。"
      : singleMarker != null
        ? `插入位置 ${singleMarker.toFixed(2)}s`
        : selectedRange
          ? `已选择 ${selectedRange.start.toFixed(2)}s–${selectedRange.end.toFixed(2)}s · ${(
              selectedRange.end - selectedRange.start
            ).toFixed(2)}s`
          : `两根竖线至少需要间隔 ${MARKER_MIN_GAP_SECS.toFixed(2)}s`;
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
              还没有待处理的成片
            </h3>
            <p className="mt-1 text-xs leading-relaxed text-book-text-muted">
              从成片合成阶段推送预览后，才能在这里确认和调整剪辑。
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
          <h3 className="text-xs font-medium text-book-text-main">推送批次</h3>
          <span className="text-caption text-book-text-muted">
            {pushes.length} 批
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
                  data-active={active ? "1" : undefined}
                  aria-current={active ? "true" : undefined}
                  className={`w-full border-b-2 px-3 py-2.5 text-left transition-colors disabled:cursor-not-allowed disabled:opacity-60 xl:border-b-0 xl:border-l-2 ${
                    active
                      ? "border-book-primary bg-book-primary/[0.07]"
                      : "border-transparent hover:bg-book-bg-tertiary/35"
                  }`}
                >
                  <div className="text-xs font-medium text-book-text-main">
                    {pushIndex == null ? "批次无效" : `批次 ${pushIndex}`}
                  </div>
                  <div className="mt-1 text-caption text-book-text-muted">
                    {sourceIds == null
                      ? "来源无效"
                      : `${sourceIds.length} 个片段`}
                    <span className="mx-1">·</span>
                    <span className={statusTone}>
                      {status == null ? "状态无效" : PUSH_STATUS_LABELS[status]}
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
                后处理预览
              </h3>
              {currentVideo && selectedPush ? (
                <p className="mt-1 text-caption leading-relaxed text-book-text-muted">
                  {selectedPushIndex == null
                    ? "批次无效"
                    : `批次 ${selectedPushIndex}`}{" "}
                  ·{" "}
                  {selectedPushStatus == null
                    ? "状态无效"
                    : PUSH_STATUS_LABELS[selectedPushStatus]}{" "}
                  · {formatSeconds(currentVideo.duration_secs)} · 第{" "}
                  {versions.length + 1} 版
                  {currentVideo.updated_at
                    ? ` · ${formatVideoCreatedAt(currentVideo.updated_at)}`
                    : ""}
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
                    同意推送
                  </BookButton>
                ) : null}
              </>
            )}
          </div>

          <div className="relative min-h-0 flex-1 overflow-hidden rounded-2xl border border-book-border/50 bg-black">
            {videoPlaybackSrc ? (
              <video
                ref={playbackVideoRef}
                controls
                playsInline
                preload="auto"
                src={videoPlaybackSrc}
                className="h-full min-h-[280px] w-full bg-black object-contain"
                onLoadStart={handleVideoLoadStart}
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
                  setScrubPreviewVisible(false);
                  setScrubFrameReady(false);
                  setVideoPlaying(true);
                }}
                onPause={() => setVideoPlaying(false)}
                onError={handleVideoError}
              />
            ) : (
              <div className="flex h-full min-h-[280px] items-center justify-center bg-book-bg-tertiary">
                <EmptyHint
                  text={
                    currentVideo ? "正在加载后处理视频。" : "暂无推送视频。"
                  }
                />
              </div>
            )}
            {videoPlaybackSrc ? (
              <canvas
                ref={scrubCanvasRef}
                aria-hidden="true"
                data-scrub-preview-visible={
                  scrubFrameReady ? "true" : "false"
                }
                className={`pointer-events-none absolute inset-0 z-[2] h-full min-h-[280px] w-full object-contain transition-none ${
                  scrubPreviewVisible ? "opacity-100" : "opacity-0"
                }`}
              />
            ) : null}
            {scrubFrameReady ? (
              <button
                type="button"
                aria-label="播放后处理预览"
                title="播放"
                onClick={() => {
                  const video = playbackVideoRef.current;
                  if (!video) return;
                  cancelSeekPump();
                  setScrubPreviewVisible(false);
                  setVideoPlaying(true);
                  void video.play().catch((error) => {
                    setVideoPlaying(false);
                    scrubPreviewActiveRef.current = true;
                    setScrubPreviewVisible(true);
                    onToast(
                      extractErrorMessage(error, "播放视频失败"),
                      "error",
                    );
                  });
                }}
                className="absolute left-1/2 top-1/2 z-[3] flex h-12 w-12 -translate-x-1/2 -translate-y-1/2 items-center justify-center rounded-full bg-black/55 text-white transition-colors hover:bg-black/70 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-white/80"
              >
                <Play size={22} fill="currentColor" />
              </button>
            ) : null}
            {videoLoading && (
              <div className="pointer-events-none absolute inset-0 z-20 flex items-center justify-center bg-black/35 text-xs text-white">
                <span className="inline-flex items-center gap-1.5 rounded-xl bg-black/55 px-2.5 py-1.5 backdrop-blur-sm">
                  <LoadingSpinner size="sm" />
                  正在加载视频
                </span>
              </div>
            )}
          </div>
          {videoLoadError && (
            <div
              role="alert"
              className="mt-2 rounded-xl border border-error/40 bg-error-bg px-2.5 py-2 text-small text-error"
            >
              {videoLoadError}
            </div>
          )}
        </section>

        <section className="a4n-flat-surface shrink-0 px-3 py-2">
          <div className="mb-1.5 flex min-h-10 min-w-0 flex-wrap items-center gap-x-3 gap-y-1.5 border-b border-book-border/35 pb-1.5 xl:flex-nowrap">
            <div className="min-w-0">
              <h3 className="text-xs font-medium text-book-text-main">
                剪辑条
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
                    上传并插入
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
                    上传并替换
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
              正在应用后处理编辑
            </div>
          ) : editDisabledReason || !currentVideo ? (
            <div className="flex min-h-20 items-center justify-center text-xs text-book-text-muted">
              当前批次不可编辑。
            </div>
          ) : (
            <div>
              <div className="grid grid-cols-3 text-caption tabular-nums text-book-text-muted">
                <span>0s</span>
                <span className="justify-self-center">
                  {timelineReadout.toFixed(2)}s
                </span>
                <span className="justify-self-end">{duration.toFixed(2)}s</span>
              </div>
              <div
                ref={timelineRef}
                role="button"
                aria-label="视频剪辑时间轴"
                tabIndex={0}
                aria-disabled={markers.length >= 2 || undefined}
                title={
                  markers.length >= 2
                    ? "已添加两根竖线，请先清除"
                    : "点击剪辑条添加竖线"
                }
                onClick={handleTimelineClick}
                onKeyDown={(event) => {
                  if (event.key === "Enter" || event.key === " ") {
                    event.preventDefault();
                    addMarker();
                  }
                }}
                className={`relative mt-1 h-8 rounded-lg bg-book-bg-tertiary ${
                  markers.length < 2 ? "cursor-crosshair" : "cursor-default"
                }`}
              >
                {videoPlaying ? (
                  <span
                    aria-hidden="true"
                    data-playback-head="true"
                    className="pointer-events-none absolute -top-1 z-[1] h-10 w-px bg-white/80"
                    style={{
                      left: `${Math.min(
                        100,
                        Math.max(0, (playbackTimeSecs / duration) * 100),
                      )}%`,
                    }}
                  />
                ) : null}
                {markers.map((marker, index) => (
                  <span
                    key={`marker-${index}`}
                    role="slider"
                    tabIndex={0}
                    aria-label={`剪辑竖线 ${index + 1}`}
                    aria-keyshortcuts="Delete Backspace"
                    aria-valuemin={0}
                    aria-valuemax={duration}
                    aria-valuenow={Number(marker.toFixed(2))}
                    onFocus={() => setSelectedMarkerIndex(index)}
                    onPointerDown={(event) => {
                      event.preventDefault();
                      event.stopPropagation();
                      if (event.isTrusted) {
                        event.currentTarget.setPointerCapture(event.pointerId);
                      }
                      setSelectedMarkerIndex(index);
                      beginScrub();
                      draggingScrubRef.current = true;
                      setDraggingMarkerIndex(index);
                      seekVideo(marker);
                    }}
                    onPointerMove={(event) => {
                      if (
                        draggingMarkerIndex !== index &&
                        !event.currentTarget.hasPointerCapture(event.pointerId)
                      ) {
                        return;
                      }
                      const value = markerValueFromClientX(event.clientX);
                      if (value == null) return;
                      const constrainedValue = constrainMarkerValue(
                        index,
                        value,
                      );
                      seekVideo(constrainedValue);
                      setMarkers((prev) =>
                        prev.map((item, itemIndex) =>
                          itemIndex === index ? constrainedValue : item,
                        ),
                      );
                    }}
                    onPointerUp={(event) => {
                      if (
                        draggingMarkerIndex !== index &&
                        !event.currentTarget.hasPointerCapture(event.pointerId)
                      ) {
                        return;
                      }
                      const value = markerValueFromClientX(event.clientX);
                      if (value != null) {
                        const constrainedValue = constrainMarkerValue(
                          index,
                          value,
                        );
                        seekVideo(constrainedValue);
                        setMarkers((prev) =>
                          prev.map((item, itemIndex) =>
                            itemIndex === index ? constrainedValue : item,
                          ),
                        );
                      }
                      if (
                        event.currentTarget.hasPointerCapture(event.pointerId)
                      ) {
                        event.currentTarget.releasePointerCapture(
                          event.pointerId,
                        );
                      }
                      setDraggingMarkerIndex(null);
                      finishScrub();
                      pendingDragReleaseRef.current = true;
                      if (desiredSeekTimeRef.current == null) {
                        draggingScrubRef.current = false;
                        pendingDragReleaseRef.current = false;
                      }
                    }}
                    onPointerCancel={(event) => {
                      if (
                        event.currentTarget.hasPointerCapture(event.pointerId)
                      ) {
                        event.currentTarget.releasePointerCapture(
                          event.pointerId,
                        );
                      }
                      setDraggingMarkerIndex(null);
                      draggingScrubRef.current = false;
                      finishScrub();
                    }}
                    onKeyDown={(event) => {
                      const step = event.shiftKey ? 1 : 0.05;
                      if (
                        event.key !== "ArrowLeft" &&
                        event.key !== "ArrowRight"
                      )
                        return;
                      event.preventDefault();
                      const direction = event.key === "ArrowLeft" ? -1 : 1;
                      const value = constrainMarkerValue(
                        index,
                        marker + direction * step,
                      );
                      beginScrub();
                      seekVideo(value);
                      finishScrub();
                      setMarkers((prev) =>
                        prev.map((item, itemIndex) =>
                          itemIndex === index ? value : item,
                        ),
                      );
                    }}
                    className="absolute -top-1 h-10 w-5 -translate-x-1/2 cursor-ew-resize touch-none"
                    style={{
                      left: `${Math.min(100, Math.max(0, (marker / duration) * 100))}%`,
                    }}
                    title={`${marker.toFixed(2)}s`}
                  >
                    <span className="absolute left-1/2 top-1 h-8 w-0.5 -translate-x-1/2 bg-book-primary" />
                    <span
                      className={`absolute left-1/2 top-1.5 h-3 w-3 -translate-x-1/2 rounded-full border border-book-primary ${
                        selectedMarkerIndex === index
                          ? "bg-book-primary"
                          : "bg-book-bg-paper"
                      }`}
                    />
                  </span>
                ))}
                {selectedRange && (
                  <span
                    className="pointer-events-none absolute top-2 h-4 rounded-md bg-book-primary/25"
                    style={{
                      left: `${(selectedRange.start / duration) * 100}%`,
                      width: `${((selectedRange.end - selectedRange.start) / duration) * 100}%`,
                    }}
                  />
                )}
              </div>
            </div>
          )}
        </section>
      </div>
    </div>
  );
};
