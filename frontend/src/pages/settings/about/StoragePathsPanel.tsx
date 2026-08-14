import React, { useCallback, useEffect, useRef, useState } from "react";
import { Copy,
  Database,
  FolderOpen,
  RefreshCw,
  Trash2, } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { BookButton, BookIconButton } from "../../../components/ui/BookButton";
import { Dropdown } from "../../../components/ui/Dropdown";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { useToast } from "../../../components/feedback/Toast";
import { extractErrorMessage } from "../../../api";
import {
  cleanupOrphanProjectDataDirs,
  getStoragePaths,
  listOrphanProjectDataDirs,
  type OrphanProjectDataDirsSummary,
  type StoragePathsSummary,
} from "../../../api/storagePaths";
import { copyTextToClipboard } from "../../../utils/clipboard";

interface StoragePathsPanelProps {
  isAdmin?: boolean;
  className?: string;
}

const formatBytes = (bytes: number): string => {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let unitIndex = 0;
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }
  return `${value >= 10 || unitIndex === 0 ? value.toFixed(0) : value.toFixed(1)} ${units[unitIndex]}`;
};

export const StoragePathsPanel: React.FC<StoragePathsPanelProps> = ({
  isAdmin = false,
  className = "mt-5",
}) => {
  const { addToast } = useToast();
  const [paths, setPaths] = useState<StoragePathsSummary | null>(null);
  const [loading, setLoading] = useState(true);
  const [orphanSummary, setOrphanSummary] =
    useState<OrphanProjectDataDirsSummary | null>(null);
  const [orphanLoading, setOrphanLoading] = useState(false);
  const [orphanCleaning, setOrphanCleaning] = useState(false);
  const [copiedKey, setCopiedKey] = useState<string | null>(null);
  const mountedRef = useRef(true);
  const refreshSeqRef = useRef(0);
  const refreshInFlightRef = useRef(false);
  const orphanSeqRef = useRef(0);
  const orphanScanningRef = useRef(false);
  const orphanCleaningRef = useRef(false);
  const copySeqRef = useRef(0);
  const copiedTimerRef = useRef<number | null>(null);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      refreshSeqRef.current += 1;
      refreshInFlightRef.current = false;
      orphanSeqRef.current += 1;
      orphanScanningRef.current = false;
      orphanCleaningRef.current = false;
      copySeqRef.current += 1;
      if (copiedTimerRef.current != null) {
        window.clearTimeout(copiedTimerRef.current);
      }
    };
  }, []);

  const refresh = useCallback(async () => {
    if (!isAdmin) {
      refreshInFlightRef.current = false;
      setPaths(null);
      setLoading(false);
      return;
    }
    if (refreshInFlightRef.current) return;
    refreshInFlightRef.current = true;
    const requestSeq = refreshSeqRef.current + 1;
    refreshSeqRef.current = requestSeq;
    setLoading(true);
    try {
      const nextPaths = await getStoragePaths();
      if (!mountedRef.current || refreshSeqRef.current !== requestSeq) return;
      setPaths(nextPaths);
    } catch (err) {
      if (!mountedRef.current || refreshSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, "加载存储路径失败"), "error");
    } finally {
      if (mountedRef.current && refreshSeqRef.current === requestSeq) {
        setLoading(false);
        refreshInFlightRef.current = false;
      }
    }
  }, [addToast, isAdmin]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const copyPath = async (key: string, value?: string | null) => {
    if (!value) return;
    const requestSeq = copySeqRef.current + 1;
    copySeqRef.current = requestSeq;
    try {
      await copyTextToClipboard(value);
      if (!mountedRef.current || copySeqRef.current !== requestSeq) return;
      setCopiedKey(key);
      if (copiedTimerRef.current != null) {
        window.clearTimeout(copiedTimerRef.current);
      }
      copiedTimerRef.current = window.setTimeout(() => {
        if (copySeqRef.current === requestSeq) setCopiedKey(null);
      }, 1500);
    } catch {
      if (!mountedRef.current || copySeqRef.current !== requestSeq) return;
      addToast("复制失败：请手动选取路径", "error");
    }
  };

  const scanOrphanProjectDirs = useCallback(async () => {
    if (
      !isAdmin ||
      orphanScanningRef.current ||
      orphanCleaningRef.current
    ) {
      return;
    }
    orphanScanningRef.current = true;
    const requestSeq = orphanSeqRef.current + 1;
    orphanSeqRef.current = requestSeq;
    setOrphanLoading(true);
    try {
      const summary = await listOrphanProjectDataDirs();
      if (!mountedRef.current || orphanSeqRef.current !== requestSeq) return;
      setOrphanSummary(summary);
    } catch (err) {
      if (!mountedRef.current || orphanSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, "扫描孤儿项目目录失败"), "error");
    } finally {
      if (orphanSeqRef.current === requestSeq) {
        orphanScanningRef.current = false;
      }
      if (mountedRef.current && orphanSeqRef.current === requestSeq) {
        setOrphanLoading(false);
      }
    }
  }, [addToast, isAdmin]);

  const cleanupOrphanProjectDirs = useCallback(async () => {
    if (
      !isAdmin ||
      orphanCleaningRef.current ||
      orphanScanningRef.current
    ) {
      return;
    }
    const summary = orphanSummary;
    if (!summary || summary.total_count <= 0) return;
    orphanCleaningRef.current = true;
    setOrphanCleaning(true);
    const confirmed = await confirmDialog({
      title: "清理孤儿项目目录",
      message: `确认删除 ${summary.total_count} 个孤儿项目目录（约 ${formatBytes(summary.total_size_bytes)}）？`,
      confirmText: "删除",
      cancelText: "取消",
      dialogType: "danger",
    });
    if (!confirmed) {
      orphanCleaningRef.current = false;
      if (mountedRef.current) setOrphanCleaning(false);
      return;
    }
    const requestSeq = orphanSeqRef.current + 1;
    orphanSeqRef.current = requestSeq;
    try {
      const summary = await cleanupOrphanProjectDataDirs();
      if (!mountedRef.current || orphanSeqRef.current !== requestSeq) return;
      setOrphanSummary({ items: [], total_count: 0, total_size_bytes: 0 });
      addToast(
        `已清理 ${summary.total_count} 个孤儿项目目录，释放约 ${formatBytes(summary.total_size_bytes)}`,
        "success",
      );
    } catch (err) {
      if (!mountedRef.current || orphanSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, "清理孤儿项目目录失败"), "error");
    } finally {
      if (orphanSeqRef.current === requestSeq) {
        orphanCleaningRef.current = false;
      }
      if (mountedRef.current && orphanSeqRef.current === requestSeq) {
        setOrphanCleaning(false);
      }
    }
  }, [addToast, isAdmin, orphanSummary]);

  const rows = paths
    ? [
        ["应用目录", paths.application_home],
        ["数据目录", paths.data_dir],
        ["配置文件", paths.config_file],
        ["数据库", paths.database_url],
        ["项目数据", paths.projects_dir],
        ["导出文件", paths.exports_dir],
        ["备份文件", paths.backups_dir],
        ["日志", paths.log_dir],
        ["WebView 数据", paths.webview_dir],
        ["前端状态", paths.frontend_state_file],
        ["缓存", paths.cache_dir],
        ["嵌入模型", paths.embedding_models_dir],
        ["临时文件", paths.temp_dir],
        ["提示词历史", paths.prompts_git_dir],
        ["RAG 索引 (Qdrant URL)", paths.qdrant_url || "（未配置 A4N_QDRANT_URL）"],
        ["RAG collection", paths.qdrant_collection],
      ]
    : [];

  return (
    <div className={`${className} space-y-3 text-xs`}>
      <section aria-label="存储位置" className="a4n-flat-surface space-y-3 p-3">
        <div className="flex items-center gap-2">
          <FolderOpen size={14} className="shrink-0 text-book-primary" />
          <div className="min-w-0 flex-1">
            <h3 className="text-xs font-semibold text-book-text-main">存储位置</h3>
            <p className="mt-0.5 text-small text-book-text-muted">完整路径默认隐藏</p>
          </div>
          <BookIconButton
            label="刷新路径"
            variant="ghost"
            size="sm"
            onClick={refresh}
            disabled={loading}
          >
            <RefreshCw size={12} className={loading ? "animate-spin" : undefined} />
          </BookIconButton>
        </div>

        {loading && !paths ? (
          <div className="inline-flex items-center gap-1.5 text-book-text-muted">
            <LoadingSpinner size="sm" /> 加载中…
          </div>
        ) : (
          <details className="border-t border-book-border/35 pt-2">
            <summary className="flex cursor-pointer list-none items-center gap-2 py-1 text-book-text-sub [&::-webkit-details-marker]:hidden">
              <span>查看存储位置</span>
              <span className="text-small text-book-text-muted">{rows.length} 项</span>
            </summary>
            <div className="mt-2 divide-y divide-book-border/30 border-y border-book-border/30">
              {rows.map(([label, value]) => (
                <div
                  key={label}
                  className="grid grid-cols-[72px_minmax(0,1fr)_auto] items-center gap-2 py-2"
                >
                  <span className="text-book-text-muted">{label}</span>
                  <span className="break-all font-mono text-small text-book-text-main">
                    {storageValueForDisplay(label, value)}
                  </span>
                  {label === "数据库" ? (
                    <span className="h-8 w-8" aria-hidden="true" />
                  ) : (
                    <BookIconButton
                      label={`复制${label}`}
                      variant="ghost"
                      size="sm"
                      onClick={() => copyPath(label, value)}
                    >
                      <Copy size={11} />
                    </BookIconButton>
                  )}
                </div>
              ))}
            </div>
            {copiedKey && (
              <div className="mt-2 inline-flex items-center gap-1 text-success">
                <Database size={11} />
                已复制：{copiedKey}
              </div>
            )}
          </details>
        )}
      </section>

      {isAdmin && (
        <section aria-label="存储清理" className="a4n-flat-surface space-y-3 p-3">
          <div className="flex items-center gap-2">
            <Trash2 size={14} className="shrink-0 text-book-primary" />
            <div className="min-w-0 flex-1">
              <h3 className="text-xs font-semibold text-book-text-main">存储清理</h3>
              <p className="mt-0.5 text-small text-book-text-muted">
                扫描并清理已失去项目归属的本地目录
              </p>
            </div>
          </div>

          <div className="flex flex-wrap items-center gap-2 border-t border-book-border/35 pt-3">
            <div className="min-w-0 flex-1">
              <div className="font-medium text-book-text-main">孤儿项目目录</div>
              <div className="mt-0.5 text-small text-book-text-muted">
                {orphanSummary
                  ? orphanSummary.total_count > 0
                    ? `${orphanSummary.total_count} 个目录，约 ${formatBytes(orphanSummary.total_size_bytes)}`
                    : "未发现可清理目录"
                  : "尚未扫描"}
              </div>
            </div>
            <BookButton
              variant="secondary"
              size="sm"
              onClick={scanOrphanProjectDirs}
              disabled={orphanLoading || orphanCleaning}
            >
              {orphanLoading ? <LoadingSpinner size="sm" /> : null}
              {orphanLoading ? "扫描中…" : "扫描"}
            </BookButton>
            {orphanCleaning ? (
              <span className="inline-flex min-h-8 items-center gap-1 px-2 text-small text-book-text-muted">
                <LoadingSpinner size="sm" />
                正在清理…
              </span>
            ) : (orphanSummary?.total_count ?? 0) > 0 ? (
              <Dropdown
                items={[
                  {
                    label: "清理孤儿项目目录",
                    icon: <Trash2 size={12} />,
                    danger: true,
                    onClick: () => void cleanupOrphanProjectDirs(),
                  },
                ]}
              />
            ) : null}
          </div>

          {orphanSummary && orphanSummary.items.length > 0 ? (
            <details className="border-t border-book-border/35 pt-2 text-caption text-book-text-muted">
              <summary className="w-fit cursor-pointer hover:text-book-text-main">
                诊断信息
              </summary>
              <div className="mt-1 space-y-1 font-mono">
                {orphanSummary.items.slice(0, 5).map((item) => (
                  <div key={item.project_id} className="break-all">
                    {item.path} · {formatBytes(item.size_bytes)}
                  </div>
                ))}
              </div>
            </details>
          ) : null}
        </section>
      )}
    </div>
  );
};

function storageValueForDisplay(label: string, value: string): string {
  if (label !== "数据库") return value;
  try {
    const url = new URL(value);
    const databaseName = url.pathname.replace(/^\//, "");
    const endpoint = `${url.hostname}${url.port ? `:${url.port}` : ""}`;
    return databaseName ? `${endpoint}/${databaseName}` : endpoint || "已配置";
  } catch {
    return value ? "已配置" : "未配置";
  }
}
