import React from "react";
import { Download, Trash2 } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import type {
  EmbeddingModelDownloadInfo,
  LocalEmbeddingModel,
} from "../../../api/config";
import { BookButton } from "../../../components/ui/BookButton";
import { Dropdown } from "../../../components/ui/Dropdown";
import { formatBytes } from "./embeddingTabUtils";
import { useI18n } from "../../../i18n";

interface FastembedModelPanelProps {
  model: LocalEmbeddingModel | null;
  downloadInfo: EmbeddingModelDownloadInfo | null;
  modelName: string;
  busy: boolean;
  modelHostLabel: string;
  onDownload: () => void;
  onDelete: () => void;
}

export const FastembedModelPanel: React.FC<FastembedModelPanelProps> = ({
  model,
  downloadInfo,
  modelName,
  busy,
  modelHostLabel,
  onDownload,
  onDelete,
}) => {
  const { locale, t } = useI18n();
  const downloadLabel =
    downloadInfo && downloadInfo.missing_size_bytes > 0
      ? `${t("common.download")} ${formatBytes(downloadInfo.missing_size_bytes, locale)}`
      : t("common.download");
  const totalSizeText = downloadInfo
    ? t("settingsEmbedding.totalSize", { size: formatBytes(downloadInfo.total_size_bytes, locale) })
    : model?.size_bytes
      ? t("settingsEmbedding.cacheSize", { host: modelHostLabel, size: formatBytes(model.size_bytes, locale) })
      : null;

  return (
    <div aria-label={t("settingsEmbedding.localModel")} className="a4n-flat-surface px-3 py-3">
      <div className="flex min-w-0 items-start gap-3">
        <div className="min-w-0 flex-1">
          <div className="truncate text-xs font-semibold text-book-text-main">
            {model?.name ?? modelName}
          </div>
          <div className="mt-0.5 flex flex-wrap gap-x-1.5 gap-y-0.5 text-small text-book-text-muted">
            <span className={model?.downloaded ? "text-success" : "text-warning"}>
              {model?.downloaded
                ? t("settingsEmbedding.downloaded")
                : t("settingsEmbedding.notDownloaded")}
            </span>
            {model ? (
              <>
                <span aria-hidden="true">·</span>
                <span>{t("settingsEmbedding.dimensionUnit", { count: model.dimension })}</span>
                {totalSizeText ? (
                  <>
                    <span aria-hidden="true">·</span>
                    <span>{totalSizeText}</span>
                  </>
                ) : null}
              </>
            ) : (
              <>
                <span aria-hidden="true">·</span>
                <span>{t("settingsEmbedding.modelDirectoryUnavailable", { host: modelHostLabel })}</span>
              </>
            )}
          </div>
        </div>
        {busy && model?.downloaded ? (
          <span className="inline-flex h-8 shrink-0 items-center gap-1.5 text-small text-book-text-muted">
            <LoadingSpinner size="sm" />
            {t("common.deleting")}
          </span>
        ) : model?.downloaded ? (
          <Dropdown
            items={[
              {
                label: t("settingsEmbedding.deleteCache"),
                icon: <Trash2 size={12} />,
                danger: true,
                onClick: onDelete,
              },
            ]}
          />
        ) : null}
      </div>
      {downloadInfo ? (
        <div className="mb-2 text-small text-book-text-muted">
          {t("settingsEmbedding.totalSize", { size: formatBytes(downloadInfo.total_size_bytes, locale) })} ·{" "}
          {t("settingsEmbedding.cachedSize", { size: formatBytes(downloadInfo.downloaded_size_bytes, locale) })} ·{" "}
          {t("settingsEmbedding.missingSize", { size: formatBytes(downloadInfo.missing_size_bytes, locale) })}
        </div>
      ) : null}
      {model?.description && (
        <p className="mb-3 text-xs leading-relaxed text-book-text-sub">
          {model.description}
        </p>
      )}
      {!model?.downloaded && model ? (
        <BookButton onClick={onDownload} disabled={busy}>
          {busy ? <LoadingSpinner size="sm" /> : <Download size={14} />}
          {downloadLabel}
        </BookButton>
      ) : null}
    </div>
  );
};
