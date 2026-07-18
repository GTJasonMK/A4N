import React from "react";
import { FolderOpen, Loader2 } from "lucide-react";
import { BookButton } from "../../../components/ui/BookButton";

interface ImportFilePanelProps {
  fileName: string;
  fileType: string;
  importBusy: boolean;
  externalBusy: boolean;
  onFileTypeChange: (value: string) => void;
  onSelectFile: () => void;
  onImport: () => void;
}

export const ImportFilePanel: React.FC<ImportFilePanelProps> = ({
  fileName,
  fileType,
  importBusy,
  externalBusy,
  onFileTypeChange,
  onSelectFile,
  onImport,
}) => (
  <section className="a4n-flat-surface p-4">
    <div className="mb-3">
      <h3 className="text-sm font-semibold text-book-text-main">导入新文件</h3>
      <p className="mt-0.5 text-[11px] text-book-text-muted">
        支持 TXT、Markdown、EPUB 与 DOCX
      </p>
    </div>
    <div className="flex flex-wrap items-end gap-2 text-xs">
      <div className="min-w-0 w-full pb-1 text-book-text-sub sm:w-auto sm:flex-1 sm:pb-2">
        <span className={fileName ? "break-all" : "text-book-text-muted"}>
          {fileName || "尚未选择文件"}
        </span>
      </div>
      <label className="flex flex-col gap-1">
        <span className="text-book-text-muted">类型</span>
        <select
          value={fileType}
          onChange={(e) => onFileTypeChange(e.target.value)}
          disabled={importBusy || externalBusy}
          className="pyqt-input"
        >
          <option value="txt">txt</option>
          <option value="markdown">markdown</option>
          <option value="epub">epub</option>
          <option value="docx">docx</option>
        </select>
      </label>
      <BookButton
        variant="secondary"
        size="sm"
        onClick={onSelectFile}
        disabled={importBusy || externalBusy}
        className="!min-h-8 !text-xs"
      >
        <FolderOpen size={14} /> 选择文件
      </BookButton>
      <BookButton
        variant="primary"
        size="sm"
        onClick={onImport}
        disabled={importBusy || externalBusy || !fileName.trim()}
        className="!min-h-8 !text-xs"
      >
        {importBusy ? (
          <>
            <Loader2 size={11} className="animate-spin" /> 导入中…
          </>
        ) : (
          "开始导入"
        )}
      </BookButton>
    </div>
  </section>
);
