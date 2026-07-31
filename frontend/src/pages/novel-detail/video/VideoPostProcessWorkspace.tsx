import React, { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { Check,
  RotateCcw,
  Scissors,
  Upload,
  X, } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { extractErrorMessage } from "../../../api";
import {
  applyAssemblyPostprocessEdit,
  updateAssemblyPushStatus,
  type GeneratedVideo,
  type VideoWorkflowMode,
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

const pushIndex = (video: GeneratedVideo): number | null => {
  const value = pushMetadata(video)?.push_index;
  return Number.isInteger(value) && Number(value) > 0 ? Number(value) : null;
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

const PUSH_STATUS_LABELS: Record<string, string> = {
  pending: "待确认",
  accepted: "已同意",
  rejected: "已拒绝",
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
        .sort(
          (a, b) =>
            (pushIndex(a) ?? Number.MAX_SAFE_INTEGER) -
              (pushIndex(b) ?? Number.MAX_SAFE_INTEGER) ||
            a.id - b.id,
        ),
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
      : selectedPushStatus === "rejected"
        ? "已拒绝推送不能编辑"
        : !editableLatest
          ? "只能编辑当前最新推送"
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
    if (
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
            onClick: () => setMarkers([]),
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

  if (pushes.length === 0) {
    return (
      <section className="a4n-flat-surface w-full self-start">
        <div className="a4n-empty-state min-h-[180px] px-4 py-6">
          <div className="flex h-9 w-9 items-center justify-center rounded-full bg-book-bg-tertiary text-book-primary">
            <Scissors size={16} />
          </div>
          <div className="max-w-sm">
            <h3 className="text-sm font-medium text-book-text-main">还没有待处理的成片</h3>
            <p className="mt-1 text-xs leading-relaxed text-book-text-muted">
              从成片合成阶段推送预览后，才能在这里确认和调整剪辑。
            </p>
          </div>
        </div>
      </section>
    );
  }

  return (
    <div className="grid h-full min-h-0 gap-3 xl:grid-cols-[220px_minmax(0,1fr)]">
      <section className="a4n-flat-surface flex min-h-0 flex-col overflow-hidden">
        <div className="flex shrink-0 items-baseline justify-between gap-3 border-b border-book-border/35 px-3 py-2.5">
          <h3 className="text-xs font-medium text-book-text-main">推送批次</h3>
          <span className="text-caption text-book-text-muted">{pushes.length} 批</span>
        </div>
        <ul className="flex min-h-0 divide-x divide-book-border/35 overflow-x-auto custom-scrollbar xl:flex-1 xl:flex-col xl:divide-x-0 xl:divide-y xl:overflow-x-hidden xl:overflow-y-auto">
          {pushes.map((push, index) => {
            const active = selectedPush?.id === push.id;
            const status = pushStatus(push);
            const sourceCount = pushSourceIds(push).length;
            const statusTone =
              status === "accepted"
                ? "text-success"
                : status === "rejected"
                  ? "text-error"
                  : status === "pending"
                    ? "text-warning"
                    : "text-book-text-muted";
            return (
              <li key={push.id} className="min-w-[148px] shrink-0 xl:min-w-0 xl:w-full">
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
                    批次 {pushIndex(push) ?? index + 1}
                  </div>
                  <div className="mt-1 text-caption text-book-text-muted">
                    {sourceCount > 0 ? `${sourceCount} 个片段` : "未记录片段"}
                    <span className="mx-1">·</span>
                    <span className={statusTone}>{PUSH_STATUS_LABELS[status] ?? "状态未知"}</span>
                  </div>
                </button>
              </li>
            );
          })}
        </ul>
      </section>

      <div className="min-h-0 space-y-3 overflow-y-auto pb-6 pr-1 custom-scrollbar">
        <section className="a4n-flat-surface flex min-h-[clamp(380px,58vh,640px)] flex-col p-3">
          <div className="mb-3 flex min-w-0 flex-wrap items-center gap-2 border-b border-book-border/35 pb-3">
            <div className="min-w-0 flex-1">
              <h3 className="text-sm font-medium text-book-text-main">后处理预览</h3>
              {currentVideo && selectedPush ? (
                <p className="mt-1 text-caption leading-relaxed text-book-text-muted">
                  批次 {pushIndex(selectedPush) ?? pushes.indexOf(selectedPush) + 1} · {PUSH_STATUS_LABELS[selectedPushStatus ?? ""] ?? "状态未知"} · {formatSeconds(currentVideo.duration_secs)} · 第 {versions.length + 1} 版
                  {currentVideo.updated_at ? ` · ${formatVideoCreatedAt(currentVideo.updated_at)}` : ""}
                </p>
              ) : null}
            </div>
            {statusActing ? (
              <span role="status" className="inline-flex h-8 w-8 items-center justify-center text-book-text-muted">
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
                controls
                playsInline
                preload="metadata"
                src={videoPlaybackSrc}
                className="h-full min-h-[280px] w-full bg-black object-contain"
                onLoadStart={handleVideoLoadStart}
                onLoadedMetadata={handleVideoLoaded}
                onLoadedData={handleVideoLoaded}
                onCanPlay={handleVideoLoaded}
                onError={handleVideoError}
              />
            ) : (
              <div className="flex h-full min-h-[280px] items-center justify-center bg-book-bg-tertiary">
                <EmptyHint text={currentVideo ? "正在加载后处理视频。" : "暂无推送视频。"} />
              </div>
            )}
            {videoLoading && (
              <div className="pointer-events-none absolute inset-0 flex items-center justify-center bg-black/35 text-xs text-white">
                <span className="inline-flex items-center gap-1.5 rounded-xl bg-black/55 px-2.5 py-1.5 backdrop-blur-sm">
                  <LoadingSpinner size="sm" />
                  正在加载视频
                </span>
              </div>
            )}
          </div>
          {videoLoadError && (
            <div role="alert" className="mt-2 rounded-xl border border-error/40 bg-error-bg px-2.5 py-2 text-small text-error">
              {videoLoadError}
            </div>
          )}
        </section>

        <section className="a4n-flat-surface shrink-0 p-3">
          <div className="mb-3 border-b border-book-border/35 pb-3">
            <h3 className="text-sm font-medium text-book-text-main">剪辑条</h3>
            {editDisabledReason ? (
              <p className="mt-1 text-caption text-book-text-muted">{editDisabledReason}</p>
            ) : null}
          </div>

          {editActing ? (
            <div role="status" className="flex min-h-24 items-center justify-center gap-2 text-xs text-book-text-muted">
              <LoadingSpinner size="sm" />
              正在应用后处理编辑
            </div>
          ) : editDisabledReason || !currentVideo ? (
            <div className="flex min-h-20 items-center justify-center text-xs text-book-text-muted">
              当前批次不可编辑。
            </div>
          ) : (
            <>
              <div>
                <div className="grid grid-cols-3 text-caption tabular-nums text-book-text-muted">
                  <span>0s</span>
                  <span className="justify-self-center">{markerValue.toFixed(2)}s</span>
                  <span className="justify-self-end">{duration.toFixed(2)}s</span>
                </div>
                <div
                  ref={timelineRef}
                  role="button"
                  tabIndex={0}
                  aria-disabled={markers.length >= 2 || undefined}
                  title={markers.length >= 2 ? "已添加两根竖线，请先清除" : "点击剪辑条添加竖线"}
                  onClick={handleTimelineClick}
                  onKeyDown={(event) => {
                    if (event.key === "Enter" || event.key === " ") {
                      event.preventDefault();
                      addMarker();
                    }
                  }}
                  className={`relative mt-2 h-11 rounded-xl bg-book-bg-tertiary ${
                    markers.length < 2 ? "cursor-crosshair" : "cursor-default"
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
                      className="absolute top-0 h-11 w-5 -translate-x-1/2 cursor-ew-resize touch-none"
                      style={{ left: `${Math.min(100, Math.max(0, (marker / duration) * 100))}%` }}
                      title={`${marker.toFixed(2)}s`}
                    >
                      <span className="absolute left-1/2 top-0 h-11 w-0.5 -translate-x-1/2 bg-book-primary" />
                      <span className="absolute left-1/2 top-1 h-3 w-3 -translate-x-1/2 rounded-full border border-book-primary bg-book-bg-paper" />
                    </span>
                  ))}
                  {selectedRange && (
                    <span
                      className="pointer-events-none absolute top-3 h-5 rounded-lg bg-book-primary/25"
                      style={{
                        left: `${(selectedRange.start / duration) * 100}%`,
                        width: `${((selectedRange.end - selectedRange.start) / duration) * 100}%`,
                      }}
                    />
                  )}
                </div>
              </div>

              <div className="mt-3 flex min-w-0 flex-col gap-2 sm:flex-row sm:items-center">
                <p className="min-w-0 flex-1 text-small leading-relaxed text-book-text-muted">
                  {markerSummary}
                </p>
                <div className="flex shrink-0 items-center gap-2">
                  {acting == null && editActionItems.length > 0 ? (
                    <Dropdown items={editActionItems} />
                  ) : null}
                  {singleMarker != null ? (
                    <BookButton
                      variant="primary"
                      size="sm"
                      onClick={() => void applyUploadEdit("insert")}
                      disabled={acting != null}
                      className="!min-h-9 min-w-0 flex-1 !px-3 text-small sm:flex-none"
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
                      className="!min-h-9 min-w-0 flex-1 !px-3 text-small sm:flex-none"
                    >
                      <Upload size={12} />
                      上传并替换
                    </BookButton>
                  ) : null}
                </div>
              </div>
            </>
          )}
        </section>
      </div>
    </div>
  );
};
