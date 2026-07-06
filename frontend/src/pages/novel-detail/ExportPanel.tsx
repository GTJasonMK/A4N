import React, { useEffect, useRef, useState } from "react";
import { Loader2 } from "lucide-react";
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
import { createProgressTaskId, useProgressTask } from "../../hooks/useProgressTask";
import { isCancelledOperation } from "../../utils/cancellation";
import { downloadBase64ExportFile } from "../../utils/downloadFile";

const downloadExportFiles = (files: ExportFileData[]): string => {
  for (const file of files) {
    downloadBase64ExportFile(file);
  }
  return files.map((file) => file.file_name).join("\n");
};

const exportMetadata = (
  meta: { title: string; author: string },
): { title: string; author: string } | null => {
  const title = meta.title.trim();
  const author = meta.author.trim();
  if (!title) {
    return null;
  }
  if (!author) {
    return null;
  }
  return { title, author };
};

export const ExportPanel: React.FC<{ project: NovelProject }> = ({ project }) => {
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
  const [lastResult, setLastResult] = useState<string | null>(null);
  const exportBusy = acting !== null;

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      exportSeqRef.current += 1;
      exportBusyRef.current = false;
    };
  }, []);

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
  }, [project.id]);

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
    const taskId = createProgressTaskId(`export-${key}`, requestProjectId);
    setActing(key);
    setLastResult(null);
    try {
      const r = await runProgressTask(
        {
          title: `导出${label}`,
          startMessage: `正在准备导出${label}`,
          successMessage: `导出${label}完成`,
          errorMessage: "导出失败",
          taskId,
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
      addToast("导出完成", "success");
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

  const requireMetadata = (formatLabel: string) => {
    const normalized = exportMetadata(meta);
    if (!normalized) {
      addToast(`${formatLabel} 导出需要填写标题和作者`, "error");
    }
    return normalized;
  };

  return (
    <div className="space-y-4">
      <div className="grid grid-cols-2 gap-2 sm:grid-cols-3">
        <label className="flex flex-col gap-1">
          <span className="text-[10px] text-book-text-muted">标题（Markdown/EPUB 元数据）</span>
          <input
            type="text"
            value={meta.title}
            onChange={(e) => setMeta({ ...meta, title: e.target.value })}
            className="pyqt-input !py-1 text-xs"
            disabled={exportBusy}
          />
        </label>
        <label className="flex flex-col gap-1">
          <span className="text-[10px] text-book-text-muted">作者</span>
          <input
            type="text"
            value={meta.author}
            onChange={(e) => setMeta({ ...meta, author: e.target.value })}
            className="pyqt-input !py-1 text-xs"
            disabled={exportBusy}
          />
        </label>
      </div>

      <div className="space-y-2 text-xs">
        <div className="flex flex-wrap items-center gap-2">
          <span className="w-10 shrink-0 text-[10px] font-semibold text-book-text-muted">文档</span>
          <BookButton
            variant="primary"
            onClick={() => void wrap("txt", "TXT", (taskId) => exportTxtData(project.id, taskId))}
            disabled={exportBusy}
          >
            {acting === "txt" ? <Loader2 size={12} className="animate-spin" /> : "TXT"}
          </BookButton>
          <BookButton
            variant="primary"
            onClick={() => {
              const metadata = requireMetadata("Markdown");
              if (!metadata) return;
              void wrap("md", "Markdown", (taskId) =>
                exportMarkdownData(project.id, metadata, taskId),
              );
            }}
            disabled={exportBusy}
          >
            {acting === "md" ? <Loader2 size={12} className="animate-spin" /> : "Markdown"}
          </BookButton>
          <BookButton
            variant="primary"
            onClick={() => {
              const metadata = requireMetadata("EPUB");
              if (!metadata) return;
              void wrap("epub", "EPUB", (taskId) =>
                exportEpubData(project.id, metadata.title, metadata.author, taskId),
              );
            }}
            disabled={exportBusy}
          >
            {acting === "epub" ? <Loader2 size={12} className="animate-spin" /> : "EPUB"}
          </BookButton>
          <BookButton
            variant="secondary"
            onClick={() => void wrap("pdf", "PDF", (taskId) => exportPdfData(project.id, taskId))}
            disabled={exportBusy}
          >
            {acting === "pdf" ? <Loader2 size={12} className="animate-spin" /> : "PDF"}
          </BookButton>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <span className="w-10 shrink-0 text-[10px] font-semibold text-book-text-muted">数据</span>
          <BookButton
            variant="secondary"
            onClick={() =>
              void wrap("outline", "大纲 JSON", (taskId) =>
                exportOutlineJsonData(project.id, taskId),
              )
            }
            disabled={exportBusy}
          >
            {acting === "outline" ? <Loader2 size={12} className="animate-spin" /> : "大纲 JSON"}
          </BookButton>
          <BookButton
            variant="secondary"
            onClick={() =>
              void wrap("characters", "角色 JSON", (taskId) =>
                exportCharactersJsonData(project.id, taskId),
              )
            }
            disabled={exportBusy}
          >
            {acting === "characters" ? (
              <Loader2 size={12} className="animate-spin" />
            ) : (
              "角色 JSON"
            )}
          </BookButton>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <span className="w-10 shrink-0 text-[10px] font-semibold text-book-text-muted">批量</span>
          <BookButton
            variant="secondary"
            onClick={() => {
              const metadata = requireMetadata("批量文件");
              if (!metadata) return;
              void wrap("batch", "批量文件", (taskId) =>
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
            }}
            disabled={exportBusy}
          >
            {acting === "batch" ? (
              <Loader2 size={12} className="animate-spin" />
            ) : (
              "小说批量（txt+md）"
            )}
          </BookButton>
        </div>
      </div>

      {lastResult != null && (
        <pre className="mt-3 max-h-44 overflow-y-auto whitespace-pre-wrap break-words rounded border border-book-border/40 bg-book-bg-tertiary/40 p-3 font-mono text-[10px] custom-scrollbar">
          已下载文件：
          {"\n"}
          {lastResult}
        </pre>
      )}

    </div>
  );
};
