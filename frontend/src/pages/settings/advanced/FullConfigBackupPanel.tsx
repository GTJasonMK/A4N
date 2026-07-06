import React, { useRef } from "react";
import { Database, Download, Loader2, Upload } from "lucide-react";
import { BookButton } from "../../../components/ui/BookButton";

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
    <div className="pyqt-card p-4">
      <div className="flex flex-wrap items-start justify-between gap-3 mb-3">
        <div className="min-w-0">
          <h4 className="text-sm font-serif text-book-text-main inline-flex items-center gap-2">
            <Database size={14} /> 全量配置导入 / 导出
          </h4>
          <p className="text-xs text-book-text-muted leading-relaxed mt-1">
            包含 LLM、Embedding、Image、Video、Prompts、Themes、SystemSettings。
          </p>
        </div>
        <label
          className="inline-flex items-center gap-1.5 text-xs text-book-text-muted cursor-pointer shrink-0"
          title="勾选后同名项会被覆盖；不勾选则保留现有项，仅插入新项"
        >
          <input
            type="checkbox"
            checked={overwrite}
            onChange={(event) => onOverwriteChange(event.target.checked)}
            className="accent-book-primary"
          />
          覆盖同名项
        </label>
      </div>

      <div className="flex items-center gap-2 text-xs">
        <BookButton variant="secondary" onClick={onExport} disabled={exporting || importing}>
          {exporting ? (
            <Loader2 size={12} className="animate-spin" />
          ) : (
            <span className="inline-flex items-center gap-1.5">
              <Download size={12} />
              导出配置文件
            </span>
          )}
        </BookButton>
        <BookButton
          variant="primary"
          onClick={() => fileInputRef.current?.click()}
          disabled={importing || exporting}
        >
          {importing ? (
            <Loader2 size={12} className="animate-spin" />
          ) : (
            <span className="inline-flex items-center gap-1.5">
              <Upload size={12} /> 导入配置文件
            </span>
          )}
        </BookButton>
        <input
          ref={fileInputRef}
          type="file"
          accept=".json,application/json"
          className="hidden"
          onChange={handleFileChange}
        />
      </div>

      {lastExportPath && (
        <div className="mt-3 text-[11px] text-book-text-muted">
          最近导出：
          <span className="font-mono text-book-text-sub break-all" title={lastExportPath}>
            {lastExportPath}
          </span>
        </div>
      )}
    </div>
  );
};
