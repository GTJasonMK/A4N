import { ThemedSelect } from "../../components/ui/ThemedSelect";
import React, { useEffect, useRef, useState } from "react";
import { CheckCircle2, ChevronDown, Download } from "lucide-react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { extractErrorMessage } from "../../api";
import {
  exportBatchData,
  exportCharactersJsonData,
  exportEpubData,
  type BatchExportInput,
  type ExportFileData,
  type ExportProjectInput,
  type ExportPublicationInput,
  type ExportPublicationMetadata,
  exportMarkdownData,
  exportOutlineJsonData,
  exportPdfData,
  exportTxtData,
} from "../../api/export";
import type { NovelProject } from "../../api/project";
import { useToast } from "../../components/feedback/Toast";
import { BookButton } from "../../components/ui/BookButton";
import { cancelWorkflowTask } from "../../api/streaming";
import { createWorkflowTaskId, useProgressTask } from "../../hooks/useProgressTask";
import { isCancelledOperation } from "../../utils/cancellation";
import { downloadBase64ExportFile } from "../../utils/downloadFile";
import { useI18n, type MessageKey } from "../../i18n";
import { useNovelDetailDirty } from "./novelDetailDirty";

type ExportFormat = "txt" | "markdown" | "epub" | "pdf" | "batch";

const EXPORT_FORMATS: Array<{
  value: ExportFormat;
  labelKey: MessageKey;
  descriptionKey: MessageKey;
  requiresMetadata: boolean;
}> = [
  {
    value: "txt",
    labelKey: "novel.exportPanel.formats.txt",
    descriptionKey: "novel.exportPanel.formats.txtDescription",
    requiresMetadata: false,
  },
  {
    value: "markdown",
    labelKey: "novel.exportPanel.formats.markdown",
    descriptionKey: "novel.exportPanel.formats.markdownDescription",
    requiresMetadata: true,
  },
  {
    value: "epub",
    labelKey: "novel.exportPanel.formats.epub",
    descriptionKey: "novel.exportPanel.formats.epubDescription",
    requiresMetadata: true,
  },
  {
    value: "pdf",
    labelKey: "novel.exportPanel.formats.pdf",
    descriptionKey: "novel.exportPanel.formats.pdfDescription",
    requiresMetadata: false,
  },
  {
    value: "batch",
    labelKey: "novel.exportPanel.formats.batch",
    descriptionKey: "novel.exportPanel.formats.batchDescription",
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
  meta: ExportPublicationMetadata,
): ExportPublicationMetadata | null => {
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
  const { t } = useI18n();
  const tRef = useRef(t);
  tRef.current = t;
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
  const selectedFormatOption = EXPORT_FORMATS.find(
    (format) => format.value === selectedFormat,
  )!;
  const selectedFormatLabel = t(selectedFormatOption.labelKey);
  const selectedFormatDescription = t(selectedFormatOption.descriptionKey);
  const selectedMetadata = exportMetadata(meta);
  const projectInput: ExportProjectInput = { project_id: project.id };
  const selectedExportReady =
    !selectedFormatOption.requiresMetadata || selectedMetadata != null;
  useNovelDetailDirty(
    `export:${project.id}`,
    meta.title.trim() !== project.title.trim() ||
      meta.author.trim().length > 0 ||
      selectedFormat !== "txt",
    () => {
      setMeta({ title: project.title, author: "" });
      setSelectedFormat("txt");
    },
  );

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

  useEffect(() => () => onBusyChange?.(false), [onBusyChange]);

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
    labelKey: MessageKey,
    fn: (taskId: string) => Promise<ExportFileData | ExportFileData[]>,
  ) => {
    if (exportBusyRef.current) return;
    const requestProjectId = project.id;
    const requestSeq = exportSeqRef.current + 1;
    exportSeqRef.current = requestSeq;
    exportBusyRef.current = true;
    const taskId = createWorkflowTaskId(`export-${key}`, requestProjectId);
    const label = tRef.current(labelKey);
    setActing(key);
    setLastResult(null);
    try {
      const result = await runProgressTask(
        {
          cancelTaskId: taskId,
          cancelTask: cancelWorkflowTask,
          pollTaskProgress: true,
          title: tRef.current("novel.exportPanel.taskTitle", { format: label }),
          startMessage: tRef.current("novel.exportPanel.taskPreparing", { format: label }),
          successMessage: tRef.current("novel.exportPanel.taskCompleted", { format: label }),
          errorMessage: tRef.current("novel.exportPanel.failed"),
          total: 4,
          cancelLabel: tRef.current("novel.exportPanel.stop"),
          runningHint: tRef.current("novel.exportPanel.runningHint"),
        },
        async () => {
          if (!isCurrentExport(requestProjectId, requestSeq)) {
            throw new Error(tRef.current("workflow.cancelled"));
          }
          const exported = await fn(taskId);
          if (!isCurrentExport(requestProjectId, requestSeq)) {
            throw new Error(tRef.current("workflow.cancelled"));
          }
          const files = Array.isArray(exported) ? exported : [exported];
          return { fileNames: downloadExportFiles(files), fileCount: files.length };
        },
      );
      if (!isCurrentExport(requestProjectId, requestSeq)) return;
      setLastResult(result.fileNames);
      addToast(
        result.fileCount > 1
          ? tRef.current("novel.exportPanel.downloadedFiles", { count: result.fileCount })
          : tRef.current("novel.exportPanel.downloadComplete"),
        "success",
      );
    } catch (err) {
      if (!isCurrentExport(requestProjectId, requestSeq)) return;
      const message = extractErrorMessage(err, tRef.current("novel.exportPanel.failed"));
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
        void wrap("txt", "novel.exportPanel.formats.txt", (taskId) => exportTxtData(projectInput, taskId));
        return;
      case "markdown": {
        const metadata = selectedMetadata;
        if (!metadata) return;
        void wrap("markdown", "novel.exportPanel.formats.markdown", (taskId) =>
          exportMarkdownData(
            {
              project_id: project.id,
              metadata,
            } satisfies ExportPublicationInput,
            taskId,
          ),
        );
        return;
      }
      case "epub": {
        const metadata = selectedMetadata;
        if (!metadata) return;
        void wrap("epub", "novel.exportPanel.formats.epub", (taskId) =>
          exportEpubData(
            {
              project_id: project.id,
              metadata,
            } satisfies ExportPublicationInput,
            taskId,
          ),
        );
        return;
      }
      case "pdf":
        void wrap("pdf", "novel.exportPanel.formats.pdf", (taskId) => exportPdfData(projectInput, taskId));
        return;
      case "batch": {
        const metadata = selectedMetadata;
        if (!metadata) return;
        void wrap("batch", "novel.exportPanel.formats.batch", (taskId) =>
          exportBatchData(
            {
              project_id: project.id,
              formats: ["txt", "markdown"],
              metadata,
            } satisfies BatchExportInput,
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
        <span className="text-small text-book-text-muted">{t("novel.exportPanel.format")}</span>
        <ThemedSelect
          value={selectedFormat}
          onChange={(selectedValue) =>
            setSelectedFormat(selectedValue as ExportFormat)
          }
          className="pyqt-input w-full !py-1.5 text-xs"
          disabled={exportBusy}
        >
          {EXPORT_FORMATS.map((format) => (
            <option key={format.value} value={format.value}>
              {t(format.labelKey)}
            </option>
          ))}
        </ThemedSelect>
      </label>

      <p className="text-small leading-relaxed text-book-text-muted">
        {selectedFormatDescription}
      </p>

      {selectedFormatOption.requiresMetadata ? (
        <section className="border-y border-book-border/45 py-3">
          <div className="mb-2 text-xs font-semibold text-book-text-main">
            {t("novel.exportPanel.metadataTitle")}
          </div>
          <p className="mb-2 text-small leading-relaxed text-book-text-muted">
            {t("novel.exportPanel.metadataHint")}
          </p>
          <div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
            <label className="flex flex-col gap-1">
              <span className="text-small text-book-text-muted">{t("novel.exportPanel.title")}</span>
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
              <span className="text-small text-book-text-muted">{t("novel.exportPanel.author")}</span>
              <input
                type="text"
                value={meta.author}
                onChange={(event) =>
                  setMeta({ ...meta, author: event.target.value })
                }
                className="pyqt-input !py-1.5 text-xs"
                disabled={exportBusy}
                placeholder={t("novel.exportPanel.authorPlaceholder")}
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
            ? t("novel.exportPanel.exporting", { format: selectedFormatLabel })
            : t("novel.exportPanel.download", { format: selectedFormatLabel })}
        </BookButton>
      </div>

      {lastResult ? (
        <div className="flex items-start gap-2 rounded-xl bg-success-bg/55 px-3 py-2.5 text-small text-book-text-sub">
          <CheckCircle2 size={14} className="mt-0.5 shrink-0 text-success" />
          <div className="min-w-0">
            <div className="font-medium text-book-text-main">
              {lastResult.length > 1
                ? t("novel.exportPanel.downloadedFiles", { count: lastResult.length })
                : t("novel.exportPanel.downloadComplete")}
            </div>
            <div className="mt-0.5 break-all text-book-text-muted">
              {lastResult.join(t("common.listSeparator"))}
            </div>
          </div>
        </div>
      ) : null}

      <details className="group a4n-flat-surface overflow-hidden">
        <summary className="flex cursor-pointer items-center gap-2 px-3 py-2.5 text-xs font-semibold text-book-text-main">
          <ChevronDown size={14} aria-hidden="true" className="shrink-0 transition-transform group-open:rotate-180" />
          <span className="flex-1">{t("novel.exportPanel.structuredData")}</span>
          <span className="text-caption font-normal text-book-text-muted">{t("novel.exportPanel.structuredDataHint")}</span>
        </summary>
        <div className="grid grid-cols-2 gap-2 border-t border-book-border/45 p-3">
          <BookButton
            variant="secondary"
            size="sm"
            className="!justify-start !text-xs"
            onClick={() =>
              void wrap("outline", "novel.exportPanel.outlineJson", (taskId) =>
                exportOutlineJsonData(projectInput, taskId),
              )
            }
            disabled={exportBusy}
          >
            {spin("outline")} {t("novel.exportPanel.outlineJson")}
          </BookButton>
          <BookButton
            variant="secondary"
            size="sm"
            className="!justify-start !text-xs"
            onClick={() =>
              void wrap("characters", "novel.exportPanel.charactersJson", (taskId) =>
                exportCharactersJsonData(projectInput, taskId),
              )
            }
            disabled={exportBusy}
          >
            {spin("characters")} {t("novel.exportPanel.charactersJson")}
          </BookButton>
        </div>
      </details>
    </div>
  );
};
