import React from "react";
import { Download, Loader2, Trash2 } from "lucide-react";
import type {
  EmbeddingModelDownloadInfo,
  LocalEmbeddingModel,
} from "../../../api/config";
import { BookButton } from "../../../components/ui/BookButton";
import { Dropdown } from "../../../components/ui/Dropdown";
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
    <div aria-label="本地嵌入模型" className="a4n-flat-surface px-3 py-3">
      <div className="flex min-w-0 items-start gap-3">
        <div className="min-w-0 flex-1">
          <div className="truncate text-xs font-semibold text-book-text-main">
            {model?.name ?? modelName}
          </div>
          <div className="mt-0.5 flex flex-wrap gap-x-1.5 gap-y-0.5 text-[11px] text-book-text-muted">
            <span className={model?.downloaded ? "text-success" : "text-warning"}>
              {model?.downloaded ? "已下载" : "未下载"}
            </span>
            {model ? (
              <>
                <span aria-hidden="true">·</span>
                <span>{model.dimension} 维</span>
                <span aria-hidden="true">·</span>
                <span>{totalSizeText}</span>
              </>
            ) : (
              <>
                <span aria-hidden="true">·</span>
                <span>未加载到{modelHostLabel}模型目录</span>
              </>
            )}
          </div>
        </div>
        {busy && model?.downloaded ? (
          <span className="inline-flex h-8 shrink-0 items-center gap-1.5 text-[11px] text-book-text-muted">
            <Loader2 size={13} className="animate-spin" />
            正在删除
          </span>
        ) : model?.downloaded ? (
          <Dropdown
            items={[
              {
                label: "删除模型缓存",
                icon: <Trash2 size={12} />,
                danger: true,
                onClick: onDelete,
              },
            ]}
          />
        ) : null}
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
      {!model?.downloaded && model ? (
        <BookButton onClick={onDownload} disabled={busy}>
          {busy ? <Loader2 size={14} className="animate-spin" /> : <Download size={14} />}
          {downloadLabel}
        </BookButton>
      ) : null}
    </div>
  );
};
