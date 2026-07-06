import React, { useCallback, useEffect, useRef, useState } from "react";
import {
  Copy,
  Database,
  FolderOpen,
  Loader2,
  RefreshCw,
  Trash2,
} from "lucide-react";
import { BookButton, BookIconButton } from "../../../components/ui/BookButton";
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
  callerUserId?: number | null;
  isAdmin?: boolean;
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
  callerUserId,
  isAdmin = false,
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
  }, [addToast]);

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
      callerUserId == null ||
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
      const summary = await listOrphanProjectDataDirs(callerUserId);
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
  }, [addToast, callerUserId, isAdmin]);

  const cleanupOrphanProjectDirs = useCallback(async () => {
    if (
      !isAdmin ||
      callerUserId == null ||
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
      const summary = await cleanupOrphanProjectDataDirs(callerUserId);
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
  }, [addToast, callerUserId, isAdmin, orphanSummary]);

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
        ["提示词", paths.prompts_dir],
        ["提示词历史", paths.prompts_git_dir],
        ["向量索引", paths.lance_dir],
        ["全文索引", paths.tantivy_dir],
      ]
    : [];

  return (
    <div className="mt-5 pyqt-card p-3 space-y-3 text-xs">
      <div className="flex items-center justify-between gap-2">
        <div className="font-serif text-sm text-book-text-main inline-flex items-center gap-1.5">
          <FolderOpen size={14} className="text-book-primary" />
          本地存储
        </div>
        <BookIconButton
          label="刷新路径"
          variant="ghost"
          size="sm"
          onClick={refresh}
          disabled={loading}
        >
          <RefreshCw
            size={12}
            className={loading ? "animate-spin" : undefined}
          />
        </BookIconButton>
      </div>

      {loading && !paths ? (
        <div className="text-book-text-muted inline-flex items-center gap-1.5">
          <Loader2 size={11} className="animate-spin" /> 加载中…
        </div>
      ) : (
        <div className="space-y-1.5">
          {rows.map(([label, value]) => (
            <div
              key={label}
              className="grid grid-cols-[72px_1fr_auto] items-center gap-2"
            >
              <span className="text-book-text-muted">{label}</span>
              <span className="font-mono text-[11px] text-book-text-main break-all">
                {value}
              </span>
              <BookIconButton
                label={`复制${label}`}
                variant="ghost"
                size="sm"
                onClick={() => copyPath(label, value)}
              >
                <Copy size={11} />
              </BookIconButton>
            </div>
          ))}
          {copiedKey && (
            <div className="text-success inline-flex items-center gap-1">
              <Database size={11} />
              已复制：{copiedKey}
            </div>
          )}
        </div>
      )}

      {isAdmin && callerUserId != null && (
        <div className="border-t border-book-border pt-3 space-y-2">
          <div className="flex items-center justify-between gap-2">
            <div className="text-book-text-main">孤儿项目目录</div>
            <div className="inline-flex items-center gap-2">
              <BookButton
                variant="ghost"
                size="sm"
                onClick={scanOrphanProjectDirs}
                disabled={orphanLoading || orphanCleaning}
              >
                {orphanLoading ? (
                  <Loader2 size={12} className="animate-spin" />
                ) : null}
                扫描
              </BookButton>
              <BookButton
                variant="danger"
                size="sm"
                onClick={cleanupOrphanProjectDirs}
                disabled={
                  orphanCleaning ||
                  orphanLoading ||
                  (orphanSummary?.total_count ?? 0) <= 0
                }
              >
                {orphanCleaning ? (
                  <Loader2 size={12} className="animate-spin" />
                ) : (
                  <Trash2 size={12} />
                )}
                清理
              </BookButton>
            </div>
          </div>
          {orphanSummary && (
            <div className="space-y-1 text-book-text-muted">
              <div>
                {orphanSummary.total_count > 0
                  ? `${orphanSummary.total_count} 个目录，约 ${formatBytes(orphanSummary.total_size_bytes)}`
                  : "未发现可清理目录"}
              </div>
              {orphanSummary.items.slice(0, 5).map((item) => (
                <div
                  key={item.project_id}
                  className="font-mono text-[11px] break-all"
                >
                  #{item.project_id} · {formatBytes(item.size_bytes)} ·{" "}
                  {item.path}
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
};
