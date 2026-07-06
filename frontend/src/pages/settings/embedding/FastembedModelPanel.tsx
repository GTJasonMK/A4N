import React from "react";
import { Download, HardDrive, Loader2, Trash2 } from "lucide-react";
import type {
  EmbeddingModelDownloadInfo,
  LocalEmbeddingModel,
} from "../../../api/config";
import { BookButton } from "../../../components/ui/BookButton";
import { formatBytes } from "./embeddingTabUtils";

interface FastembedModelPanelProps {
  model: LocalEmbeddingModel | null;
  downloadInfo: EmbeddingModelDownloadInfo | null;
  downloadInfoError: string | null;
  modelName: string;
  busy: boolean;
  modelHostLabel: string;
  onDownload: () => void;
  onDelete: () => void;
}

export const FastembedModelPanel: React.FC<FastembedModelPanelProps> = ({
  model,
  downloadInfo,
  downloadInfoError,
  modelName,
  busy,
  modelHostLabel,
  onDownload,
  onDelete,
}) => {
  const downloadLabel =
    downloadInfo && downloadInfo.missing_size_bytes > 0
      ? `下载 ${formatBytes(downloadInfo.missing_size_bytes)}`
      : "下载";
  const totalSizeText = downloadInfo
    ? `预计总大小 ${formatBytes(downloadInfo.total_size_bytes)}`
    : model?.size_bytes
      ? `${modelHostLabel}缓存 ${formatBytes(model.size_bytes)}`
      : "预计总大小获取中";

  return (
    <div className="a4n-page-surface px-3 py-3">
      <div className="mb-2 flex items-center justify-between gap-3">
        <div className="min-w-0">
          <div className="flex items-center gap-2 text-xs font-semibold text-book-text-main">
            <HardDrive size={14} />
            <span className="truncate">{model?.name ?? modelName}</span>
          </div>
          <div className="mt-1 truncate text-[11px] text-book-text-muted">
            {model
              ? `${model.model_code} · ${model.dimension} 维 · ${totalSizeText}`
              : `未加载到${modelHostLabel}模型目录`}
          </div>
        </div>
        <span
          className={`shrink-0 rounded border px-2 py-0.5 text-[11px] ${
            model?.downloaded
              ? "border-success/40 bg-success-bg text-success"
              : "border-warning/40 bg-warning-bg text-warning"
          }`}
        >
          {model?.downloaded ? "已下载" : "未下载"}
        </span>
      </div>
      {!downloadInfo && !downloadInfoError ? (
        <div className="mb-2 text-[11px] text-book-text-muted">正在获取模型大小…</div>
      ) : downloadInfo ? (
        <div className="mb-2 text-[11px] text-book-text-muted">
          预计总大小 {formatBytes(downloadInfo.total_size_bytes)} · 已缓存{" "}
          {formatBytes(downloadInfo.downloaded_size_bytes)} · 还需下载{" "}
          {formatBytes(downloadInfo.missing_size_bytes)}
        </div>
      ) : downloadInfoError ? (
        <div className="mb-2 text-[11px] text-error">
          {downloadInfoError}
        </div>
      ) : null}
      {model?.description && (
        <p className="mb-3 text-xs leading-relaxed text-book-text-sub">
          {model.description}
        </p>
      )}
      <div className="flex items-center gap-2">
        <BookButton
          variant="secondary"
          onClick={onDownload}
          disabled={busy || !model || model.downloaded}
        >
          {busy && !model?.downloaded ? (
            <Loader2 size={14} className="animate-spin" />
          ) : (
            <Download size={14} />
          )}
          {downloadLabel}
        </BookButton>
        <BookButton
          variant="danger"
          onClick={onDelete}
          disabled={busy || !model?.downloaded}
        >
          {busy && model?.downloaded ? (
            <Loader2 size={14} className="animate-spin" />
          ) : (
            <Trash2 size={14} />
          )}
          删除缓存
        </BookButton>
      </div>
    </div>
  );
};
