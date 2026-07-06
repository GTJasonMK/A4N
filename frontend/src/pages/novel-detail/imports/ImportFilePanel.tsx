import React from "react";
import { FolderOpen, Loader2 } from "lucide-react";
import { BookButton } from "../../../components/ui/BookButton";
import { BookCard } from "../../../components/ui/BookCard";
import { BookInput } from "../../../components/ui/BookInput";

interface ImportFilePanelProps {
  fileName: string;
  fileType: string;
  importBusy: boolean;
  onFileTypeChange: (value: string) => void;
  onSelectFile: () => void;
  onImport: () => void;
}

export const ImportFilePanel: React.FC<ImportFilePanelProps> = ({
  fileName,
  fileType,
  importBusy,
  onFileTypeChange,
  onSelectFile,
  onImport,
}) => (
  <BookCard className="p-4">
    <h3 className="text-sm font-serif text-book-text-main mb-2">导入新文件</h3>
    <div className="grid grid-cols-1 sm:grid-cols-[1fr_auto_auto_auto] gap-2 items-end text-xs">
      <label className="flex flex-col gap-1">
        <span className="text-book-text-muted">已选择文件</span>
        <BookInput
          value={fileName}
          readOnly
          placeholder="请选择 txt / markdown / epub / docx 文件"
        />
      </label>
      <label className="flex flex-col gap-1">
        <span className="text-book-text-muted">类型</span>
        <select
          value={fileType}
          onChange={(e) => onFileTypeChange(e.target.value)}
          className="pyqt-input"
        >
          <option value="txt">txt</option>
          <option value="markdown">markdown</option>
          <option value="epub">epub</option>
          <option value="docx">docx</option>
        </select>
      </label>
      <BookButton variant="secondary" onClick={onSelectFile} disabled={importBusy}>
        <span className="inline-flex items-center gap-1">
          <FolderOpen size={14} /> 选择文件
        </span>
      </BookButton>
      <BookButton variant="primary" onClick={onImport} disabled={importBusy || !fileName.trim()}>
        {importBusy ? <Loader2 size={11} className="animate-spin" /> : "导入"}
      </BookButton>
    </div>
  </BookCard>
);
