import React, { useRef } from "react";
import { ChevronRight, Download, Upload } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { useI18n } from "../../../i18n";
import { openFilePicker } from "../../../utils/fileUpload";
import type { ConfigBackupScope } from "./useFullConfigBackup";

interface FullConfigBackupPanelProps {
  scope: ConfigBackupScope;
  overwrite: boolean;
  exporting: boolean;
  preparingImport: boolean;
  importing: boolean;
  lastExportFileName: string;
  onOverwriteChange: (value: boolean) => void;
  onExport: () => void;
  onImportFile: (file: File) => Promise<void> | void;
}

export const FullConfigBackupPanel: React.FC<FullConfigBackupPanelProps> = ({
  scope,
  overwrite,
  exporting,
  preparingImport,
  importing,
  lastExportFileName,
  onOverwriteChange,
  onExport,
  onImportFile,
}) => {
  const { t } = useI18n();
  const title = t(scope === "local" ? "settingsAdvanced.localBackup" : "settingsAdvanced.serverBackup");
  const fileInputRef = useRef<HTMLInputElement | null>(null);
  const busy = exporting || preparingImport || importing;

  const handleFileChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    event.target.value = "";
    if (!file) return;
    void onImportFile(file);
  };

  return (
    <section aria-label={title} className="w-full max-w-3xl">
      <h3 className="text-sm font-semibold text-book-text-main">{title}</h3>

      <div className="mt-3 grid items-start gap-2 text-xs sm:grid-cols-2">
        <section className="a4n-flat-surface overflow-hidden">
          <button
            type="button"
            onClick={onExport}
            disabled={busy}
            className="flex min-h-11 w-full items-center gap-3 px-3 py-3 text-left text-book-text-main transition-colors hover:bg-book-bg-tertiary/40 hover:text-book-primary disabled:opacity-50"
          >
            {exporting ? <LoadingSpinner size="sm" /> : <Download size={14} />}
            <span className="min-w-0 flex-1">
              <span className="block font-medium">{t("settingsAdvanced.exportFile")}</span>
              <span className="mt-0.5 block text-small text-book-text-muted">
                {t(scope === "local" ? "settingsAdvanced.localBackupHint" : "settingsAdvanced.serverBackupHint")}
              </span>
            </span>
            <ChevronRight size={13} className="text-book-text-muted" />
          </button>
          {lastExportFileName ? (
            <details className="border-t border-book-border/35 text-small text-book-text-muted">
              <summary className="cursor-pointer px-3 py-2.5 transition-colors hover:bg-book-bg-tertiary/40 hover:text-book-text-main">
                {t("settingsAdvanced.lastExportLocation")}
              </summary>
              <div className="break-all border-t border-book-border/35 px-3 py-2.5 font-mono text-book-text-sub">
                {lastExportFileName}
              </div>
            </details>
          ) : null}
        </section>

        <section className="a4n-flat-surface overflow-hidden">
          <button
            type="button"
            onClick={() => openFilePicker(fileInputRef.current)}
            disabled={busy}
            className="flex min-h-11 w-full items-center gap-3 px-3 py-3 text-left text-book-text-main transition-colors hover:bg-book-bg-tertiary/40 hover:text-book-primary disabled:opacity-50"
          >
            {preparingImport || importing ? <LoadingSpinner size="sm" /> : <Upload size={14} />}
            <span className="min-w-0 flex-1">
              <span className="block font-medium">{t("settingsAdvanced.importFile")}</span>
              <span className="mt-0.5 block text-small text-book-text-muted">
                {importing
                  ? t("settingsAdvanced.importing")
                  : preparingImport
                    ? t("settingsAdvanced.preparingImport")
                    : t("settingsAdvanced.importFileHint")}
              </span>
            </span>
            <ChevronRight size={13} className="text-book-text-muted" />
          </button>
          <input
            ref={fileInputRef}
            type="file"
            accept=".json,application/json"
            className="sr-only"
            onChange={handleFileChange}
          />
          {scope === "server" && <details className="border-t border-book-border/35 text-xs text-book-text-muted">
            <summary className="cursor-pointer px-3 py-2.5 transition-colors hover:bg-book-bg-tertiary/40 hover:text-book-text-main">
              {t("settingsAdvanced.importOptions")}
            </summary>
            <button
              type="button"
              role="switch"
              aria-checked={overwrite}
              onClick={() => onOverwriteChange(!overwrite)}
              disabled={busy}
              className="flex w-full items-center gap-3 border-t border-book-border/35 px-3 py-3 text-left disabled:cursor-not-allowed disabled:opacity-50"
              title={t("settingsAdvanced.overwriteTitle")}
            >
              <span className="min-w-0 flex-1">
                <span className="block text-book-text-sub">{t("settingsAdvanced.overwrite")}</span>
                <span className="mt-0.5 block text-small text-book-text-muted">
                  {overwrite
                    ? t("settingsAdvanced.overwriteHint")
                    : t("settingsAdvanced.keepExistingHint")}
                </span>
              </span>
              <span
                className={`relative inline-flex h-[18px] w-8 shrink-0 rounded-full transition-colors ${
                  overwrite ? "bg-book-primary" : "bg-book-text-muted/25"
                }`}
              >
                <span
                  className={`absolute top-[2px] h-3.5 w-3.5 rounded-full bg-white shadow-sm transition-transform ${
                    overwrite ? "translate-x-4" : "translate-x-[2px]"
                  }`}
                />
              </span>
            </button>
          </details>}
        </section>
      </div>
    </section>
  );
};
