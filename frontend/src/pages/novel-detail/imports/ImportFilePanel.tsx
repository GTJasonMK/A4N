import React from "react";
import { FileText, FolderOpen, Upload } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { BookButton } from "../../../components/ui/BookButton";

interface ImportFilePanelProps {
  fileName: string;
  importBusy: boolean;
  externalBusy: boolean;
  onSelectFile: () => void;
  onImport: () => void;
}

export const ImportFilePanel: React.FC<ImportFilePanelProps> = ({
  fileName,
  importBusy,
  externalBusy,
  onSelectFile,
  onImport,
}) => (
  <section className="a4n-flat-surface p-4 sm:p-5">
    <div className="mb-4">
      <h3 className="text-sm font-semibold text-book-text-main">导入新文件</h3>
      <p className="mt-0.5 text-small text-book-text-muted">
        支持 TXT、Markdown、EPUB 与 DOCX
      </p>
    </div>
    <div className="grid items-end gap-2 sm:grid-cols-[minmax(0,1fr)_auto]">
      <button
        type="button"
        onClick={onSelectFile}
        disabled={importBusy || externalBusy}
        className="pyqt-input flex h-10 min-w-0 items-center gap-2.5 px-3 text-left text-xs transition-colors disabled:cursor-not-allowed"
        title={fileName || "选择导入文件"}
      >
        <FileText size={15} className="shrink-0 text-book-text-muted" />
        <span
          className={`min-w-0 flex-1 truncate ${
            fileName ? "text-book-text-main" : "text-book-text-muted"
          }`}
        >
          {fileName || "尚未选择文件"}
        </span>
        <span className="inline-flex shrink-0 items-center gap-1.5 font-medium text-book-text-sub">
          <FolderOpen size={14} />
          {fileName ? "重新选择" : "选择文件"}
        </span>
      </button>
      <BookButton
        variant="primary"
        size="sm"
        onClick={onImport}
        disabled={importBusy || externalBusy || !fileName.trim()}
        className="h-10 !min-h-10 !px-4 !text-xs"
      >
        {importBusy ? (
          <>
            <LoadingSpinner size="sm" /> 导入中…
          </>
        ) : (
          <>
            <Upload size={14} /> 开始导入
          </>
        )}
      </BookButton>
    </div>
  </section>
);
