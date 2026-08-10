import React, { useEffect, useRef, useState } from "react";
import {
  ArrowLeft,
  Check,
  Copy,
  Eye,
  Film,
  Pencil,
  RefreshCw,
  Save,
  Scissors,
  Star,
  Trash2,
} from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { extractErrorMessage } from "../../../api";
import type { GeneratedVideo, UpdateVideoInput } from "../../../api/video";
import { useToast } from "../../../components/feedback/Toast";
import { BookButton } from "../../../components/ui/BookButton";
import { Dropdown } from "../../../components/ui/Dropdown";
import { Modal } from "../../../components/ui/Modal";
import { EmptyHint } from "../shared";
import { copyTextToClipboard } from "../../../utils/clipboard";
import { toMediaSrc } from "../../../utils/mediaSrc";
import { getOperationStatusLabel } from "../../../utils/workflowPresentation";
import { useVideoPlaybackSource } from "./useVideoPlaybackSource";
import {
  formatVideoCreatedAt,
  promptCharCount,
  videoProviderRequestPreviewBlockedReason,
} from "./videoTabUtils";

interface VideoResultsPaneProps {
  selectedVideo: GeneratedVideo | null;
  adoptedVideoId: number | null;
  selectedVideoPath: string;
  trimStart: string;
  trimEnd: string;
  acting: string | null;
  onPollVideo: (video: GeneratedVideo) => void | Promise<void>;
  onPreviewVideoRequest: (video: GeneratedVideo) => void | Promise<void>;
  onToggleVideoFavorite: (video: GeneratedVideo) => void | Promise<void>;
  onUpdateVideo: (video: GeneratedVideo, input: UpdateVideoInput) => void | Promise<void>;
  onDeleteVideo: (video: GeneratedVideo) => void | Promise<void>;
  onTrimStartChange: (value: string) => void;
  onTrimEndChange: (value: string) => void;
  onTrim: () => void | Promise<void>;
  onOpenSegmentConfig: () => void;
  onAdoptVideo: (video: GeneratedVideo) => void | Promise<void>;
}

type VideoActionItems = React.ComponentProps<typeof Dropdown>["items"];

const videoRecordTitle = (video: GeneratedVideo): string => {
  const metadata =
    video.metadata_json && typeof video.metadata_json === "object" && !Array.isArray(video.metadata_json)
      ? (video.metadata_json as Record<string, unknown>)
      : null;
  const shotNumber = metadata?.shot_number;
  if (video.video_type === "chapter_video") {
    return typeof shotNumber === "number" && Number.isInteger(shotNumber) && shotNumber > 0
      ? `镜头 ${shotNumber}`
      : "章节视频";
  }
  if (video.video_type === "reference_media_video") return "参考媒体视频";
  if (video.video_type === "trimmed_clip") return "裁剪片段";
  if (video.video_type === "canvas_video") return "画布视频";
  return "生成视频";
};

export const VideoResultsPane: React.FC<VideoResultsPaneProps> = ({
  selectedVideo,
  adoptedVideoId,
  selectedVideoPath,
  trimStart,
  trimEnd,
  acting,
  onPollVideo,
  onPreviewVideoRequest,
  onToggleVideoFavorite,
  onUpdateVideo,
  onDeleteVideo,
  onTrimStartChange,
  onTrimEndChange,
  onTrim,
  onOpenSegmentConfig,
  onAdoptVideo,
}) => {
  const addToast = useToast((state) => state.addToast);
  const [promptOpen, setPromptOpen] = useState(false);
  const [promptEditing, setPromptEditing] = useState(false);
  const [promptDraft, setPromptDraft] = useState("");
  const [negativePromptDraft, setNegativePromptDraft] = useState("");
  const [promptSaving, setPromptSaving] = useState(false);
  const promptSavingRef = useRef(false);
  const selectedVideoRecord = selectedVideo;
  const selectedVideoPoster = toMediaSrc(selectedVideoRecord?.thumbnail_path);
  const {
    videoPlaybackSrc,
    videoLoadError,
    videoLoading,
    handleVideoLoadStart,
    handleVideoLoaded,
    handleVideoError,
  } = useVideoPlaybackSource({
    selectedVideo: selectedVideoRecord,
    mediaKind: "preview",
  });
  useEffect(() => {
    promptSavingRef.current = false;
    setPromptEditing(false);
    setPromptSaving(false);
    setPromptDraft(selectedVideoRecord?.prompt ?? "");
    setNegativePromptDraft(selectedVideoRecord?.negative_prompt ?? "");
  }, [
    selectedVideoRecord?.id,
    selectedVideoRecord?.negative_prompt,
    selectedVideoRecord?.prompt,
  ]);

  const copyText = async (label: string, text: string | null | undefined) => {
    const value = text?.trim();
    if (!value) {
      addToast(`${label}为空，无法复制`, "error");
      return;
    }
    try {
      await copyTextToClipboard(value);
      addToast(`已复制${label}`, "success");
    } catch (err) {
      addToast(extractErrorMessage(err, `复制${label}失败`), "error");
    }
  };

  const providerRequestSnapshotText = (video: GeneratedVideo): string | null => {
    if (!video.metadata_json || typeof video.metadata_json !== "object") return null;
    const metadata = video.metadata_json as Record<string, unknown>;
    const snapshot = metadata.provider_request_snapshot;
    if (snapshot == null) return null;
    return JSON.stringify(snapshot, null, 2);
  };

  const openPromptModal = () => {
    if (!selectedVideoRecord) return;
    setPromptDraft(selectedVideoRecord.prompt ?? "");
    setNegativePromptDraft(selectedVideoRecord.negative_prompt ?? "");
    setPromptEditing(false);
    setPromptOpen(true);
  };

  const closePromptModal = () => {
    if (promptSavingRef.current) return;
    setPromptOpen(false);
    setPromptEditing(false);
    setPromptSaving(false);
  };

  const beginPromptEdit = () => {
    if (!selectedVideoRecord) return;
    setPromptDraft(selectedVideoRecord.prompt ?? "");
    setNegativePromptDraft(selectedVideoRecord.negative_prompt ?? "");
    setPromptEditing(true);
  };

  const promptRecordDirty =
    selectedVideoRecord != null &&
    (promptDraft.trim() !== selectedVideoRecord.prompt.trim() ||
      negativePromptDraft.trim() !== (selectedVideoRecord.negative_prompt ?? "").trim());
  const promptActionBusy = promptSaving || acting != null;
  const selectedPromptChars = promptCharCount(selectedVideoRecord?.prompt);
  const selectedVideoAdopted = selectedVideoRecord?.id === adoptedVideoId;

  const handleSavePromptRecord = async () => {
    if (!selectedVideoRecord) return;
    if (promptSavingRef.current || acting != null) return;
    const prompt = promptDraft.trim();
    if (!prompt) {
      addToast("视频 Prompt 不能为空", "error");
      return;
    }
    promptSavingRef.current = true;
    setPromptSaving(true);
    try {
      await onUpdateVideo(selectedVideoRecord, {
        prompt,
        negative_prompt: negativePromptDraft.trim() || null,
      });
      setPromptEditing(false);
    } catch {
      // 调用方负责 toast；保留编辑内容便于修正后重试。
    } finally {
      promptSavingRef.current = false;
      setPromptSaving(false);
    }
  };

  const renderSelectedVideoPreview = () => {
    if (!selectedVideoRecord) {
      return (
        <div className="a4n-flat-surface min-h-[180px] self-start">
          <EmptyHint text="选择一个视频后查看生成效果。" />
        </div>
      );
    }

    const previewBlockedReason = videoProviderRequestPreviewBlockedReason(selectedVideoRecord);
    const selectedActions: VideoActionItems = [
      {
        label: "复制提示词",
        icon: <Copy size={12} />,
        onClick: () => void copyText("视频 Prompt", selectedVideoRecord.prompt),
      },
      ...(!previewBlockedReason
        ? [
            {
              label: "查看模型请求",
              icon: <Eye size={12} />,
              onClick: () => void onPreviewVideoRequest(selectedVideoRecord),
            },
          ]
        : []),
      ...(selectedVideoRecord.provider_job_id && selectedVideoRecord.status !== "completed"
        ? [
            {
              label: "刷新生成状态",
              icon: <RefreshCw size={12} />,
              onClick: () => void onPollVideo(selectedVideoRecord),
            },
          ]
        : []),
      {
        label: selectedVideoRecord.is_favorite ? "取消收藏" : "收藏视频",
        icon: <Star size={12} fill={selectedVideoRecord.is_favorite ? "currentColor" : "none"} />,
        onClick: () => void onToggleVideoFavorite(selectedVideoRecord),
      },
      { type: "divider" },
      {
        label: "删除视频",
        icon: <Trash2 size={12} />,
        danger: true,
        onClick: () => void onDeleteVideo(selectedVideoRecord),
      },
    ];
    return (
      <div
        data-video-preview-id={selectedVideoRecord.id}
        className="a4n-flat-surface min-w-0 overflow-hidden"
      >
        <div className="relative z-10 flex min-w-0 flex-wrap items-center gap-2 border-b border-book-border/40 px-3 py-2.5">
          <Film size={12} className="shrink-0 text-book-primary" />
          <div className="min-w-0 flex-1">
            <div className="truncate text-xs font-medium text-book-text-main">
              {videoRecordTitle(selectedVideoRecord)}
            </div>
            <div className="mt-0.5 truncate text-caption text-book-text-muted">
              {getOperationStatusLabel(selectedVideoRecord.status)} · {Math.round(selectedVideoRecord.duration_secs)} 秒
              {selectedVideoRecord.is_favorite ? " · 已收藏" : ""}
              {` · ${formatVideoCreatedAt(selectedVideoRecord.created_at)}`}
            </div>
          </div>
          <button
            type="button"
            onClick={openPromptModal}
            className="pyqt-btn-secondary inline-flex !min-h-9 items-center gap-1 !px-2.5 text-small sm:!min-h-7"
            title="查看完整视频提示词"
          >
            <Eye size={11} />
            提示词
          </button>
          {selectedVideoRecord.status === "completed" ? (
            <BookButton
              variant={selectedVideoAdopted ? "secondary" : "primary"}
              size="sm"
              onClick={() => void onAdoptVideo(selectedVideoRecord)}
              disabled={selectedVideoAdopted || acting === `adopt-video-${selectedVideoRecord.id}`}
              className="!min-h-9 !px-2.5 sm:!min-h-7"
              title={selectedVideoAdopted ? "当前片段已采用此视频" : "设为当前片段的合成结果"}
            >
              {acting === `adopt-video-${selectedVideoRecord.id}` ? (
                <LoadingSpinner size="sm" />
              ) : (
                <Check size={11} />
              )}
              {selectedVideoAdopted ? "已采用" : "采用"}
            </BookButton>
          ) : null}
          {acting === `poll-${selectedVideoRecord.id}` ||
          acting === `favorite-${selectedVideoRecord.id}` ||
          acting === `delete-video-${selectedVideoRecord.id}` ? (
            <span className="inline-flex h-8 w-8 items-center justify-center text-book-text-muted">
              <LoadingSpinner size="sm" />
            </span>
          ) : (
            <Dropdown items={selectedActions} />
          )}
        </div>
        {selectedVideoPath ? (
          <div className="relative bg-black">
            {videoPlaybackSrc ? (
              <video
                key={videoPlaybackSrc}
                controls
                playsInline
                preload="metadata"
                poster={selectedVideoPoster || undefined}
                src={videoPlaybackSrc}
                className="aspect-video w-full bg-black"
                onLoadStart={handleVideoLoadStart}
                onLoadedMetadata={handleVideoLoaded}
                onLoadedData={handleVideoLoaded}
                onCanPlay={handleVideoLoaded}
                onError={handleVideoError}
              />
            ) : (
              <div className="flex aspect-video w-full items-center justify-center bg-black text-xs text-white">
                正在读取视频文件
              </div>
            )}
            {videoLoading && !videoLoadError && (
              <div className="pointer-events-none absolute inset-0 flex items-center justify-center bg-black/35 text-xs text-white">
                <span className="inline-flex items-center gap-1.5 rounded bg-black/55 px-2 py-1">
                  <LoadingSpinner size="sm" />
                  正在加载视频
                </span>
              </div>
            )}
          </div>
        ) : (
          <div className="flex aspect-video w-full items-center justify-center bg-book-bg-tertiary text-xs text-book-text-muted">
            {selectedVideoRecord.provider_job_id ? "等待视频服务返回结果" : "无本地视频文件"}
          </div>
        )}
        {videoLoadError && (
          <div className="border-t border-book-border/40 px-2 py-1.5 text-small text-error">
            {videoLoadError}
          </div>
        )}
        {selectedVideoRecord.error_message && (
          <div className="border-t border-book-border/40 px-2 py-1.5 text-small text-error">
            {selectedVideoRecord.error_message}
          </div>
        )}
        <div className="border-t border-book-border/40 px-2 py-2">
          <div className="mb-1 text-caption text-book-text-muted">
            提示词 · {selectedPromptChars.toLocaleString()} 字
          </div>
          <div className="line-clamp-3 text-small leading-relaxed text-book-text-sub">
            {selectedVideoRecord.prompt}
          </div>
          {selectedVideoPath ? (
            <details className="mt-2 border-t border-book-border/35 pt-2 text-xs">
              <summary className="w-fit cursor-pointer text-caption text-book-text-muted transition-colors hover:text-book-text-main">
                裁剪视频
              </summary>
              <div className="mt-2 grid gap-2 sm:grid-cols-[90px_90px_auto]">
                <label className="grid gap-1 text-caption text-book-text-muted">
                  开始时间
                  <input
                    value={trimStart}
                    onChange={(event) => onTrimStartChange(event.target.value)}
                    className="pyqt-input !px-2 !py-1 text-xs"
                    placeholder="秒"
                  />
                </label>
                <label className="grid gap-1 text-caption text-book-text-muted">
                  结束时间
                  <input
                    value={trimEnd}
                    onChange={(event) => onTrimEndChange(event.target.value)}
                    className="pyqt-input !px-2 !py-1 text-xs"
                    placeholder="秒"
                  />
                </label>
                <button
                  type="button"
                  onClick={() => void onTrim()}
                  disabled={acting === `trim-${selectedVideoRecord.id}`}
                  className="pyqt-btn-secondary inline-flex !min-h-9 items-center justify-center gap-1 !px-2 text-xs sm:self-end sm:!min-h-[28px]"
                  title="裁剪选中视频"
                >
                  {acting === `trim-${selectedVideoRecord.id}` ? (
                    <LoadingSpinner size="sm" />
                  ) : (
                    <Scissors size={12} />
                  )}
                  裁剪
                </button>
              </div>
            </details>
          ) : null}
        </div>
      </div>
    );
  };

  if (!selectedVideoRecord) {
    return (
      <div className="a4n-flat-surface w-full self-start">
        <div className="a4n-empty-state min-h-[180px] px-4 py-6">
          <div className="flex h-9 w-9 items-center justify-center rounded-full bg-book-bg-tertiary text-book-primary">
            <Film size={16} />
          </div>
          <div className="max-w-sm">
            <h3 className="text-sm font-medium text-book-text-main">还没有生成视频</h3>
            <p className="mt-1 text-xs leading-relaxed text-book-text-muted">
              完成片段配置并生成视频后，可以在这里预览、整理和剪辑成片。
            </p>
            <div className="mt-3 xl:hidden">
              <BookButton
                variant="secondary"
                size="sm"
                onClick={onOpenSegmentConfig}
              >
                <ArrowLeft size={13} />
                前往配置
              </BookButton>
            </div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <>
      <div className="min-h-0 flex-1 overflow-y-auto custom-scrollbar pr-1">
        {renderSelectedVideoPreview()}
      </div>

      <Modal
        isOpen={promptOpen && selectedVideoRecord != null}
        onClose={closePromptModal}
        title="视频提示词"
        subtitle={
          selectedVideoRecord
            ? `${videoRecordTitle(selectedVideoRecord)} · ${Math.round(selectedVideoRecord.duration_secs)} 秒`
            : undefined
        }
        icon={<Eye size={15} />}
        maxWidthClassName="max-w-4xl"
        footer={
          promptEditing && selectedVideoRecord ? (
            <BookButton
              variant="primary"
              onClick={() => void handleSavePromptRecord()}
              disabled={promptActionBusy || !promptDraft.trim() || !promptRecordDirty}
            >
              <span className="inline-flex items-center gap-1.5">
                {promptSaving ? (
                  <LoadingSpinner size="sm" />
                ) : (
                  <Save size={12} />
                )}
                {promptSaving ? "保存中" : "保存记录"}
              </span>
            </BookButton>
          ) : undefined
        }
      >
        {selectedVideoRecord ? (
          <div className="space-y-3">
            {!promptEditing && (
              <div className="flex items-center justify-end gap-2">
                <BookButton
                  variant="primary"
                  size="sm"
                  onClick={beginPromptEdit}
                  disabled={acting != null}
                >
                  <span className="inline-flex items-center gap-1.5">
                    <Pencil size={12} />
                    编辑记录
                  </span>
                </BookButton>
                <Dropdown
                  items={[
                    {
                      label: "复制提示词",
                      icon: <Copy size={12} />,
                      onClick: () => void copyText("视频提示词", selectedVideoRecord.prompt),
                    },
                    ...(selectedVideoRecord.negative_prompt
                      ? [
                          {
                            label: "复制负面提示词",
                            icon: <Copy size={12} />,
                            onClick: () =>
                              void copyText(
                                "负面提示词",
                                selectedVideoRecord.negative_prompt,
                              ),
                          },
                        ]
                      : []),
                  ]}
                />
              </div>
            )}
            {promptEditing ? (
              <>
                <label className="block">
                  <span className="mb-1 block text-xs font-medium text-book-text-main">
                    视频提示词
                  </span>
                  <textarea
                    value={promptDraft}
                    onChange={(event) => setPromptDraft(event.target.value)}
                    disabled={promptActionBusy}
                    className="book-control custom-scrollbar min-h-[260px] max-h-[48vh] w-full resize-y px-3 py-2 text-small leading-relaxed"
                  />
                </label>
                <label className="block">
                  <span className="mb-1 block text-xs font-medium text-book-text-main">
                    负面提示词
                  </span>
                  <textarea
                    value={negativePromptDraft}
                    onChange={(event) => setNegativePromptDraft(event.target.value)}
                    disabled={promptActionBusy}
                    className="book-control custom-scrollbar min-h-[120px] max-h-56 w-full resize-y px-3 py-2 text-small leading-relaxed"
                  />
                </label>
              </>
            ) : (
              <section className="a4n-flat-surface divide-y divide-book-border/40 overflow-hidden">
                <div>
                  <div className="px-3 pt-3 text-xs font-medium text-book-text-main">
                    视频提示词
                  </div>
                  <div className="whitespace-pre-wrap break-words px-3 pb-3 pt-2 text-small leading-relaxed text-book-text-main">
                    {selectedVideoRecord.prompt || "（空）"}
                  </div>
                </div>
                {selectedVideoRecord.negative_prompt ? (
                  <div>
                    <div className="px-3 pt-3 text-xs font-medium text-book-text-main">
                      负面提示词
                    </div>
                    <div className="whitespace-pre-wrap break-words px-3 pb-3 pt-2 text-small leading-relaxed text-book-text-main">
                      {selectedVideoRecord.negative_prompt}
                    </div>
                  </div>
                ) : null}
              </section>
            )}
            {selectedVideoRecord.model || selectedVideoRecord.metadata_json ? (
              <details className="a4n-flat-surface overflow-hidden">
                <summary className="cursor-pointer px-3 py-2 text-xs font-semibold text-book-text-muted hover:text-book-text-main">
                  诊断信息
                </summary>
                <div className="border-t border-book-border/40">
                  <dl className="grid grid-cols-[72px_minmax(0,1fr)] gap-x-2 gap-y-1 px-3 py-2 text-caption">
                    <dt className="text-book-text-muted">模型</dt>
                    <dd className="break-all font-mono text-book-text-sub">
                      {selectedVideoRecord.model || "未记录"}
                    </dd>
                    <dt className="text-book-text-muted">画面规格</dt>
                    <dd className="text-book-text-sub">
                      {selectedVideoRecord.width}×{selectedVideoRecord.height} · {selectedVideoRecord.fps} FPS
                    </dd>
                  </dl>
                  {providerRequestSnapshotText(selectedVideoRecord) ? (
                    <div className="flex justify-end border-t border-book-border/35 px-3 pt-2">
                      <BookButton
                        variant="secondary"
                        size="sm"
                        onClick={() =>
                          void copyText(
                            "模型请求",
                            providerRequestSnapshotText(selectedVideoRecord),
                          )
                        }
                        className="!min-h-7 !text-small"
                      >
                        <Copy size={11} />
                        复制模型请求
                      </BookButton>
                    </div>
                  ) : null}
                  {selectedVideoRecord.metadata_json ? (
                    <pre className="whitespace-pre-wrap break-words px-3 py-2 font-mono text-small leading-relaxed text-book-text-sub">
                      {JSON.stringify(selectedVideoRecord.metadata_json, null, 2)}
                    </pre>
                  ) : null}
                </div>
              </details>
            ) : null}
          </div>
        ) : null}
      </Modal>
    </>
  );
};
