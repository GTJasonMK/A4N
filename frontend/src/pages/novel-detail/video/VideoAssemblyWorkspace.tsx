import React, {
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type SyntheticEvent,
} from "react";
import {
  Download,
  Film,
  ListVideo,
  Plus,
  RefreshCw,
  Trash2,
} from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { extractErrorMessage } from "../../../api";
import {
  assembleChapterVideos,
  deleteVideo,
  loadCachedAssembledChapterVideoPreview,
  pushAssembledChapterVideo,
  uploadAssemblyInsertVideo,
  VIDEO_TYPE_ASSEMBLY_INSERT,
  VIDEO_TYPE_ASSEMBLY_PUSH,
  VIDEO_TYPE_CHAPTER,
  VIDEO_TYPE_REFERENCE_MEDIA,
  VIDEO_WORKFLOW_MODE_MIXED,
  type AssembledChapterVideoPreview,
  type GeneratedVideo,
  type VideoTimelineSummary,
} from "../../../api/video";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import type { ToastType } from "../../../components/feedback/Toast";
import { BookButton } from "../../../components/ui/BookButton";
import { Dropdown } from "../../../components/ui/Dropdown";
import { asJsonRecord as asRecord } from "../../../utils/jsonRecord";
import { downloadBlob } from "../../../utils/downloadFile";
import { readFileAsBase64 } from "../../../utils/fileUpload";
import {
  pickVideoUploadFile,
  type VideoUploadFile,
} from "../../../utils/videoUpload";
import {
  videoPlaybackDataToBlob,
  videoPlaybackDataToDataUrl,
} from "../../../utils/videoPlayback";
import { EmptyHint, type ChapterRow } from "../shared";
import { assemblyPushSourceIds, assemblyPushStatus } from "./videoAssemblyUtils";
import { formatVideoDuration, formatVideoCreatedAt } from "./videoImageUtils";
import { getOperationStatusLabel } from "../../../utils/workflowPresentation";
import { useI18n, type MessageTranslator } from "../../../i18n";

interface VideoAssemblyWorkspaceProps {
  projectId: number;
  chapter: ChapterRow;
  videos: GeneratedVideo[];
  timeline: VideoTimelineSummary | null;
  adoptedVideoIds: ReadonlySet<number>;
  unavailableSegmentNumbers?: number[];
  onToast: (message: string, type: ToastType) => void;
  onRefresh: () => Promise<boolean | void> | boolean | void;
  onVideoRecordUpdated: (video: GeneratedVideo) => void;
}

const ASSEMBLY_ORDER_STEP = 1000;

const assemblyInsertMetadata = (
  video: GeneratedVideo,
): Record<string, unknown> | null =>
  asRecord(asRecord(video.metadata_json)?.assembly_insert);

const assemblyInsertAfterVideoId = (video: GeneratedVideo): number | null => {
  const value = assemblyInsertMetadata(video)?.insert_after_video_id;
  return Number.isInteger(value) && Number(value) > 0 ? Number(value) : null;
};

const assemblyClipSortOrder = (
  video: GeneratedVideo,
  timelineDisplayIndexByVideoId: ReadonlyMap<number, number>,
): number | null => {
  if (
    video.video_type === VIDEO_TYPE_CHAPTER ||
    video.video_type === VIDEO_TYPE_REFERENCE_MEDIA
  ) {
    const displayIndex = timelineDisplayIndexByVideoId.get(video.id);
    return displayIndex == null ? null : displayIndex * ASSEMBLY_ORDER_STEP;
  }
  if (video.video_type === VIDEO_TYPE_ASSEMBLY_INSERT) {
    const value = assemblyInsertMetadata(video)?.sort_order;
    return typeof value === "number" && Number.isFinite(value) ? value : null;
  }
  return null;
};

const assemblyClipLabel = (
  video: GeneratedVideo,
  timelineDisplayIndexByVideoId: ReadonlyMap<number, number>,
  t: MessageTranslator,
): string => {
  if (video.video_type !== VIDEO_TYPE_ASSEMBLY_INSERT) {
    const displayIndex = timelineDisplayIndexByVideoId.get(video.id);
    return displayIndex == null
      ? t("video.chapter.assemblyOrderMissing")
      : t("video.chapter.timelineItem", { number: displayIndex });
  }
  const afterShot = assemblyInsertMetadata(video)?.insert_after_shot_number;
  if (Number.isInteger(afterShot) && Number(afterShot) > 0) {
    return t("video.chapter.assemblyAfter", { number: Number(afterShot) });
  }
  return t("video.chapter.assemblyStart");
};

export const VideoAssemblyWorkspace: React.FC<VideoAssemblyWorkspaceProps> = ({
  projectId,
  chapter,
  videos,
  timeline,
  adoptedVideoIds,
  unavailableSegmentNumbers = [],
  onToast,
  onRefresh,
  onVideoRecordUpdated,
}) => {
  const { locale, t } = useI18n();
  const [preview, setPreview] = useState<AssembledChapterVideoPreview | null>(
    null,
  );
  const [playbackSrc, setPlaybackSrc] = useState("");
  const [loading, setLoading] = useState(false);
  const [cacheLoading, setCacheLoading] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [pushing, setPushing] = useState(false);
  const [pushClipCount, setPushClipCount] = useState<number | null>(1);
  const [acting, setActing] = useState<string | null>(null);
  const [videoLoading, setVideoLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const scopeKey = `${projectId}:${chapter.id}`;
  const activeScopeRef = useRef(scopeKey);
  const mountedRef = useRef(false);
  const actionBusyRef = useRef(false);
  const playbackVideoRef = useRef<HTMLVideoElement | null>(null);
  const timelineDisplayIndexByVideoId = useMemo(
    () =>
      new Map(
        (timeline?.items ?? [])
          .filter(
            (item) =>
              item.status === "active" && item.generated_video_id != null,
          )
          .map((item) => [
            item.generated_video_id as number,
            item.display_index,
          ]),
      ),
    [timeline],
  );

  const clearPreview = () => {
    setPreview(null);
    setPlaybackSrc("");
    setVideoLoading(false);
  };

  const clips = useMemo(
    () =>
      videos
        .filter(
          (video) =>
            (video.video_type === VIDEO_TYPE_CHAPTER ||
              video.video_type === VIDEO_TYPE_REFERENCE_MEDIA ||
              video.video_type === VIDEO_TYPE_ASSEMBLY_INSERT) &&
            (video.video_type === VIDEO_TYPE_ASSEMBLY_INSERT
              ? video.workflow_mode === VIDEO_WORKFLOW_MODE_MIXED
              : adoptedVideoIds.has(video.id)) &&
            video.status === "completed" &&
            Boolean(video.file_path?.trim()) &&
            assemblyClipSortOrder(video, timelineDisplayIndexByVideoId) != null,
        )
        .sort((a, b) => {
          const leftOrder =
            assemblyClipSortOrder(a, timelineDisplayIndexByVideoId) ??
            Number.MAX_SAFE_INTEGER;
          const rightOrder =
            assemblyClipSortOrder(b, timelineDisplayIndexByVideoId) ??
            Number.MAX_SAFE_INTEGER;
          return leftOrder - rightOrder || a.id - b.id;
        }),
    [adoptedVideoIds, timelineDisplayIndexByVideoId, videos],
  );
  const pushedSourceIds = useMemo(() => {
    const ids = new Set<number>();
    for (const video of videos) {
      const status = assemblyPushStatus(video);
      const sourceIds = assemblyPushSourceIds(video);
      if (
        video.video_type !== VIDEO_TYPE_ASSEMBLY_PUSH ||
        video.workflow_mode !== VIDEO_WORKFLOW_MODE_MIXED ||
        status == null ||
        status === "rejected" ||
        sourceIds == null
      ) {
        continue;
      }
      for (const id of sourceIds) ids.add(id);
    }
    return ids;
  }, [videos]);
  const hasPendingPush = useMemo(
    () =>
      videos.some(
        (video) =>
          video.video_type === VIDEO_TYPE_ASSEMBLY_PUSH &&
          video.workflow_mode === VIDEO_WORKFLOW_MODE_MIXED &&
          video.status === "completed" &&
          assemblyPushStatus(video) === "pending",
      ),
    [videos],
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
    pushableClips.length === 0 || pushClipCount == null
      ? 0
      : Math.max(1, Math.min(pushClipCount, pushableClips.length));
  const selectedPushRange = useMemo(() => {
    if (normalizedPushClipCount <= 0) return [];
    return pushableClips.slice(0, normalizedPushClipCount);
  }, [pushableClips, normalizedPushClipCount]);
  const selectedPushEndVideo =
    selectedPushRange[selectedPushRange.length - 1] ?? null;
  const selectedPushRangeIds = useMemo(
    () => new Set(selectedPushRange.map((clip) => clip.id)),
    [selectedPushRange],
  );
  const totalDuration = clips.reduce(
    (sum, video) => sum + Math.max(0, video.duration_secs),
    0,
  );
  const clipSignature = clips
    .map(
      (video) =>
        `${video.id}:${assemblyClipSortOrder(video, timelineDisplayIndexByVideoId) ?? ""}:${video.file_size ?? ""}:${
          video.updated_at
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

  useLayoutEffect(() => {
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
    void loadCachedAssembledChapterVideoPreview(
      projectId,
      chapter.id,
      VIDEO_WORKFLOW_MODE_MIXED,
    )
      .then((data) => {
        if (cancelled || !data) return;
        setPlaybackSrc(videoPlaybackDataToDataUrl(data));
        setPreview(data);
      })
      .catch((err) => {
        if (cancelled) return;
        setError(extractErrorMessage(err, t("video.chapter.cacheLoadFailed")));
      })
      .finally(() => {
        if (!cancelled) setCacheLoading(false);
      });
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectId, chapter.id, clipSignature]);

  useEffect(() => {
    setPushClipCount((current) =>
      pushableClips.length === 0 || current == null
        ? 1
        : Math.max(1, Math.min(current, pushableClips.length)),
    );
  }, [pushableClips.length]);

  const handleAssemble = async () => {
    const requestScope = scopeKey;
    if (!beginAction()) return;
    setLoading(true);
    setError(null);
    try {
      const data = await assembleChapterVideos(
        projectId,
        chapter.id,
        VIDEO_WORKFLOW_MODE_MIXED,
      );
      if (!isCurrentScope(requestScope)) return;
      const url = videoPlaybackDataToDataUrl(data);
      if (playbackVideoRef.current?.getAttribute("src") === url) {
        playbackVideoRef.current.load();
      }
      setPlaybackSrc(url);
      setPreview(data);
      onToast(data.cached ? t("video.chapter.assemblyPreviewLoaded") : t("video.chapter.assemblyPreviewGenerated"), "success");
    } catch (err) {
      if (!isCurrentScope(requestScope)) return;
      const message = extractErrorMessage(err, t("video.chapter.assemblyPreviewFailed"));
      setError(message);
      onToast(message, "error");
    } finally {
      finishAction(requestScope);
      if (isCurrentScope(requestScope)) setLoading(false);
    }
  };

  const handleDownloadPreview = async () => {
    if (!preview) return;
    const requestScope = scopeKey;
    if (!beginAction()) return;
    setExporting(true);
    setError(null);
    try {
      downloadBlob(
        `a4n-chapter-${chapter.chapter_number}-${VIDEO_WORKFLOW_MODE_MIXED}-assembled.mp4`,
        videoPlaybackDataToBlob(preview),
      );
      if (!isCurrentScope(requestScope)) return;
      onToast(t("video.chapter.assemblyPreviewDownloaded"), "success");
    } catch (err) {
      if (!isCurrentScope(requestScope)) return;
      const message = extractErrorMessage(err, t("video.chapter.assemblyPreviewDownloadFailed"));
      setError(message);
      onToast(message, "error");
    } finally {
      finishAction(requestScope);
      if (isCurrentScope(requestScope)) setExporting(false);
    }
  };

  const handlePushPreview = async () => {
    if (!selectedPushEndVideo) return;
    const requestScope = scopeKey;
    if (!beginAction()) return;
    setPushing(true);
    setError(null);
    try {
      const result = await pushAssembledChapterVideo(
        projectId,
        chapter.id,
        VIDEO_WORKFLOW_MODE_MIXED,
        { end_video_id: selectedPushEndVideo.id },
      );
      if (!isCurrentScope(requestScope)) return;
      onVideoRecordUpdated(result.video);
      clearPreview();
      const refreshed = await onRefresh();
      if (!isCurrentScope(requestScope)) return;
      onToast(
        t("video.chapter.pushSuccess", { count: result.source_video_ids.length }),
        "success",
      );
      if (refreshed === false) onToast(t("video.chapter.postprocessRefreshFailed"), "error");
    } catch (err) {
      if (!isCurrentScope(requestScope)) return;
      const message = extractErrorMessage(err, t("video.chapter.pushFailed"));
      setError(message);
      onToast(message, "error");
    } finally {
      finishAction(requestScope);
      if (isCurrentScope(requestScope)) setPushing(false);
    }
  };

  const handleImportInsert = async (insertAfterVideoId: number | null) => {
    const requestScope = scopeKey;
    const key =
      insertAfterVideoId == null
        ? "insert-start"
        : `insert-${insertAfterVideoId}`;
    if (!beginAction()) return;
    setActing(key);
    setError(null);
    let upload: VideoUploadFile | null = null;
    try {
      upload = await pickVideoUploadFile(t);
    } catch (err) {
      if (!isCurrentScope(requestScope)) return;
      onToast(extractErrorMessage(err, t("video.chapter.filePickerFailed")), "error");
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
      const inserted = await uploadAssemblyInsertVideo(
        projectId,
        chapter.id,
        VIDEO_WORKFLOW_MODE_MIXED,
        {
          file_name: file.name,
          mime_type: mimeType,
          data_base64: dataBase64,
          insert_after_video_id: insertAfterVideoId,
        },
      );
      if (!isCurrentScope(requestScope)) return;
      onVideoRecordUpdated(inserted);
      clearPreview();
      const refreshed = await onRefresh();
      if (!isCurrentScope(requestScope)) return;
      onToast(t("video.chapter.insertSuccess"), "success");
      if (refreshed === false) onToast(t("video.chapter.postprocessRefreshFailed"), "error");
    } catch (err) {
      if (!isCurrentScope(requestScope)) return;
      const message = extractErrorMessage(err, t("video.chapter.insertFailed"));
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
      title: t("video.chapter.confirmDeleteInsertedTitle"),
      message: t("video.chapter.confirmDeleteInsertedMessage"),
      confirmText: t("video.chapter.confirmDelete"),
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
      const refreshed = await onRefresh();
      if (!isCurrentScope(requestScope)) return;
      onToast(t("video.chapter.deleteSuccess"), "success");
      if (refreshed === false) onToast(t("video.chapter.postprocessRefreshFailed"), "error");
    } catch (err) {
      if (!isCurrentScope(requestScope)) return;
      const message = extractErrorMessage(err, t("video.chapter.deleteFailed"));
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
    setError(detail ? t("video.chapter.playbackFailed", { detail }) : t("video.chapter.playbackFailedEmpty"));
  };

  const assemblyMenuItems: React.ComponentProps<typeof Dropdown>["items"] = [
    ...(pushedPrefixLength === 0
      ? [
          {
            label: t("video.chapter.insertIntro"),
            icon: <Plus size={12} />,
            onClick: () => void handleImportInsert(null),
          },
        ]
      : []),
    ...(preview
      ? [
          ...(pushedPrefixLength === 0 ? [{ type: "divider" as const }] : []),
          {
            label: t("video.chapter.downloadPreview"),
            icon: <Download size={12} />,
            onClick: () => void handleDownloadPreview(),
          },
        ]
      : []),
  ];
  const assemblyBusy =
    loading || cacheLoading || exporting || pushing || acting != null;

  if (clips.length === 0) {
    return (
      <section className="a4n-flat-surface w-full self-start">
        <div className="a4n-empty-state min-h-[180px] px-4 py-6">
          <div className="flex h-9 w-9 items-center justify-center rounded-full bg-book-bg-tertiary text-book-primary">
            <ListVideo size={16} />
          </div>
          <div className="max-w-sm">
            <h3 className="text-sm font-medium text-book-text-main">
              {t("video.chapter.assemblyNoClips")}
            </h3>
          </div>
          <BookButton
            variant="secondary"
            size="sm"
            onClick={() => void handleImportInsert(null)}
            disabled={assemblyBusy}
            className="!min-h-8 !px-3"
          >
            {acting === "insert-start" ? (
              <LoadingSpinner size="sm" />
            ) : (
              <Plus size={13} />
            )}
            {t("video.chapter.insertIntro")}
          </BookButton>
          {error ? (
            <div role="alert" className="text-xs text-error">
              {error}
            </div>
          ) : null}
        </div>
      </section>
    );
  }

  return (
    <div className="grid h-full min-h-0 gap-3 xl:grid-cols-[minmax(0,1.15fr)_340px]">
      <section className="a4n-flat-surface flex min-h-0 flex-col overflow-hidden p-3">
        <div className="mb-3 flex min-w-0 flex-wrap items-center gap-2 border-b border-book-border/35 pb-3">
          <div className="min-w-0 flex-1">
            <h3 className="text-sm font-medium text-book-text-main">
              {t("video.chapter.assemblyPreview")}
            </h3>
            <p className="mt-1 text-small text-book-text-muted">
              {t("video.chapter.assemblyClipCount", { count: clips.length, duration: formatVideoDuration(totalDuration, locale, t) })}
              {pushedSourceIds.size > 0
                ? t("video.chapter.pushedClipCount", { count: pushedSourceIds.size })
                : ""}
            </p>
            {unavailableSegmentNumbers.length > 0 ? (
              <p className="mt-1 text-caption text-warning">
                {t("video.chapter.unavailableClips", { numbers: unavailableSegmentNumbers.join(", ") })}
              </p>
            ) : null}
          </div>
          {acting === "insert-start" || exporting ? (
            <span
              role="status"
              className="inline-flex h-8 w-8 items-center justify-center text-book-text-muted"
            >
              <LoadingSpinner size="sm" />
            </span>
          ) : !assemblyBusy &&
            assemblyMenuItems.length > 0 ? (
            <Dropdown items={assemblyMenuItems} />
          ) : null}
          <BookButton
            variant={preview ? "secondary" : "primary"}
            size="sm"
            onClick={() => void handleAssemble()}
            disabled={assemblyBusy}
            className="!min-h-8 !px-3"
          >
            <span className="inline-flex items-center gap-1">
              {loading ? <LoadingSpinner size="sm" /> : <RefreshCw size={13} />}
              {preview ? t("video.chapter.assemblyPreviewUpdated") : t("video.chapter.assemblyPreviewGenerate")}
            </span>
          </BookButton>
        </div>
        {preview ? (
          <div className="mb-3 flex min-w-0 flex-wrap items-center gap-2 rounded-xl bg-book-bg-tertiary/35 p-2.5">
            <label className="flex min-h-8 items-center gap-2 text-small text-book-text-muted">
              <span>{t("video.chapter.pushClipCount")}</span>
              <input
                type="number"
                min={1}
                max={Math.max(1, pushableClips.length)}
                value={
                  pushableClips.length === 0 || pushClipCount == null
                    ? ""
                    : pushClipCount
                }
                onChange={(event) => {
                  if (event.target.value === "") {
                    setPushClipCount(null);
                    return;
                  }
                  const value = Number(event.target.value);
                  if (!Number.isFinite(value)) return;
                  setPushClipCount(
                    Math.max(
                      1,
                      Math.min(
                        Math.round(value),
                        Math.max(1, pushableClips.length),
                      ),
                    ),
                  );
                }}
                disabled={
                  loading ||
                  cacheLoading ||
                  exporting ||
                  pushing ||
                  pushableClips.length === 0
                }
                className="pyqt-input h-8 w-16 !px-2 !py-1 text-small"
              />
            </label>
            <span className="min-w-0 flex-1 truncate text-small text-book-text-muted">
              {selectedPushEndVideo
                ? t("video.chapter.pushTo", {
                    label: assemblyClipLabel(
                      selectedPushEndVideo,
                      timelineDisplayIndexByVideoId,
                      t,
                    ),
                  })
                : t("video.chapter.noPushableClips")}
            </span>
            <BookButton
              variant="primary"
              size="sm"
              onClick={() => void handlePushPreview()}
              disabled={
                pushing ||
                exporting ||
                loading ||
                cacheLoading ||
                acting != null ||
                !selectedPushEndVideo ||
                hasPendingPush
              }
              title={
                hasPendingPush
                  ? t("video.chapter.pushBlockedPending")
                  : !selectedPushEndVideo
                    ? t("video.chapter.pushBlockedNone")
                    : undefined
              }
              className="!min-h-8 !px-3"
            >
              <span className="inline-flex items-center gap-1">
                {pushing ? <LoadingSpinner size="sm" /> : <Film size={13} />}
                {t("video.chapter.pushPostprocess")}
              </span>
            </BookButton>
          </div>
        ) : null}

        <div className="relative min-h-0 flex-1 overflow-hidden rounded-2xl border border-book-border/50 bg-black">
          {playbackSrc ? (
            <video
              ref={playbackVideoRef}
              key={playbackSrc}
              controls
              playsInline
              preload="auto"
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
              <EmptyHint text={t("video.chapter.waitingAssemblyPreview")} />
            </div>
          )}
          {(loading || cacheLoading || videoLoading) && (
            <div className="pointer-events-none absolute inset-0 flex items-center justify-center bg-black/35 text-xs text-white">
              <span className="inline-flex items-center gap-1.5 rounded-xl bg-black/55 px-2.5 py-1.5 backdrop-blur-sm">
                <LoadingSpinner size="sm" />
                {loading
                  ? t("video.chapter.generatingPreview")
                  : cacheLoading
                    ? t("video.chapter.loadingCachedPreview")
                    : t("video.chapter.loadingPreview")}
              </span>
            </div>
          )}
        </div>

        {error && (
          <div
            role="alert"
            className="mt-2 rounded-xl border border-error/40 bg-error-bg px-2.5 py-2 text-small text-error"
          >
            {error}
          </div>
        )}
        {preview && (
          <div className="mt-2 text-caption leading-relaxed text-book-text-muted">
            {t("video.chapter.assemblyClipCount", {
              count: preview.clips.length,
              duration: formatVideoDuration(preview.duration_secs, locale, t),
            })} · {preview.cached ? t("video.chapter.cachedPreview") : t("video.chapter.currentGeneration")}
            {preview.updated_at
              ? ` · ${formatVideoCreatedAt(preview.updated_at, locale, t)}`
              : ""}
          </div>
        )}
      </section>

      <section className="a4n-flat-surface flex min-h-0 flex-col overflow-hidden">
        <div className="flex shrink-0 items-baseline justify-between gap-3 border-b border-book-border/35 px-3 py-2.5">
          <h3 className="text-xs font-medium text-book-text-main">{t("video.chapter.clipOrder")}</h3>
          <span className="text-caption text-book-text-muted">
            {t("video.chapter.segments", { count: clips.length })}
          </span>
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto custom-scrollbar">
          <ul className="divide-y divide-book-border/35">
            {clips.map((video, index) => {
              const isInsert = video.video_type === VIDEO_TYPE_ASSEMBLY_INSERT;
              const insertAfterVideoId = assemblyInsertAfterVideoId(video);
              const pushed = pushedSourceIds.has(video.id);
              const inSelectedPushRange = selectedPushRangeIds.has(video.id);
              const insertInsidePushed =
                pushedPrefixLength > 0 &&
                index < Math.max(0, pushedPrefixLength - 1);
              const rowActionBusy =
                acting === `insert-${video.id}` ||
                acting === `delete-${video.id}`;
              const rowActionItems: React.ComponentProps<
                typeof Dropdown
              >["items"] = [
                ...(!insertInsidePushed
                  ? [
                      {
                        label: t("video.chapter.insertAfter"),
                        icon: <Plus size={12} />,
                        onClick: () => void handleImportInsert(video.id),
                      },
                    ]
                  : []),
                ...(isInsert && !pushed
                  ? [
                      ...(!insertInsidePushed
                        ? [{ type: "divider" as const }]
                        : []),
                      {
                        label: t("video.chapter.deleteInserted"),
                        icon: <Trash2 size={12} />,
                        danger: true,
                        onClick: () => void handleDeleteInsert(video),
                      },
                    ]
                  : []),
              ];
              return (
                <li
                  key={video.id}
                  className={`relative p-3 transition-colors ${
                    inSelectedPushRange
                      ? "border-l-2 border-book-primary bg-book-primary/[0.07]"
                      : isInsert
                        ? "border-l-2 border-book-primary/55 bg-book-primary/[0.04]"
                        : "border-l-2 border-transparent"
                  }`}
                >
                  <div className="flex min-w-0 items-start gap-2">
                    <div className="min-w-0 flex-1">
                      <div className="truncate text-xs font-medium text-book-text-main">
                        {t("video.chapter.clipNumber", { number: index + 1 })} ·{" "}
                        {assemblyClipLabel(
                          video,
                          timelineDisplayIndexByVideoId,
                          t,
                        )}
                      </div>
                      <div className="mt-1 text-caption text-book-text-muted">
                        {formatVideoDuration(video.duration_secs, locale, t)} ·{" "}
                        {getOperationStatusLabel(video.status, locale)}
                        {pushed ? t("video.chapter.pushed") : ""}
                      </div>
                    </div>
                    {rowActionBusy ? (
                      <span
                        role="status"
                        className="inline-flex h-8 w-8 shrink-0 items-center justify-center text-book-text-muted"
                      >
                        <LoadingSpinner size="sm" />
                      </span>
                    ) : !assemblyBusy && rowActionItems.length > 0 ? (
                      <Dropdown items={rowActionItems} />
                    ) : null}
                  </div>
                  {isInsert && (
                    <div className="mt-1 text-caption text-book-text-muted">
                      {insertAfterVideoId == null
                        ? t("video.chapter.insertAtStart")
                        : t("video.chapter.insertAfterSelected")}
                    </div>
                  )}
                  <div className="mt-1 line-clamp-2 text-caption leading-relaxed text-book-text-sub">
                    {video.prompt}
                  </div>
                </li>
              );
            })}
          </ul>
        </div>
      </section>
    </div>
  );
};
