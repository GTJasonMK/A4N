import React, { useRef } from "react";
import { ChevronRight, Download, Upload } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";

interface FullConfigBackupPanelProps {
  overwrite: boolean;
  exporting: boolean;
  importing: boolean;
  lastExportPath: string;
  onOverwriteChange: (value: boolean) => void;
  onExport: () => void;
  onImportFile: (file: File) => Promise<void> | void;
}

export const FullConfigBackupPanel: React.FC<FullConfigBackupPanelProps> = ({
  overwrite,
  exporting,
  importing,
  lastExportPath,
  onOverwriteChange,
  onExport,
  onImportFile,
}) => {
  const fileInputRef = useRef<HTMLInputElement | null>(null);

  const handleFileChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    event.target.value = "";
    if (!file) return;
    void onImportFile(file);
  };

  return (
    <section aria-label="配置备份" className="w-full max-w-3xl">
      <h3 className="text-sm font-semibold text-book-text-main">配置备份</h3>
      <p className="mt-1 text-small leading-relaxed text-book-text-muted">
        备份或恢复模型、提示词、主题和系统配置。
      </p>

      <div className="mt-3 grid items-start gap-2 text-xs sm:grid-cols-2">
        <section className="a4n-flat-surface overflow-hidden">
          <button
            type="button"
            onClick={onExport}
            disabled={exporting || importing}
            className="flex min-h-11 w-full items-center gap-3 px-3 py-3 text-left text-book-text-main transition-colors hover:bg-book-bg-tertiary/40 hover:text-book-primary disabled:opacity-50"
          >
            {exporting ? <LoadingSpinner size="sm" /> : <Download size={14} />}
            <span className="min-w-0 flex-1">
              <span className="block font-medium">导出配置文件</span>
              <span className="mt-0.5 block text-small text-book-text-muted">
                保存一份完整配置备份
              </span>
            </span>
            <ChevronRight size={13} className="text-book-text-muted" />
          </button>
          {lastExportPath ? (
            <details className="border-t border-book-border/35 text-small text-book-text-muted">
              <summary className="cursor-pointer px-3 py-2.5 transition-colors hover:bg-book-bg-tertiary/40 hover:text-book-text-main">
                最近导出位置
              </summary>
              <div className="break-all border-t border-book-border/35 px-3 py-2.5 font-mono text-book-text-sub">
                {lastExportPath}
              </div>
            </details>
          ) : null}
        </section>

        <section className="a4n-flat-surface overflow-hidden">
          <button
            type="button"
            onClick={() => fileInputRef.current?.click()}
            disabled={importing || exporting}
            className="flex min-h-11 w-full items-center gap-3 px-3 py-3 text-left text-book-text-main transition-colors hover:bg-book-bg-tertiary/40 hover:text-book-primary disabled:opacity-50"
          >
            {importing ? <LoadingSpinner size="sm" /> : <Upload size={14} />}
            <span className="min-w-0 flex-1">
              <span className="block font-medium">导入配置文件</span>
              <span className="mt-0.5 block text-small text-book-text-muted">
                从 JSON 备份恢复配置
              </span>
            </span>
            <ChevronRight size={13} className="text-book-text-muted" />
          </button>
          <input
            ref={fileInputRef}
            type="file"
            accept=".json,application/json"
            className="hidden"
            onChange={handleFileChange}
          />
          <details className="border-t border-book-border/35 text-xs text-book-text-muted">
            <summary className="cursor-pointer px-3 py-2.5 transition-colors hover:bg-book-bg-tertiary/40 hover:text-book-text-main">
              导入选项
            </summary>
            <button
              type="button"
              role="switch"
              aria-checked={overwrite}
              onClick={() => onOverwriteChange(!overwrite)}
              disabled={exporting || importing}
              className="flex w-full items-center gap-3 border-t border-book-border/35 px-3 py-3 text-left disabled:cursor-not-allowed disabled:opacity-50"
              title="开启后同名项会被覆盖；关闭时保留现有项，仅插入新项"
            >
              <span className="min-w-0 flex-1">
                <span className="block text-book-text-sub">覆盖同名项</span>
                <span className="mt-0.5 block text-small text-book-text-muted">
                  {overwrite
                    ? "导入时替换已有同名配置"
                    : "保留已有配置，只补充新项目"}
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
          </details>
        </section>
      </div>
    </section>
  );
};
