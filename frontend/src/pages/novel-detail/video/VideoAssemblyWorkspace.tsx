import React, { useEffect, useLayoutEffect, useMemo, useRef, useState, type SyntheticEvent } from "react";
import {
  Clock3,
  Download,
  Film,
  ListVideo,
  Loader2,
  Plus,
  RefreshCw,
  Trash2,
} from "lucide-react";
import { extractErrorMessage } from "../../../api";
import {
  assembleChapterVideos,
  deleteVideo,
  loadCachedAssembledChapterVideoPreview,
  pushAssembledChapterVideo,
  uploadAssemblyInsertVideo,
  type AssembledChapterVideoPreview,
  type GeneratedVideo,
  type VideoWorkflowMode,
} from "../../../api/video";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import type { ToastType } from "../../../components/feedback/Toast";
import { BookButton } from "../../../components/ui/BookButton";
import { BookCard } from "../../../components/ui/BookCard";
import { asJsonRecord as asRecord } from "../../../utils/jsonRecord";
import { downloadBlob } from "../../../utils/downloadFile";
import { pickLocalFile, readFileAsBase64 } from "../../../utils/fileUpload";
import {
  MAX_UPLOAD_VIDEO_BYTES,
  supportedVideoAccept,
  videoMimeForUploadFile,
} from "../../../utils/videoUpload";
import { videoPlaybackDataToBlob, videoPlaybackDataToObjectUrl } from "../../../utils/videoPlayback";
import { EmptyHint, type ChapterRow } from "../shared";
import { statusClass } from "./videoGenerationDisplay";
import { formatVideoCreatedAt } from "./videoTabUtils";

interface VideoAssemblyWorkspaceProps {
  projectId: number;
  chapter: ChapterRow;
  workflowMode: VideoWorkflowMode;
  videos: GeneratedVideo[];
  onToast: (message: unknown, type: ToastType) => void;
  onRefresh: () => Promise<void> | void;
}

const CHAPTER_VIDEO_TYPE = "chapter_video";
const ASSEMBLY_INSERT_VIDEO_TYPE = "assembly_insert";
const ASSEMBLY_PUSH_VIDEO_TYPE = "assembly_push";
const ASSEMBLY_ORDER_STEP = 1000;

const clipShotNumber = (video: GeneratedVideo): number | null => {
  const value = asRecord(video.metadata_json)?.shot_number;
  return Number.isInteger(value) && Number(value) > 0 ? Number(value) : null;
};

const assemblyInsertMetadata = (video: GeneratedVideo): Record<string, unknown> | null =>
  asRecord(asRecord(video.metadata_json)?.assembly_insert);

const assemblyInsertAfterVideoId = (video: GeneratedVideo): number | null => {
  const value = assemblyInsertMetadata(video)?.insert_after_video_id;
  return Number.isInteger(value) && Number(value) > 0 ? Number(value) : null;
};

const assemblyClipSortOrder = (video: GeneratedVideo): number | null => {
  if (video.video_type === CHAPTER_VIDEO_TYPE) {
    const shotNumber = clipShotNumber(video);
    return shotNumber == null ? null : shotNumber * ASSEMBLY_ORDER_STEP;
  }
  if (video.video_type === ASSEMBLY_INSERT_VIDEO_TYPE) {
    const value = assemblyInsertMetadata(video)?.sort_order;
    return typeof value === "number" && Number.isFinite(value) ? value : null;
  }
  return null;
};

const assemblyClipLabel = (video: GeneratedVideo): string => {
  if (video.video_type !== ASSEMBLY_INSERT_VIDEO_TYPE) {
    const shotNumber = clipShotNumber(video);
    return shotNumber == null ? "顺序缺失" : `镜头 ${shotNumber}`;
  }
  const afterShot = assemblyInsertMetadata(video)?.insert_after_shot_number;
  if (Number.isInteger(afterShot) && Number(afterShot) > 0) {
    return `衔接 · 镜头 ${Number(afterShot)} 后`;
  }
  return "衔接 · 开头";
};

const assemblyPushMetadata = (video: GeneratedVideo): Record<string, unknown> | null =>
  asRecord(asRecord(video.metadata_json)?.assembly_push);

const assemblyPushStatus = (video: GeneratedVideo): string =>
  String(assemblyPushMetadata(video)?.status ?? "pending");

const assemblyPushSourceIds = (video: GeneratedVideo): number[] => {
  const value = assemblyPushMetadata(video)?.source_video_ids;
  return Array.isArray(value)
    ? value.filter((item): item is number => Number.isInteger(item) && Number(item) > 0)
    : [];
};

const formatSeconds = (value: number): string => {
  if (!Number.isFinite(value) || value <= 0) return "0s";
  if (value < 60) return `${Math.round(value)}s`;
  const minutes = Math.floor(value / 60);
  const seconds = Math.round(value - minutes * 60);
  return `${minutes}m ${seconds}s`;
};

const formatBytes = (value: number): string => {
  if (!Number.isFinite(value) || value <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  let size = value;
  let unitIndex = 0;
  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024;
    unitIndex += 1;
  }
  return `${size >= 10 || unitIndex === 0 ? size.toFixed(0) : size.toFixed(1)} ${units[unitIndex]}`;
};

const defaultAssemblyFileName = (chapter: ChapterRow, workflowMode: VideoWorkflowMode): string =>
  `a4n-chapter-${chapter.chapter_number}-${workflowMode}-assembled.mp4`;

const downloadAssemblyPreview = (
  fileName: string,
  preview: AssembledChapterVideoPreview,
): void => {
  downloadBlob(fileName, videoPlaybackDataToBlob(preview));
};

export const VideoAssemblyWorkspace: React.FC<VideoAssemblyWorkspaceProps> = ({
  projectId,
  chapter,
  workflowMode,
  videos,
  onToast,
  onRefresh,
}) => {
  const [preview, setPreview] = useState<AssembledChapterVideoPreview | null>(null);
  const [playbackSrc, setPlaybackSrc] = useState("");
  const [loading, setLoading] = useState(false);
  const [cacheLoading, setCacheLoading] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [pushing, setPushing] = useState(false);
  const [pushClipCount, setPushClipCount] = useState(1);
  const [acting, setActing] = useState<string | null>(null);
  const [videoLoading, setVideoLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const scopeKey = `${projectId}:${chapter.id}:${workflowMode}`;
  const activeScopeRef = useRef(scopeKey);
  const mountedRef = useRef(false);
  const actionBusyRef = useRef(false);

  const clearPreview = () => {
    setPreview(null);
    setPlaybackSrc((prev) => {
      if (prev) URL.revokeObjectURL(prev);
      return "";
    });
    setVideoLoading(false);
  };

  const clips = useMemo(
    () =>
      videos
        .filter(
          (video) =>
            (video.video_type === CHAPTER_VIDEO_TYPE ||
              video.video_type === ASSEMBLY_INSERT_VIDEO_TYPE) &&
            video.workflow_mode === workflowMode &&
            video.status === "completed" &&
            Boolean(video.file_path?.trim()) &&
            assemblyClipSortOrder(video) != null,
        )
        .sort((a, b) => {
          const leftOrder = assemblyClipSortOrder(a) ?? Number.MAX_SAFE_INTEGER;
          const rightOrder = assemblyClipSortOrder(b) ?? Number.MAX_SAFE_INTEGER;
          return leftOrder - rightOrder || a.id - b.id;
        }),
    [videos, workflowMode],
  );
  const pushedSourceIds = useMemo(() => {
    const ids = new Set<number>();
    for (const video of videos) {
      if (
        video.video_type !== ASSEMBLY_PUSH_VIDEO_TYPE ||
        video.workflow_mode !== workflowMode ||
        assemblyPushStatus(video) === "rejected"
      ) {
        continue;
      }
      for (const id of assemblyPushSourceIds(video)) ids.add(id);
    }
    return ids;
  }, [videos, workflowMode]);
  const hasPendingPush = useMemo(
    () =>
      videos.some(
        (video) =>
          video.video_type === ASSEMBLY_PUSH_VIDEO_TYPE &&
          video.workflow_mode === workflowMode &&
          video.status === "completed" &&
          assemblyPushStatus(video) === "pending",
      ),
    [videos, workflowMode],
  );
  const pushedPrefixLength = useMemo(() => {
    let length = 0;
    for (const clip of clips) {
      if (!pushedSourceIds.has(clip.id)) break;
      length += 1;
    }
    return length;
  }, [clips, pushedSourceIds]);
  const pushableClips = useMemo(
    () => clips.slice(pushedPrefixLength),
    [clips, pushedPrefixLength],
  );
  const normalizedPushClipCount =
    pushableClips.length === 0
      ? 0
      : Math.max(1, Math.min(pushClipCount, pushableClips.length));
  const selectedPushRange = useMemo(() => {
    if (normalizedPushClipCount <= 0) return [];
    return pushableClips.slice(0, normalizedPushClipCount);
  }, [pushableClips, normalizedPushClipCount]);
  const selectedPushEndVideo = selectedPushRange[selectedPushRange.length - 1] ?? null;
  const selectedPushRangeIds = useMemo(
    () => new Set(selectedPushRange.map((clip) => clip.id)),
    [selectedPushRange],
  );
  const totalDuration = clips.reduce((sum, video) => sum + Math.max(0, video.duration_secs), 0);
  const selectedPushDuration = selectedPushRange.reduce(
    (sum, video) => sum + Math.max(0, video.duration_secs),
    0,
  );
  const clipSignature = clips
    .map(
      (video) =>
        `${video.id}:${assemblyClipSortOrder(video) ?? ""}:${video.file_size ?? ""}:${
          video.updated_at ?? ""
        }`,
    )
    .join(",");

  useLayoutEffect(() => {
    activeScopeRef.current = scopeKey;
    actionBusyRef.current = false;
    setLoading(false);
    setCacheLoading(false);
    setExporting(false);
    setPushing(false);
    setActing(null);
    setVideoLoading(false);
    setError(null);
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
    let cancelled = false;
    clearPreview();
    setError(null);
    if (clips.length === 0) {
      setCacheLoading(false);
      return () => {
        cancelled = true;
      };
    }
    setCacheLoading(true);
    void loadCachedAssembledChapterVideoPreview(projectId, chapter.id, workflowMode)
      .then((data) => {
        if (cancelled || !data) return;
        const objectUrl = videoPlaybackDataToObjectUrl(data);
        setPlaybackSrc((prev) => {
          if (prev) URL.revokeObjectURL(prev);
          return objectUrl;
        });
        setPreview(data);
        setVideoLoading(true);
      })
      .catch((err) => {
        if (cancelled) return;
        setError(extractErrorMessage(err, "加载成片缓存失败"));
      })
      .finally(() => {
        if (!cancelled) setCacheLoading(false);
      });
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectId, chapter.id, workflowMode, clipSignature]);

  useEffect(
    () => () => {
      if (playbackSrc) URL.revokeObjectURL(playbackSrc);
    },
    [playbackSrc],
  );

  useEffect(() => {
    setPushClipCount((current) =>
      pushableClips.length === 0 ? 1 : Math.max(1, Math.min(current, pushableClips.length)),
    );
  }, [pushableClips.length]);

  const handleAssemble = async () => {
    if (clips.length === 0) {
      setError("当前没有可拼接的已完成视频");
      return;
    }
    const requestScope = scopeKey;
    if (!beginAction()) return;
    setLoading(true);
    setError(null);
    try {
      const data = await assembleChapterVideos(projectId, chapter.id, workflowMode);
      if (!isCurrentScope(requestScope)) return;
      const objectUrl = videoPlaybackDataToObjectUrl(data);
      setPlaybackSrc((prev) => {
        if (prev) URL.revokeObjectURL(prev);
        return objectUrl;
      });
      setPreview(data);
      setVideoLoading(true);
      onToast(data.cached ? "已加载缓存预览" : "成片预览已生成", "success");
    } catch (err) {
      if (!isCurrentScope(requestScope)) return;
      const message = extractErrorMessage(err, "生成成片预览失败");
      setError(message);
      onToast(message, "error");
    } finally {
      finishAction(requestScope);
      if (isCurrentScope(requestScope)) setLoading(false);
    }
  };

  const handleDownloadPreview = async () => {
    if (!preview) {
      setError("请先生成成片预览");
      return;
    }
    const requestScope = scopeKey;
    if (!beginAction()) return;
    setExporting(true);
    setError(null);
    try {
      downloadAssemblyPreview(defaultAssemblyFileName(chapter, workflowMode), preview);
      if (!isCurrentScope(requestScope)) return;
      onToast("成片预览已下载", "success");
    } catch (err) {
      if (!isCurrentScope(requestScope)) return;
      const message = extractErrorMessage(err, "下载成片预览失败");
      setError(message);
      onToast(message, "error");
    } finally {
      finishAction(requestScope);
      if (isCurrentScope(requestScope)) setExporting(false);
    }
  };

  const handlePushPreview = async () => {
    if (!preview) {
      setError("请先生成成片预览");
      return;
    }
    if (!selectedPushEndVideo) {
      setError("当前没有可推送的后续片段");
      return;
    }
    if (hasPendingPush) {
      setError("当前最新推送尚未同意或拒绝，不能继续推送后续片段");
      return;
    }
    const requestScope = scopeKey;
    if (!beginAction()) return;
    setPushing(true);
    setError(null);
    try {
      const result = await pushAssembledChapterVideo(projectId, chapter.id, workflowMode, {
        end_video_id: selectedPushEndVideo.id,
      });
      if (!isCurrentScope(requestScope)) return;
      clearPreview();
      await onRefresh();
      if (!isCurrentScope(requestScope)) return;
      onToast(
        `已推送 ${result.source_video_ids.length} 个片段到成片后处理`,
        "success",
      );
    } catch (err) {
      if (!isCurrentScope(requestScope)) return;
      const message = extractErrorMessage(err, "推送成片片段失败");
      setError(message);
      onToast(message, "error");
    } finally {
      finishAction(requestScope);
      if (isCurrentScope(requestScope)) setPushing(false);
    }
  };

  const handleImportInsert = async (insertAfterVideoId: number | null) => {
    const requestScope = scopeKey;
    const key = insertAfterVideoId == null ? "insert-start" : `insert-${insertAfterVideoId}`;
    if (!beginAction()) return;
    setActing(key);
    setError(null);
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
      await uploadAssemblyInsertVideo(projectId, chapter.id, workflowMode, {
        file_name: file.name,
        mime_type: mimeType,
        data_base64: dataBase64,
        insert_after_video_id: insertAfterVideoId,
      });
      if (!isCurrentScope(requestScope)) return;
      clearPreview();
      await onRefresh();
      if (!isCurrentScope(requestScope)) return;
      onToast("衔接片段已插入", "success");
    } catch (err) {
      if (!isCurrentScope(requestScope)) return;
      const message = extractErrorMessage(err, "插入衔接片段失败");
      setError(message);
      onToast(message, "error");
    } finally {
      finishAction(requestScope);
      if (isCurrentScope(requestScope)) setActing(null);
    }
  };

  const handleDeleteInsert = async (video: GeneratedVideo) => {
    const requestScope = scopeKey;
    if (!beginAction()) return;
    setActing(`delete-${video.id}`);
    const ok = await confirmDialog({
      title: "删除衔接片段",
      message: `删除衔接片段 #${video.id}？`,
      confirmText: "删除",
      dialogType: "danger",
    });
    if (!ok) {
      finishAction(requestScope);
      if (isCurrentScope(requestScope)) setActing(null);
      return;
    }
    if (!isCurrentScope(requestScope)) {
      finishAction(requestScope);
      return;
    }
    setError(null);
    try {
      await deleteVideo(video.id);
      if (!isCurrentScope(requestScope)) return;
      clearPreview();
      await onRefresh();
      if (!isCurrentScope(requestScope)) return;
      onToast("衔接片段已删除", "success");
    } catch (err) {
      if (!isCurrentScope(requestScope)) return;
      const message = extractErrorMessage(err, "删除衔接片段失败");
      setError(message);
      onToast(message, "error");
    } finally {
      finishAction(requestScope);
      if (isCurrentScope(requestScope)) setActing(null);
    }
  };

  const handleVideoLoaded = () => setVideoLoading(false);
  const handleVideoError = (event: SyntheticEvent<HTMLVideoElement>) => {
    setVideoLoading(false);
    const detail = event.currentTarget.error?.message?.trim();
    setError(detail ? `成片预览无法播放：${detail}` : "成片预览无法播放");
  };

  return (
    <div className="grid h-full min-h-0 gap-3 xl:grid-cols-[minmax(0,1.15fr)_340px]">
      <BookCard className="flex min-h-0 flex-col p-3">
        <div className="mb-3 space-y-2">
          <div className="flex flex-wrap items-start justify-between gap-2">
            <div className="min-w-0">
              <h3 className="inline-flex items-center gap-1.5 font-serif text-sm text-book-text-main">
                <Film size={14} className="text-book-primary" />
                成片合成
              </h3>
              <div className="mt-1 flex flex-wrap items-center gap-1 text-[11px] text-book-text-muted">
                <span className="pyqt-pill !text-[10px]">片段 {clips.length}</span>
                <span className="pyqt-pill !text-[10px]">{formatSeconds(totalDuration)}</span>
                {preview && (
                  <span className="pyqt-pill !text-[10px]">
                    {formatBytes(preview.file_size)}
                  </span>
                )}
                {preview && (
                  <span className="pyqt-pill !text-[10px]">
                    {preview.cached ? "缓存" : "新生成"}
                  </span>
                )}
                {pushedSourceIds.size > 0 && (
                  <span className="pyqt-pill !text-[10px]">已推送 {pushedSourceIds.size}</span>
                )}
                {selectedPushRange.length > 0 && (
                  <span className="pyqt-pill !text-[10px]">
                    本次 {selectedPushRange.length} · {formatSeconds(selectedPushDuration)}
                  </span>
                )}
              </div>
            </div>
            <div className="flex flex-wrap items-center justify-end gap-2">
              <BookButton
                variant="secondary"
                size="sm"
                onClick={() => void handleImportInsert(null)}
                disabled={
                  loading ||
                  cacheLoading ||
                  exporting ||
                  pushing ||
                  acting != null ||
                  pushedPrefixLength > 0
                }
                title={pushedPrefixLength > 0 ? "已有推送后不能再向开头插入衔接片段" : undefined}
                className="!min-h-8 !px-3"
              >
                <span className="inline-flex items-center gap-1">
                  {acting === "insert-start" ? (
                    <Loader2 size={13} className="animate-spin" />
                  ) : (
                    <Plus size={13} />
                  )}
                  插入开头
                </span>
              </BookButton>
              <BookButton
                variant="primary"
                size="sm"
                onClick={() => void handleAssemble()}
                disabled={loading || cacheLoading || pushing || clips.length === 0}
                title={clips.length === 0 ? "当前没有可拼接的已完成视频" : undefined}
                className="!min-h-8 !px-3"
              >
                <span className="inline-flex items-center gap-1">
                  {loading ? <Loader2 size={13} className="animate-spin" /> : <RefreshCw size={13} />}
                  生成预览
                </span>
              </BookButton>
            </div>
          </div>
          <div className="flex flex-wrap items-center justify-between gap-2 rounded border border-book-border/50 bg-book-bg-paper/55 px-2 py-1.5">
            <div className="flex min-w-0 flex-1 flex-wrap items-center gap-2 sm:min-w-[220px]">
              <label className="flex items-center gap-1 text-[11px] text-book-text-muted">
                推送段数
                <input
                  type="number"
                  min={1}
                  max={Math.max(1, pushableClips.length)}
                  value={pushableClips.length === 0 ? "" : normalizedPushClipCount}
                  onChange={(event) => {
                    const value = Number(event.target.value);
                    if (!Number.isFinite(value)) return;
                    setPushClipCount(
                      Math.max(1, Math.min(Math.round(value), Math.max(1, pushableClips.length))),
                    );
                  }}
                  disabled={
                    loading ||
                    cacheLoading ||
                    exporting ||
                    pushing ||
                    pushableClips.length === 0
                  }
                  className="pyqt-input h-8 w-20 !px-2 !py-1 text-[11px]"
                />
              </label>
              <span className="min-w-0 flex-1 truncate text-[11px] text-book-text-muted">
                {selectedPushEndVideo
                  ? `终点 ${assemblyClipLabel(selectedPushEndVideo)} · #${selectedPushEndVideo.id}`
                  : "无后续片段"}
              </span>
            </div>
            <div className="flex shrink-0 flex-wrap items-center justify-end gap-2">
            <BookButton
              variant="secondary"
              size="sm"
              onClick={() => void handleDownloadPreview()}
              disabled={exporting || loading || cacheLoading || pushing || !preview}
              title={!preview ? "请先生成成片预览" : undefined}
              className="!min-h-8 !px-3"
            >
              <span className="inline-flex items-center gap-1">
                {exporting ? <Loader2 size={13} className="animate-spin" /> : <Download size={13} />}
                下载预览
              </span>
            </BookButton>
            <BookButton
              variant="secondary"
              size="sm"
              onClick={() => void handlePushPreview()}
              disabled={
                pushing ||
                exporting ||
                loading ||
                cacheLoading ||
                !preview ||
                !selectedPushEndVideo ||
                hasPendingPush
              }
              title={
                !preview
                  ? "请先生成成片预览"
                  : hasPendingPush
                    ? "当前最新推送尚未同意或拒绝，不能继续推送后续片段"
                  : !selectedPushEndVideo
                    ? "当前没有可推送的后续片段"
                    : undefined
              }
              className="!min-h-8 !px-3"
            >
              <span className="inline-flex items-center gap-1">
                {pushing ? <Loader2 size={13} className="animate-spin" /> : <Film size={13} />}
                推送后处理
              </span>
            </BookButton>
            </div>
          </div>
        </div>

        <div className="relative min-h-0 flex-1 overflow-hidden rounded border border-book-border/50 bg-black">
          {playbackSrc ? (
            <video
              controls
              playsInline
              preload="metadata"
              src={playbackSrc}
              className="h-full min-h-[260px] w-full bg-black object-contain"
              onLoadStart={() => setVideoLoading(true)}
              onLoadedMetadata={handleVideoLoaded}
              onLoadedData={handleVideoLoaded}
              onCanPlay={handleVideoLoaded}
              onError={handleVideoError}
            />
          ) : (
            <div className="flex h-full min-h-[320px] items-center justify-center bg-book-bg-tertiary">
              <EmptyHint text={clips.length === 0 ? "还没有可拼接的视频。" : "等待成片预览。"} />
            </div>
          )}
          {(loading || cacheLoading || videoLoading) && (
            <div className="pointer-events-none absolute inset-0 flex items-center justify-center bg-black/35 text-xs text-white">
              <span className="inline-flex items-center gap-1.5 rounded bg-black/55 px-2 py-1">
                <Loader2 size={12} className="animate-spin" />
                {loading
                  ? "正在生成预览"
                  : cacheLoading
                    ? "正在加载缓存预览"
                    : "正在加载预览"}
              </span>
            </div>
          )}
        </div>

        {error && (
          <div className="mt-2 rounded border border-error/40 bg-error/10 px-2 py-1.5 text-[11px] text-error">
            {error}
          </div>
        )}
        {preview && (
          <div className="mt-2 flex flex-wrap items-center gap-2 text-[10px] text-book-text-muted">
            <span className="font-mono">videos: {preview.video_ids.join(", ")}</span>
            <span>{preview.clips.length} 段</span>
            <span>{preview.updated_at ? formatVideoCreatedAt(preview.updated_at) : ""}</span>
          </div>
        )}
      </BookCard>

      <BookCard className="flex min-h-0 flex-col p-3">
        <div className="mb-2 flex items-center justify-between gap-2 text-xs">
          <h3 className="inline-flex items-center gap-1.5 font-serif text-sm text-book-text-main">
            <ListVideo size={14} className="text-book-primary" />
            片段
          </h3>
          <span className="text-book-text-muted">{clips.length} 条</span>
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto custom-scrollbar pr-1">
          {clips.length === 0 ? (
            <EmptyHint text="还没有完成的视频片段。" />
          ) : (
            <ul className="space-y-2">
              {clips.map((video, index) => {
                const isInsert = video.video_type === ASSEMBLY_INSERT_VIDEO_TYPE;
                const insertAfterVideoId = assemblyInsertAfterVideoId(video);
                const pushed = pushedSourceIds.has(video.id);
                const inSelectedPushRange = selectedPushRangeIds.has(video.id);
                const insertInsidePushed =
                  pushedPrefixLength > 0 && index < Math.max(0, pushedPrefixLength - 1);
                return (
                  <li
                    key={video.id}
                    className={`rounded border p-2 ${
                      isInsert
                        ? "border-book-primary/35 bg-book-primary/10"
                        : "border-book-border/50 bg-book-bg-paper/50"
                    }`}
                  >
                    <div className="mb-1.5 flex min-w-0 flex-wrap items-center gap-1 text-xs">
                      <span className="font-mono text-book-text-main">{index + 1}</span>
                      <span className="pyqt-pill !text-[10px]">
                        {assemblyClipLabel(video)}
                      </span>
                      <span className={`pyqt-pill !text-[10px] ${statusClass(video.status)}`}>
                        {video.status}
                      </span>
                      {pushed && <span className="pyqt-pill !text-[10px]">已推送</span>}
                      {inSelectedPushRange && (
                        <span className="pyqt-pill !text-[10px]">本次推送</span>
                      )}
                      <span className="ml-auto inline-flex items-center gap-1 font-mono text-[10px] text-book-text-muted">
                        <Clock3 size={10} />
                        {formatSeconds(video.duration_secs)}
                      </span>
                    </div>
                    <div className="truncate text-[11px] text-book-text-main">
                      视频 #{video.id} · {video.model}
                    </div>
                    {isInsert && (
                      <div className="mt-1 text-[10px] text-book-text-muted">
                        {insertAfterVideoId == null
                          ? "插入位置：成片开头"
                          : `插入位置：视频 #${insertAfterVideoId} 之后`}
                      </div>
                    )}
                    <div className="mt-1 line-clamp-2 text-[10px] leading-relaxed text-book-text-sub">
                      {video.prompt}
                    </div>
                    <div className="mt-2 flex flex-wrap items-center justify-end gap-1">
                      <BookButton
                        variant="secondary"
                        size="sm"
                        onClick={() => void handleImportInsert(video.id)}
                        disabled={
                          loading ||
                          cacheLoading ||
                          exporting ||
                          pushing ||
                          acting != null ||
                          insertInsidePushed
                        }
                        title={insertInsidePushed ? "不能在已推送列表中间插入衔接片段" : undefined}
                        className="!min-h-7 !px-2 text-[11px]"
                      >
                        <span className="inline-flex items-center gap-1">
                          {acting === `insert-${video.id}` ? (
                            <Loader2 size={11} className="animate-spin" />
                          ) : (
                            <Plus size={11} />
                          )}
                          在此后插入
                        </span>
                      </BookButton>
                      {isInsert && (
                        <BookButton
                          variant="secondary"
                          size="sm"
                          onClick={() => void handleDeleteInsert(video)}
                          disabled={
                            loading ||
                            cacheLoading ||
                            exporting ||
                            pushing ||
                            acting != null ||
                            pushed
                          }
                          title={pushed ? "已推送衔接片段不能删除" : undefined}
                          className="!min-h-7 !px-2 text-[11px] text-error"
                        >
                          <span className="inline-flex items-center gap-1">
                            {acting === `delete-${video.id}` ? (
                              <Loader2 size={11} className="animate-spin" />
                            ) : (
                              <Trash2 size={11} />
                            )}
                            删除
                          </span>
                        </BookButton>
                      )}
                    </div>
                  </li>
                );
              })}
            </ul>
          )}
        </div>
      </BookCard>
    </div>
  );
};
