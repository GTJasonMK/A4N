import React, { useEffect, useRef, useState } from "react";
import {
  Copy,
  Eye,
  Film,
  Loader2,
  Pencil,
  RefreshCw,
  Save,
  Scissors,
  Star,
  Trash2,
  X,
} from "lucide-react";
import { extractErrorMessage } from "../../../api";
import type { GeneratedVideo, UpdateVideoInput } from "../../../api/video";
import { useToast } from "../../../components/feedback/Toast";
import { BookButton } from "../../../components/ui/BookButton";
import { Modal } from "../../../components/ui/Modal";
import { EmptyHint } from "../shared";
import { copyTextToClipboard } from "../../../utils/clipboard";
import { toMediaSrc } from "../../../utils/mediaSrc";
import { statusClass } from "./videoGenerationDisplay";
import { useVideoPlaybackSource } from "./useVideoPlaybackSource";
import {
  formatVideoCreatedAt,
  promptByteCount,
  promptCharCount,
  videoProviderRequestPreviewBlockedReason,
} from "./videoTabUtils";

interface VideoResultsPaneProps {
  videos: GeneratedVideo[];
  selectedVideo: GeneratedVideo | null;
  selectedVideoId: number | null;
  totalVideoAssetCount: number;
  selectedVideoPath: string;
  trimStart: string;
  trimEnd: string;
  acting: string | null;
  onSelectVideo: (id: number) => void;
  onPollVideo: (video: GeneratedVideo) => void | Promise<void>;
  onPreviewVideoRequest?: (video: GeneratedVideo) => void | Promise<void>;
  onToggleVideoFavorite: (video: GeneratedVideo) => void | Promise<void>;
  onUpdateVideo?: (video: GeneratedVideo, input: UpdateVideoInput) => void | Promise<void>;
  onDeleteVideo: (video: GeneratedVideo) => void | Promise<void>;
  onDeleteAllVideoAssets: () => void | Promise<void>;
  onTrimStartChange: (value: string) => void;
  onTrimEndChange: (value: string) => void;
  onTrim: () => void | Promise<void>;
}

export const VideoResultsPane: React.FC<VideoResultsPaneProps> = ({
  videos,
  selectedVideo,
  selectedVideoId,
  totalVideoAssetCount,
  selectedVideoPath,
  trimStart,
  trimEnd,
  acting,
  onSelectVideo,
  onPollVideo,
  onPreviewVideoRequest,
  onToggleVideoFavorite,
  onUpdateVideo,
  onDeleteVideo,
  onDeleteAllVideoAssets,
  onTrimStartChange,
  onTrimEndChange,
  onTrim,
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
    selectedVideoPath: selectedVideoRecord ? selectedVideoPath : "",
  });
  const deleteAllVideoAssetsBlockedReason =
    totalVideoAssetCount === 0 ? "没有可删除的视频资产" : undefined;

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

  const cancelPromptEdit = () => {
    if (!selectedVideoRecord) return;
    setPromptDraft(selectedVideoRecord.prompt ?? "");
    setNegativePromptDraft(selectedVideoRecord.negative_prompt ?? "");
    setPromptEditing(false);
  };

  const promptRecordDirty =
    selectedVideoRecord != null &&
    (promptDraft.trim() !== selectedVideoRecord.prompt.trim() ||
      negativePromptDraft.trim() !== (selectedVideoRecord.negative_prompt ?? "").trim());
  const promptActionBusy = promptSaving || acting != null;
  const selectedPromptChars = promptCharCount(selectedVideoRecord?.prompt);
  const selectedPromptBytes = promptByteCount(selectedVideoRecord?.prompt);

  const handleSavePromptRecord = async () => {
    if (!selectedVideoRecord || !onUpdateVideo) return;
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
        <div className="a4n-page-surface flex min-h-[260px] items-center justify-center">
          <EmptyHint text="选择一个视频后查看生成效果。" />
        </div>
      );
    }

    const previewBlockedReason = videoProviderRequestPreviewBlockedReason(selectedVideoRecord);
    const previewBlockedLabel = previewBlockedReason?.includes("Chat 生视频")
      ? "外部生成"
      : "请求快照缺失";
    return (
      <div className="a4n-page-surface min-w-0 overflow-hidden">
        <div className="relative z-10 flex flex-wrap items-center justify-between gap-2 border-b border-book-border/40 px-2 py-1.5 text-xs">
          <div className="flex min-w-0 flex-wrap items-center gap-1">
            <span className="font-mono text-book-text-main">#{selectedVideoRecord.id}</span>
            <span className={`pyqt-pill !text-[10px] ${statusClass(selectedVideoRecord.status)}`}>
              {selectedVideoRecord.status}
            </span>
            <span className="min-w-0 truncate text-book-text-sub">{selectedVideoRecord.model}</span>
            <span className="font-mono text-book-text-muted">
              {Math.round(selectedVideoRecord.duration_secs)}s
            </span>
            {selectedVideoRecord.is_favorite && (
              <Star size={11} className="fill-book-primary text-book-primary" />
            )}
          </div>
          <div className="relative z-20 flex shrink-0 items-center gap-1">
            <button
              type="button"
              onClick={() => void copyText("视频 Prompt", selectedVideoRecord.prompt)}
              className="pyqt-btn-secondary !min-h-7 !px-2 text-[11px] inline-flex items-center gap-1"
              title="复制当前视频提示词"
            >
              <Copy size={11} />
              复制提示词
            </button>
            <button
              type="button"
              onClick={openPromptModal}
              className="pyqt-btn-secondary !min-h-7 !px-2 text-[11px] inline-flex items-center gap-1"
              title="查看完整视频提示词"
            >
              <Eye size={11} />
              完整提示词
            </button>
            {onPreviewVideoRequest && !previewBlockedReason ? (
              <button
                type="button"
                onClick={() => void onPreviewVideoRequest(selectedVideoRecord)}
                className="pyqt-btn-secondary !min-h-9 !px-2 text-[11px] inline-flex items-center gap-1 sm:!min-h-7"
                title="查看生成时发给 provider 的完整请求"
              >
                <Eye size={11} />
                模型请求
              </button>
            ) : previewBlockedReason ? (
              <span
                className="inline-flex min-h-7 items-center text-[10px] text-book-text-muted"
                title={previewBlockedReason}
              >
                {previewBlockedLabel}
              </span>
            ) : null}
            {selectedVideoRecord.provider_job_id && selectedVideoRecord.status !== "completed" && (
              <button
                type="button"
                onClick={() => void onPollVideo(selectedVideoRecord)}
                disabled={acting === `poll-${selectedVideoRecord.id}`}
                className="pyqt-btn-secondary !min-h-7 !px-2 text-[11px] inline-flex items-center gap-1"
              >
                {acting === `poll-${selectedVideoRecord.id}` ? (
                  <Loader2 size={11} className="animate-spin" />
                ) : (
                  <RefreshCw size={11} />
                )}
                轮询
              </button>
            )}
            <button
              type="button"
              onClick={() => void onToggleVideoFavorite(selectedVideoRecord)}
              disabled={acting === `favorite-${selectedVideoRecord.id}`}
              className="pyqt-btn-icon !h-9 !w-9 sm:!h-7 sm:!w-7"
              title={selectedVideoRecord.is_favorite ? "取消收藏" : "收藏"}
            >
              <Star
                size={12}
                fill={selectedVideoRecord.is_favorite ? "currentColor" : "none"}
              />
            </button>
            <button
              type="button"
              onClick={() => void onDeleteVideo(selectedVideoRecord)}
              disabled={acting === `delete-video-${selectedVideoRecord.id}`}
              className="pyqt-btn-icon !h-9 !w-9 !border-error/50 !text-error sm:!h-7 sm:!w-7"
              title="删除视频"
            >
              <Trash2 size={12} />
            </button>
          </div>
        </div>
        {selectedVideoRecord && selectedVideoPath ? (
          <div className="relative bg-black">
            {videoPlaybackSrc ? (
              <video
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
                  <Loader2 size={12} className="animate-spin" />
                  正在加载视频
                </span>
              </div>
            )}
          </div>
        ) : (
          <div className="flex aspect-video w-full items-center justify-center bg-book-bg-tertiary text-xs text-book-text-muted">
            {selectedVideoRecord.provider_job_id ? "等待 provider 生成结果" : "无本地视频文件"}
          </div>
        )}
        {videoLoadError && (
          <div className="border-t border-book-border/40 px-2 py-1.5 text-[11px] text-error">
            {videoLoadError}
          </div>
        )}
        {selectedVideoRecord.error_message && (
          <div className="border-t border-book-border/40 px-2 py-1.5 text-[11px] text-error">
            {selectedVideoRecord.error_message}
          </div>
        )}
        <div className="border-t border-book-border/40 px-2 py-2">
          <div className="mb-1 font-mono text-[10px] text-book-text-muted">
            Prompt {selectedPromptChars.toLocaleString()} 字符 / {selectedPromptBytes.toLocaleString()} 字节
          </div>
          <div className="mb-2 line-clamp-3 text-[11px] leading-relaxed text-book-text-sub">
            {selectedVideoRecord.prompt}
          </div>
          <div className="grid grid-cols-[90px_90px_auto] gap-1 text-xs">
            <input
              value={trimStart}
              onChange={(event) => onTrimStartChange(event.target.value)}
              className="pyqt-input !px-2 !py-1"
              placeholder="开始秒"
            />
            <input
              value={trimEnd}
              onChange={(event) => onTrimEndChange(event.target.value)}
              className="pyqt-input !px-2 !py-1"
              placeholder="结束秒"
            />
            <button
              type="button"
              onClick={() => void onTrim()}
              disabled={acting === `trim-${selectedVideoRecord.id}` || !selectedVideoPath}
              className="pyqt-btn-secondary !min-h-9 !px-2 text-xs inline-flex items-center justify-center gap-1 sm:!min-h-[28px]"
              title="裁剪选中视频"
            >
              {acting === `trim-${selectedVideoRecord.id}` ? (
                <Loader2 size={12} className="animate-spin" />
              ) : (
                <Scissors size={12} />
              )}
              裁剪
            </button>
          </div>
        </div>
      </div>
    );
  };

  const renderVideoHistory = () => {
    if (videos.length === 0) {
      return <EmptyHint text="还没有生成视频。" />;
    }

    return (
      <ul className="space-y-2">
        {videos.map((video) => {
          const selected = selectedVideoId === video.id;
          return (
            <li
              key={video.id}
              className={`a4n-interactive-row flex items-stretch overflow-hidden ${
                selected ? "border-book-primary bg-book-primary/10" : ""
              }`}
            >
              <button
                type="button"
                onClick={() => onSelectVideo(video.id)}
                className="min-w-0 flex-1 p-2 text-left focus-visible:outline focus-visible:outline-1 focus-visible:outline-book-primary"
              >
                <div className="mb-1.5 flex min-w-0 flex-wrap items-center gap-1 text-xs">
                  <span className="font-mono text-book-text-main">#{video.id}</span>
                  <span className={`pyqt-pill !text-[10px] ${statusClass(video.status)}`}>
                    {video.status}
                  </span>
                  <span className="min-w-0 flex-1 truncate text-book-text-main">
                    {video.model}
                  </span>
                  <span className="font-mono text-book-text-muted">
                    {Math.round(video.duration_secs)}s
                  </span>
                  {video.is_favorite && (
                    <Star size={11} className="fill-book-primary text-book-primary" />
                  )}
                </div>
                <div className="line-clamp-2 text-[11px] leading-relaxed text-book-text-sub">
                  {video.prompt}
                </div>
                <div className="mt-1 font-mono text-[10px] text-book-text-muted">
                  Prompt {promptCharCount(video.prompt).toLocaleString()} 字符 / {promptByteCount(video.prompt).toLocaleString()} 字节
                </div>
                <div className="mt-1 truncate text-[10px] text-book-text-muted">
                  {formatVideoCreatedAt(video.created_at)}
                </div>
                {video.error_message && (
                  <div className="mt-1 line-clamp-2 text-[10px] text-error">
                    {video.error_message}
                  </div>
                )}
              </button>
              <div className="flex shrink-0 items-start border-l border-book-border/40 p-1.5">
                <button
                  type="button"
                  onClick={() => void onDeleteVideo(video)}
                  disabled={acting === `delete-video-${video.id}`}
                  className="pyqt-btn-icon relative z-20 !h-9 !w-9 !border-error/50 !text-error sm:!h-7 sm:!w-7"
                  title="删除视频"
                >
                  {acting === `delete-video-${video.id}` ? (
                    <Loader2 size={12} className="animate-spin" />
                  ) : (
                    <Trash2 size={12} />
                  )}
                </button>
              </div>
            </li>
          );
        })}
      </ul>
    );
  };

  return (
    <>
      <div className="grid min-h-0 flex-1 gap-3 xl:grid-cols-[minmax(0,1.15fr)_340px]">
        <div className="min-h-0 overflow-y-auto custom-scrollbar pr-1">
          {renderSelectedVideoPreview()}
        </div>
        <div className="flex min-h-0 flex-col">
          <div className="mb-2 flex items-center justify-between gap-2 text-xs">
            <h3 className="inline-flex items-center gap-1.5 font-serif text-sm text-book-text-main">
              <Film size={14} className="text-book-primary" />
              成片记录
            </h3>
            <div className="flex shrink-0 items-center gap-2">
              <span className="text-book-text-muted">
                {videos.length} 条
              </span>
              <BookButton
                variant="danger"
                size="sm"
                onClick={() => void onDeleteAllVideoAssets()}
                disabled={
                  acting === "delete-video-assets" ||
                  deleteAllVideoAssetsBlockedReason != null
                }
                title={deleteAllVideoAssetsBlockedReason}
                className="!min-h-7 !px-2"
              >
                <span className="inline-flex items-center gap-1">
                  {acting === "delete-video-assets" ? (
                    <Loader2 size={12} className="animate-spin" />
                  ) : (
                    <Trash2 size={12} />
                  )}
                  删除全部
                </span>
              </BookButton>
            </div>
          </div>
          <div className="min-h-0 flex-1 overflow-y-auto custom-scrollbar pr-1">
            {renderVideoHistory()}
          </div>
        </div>
      </div>

      <Modal
        isOpen={promptOpen && selectedVideoRecord != null}
        onClose={closePromptModal}
        title={selectedVideoRecord ? `视频提示词 #${selectedVideoRecord.id}` : "视频提示词"}
        subtitle={selectedVideoRecord ? `${selectedVideoRecord.video_type} · ${selectedVideoRecord.model}` : undefined}
        icon={<Eye size={15} />}
        maxWidthClassName="max-w-4xl"
        footer={
          <div className="flex flex-wrap items-center justify-end gap-2">
            {selectedVideoRecord ? (
              <>
                {promptEditing ? (
                  <>
                    <BookButton
                      variant="secondary"
                      onClick={cancelPromptEdit}
                      disabled={promptActionBusy}
                    >
                      <span className="inline-flex items-center gap-1.5">
                        <X size={12} />
                        取消编辑
                      </span>
                    </BookButton>
                    <BookButton
                      variant="primary"
                      onClick={() => void handleSavePromptRecord()}
                      disabled={promptActionBusy || !promptDraft.trim() || !promptRecordDirty}
                    >
                      <span className="inline-flex items-center gap-1.5">
                        {promptSaving ? (
                          <Loader2 size={12} className="animate-spin" />
                        ) : (
                          <Save size={12} />
                        )}
                        保存记录
                      </span>
                    </BookButton>
                  </>
                ) : (
                  <>
                    <BookButton
                      variant="secondary"
                      onClick={() => void copyText("视频 Prompt", selectedVideoRecord.prompt)}
                    >
                      <span className="inline-flex items-center gap-1.5">
                        <Copy size={12} />
                        复制 Prompt
                      </span>
                    </BookButton>
                    <BookButton
                      variant="secondary"
                      onClick={() => void copyText("Negative Prompt", selectedVideoRecord.negative_prompt)}
                    >
                      <span className="inline-flex items-center gap-1.5">
                        <Copy size={12} />
                        复制 Negative
                      </span>
                    </BookButton>
                    <BookButton
                      variant="secondary"
                      onClick={() =>
                        void copyText("模型请求 JSON", providerRequestSnapshotText(selectedVideoRecord))
                      }
                    >
                      <span className="inline-flex items-center gap-1.5">
                        <Copy size={12} />
                        复制请求 JSON
                      </span>
                    </BookButton>
                    {onUpdateVideo ? (
                      <BookButton
                        variant="secondary"
                        onClick={beginPromptEdit}
                        disabled={acting != null}
                      >
                        <span className="inline-flex items-center gap-1.5">
                          <Pencil size={12} />
                          编辑记录
                        </span>
                      </BookButton>
                    ) : null}
                  </>
                )}
              </>
            ) : null}
            <BookButton variant="secondary" onClick={closePromptModal} disabled={promptActionBusy}>
              关闭
            </BookButton>
          </div>
        }
      >
        {selectedVideoRecord ? (
          <div className="space-y-3">
            {promptEditing ? (
              <>
                <label className="a4n-page-surface block overflow-hidden">
                  <span className="block border-b border-book-border/40 px-3 py-2 text-xs font-semibold text-book-text-main">
                    视频模型 Prompt
                  </span>
                  <textarea
                    value={promptDraft}
                    onChange={(event) => setPromptDraft(event.target.value)}
                    disabled={promptActionBusy}
                    className="custom-scrollbar min-h-[260px] max-h-[48vh] w-full resize-y border-0 bg-transparent px-3 py-2 font-mono text-[11px] leading-relaxed text-book-text-main outline-none"
                  />
                </label>
                <label className="a4n-page-surface block overflow-hidden">
                  <span className="block border-b border-book-border/40 px-3 py-2 text-xs font-semibold text-book-text-main">
                    Negative Prompt
                  </span>
                  <textarea
                    value={negativePromptDraft}
                    onChange={(event) => setNegativePromptDraft(event.target.value)}
                    disabled={promptActionBusy}
                    className="custom-scrollbar min-h-[120px] max-h-56 w-full resize-y border-0 bg-transparent px-3 py-2 font-mono text-[11px] leading-relaxed text-book-text-main outline-none"
                  />
                </label>
              </>
            ) : (
              <>
                <section className="a4n-page-surface overflow-hidden">
                  <div className="border-b border-book-border/40 px-3 py-2 text-xs font-semibold text-book-text-main">
                    视频模型 Prompt
                  </div>
                  <pre className="custom-scrollbar max-h-[48vh] overflow-auto whitespace-pre-wrap break-words px-3 py-2 text-[11px] leading-relaxed text-book-text-main">
                    {selectedVideoRecord.prompt || "（空）"}
                  </pre>
                </section>
                {selectedVideoRecord.negative_prompt ? (
                  <section className="a4n-page-surface overflow-hidden">
                    <div className="border-b border-book-border/40 px-3 py-2 text-xs font-semibold text-book-text-main">
                      Negative Prompt
                    </div>
                    <pre className="custom-scrollbar max-h-40 overflow-auto whitespace-pre-wrap break-words px-3 py-2 text-[11px] leading-relaxed text-book-text-main">
                      {selectedVideoRecord.negative_prompt}
                    </pre>
                  </section>
                ) : null}
              </>
            )}
            {selectedVideoRecord.metadata_json ? (
              <section className="a4n-page-surface overflow-hidden">
                <div className="border-b border-book-border/40 px-3 py-2 text-xs font-semibold text-book-text-main">
                  请求元数据
                </div>
                <pre className="custom-scrollbar max-h-72 overflow-auto whitespace-pre-wrap break-words px-3 py-2 text-[11px] leading-relaxed text-book-text-sub">
                  {JSON.stringify(selectedVideoRecord.metadata_json, null, 2)}
                </pre>
              </section>
            ) : null}
          </div>
        ) : null}
      </Modal>
    </>
  );
};
