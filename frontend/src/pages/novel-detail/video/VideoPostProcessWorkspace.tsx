import React, { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import {
  Check,
  Film,
  Loader2,
  RotateCcw,
  Scissors,
  Upload,
  X,
} from "lucide-react";
import { extractErrorMessage } from "../../../api";
import {
  applyAssemblyPostprocessEdit,
  updateAssemblyPushStatus,
  type GeneratedVideo,
  type VideoWorkflowMode,
} from "../../../api/video";
import type { ToastType } from "../../../components/feedback/Toast";
import { BookButton } from "../../../components/ui/BookButton";
import { BookCard } from "../../../components/ui/BookCard";
import { asJsonRecord as asRecord } from "../../../utils/jsonRecord";
import { pickLocalFile, readFileAsBase64 } from "../../../utils/fileUpload";
import {
  MAX_UPLOAD_VIDEO_BYTES,
  supportedVideoAccept,
  videoMimeForUploadFile,
} from "../../../utils/videoUpload";
import { EmptyHint, type ChapterRow } from "../shared";
import { statusClass } from "./videoGenerationDisplay";
import { useVideoPlaybackSource } from "./useVideoPlaybackSource";
import { formatVideoCreatedAt } from "./videoTabUtils";

interface VideoPostProcessWorkspaceProps {
  projectId: number;
  chapter: ChapterRow;
  workflowMode: VideoWorkflowMode;
  videos: GeneratedVideo[];
  onToast: (message: unknown, type: ToastType) => void;
  onRefresh: () => Promise<void> | void;
}

const ASSEMBLY_PUSH_VIDEO_TYPE = "assembly_push";
const POSTPROCESS_EDIT_VIDEO_TYPE = "assembly_postprocess_edit";
const MARKER_MIN_GAP_SECS = 0.05;

const formatSeconds = (value: number): string => {
  if (!Number.isFinite(value) || value <= 0) return "0s";
  if (value < 60) return `${Math.round(value)}s`;
  const minutes = Math.floor(value / 60);
  const seconds = Math.round(value - minutes * 60);
  return `${minutes}m ${seconds}s`;
};

const pushMetadata = (video: GeneratedVideo): Record<string, unknown> | null =>
  asRecord(asRecord(video.metadata_json)?.assembly_push);

const pushStatus = (video: GeneratedVideo): string =>
  String(pushMetadata(video)?.status ?? "pending");

const pushIndex = (video: GeneratedVideo): number => {
  const value = pushMetadata(video)?.push_index;
  return Number.isInteger(value) ? Number(value) : video.id;
};

const pushSourceIds = (video: GeneratedVideo): number[] => {
  const value = pushMetadata(video)?.source_video_ids;
  return Array.isArray(value)
    ? value.filter((item): item is number => Number.isInteger(item) && Number(item) > 0)
    : [];
};

const editPushId = (video: GeneratedVideo): number | null => {
  const value = asRecord(asRecord(video.metadata_json)?.postprocess_edit)?.push_video_id;
  return Number.isInteger(value) && Number(value) > 0 ? Number(value) : null;
};

const editOperationLabel: Record<string, string> = {
  insert: "插入",
  delete: "删除",
  replace: "替换",
};

export const VideoPostProcessWorkspace: React.FC<VideoPostProcessWorkspaceProps> = ({
  projectId,
  chapter,
  workflowMode,
  videos,
  onToast,
  onRefresh,
}) => {
  const pushes = useMemo(
    () =>
      videos
        .filter(
          (video) =>
            video.video_type === ASSEMBLY_PUSH_VIDEO_TYPE &&
            video.workflow_mode === workflowMode &&
            video.status === "completed",
        )
        .sort((a, b) => pushIndex(a) - pushIndex(b) || a.id - b.id),
    [videos, workflowMode],
  );
  const editsByPushId = useMemo(() => {
    const map = new Map<number, GeneratedVideo[]>();
    for (const video of videos) {
      if (
        video.video_type !== POSTPROCESS_EDIT_VIDEO_TYPE ||
        video.workflow_mode !== workflowMode ||
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
  }, [videos, workflowMode]);

  const latestPush = useMemo(
    () => [...pushes].reverse().find((push) => pushStatus(push) !== "rejected") ?? null,
    [pushes],
  );
  const [selectedPushId, setSelectedPushId] = useState<number | null>(null);
  const selectedPush =
    selectedPushId == null
      ? null
      : pushes.find((push) => push.id === selectedPushId) ?? null;
  const versions = selectedPush ? editsByPushId.get(selectedPush.id) ?? [] : [];
  const currentVideo = versions[versions.length - 1] ?? selectedPush;
  const currentVideoPath = currentVideo?.file_path?.trim() ?? "";
  const {
    videoPlaybackSrc,
    videoLoadError,
    videoLoading,
    handleVideoLoadStart,
    handleVideoLoaded,
    handleVideoError,
  } = useVideoPlaybackSource({
    selectedVideo: currentVideo ?? null,
    selectedVideoPath: currentVideoPath,
  });

  const [acting, setActing] = useState<string | null>(null);
  const [markerValue, setMarkerValue] = useState(0);
  const [markers, setMarkers] = useState<number[]>([]);
  const [draggingMarkerIndex, setDraggingMarkerIndex] = useState<number | null>(null);
  const timelineRef = useRef<HTMLDivElement | null>(null);
  const scopeKey = `${projectId}:${chapter.id}:${workflowMode}`;
  const activeScopeRef = useRef(scopeKey);
  const mountedRef = useRef(false);
  const actionBusyRef = useRef(false);

  useLayoutEffect(() => {
    activeScopeRef.current = scopeKey;
    actionBusyRef.current = false;
    setActing(null);
    setMarkers([]);
    setMarkerValue(0);
    setDraggingMarkerIndex(null);
  }, [scopeKey]);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      actionBusyRef.current = false;
    };
  }, []);

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
      if (current != null && pushes.some((push) => push.id === current)) return current;
      return latestPush?.id ?? pushes[0]?.id ?? null;
    });
  }, [latestPush?.id, pushes]);

  useEffect(() => {
    setMarkers([]);
    setMarkerValue(0);
  }, [currentVideo?.id]);

  const duration = Math.max(0.1, currentVideo?.duration_secs ?? 0);
  const selectedRange = useMemo(() => {
    if (markers.length < 2) return null;
    const sorted = [...markers].sort((a, b) => a - b);
    if (sorted[1] - sorted[0] < MARKER_MIN_GAP_SECS) return null;
    return { start: sorted[0], end: sorted[1] };
  }, [markers]);
  const singleMarker = markers.length === 1 ? markers[0] : null;
  const editableLatest = selectedPush != null && latestPush?.id === selectedPush.id;
  const selectedPushStatus = selectedPush ? pushStatus(selectedPush) : null;
  const editDisabledReason =
    !selectedPush
      ? "没有可编辑推送"
      : !editableLatest
        ? "只能编辑当前最新推送"
        : selectedPushStatus === "rejected"
          ? "已拒绝推送不能编辑"
          : null;

  const appendMarker = (rawValue: number) => {
    const value = Math.max(0, Math.min(duration, rawValue));
    setMarkers((prev) => {
      if (prev.length >= 2) return prev;
      const next = [...prev, value]
        .sort((a, b) => a - b)
        .filter(
          (item, index, list) =>
            index === 0 || Math.abs(item - list[index - 1]) > MARKER_MIN_GAP_SECS,
        );
      return next.slice(0, 2);
    });
  };
  const addMarker = () => appendMarker(markerValue);

  const markerValueFromClientX = (clientX: number): number | null => {
    const rect = timelineRef.current?.getBoundingClientRect();
    if (!rect || rect.width <= 0) return null;
    const ratio = Math.max(0, Math.min(1, (clientX - rect.left) / rect.width));
    return ratio * duration;
  };

  const handleTimelineClick = (event: React.MouseEvent<HTMLDivElement>) => {
    if (!currentVideo || markers.length >= 2 || draggingMarkerIndex != null) return;
    const value = markerValueFromClientX(event.clientX);
    if (value == null) return;
    setMarkerValue(value);
    appendMarker(value);
  };

  useEffect(() => {
    if (draggingMarkerIndex == null) return;
    const handlePointerMove = (event: PointerEvent) => {
      const value = markerValueFromClientX(event.clientX);
      if (value == null) return;
      setMarkerValue(value);
      setMarkers((prev) =>
        prev.map((marker, index) => (index === draggingMarkerIndex ? value : marker)),
      );
    };
    const handlePointerUp = () => {
      setMarkers((prev) => [...prev].sort((a, b) => a - b));
      setDraggingMarkerIndex(null);
    };
    window.addEventListener("pointermove", handlePointerMove);
    window.addEventListener("pointerup", handlePointerUp, { once: true });
    window.addEventListener("pointercancel", handlePointerUp, { once: true });
    return () => {
      window.removeEventListener("pointermove", handlePointerMove);
      window.removeEventListener("pointerup", handlePointerUp);
      window.removeEventListener("pointercancel", handlePointerUp);
    };
  }, [draggingMarkerIndex, duration]);

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
      onToast(action === "accept" ? "已同意最新推送" : action === "reject" ? "已拒绝最新推送" : "已撤销最新推送同意", "success");
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
    let file: File | null = null;
    try {
      file = await pickLocalFile(supportedVideoAccept());
    } catch (err) {
      if (!isCurrentScope(requestScope)) return;
      onToast(extractErrorMessage(err, "打开文件选择器失败"), "error");
      return;
    } finally {
      if (!file) {
        finishAction(requestScope);
        if (isCurrentScope(requestScope)) setActing(null);
      }
    }
    if (!file) return;
    const mimeType = videoMimeForUploadFile(file);
    if (!mimeType) {
      onToast("只支持 MP4/WebM/MOV/M4V 视频，且文件类型必须与扩展名一致", "error");
      finishAction(requestScope);
      if (isCurrentScope(requestScope)) setActing(null);
      return;
    }
    if (file.size > MAX_UPLOAD_VIDEO_BYTES) {
      onToast(`上传视频不能超过 ${MAX_UPLOAD_VIDEO_BYTES / 1024 / 1024}MB`, "error");
      finishAction(requestScope);
      if (isCurrentScope(requestScope)) setActing(null);
      return;
    }
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
      setMarkers([]);
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
      setMarkers([]);
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

  return (
    <div className="grid h-full min-h-0 gap-3 xl:grid-cols-[220px_minmax(0,1fr)]">
      <BookCard className="flex min-h-0 flex-col p-3">
        <div className="mb-2 flex items-center justify-between gap-2">
          <h3 className="inline-flex items-center gap-1.5 font-serif text-sm text-book-text-main">
            <Film size={14} className="text-book-primary" />
            推送片段
          </h3>
          <span className="text-[11px] text-book-text-muted">{pushes.length} 条</span>
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto custom-scrollbar pr-1">
          {pushes.length === 0 ? (
            <EmptyHint text="成片合成页推送后会出现在这里。" />
          ) : (
            <ul className="space-y-2">
              {pushes.map((push) => {
                const active = selectedPush?.id === push.id;
                const status = pushStatus(push);
                return (
                  <li key={push.id}>
                    <button
                      type="button"
                      onClick={() => setSelectedPushId(push.id)}
                      data-active={active ? "1" : undefined}
                      className="a4n-interactive-row w-full px-2 py-2 text-left"
                    >
                      <div className="flex items-center gap-1 text-xs">
                        <span className="font-mono text-book-text-main">#{push.id}</span>
                        <span className="pyqt-pill !text-[10px]">批次 {pushIndex(push)}</span>
                        <span className={`pyqt-pill !text-[10px] ${status === "rejected" ? "text-error" : statusClass(push.status)}`}>
                          {status}
                        </span>
                      </div>
                      <div className="mt-1 text-[10px] text-book-text-muted">
                        源片段：{pushSourceIds(push).join(", ") || "-"}
                      </div>
                    </button>
                  </li>
                );
              })}
            </ul>
          )}
        </div>
      </BookCard>

      <div className="min-h-0 space-y-3 overflow-y-auto custom-scrollbar pr-1">
        <BookCard className="flex min-h-[clamp(420px,58vh,640px)] flex-col p-3">
          <div className="mb-2 flex flex-wrap items-center justify-between gap-2">
            <div className="min-w-0">
              <h3 className="font-serif text-sm text-book-text-main">后处理预览</h3>
              {currentVideo ? (
                <div className="mt-1 flex flex-wrap items-center gap-1 text-[11px] text-book-text-muted">
                  <span className="pyqt-pill !text-[10px]">视频 #{currentVideo.id}</span>
                  <span className="pyqt-pill !text-[10px]">{formatSeconds(currentVideo.duration_secs)}</span>
                  <span className="pyqt-pill !text-[10px]">版本 {versions.length + 1}</span>
                  {currentVideo.updated_at && (
                    <span className="pyqt-pill !text-[10px]">{formatVideoCreatedAt(currentVideo.updated_at)}</span>
                  )}
                </div>
              ) : null}
            </div>
            <div className="flex items-center gap-1">
              <BookButton
                variant="secondary"
                size="sm"
                onClick={() => void handlePushStatus("reject")}
                disabled={!editableLatest || selectedPushStatus !== "pending" || acting != null}
                className="!min-h-8 !px-2 text-error"
              >
                {acting === "status-reject" ? <Loader2 size={12} className="animate-spin" /> : <X size={12} />}
                拒绝
              </BookButton>
              <BookButton
                variant="secondary"
                size="sm"
                onClick={() => void handlePushStatus("undo")}
                disabled={!editableLatest || selectedPushStatus !== "accepted" || acting != null}
                className="!min-h-8 !px-2"
              >
                {acting === "status-undo" ? <Loader2 size={12} className="animate-spin" /> : <RotateCcw size={12} />}
                撤销
              </BookButton>
              <BookButton
                variant="primary"
                size="sm"
                onClick={() => void handlePushStatus("accept")}
                disabled={!editableLatest || selectedPushStatus !== "pending" || acting != null}
                className="!min-h-8 !px-2"
              >
                {acting === "status-accept" ? <Loader2 size={12} className="animate-spin" /> : <Check size={12} />}
                同意
              </BookButton>
            </div>
          </div>

          <div className="relative min-h-0 flex-1 overflow-hidden rounded border border-book-border/50 bg-black">
            {videoPlaybackSrc ? (
              <video
                controls
                playsInline
                preload="metadata"
                src={videoPlaybackSrc}
                className="h-full min-h-[320px] w-full bg-black object-contain"
                onLoadStart={handleVideoLoadStart}
                onLoadedMetadata={handleVideoLoaded}
                onLoadedData={handleVideoLoaded}
                onCanPlay={handleVideoLoaded}
                onError={handleVideoError}
              />
            ) : (
              <div className="flex h-full min-h-[320px] items-center justify-center bg-book-bg-tertiary">
                <EmptyHint text={currentVideo ? "正在加载后处理视频。" : "暂无推送视频。"} />
              </div>
            )}
            {videoLoading && (
              <div className="pointer-events-none absolute inset-0 flex items-center justify-center bg-black/35 text-xs text-white">
                <span className="inline-flex items-center gap-1.5 rounded bg-black/55 px-2 py-1">
                  <Loader2 size={12} className="animate-spin" />
                  正在加载视频
                </span>
              </div>
            )}
          </div>
          {videoLoadError && (
            <div className="mt-2 rounded border border-error/40 bg-error/10 px-2 py-1.5 text-[11px] text-error">
              {videoLoadError}
            </div>
          )}
        </BookCard>

        <BookCard className="shrink-0 p-3">
          <div className="mb-2 flex flex-wrap items-start justify-between gap-2">
            <div className="min-w-0">
              <h3 className="inline-flex items-center gap-1.5 font-serif text-sm text-book-text-main">
                <Scissors size={14} className="text-book-primary" />
                剪辑条
              </h3>
              {editDisabledReason && (
                <div className="mt-1 text-[11px] text-book-text-muted">{editDisabledReason}</div>
              )}
            </div>
            <div className="flex flex-wrap items-center gap-1">
              <BookButton
                variant="secondary"
                size="sm"
                onClick={addMarker}
                disabled={!currentVideo || markers.length >= 2}
                className="!min-h-7 !px-2 text-[11px]"
              >
                添加竖线
              </BookButton>
              <BookButton
                variant="secondary"
                size="sm"
                onClick={() => setMarkers([])}
                disabled={markers.length === 0}
                className="!min-h-7 !px-2 text-[11px]"
              >
                清除
              </BookButton>
            </div>
          </div>

          <div className="rounded border border-book-border/50 bg-book-bg-paper/60 p-2">
            <div className="flex items-center justify-between text-[10px] text-book-text-muted">
              <span>0s</span>
              <span className="font-mono">{markerValue.toFixed(2)}s</span>
              <span>{duration.toFixed(2)}s</span>
            </div>
            <div
              ref={timelineRef}
              role="button"
              tabIndex={0}
              title={markers.length >= 2 ? "已添加两根竖线，请先清除" : "点击剪辑条添加竖线"}
              onClick={handleTimelineClick}
              onKeyDown={(event) => {
                if (event.key === "Enter" || event.key === " ") {
                  event.preventDefault();
                  addMarker();
                }
              }}
              className={`relative mt-2 h-10 rounded bg-book-bg-tertiary ${
                currentVideo && markers.length < 2 ? "cursor-crosshair" : "cursor-default"
              }`}
            >
              {markers.map((marker, index) => (
                <span
                  key={`${marker}-${index}`}
                  role="slider"
                  tabIndex={0}
                  aria-label={`剪辑竖线 ${index + 1}`}
                  aria-valuemin={0}
                  aria-valuemax={duration}
                  aria-valuenow={Number(marker.toFixed(2))}
                  onPointerDown={(event) => {
                    event.preventDefault();
                    event.stopPropagation();
                    setDraggingMarkerIndex(index);
                    setMarkerValue(marker);
                  }}
                  onKeyDown={(event) => {
                    const step = event.shiftKey ? 1 : 0.05;
                    if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
                    event.preventDefault();
                    const direction = event.key === "ArrowLeft" ? -1 : 1;
                    const value = Math.max(0, Math.min(duration, marker + direction * step));
                    setMarkerValue(value);
                    setMarkers((prev) =>
                      prev
                        .map((item, itemIndex) => (itemIndex === index ? value : item))
                        .sort((a, b) => a - b),
                    );
                  }}
                  className="absolute top-0 h-10 w-4 -translate-x-1/2 cursor-ew-resize touch-none"
                  style={{ left: `${Math.min(100, Math.max(0, (marker / duration) * 100))}%` }}
                  title={`${marker.toFixed(2)}s`}
                >
                  <span className="absolute left-1/2 top-0 h-10 w-0.5 -translate-x-1/2 bg-book-primary" />
                  <span className="absolute left-1/2 top-1 h-2.5 w-2.5 -translate-x-1/2 rounded-full border border-book-primary bg-book-bg-paper shadow-sm" />
                </span>
              ))}
              {selectedRange && (
                <span
                  className="pointer-events-none absolute top-2.5 h-5 rounded bg-book-primary/25"
                  style={{
                    left: `${(selectedRange.start / duration) * 100}%`,
                    width: `${((selectedRange.end - selectedRange.start) / duration) * 100}%`,
                  }}
                />
              )}
            </div>
          </div>

          <div className="mt-3 grid gap-2 sm:grid-cols-2">
            <div className="space-y-2 rounded border border-book-border/50 p-2">
              <div className="text-xs font-medium text-book-text-main">单竖线：添加片段</div>
              <div className="grid gap-1">
                <BookButton
                  variant="secondary"
                  size="sm"
                  onClick={() => void applyUploadEdit("insert")}
                  disabled={!!editDisabledReason || singleMarker == null || acting != null}
                  className="!min-h-8 !px-2 text-[11px]"
                >
                  <Upload size={12} />
                  上传添加
                </BookButton>
              </div>
            </div>
            <div className="space-y-2 rounded border border-book-border/50 p-2">
              <div className="text-xs font-medium text-book-text-main">双竖线：删除/替换片段</div>
              <div className="grid grid-cols-2 gap-1">
                <BookButton
                  variant="secondary"
                  size="sm"
                  onClick={() => void applyDelete()}
                  disabled={!!editDisabledReason || !selectedRange || acting != null}
                  className="!min-h-8 !px-2 text-[11px] text-error"
                >
                  {acting === "edit-delete" ? <Loader2 size={12} className="animate-spin" /> : <Scissors size={12} />}
                  删除
                </BookButton>
                <BookButton
                  variant="secondary"
                  size="sm"
                  onClick={() => void applyUploadEdit("replace")}
                  disabled={!!editDisabledReason || !selectedRange || acting != null}
                  className="!min-h-8 !px-2 text-[11px]"
                >
                  <Upload size={12} />
                  上传替换
                </BookButton>
              </div>
            </div>
          </div>
        </BookCard>
      </div>
    </div>
  );
};
