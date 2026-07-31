import React, { useEffect, useRef, useState } from "react";
import { CheckCircle2, Download } from "lucide-react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { extractErrorMessage } from "../../api";
import {
  exportBatchData,
  exportCharactersJsonData,
  exportEpubData,
  type ExportFileData,
  exportMarkdownData,
  exportOutlineJsonData,
  exportPdfData,
  exportTxtData,
} from "../../api/export";
import type { NovelProject } from "../../api/project";
import { useToast } from "../../components/feedback/Toast";
import { BookButton } from "../../components/ui/BookButton";
import { allocateHostTaskId, useProgressTask } from "../../hooks/useProgressTask";
import { isCancelledOperation } from "../../utils/cancellation";
import { downloadBase64ExportFile } from "../../utils/downloadFile";

type ExportFormat = "txt" | "markdown" | "epub" | "pdf" | "batch";

const EXPORT_FORMATS: Array<{
  value: ExportFormat;
  label: string;
  description: string;
  requiresMetadata: boolean;
}> = [
  {
    value: "txt",
    label: "TXT 纯文本",
    description: "适合阅读、归档和在其他写作工具中继续编辑。",
    requiresMetadata: false,
  },
  {
    value: "markdown",
    label: "Markdown",
    description: "保留章节层级，适合版本管理和二次排版。",
    requiresMetadata: true,
  },
  {
    value: "epub",
    label: "EPUB 电子书",
    description: "适合电子书阅读器，需要填写书名和作者。",
    requiresMetadata: true,
  },
  {
    value: "pdf",
    label: "PDF",
    description: "生成固定版式文档，便于分享和打印。",
    requiresMetadata: false,
  },
  {
    value: "batch",
    label: "TXT + Markdown",
    description: "一次下载纯文本和 Markdown 两种文件。",
    requiresMetadata: true,
  },
];

const downloadExportFiles = (files: ExportFileData[]): string[] => {
  for (const file of files) {
    downloadBase64ExportFile(file);
  }
  return files.map((file) => file.file_name);
};

const exportMetadata = (
  meta: { title: string; author: string },
): { title: string; author: string } | null => {
  const title = meta.title.trim();
  const author = meta.author.trim();
  if (!title || !author) return null;
  return { title, author };
};

export const ExportPanel: React.FC<{
  project: NovelProject;
  onBusyChange?: (busy: boolean) => void;
}> = ({ project, onBusyChange }) => {
  const { addToast } = useToast();
  const { runProgressTask } = useProgressTask();
  const [acting, setActing] = useState<string | null>(null);
  const mountedRef = useRef(false);
  const activeProjectIdRef = useRef(project.id);
  const exportSeqRef = useRef(0);
  const exportBusyRef = useRef(false);
  const [meta, setMeta] = useState({
    title: project.title,
    author: "",
  });
  const [selectedFormat, setSelectedFormat] = useState<ExportFormat>("txt");
  const [lastResult, setLastResult] = useState<string[] | null>(null);
  const exportBusy = acting !== null;
  const selectedFormatOption =
    EXPORT_FORMATS.find((format) => format.value === selectedFormat) ??
    EXPORT_FORMATS[0];
  const selectedMetadata = exportMetadata(meta);
  const selectedExportReady =
    !selectedFormatOption.requiresMetadata || selectedMetadata != null;

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      exportSeqRef.current += 1;
      exportBusyRef.current = false;
    };
  }, []);

  useEffect(() => {
    onBusyChange?.(exportBusy);
  }, [exportBusy, onBusyChange]);

  useEffect(() => {
    activeProjectIdRef.current = project.id;
    exportSeqRef.current += 1;
    exportBusyRef.current = false;
    setActing(null);
    setLastResult(null);
    setMeta({
      title: project.title,
      author: "",
    });
    setSelectedFormat("txt");
  }, [project.id, project.title]);

  const isCurrentExport = (requestProjectId: number, requestSeq: number) =>
    mountedRef.current &&
    activeProjectIdRef.current === requestProjectId &&
    exportSeqRef.current === requestSeq;

  const wrap = async (
    key: string,
    label: string,
    fn: (taskId: string) => Promise<ExportFileData | ExportFileData[]>,
  ) => {
    if (exportBusyRef.current) return;
    const requestProjectId = project.id;
    const requestSeq = exportSeqRef.current + 1;
    exportSeqRef.current = requestSeq;
    exportBusyRef.current = true;
    const taskId = allocateHostTaskId(`export-${key}`, requestProjectId);
    setActing(key);
    setLastResult(null);
    try {
      const r = await runProgressTask(
        {
          cancelTaskId: taskId,
          title: `导出${label}`,
          startMessage: `正在准备导出${label}`,
          successMessage: `导出${label}完成`,
          errorMessage: "导出失败",
          total: 4,
          cancelLabel: "停止导出",
          runningHint: "导出停止后，已经写入完成的文件会保留",
        },
        () => {
          if (!isCurrentExport(requestProjectId, requestSeq)) {
            return Promise.reject(new Error("任务已停止"));
          }
          return fn(taskId);
        },
      );
      if (!isCurrentExport(requestProjectId, requestSeq)) return;
      const files = Array.isArray(r) ? r : [r];
      const fileNames = downloadExportFiles(files);
      setLastResult(fileNames);
      addToast(files.length > 1 ? `已下载 ${files.length} 个文件` : "下载完成", "success");
    } catch (err) {
      if (!isCurrentExport(requestProjectId, requestSeq)) return;
      const message = extractErrorMessage(err, "导出失败");
      addToast(message, isCancelledOperation(message) ? "info" : "error");
    } finally {
      if (exportSeqRef.current === requestSeq) {
        exportBusyRef.current = false;
      }
      if (isCurrentExport(requestProjectId, requestSeq)) {
        setActing(null);
      }
    }
  };

  const runSelectedExport = () => {
    if (!selectedExportReady) return;
    switch (selectedFormat) {
      case "txt":
        void wrap("txt", "TXT", (taskId) => exportTxtData(project.id, taskId));
        return;
      case "markdown": {
        const metadata = selectedMetadata;
        if (!metadata) return;
        void wrap("markdown", "Markdown", (taskId) =>
          exportMarkdownData(project.id, metadata, taskId),
        );
        return;
      }
      case "epub": {
        const metadata = selectedMetadata;
        if (!metadata) return;
        void wrap("epub", "EPUB", (taskId) =>
          exportEpubData(
            project.id,
            metadata.title,
            metadata.author,
            taskId,
          ),
        );
        return;
      }
      case "pdf":
        void wrap("pdf", "PDF", (taskId) => exportPdfData(project.id, taskId));
        return;
      case "batch": {
        const metadata = selectedMetadata;
        if (!metadata) return;
        void wrap("batch", "TXT + Markdown", (taskId) =>
          exportBatchData(
            {
              project_id: project.id,
              formats: ["txt", "markdown"],
              title: metadata.title,
              author: metadata.author,
            },
            taskId,
          ),
        );
      }
    }
  };

  const spin = (key: string) =>
    acting === key ? <LoadingSpinner size="sm" /> : null;

  return (
    <div className="space-y-4">
      <label className="flex min-w-0 flex-col gap-1">
        <span className="text-small text-book-text-muted">导出格式</span>
        <select
          value={selectedFormat}
          onChange={(event) =>
            setSelectedFormat(event.target.value as ExportFormat)
          }
          className="pyqt-input w-full !py-1.5 text-xs"
          disabled={exportBusy}
        >
          {EXPORT_FORMATS.map((format) => (
            <option key={format.value} value={format.value}>
              {format.label}
            </option>
          ))}
        </select>
      </label>

      <p className="text-small leading-relaxed text-book-text-muted">
        {selectedFormatOption.description}
      </p>

      {selectedFormatOption.requiresMetadata ? (
        <section className="border-y border-book-border/45 py-3">
          <div className="mb-2 text-xs font-semibold text-book-text-main">
            作品信息
          </div>
          <p className="mb-2 text-small leading-relaxed text-book-text-muted">
            书名和作者会写入导出文件，均为必填。
          </p>
          <div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
            <label className="flex flex-col gap-1">
              <span className="text-small text-book-text-muted">书名</span>
              <input
                type="text"
                value={meta.title}
                onChange={(event) =>
                  setMeta({ ...meta, title: event.target.value })
                }
                className="pyqt-input !py-1.5 text-xs"
                disabled={exportBusy}
              />
            </label>
            <label className="flex flex-col gap-1">
              <span className="text-small text-book-text-muted">作者</span>
              <input
                type="text"
                value={meta.author}
                onChange={(event) =>
                  setMeta({ ...meta, author: event.target.value })
                }
                className="pyqt-input !py-1.5 text-xs"
                disabled={exportBusy}
                placeholder="输入作者名"
              />
            </label>
          </div>
        </section>
      ) : null}

      <div className="flex justify-end">
        <BookButton
          variant="primary"
          size="sm"
          className="w-full sm:w-auto"
          disabled={exportBusy || !selectedExportReady}
          onClick={runSelectedExport}
        >
          {exportBusy ? (
            <LoadingSpinner size="sm" />
          ) : (
            <Download size={13} />
          )}
          {exportBusy
            ? `正在导出 ${selectedFormatOption.label}…`
            : `下载 ${selectedFormatOption.label}`}
        </BookButton>
      </div>

      {lastResult ? (
        <div className="flex items-start gap-2 rounded-xl bg-success-bg/55 px-3 py-2.5 text-small text-book-text-sub">
          <CheckCircle2 size={14} className="mt-0.5 shrink-0 text-success" />
          <div className="min-w-0">
            <div className="font-medium text-book-text-main">
              {lastResult.length > 1
                ? `已下载 ${lastResult.length} 个文件`
                : "下载完成"}
            </div>
            <div className="mt-0.5 break-all text-book-text-muted">
              {lastResult.join("、")}
            </div>
          </div>
        </div>
      ) : null}

      <details className="a4n-flat-surface overflow-hidden">
        <summary className="flex cursor-pointer items-center justify-between gap-3 px-3 py-2.5 text-xs font-semibold text-book-text-main">
          <span>结构数据</span>
          <span className="text-caption font-normal text-book-text-muted">大纲与角色 JSON</span>
        </summary>
        <div className="grid grid-cols-2 gap-2 border-t border-book-border/45 p-3">
          <BookButton
            variant="secondary"
            size="sm"
            className="!justify-start !text-xs"
            onClick={() =>
              void wrap("outline", "大纲 JSON", (taskId) =>
                exportOutlineJsonData(project.id, taskId),
              )
            }
            disabled={exportBusy}
          >
            {spin("outline")} 大纲 JSON
          </BookButton>
          <BookButton
            variant="secondary"
            size="sm"
            className="!justify-start !text-xs"
            onClick={() =>
              void wrap("characters", "角色 JSON", (taskId) =>
                exportCharactersJsonData(project.id, taskId),
              )
            }
            disabled={exportBusy}
          >
            {spin("characters")} 角色 JSON
          </BookButton>
        </div>
      </details>
    </div>
  );
};
