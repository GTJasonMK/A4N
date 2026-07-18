import React, { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { Check, Loader2 } from "lucide-react";
import { BookButton } from "../../../components/ui/BookButton";
import { Dropdown } from "../../../components/ui/Dropdown";
import { useToast } from "../../../components/feedback/Toast";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { extractErrorMessage } from "../../../api";
import {
  diffBlueprintVersions,
  listBlueprintVersions,
  restoreBlueprintVersion,
  type BlueprintVersion,
} from "../../../api/blueprint";
import { formatUiDateTime } from "../../../utils/dateTime";

const blueprintVersionLabel = (version: string, index: number): string => {
  const value = version.trim();
  if (/^v?\d+(?:\.\d+)*$/i.test(value)) return `版本 ${value}`;
  return index === 0 ? "最新版本" : `历史版本 ${index}`;
};

interface BlueprintVersionsPanelProps {
  projectId: number;
  onMutated: () => Promise<void>;
}

export const BlueprintVersionsPanel: React.FC<BlueprintVersionsPanelProps> = ({
  projectId,
  onMutated,
}) => {
  const { addToast } = useToast();
  const [versions, setVersions] = useState<BlueprintVersion[]>([]);
  const [loading, setLoading] = useState(false);
  const [picked, setPicked] = useState<string[]>([]);
  const [diff, setDiff] = useState<unknown>(null);
  const [diffing, setDiffing] = useState(false);
  const [restoring, setRestoring] = useState<string | null>(null);
  const loadSeqRef = useRef(0);
  const diffSeqRef = useRef(0);
  const mountedRef = useRef(true);
  const activeProjectRef = useRef(projectId);
  const loadingRef = useRef(false);
  const diffingRef = useRef(false);
  const restoringRef = useRef<string | null>(null);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      loadSeqRef.current += 1;
      diffSeqRef.current += 1;
      loadingRef.current = false;
      diffingRef.current = false;
      restoringRef.current = null;
    };
  }, []);

  const load = useCallback(async () => {
    if (loadingRef.current || restoringRef.current != null) return;
    const requestProjectId = projectId;
    const requestSeq = loadSeqRef.current + 1;
    loadSeqRef.current = requestSeq;
    loadingRef.current = true;
    setLoading(true);
    try {
      const data = await listBlueprintVersions(requestProjectId);
      if (
        !mountedRef.current ||
        activeProjectRef.current !== requestProjectId ||
        loadSeqRef.current !== requestSeq
      ) {
        return;
      }
      setVersions(data);
    } catch (err) {
      if (
        !mountedRef.current ||
        activeProjectRef.current !== requestProjectId ||
        loadSeqRef.current !== requestSeq
      ) {
        return;
      }
      addToast(extractErrorMessage(err, "加载蓝图版本失败"), "error");
    } finally {
      if (loadSeqRef.current === requestSeq) loadingRef.current = false;
      if (
        mountedRef.current &&
        activeProjectRef.current === requestProjectId &&
        loadSeqRef.current === requestSeq
      ) {
        setLoading(false);
      }
    }
  }, [addToast, projectId]);

  useLayoutEffect(() => {
    activeProjectRef.current = projectId;
    loadSeqRef.current += 1;
    diffSeqRef.current += 1;
    loadingRef.current = false;
    diffingRef.current = false;
    restoringRef.current = null;
    setVersions([]);
    setPicked([]);
    setDiff(null);
    setLoading(false);
    setDiffing(false);
    setRestoring(null);
  }, [projectId]);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    const available = new Set(versions.map((version) => version.version));
    setPicked((prev) => {
      const next = prev.filter((version) => available.has(version));
      if (next.length !== prev.length) setDiff(null);
      return next;
    });
  }, [versions]);

  const togglePick = (version: string) => {
    diffSeqRef.current += 1;
    diffingRef.current = false;
    setDiffing(false);
    setPicked((prev) => {
      const has = prev.includes(version);
      if (has) return prev.filter((item) => item !== version);
      const next = [...prev, version];
      return next.length > 2 ? next.slice(-2) : next;
    });
    setDiff(null);
  };

  const handleDiff = async () => {
    if (diffingRef.current || restoringRef.current != null) return;
    if (picked.length !== 2) {
      addToast("请勾选两个版本进行对比", "error");
      return;
    }
    const requestProjectId = projectId;
    const requestPair = [...picked] as [string, string];
    const requestSeq = diffSeqRef.current + 1;
    diffSeqRef.current = requestSeq;
    diffingRef.current = true;
    setDiffing(true);
    try {
      const data = await diffBlueprintVersions({
        project_id: requestProjectId,
        version_a: requestPair[0],
        version_b: requestPair[1],
      });
      if (
        !mountedRef.current ||
        activeProjectRef.current !== requestProjectId ||
        diffSeqRef.current !== requestSeq
      ) {
        return;
      }
      setDiff(data);
    } catch (err) {
      if (
        !mountedRef.current ||
        activeProjectRef.current !== requestProjectId ||
        diffSeqRef.current !== requestSeq
      ) {
        return;
      }
      addToast(extractErrorMessage(err, "对比失败"), "error");
    } finally {
      if (diffSeqRef.current === requestSeq) diffingRef.current = false;
      if (
        mountedRef.current &&
        activeProjectRef.current === requestProjectId &&
        diffSeqRef.current === requestSeq
      ) {
        setDiffing(false);
      }
    }
  };

  const handleRestore = async (version: BlueprintVersion) => {
    if (restoringRef.current != null) return;
    const requestProjectId = projectId;
    const versionIndex = versions.findIndex((item) => item.version === version.version);
    const versionLabel = blueprintVersionLabel(version.version, Math.max(0, versionIndex));
    restoringRef.current = version.version;
    const ok = await confirmDialog({
      title: "恢复蓝图版本",
      message: `恢复到${versionLabel}？\n\n当前蓝图会被覆盖，但版本仓库会保留全部历史。`,
      confirmText: "恢复",
      dialogType: "warning",
    });
    if (!ok) {
      if (restoringRef.current === version.version) restoringRef.current = null;
      return;
    }
    if (!mountedRef.current || activeProjectRef.current !== requestProjectId) {
      if (restoringRef.current === version.version) restoringRef.current = null;
      return;
    }
    loadSeqRef.current += 1;
    diffSeqRef.current += 1;
    loadingRef.current = false;
    diffingRef.current = false;
    setLoading(false);
    setDiffing(false);
    setDiff(null);
    setRestoring(version.version);
    try {
      await restoreBlueprintVersion({
        project_id: projectId,
        version: version.version,
      });
      if (!mountedRef.current || activeProjectRef.current !== requestProjectId) return;
      addToast(`已恢复到${versionLabel}`, "success");
      await onMutated();
      if (!mountedRef.current || activeProjectRef.current !== requestProjectId) return;
      await load();
    } catch (err) {
      if (!mountedRef.current || activeProjectRef.current !== requestProjectId) return;
      addToast(extractErrorMessage(err, "恢复失败"), "error");
    } finally {
      if (restoringRef.current === version.version) restoringRef.current = null;
      if (mountedRef.current && activeProjectRef.current === requestProjectId) setRestoring(null);
    }
  };

  return (
    <div className="space-y-3">
      <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
        <p className="text-[11px] leading-relaxed text-book-text-muted">
          {versions.length > 0
            ? `共 ${versions.length} 个版本；选择两个版本可查看差异`
            : "每次保存或优化蓝图都会生成版本记录"}
        </p>
        <div className="flex w-full items-center gap-2 sm:w-auto sm:shrink-0">
          <Dropdown
            items={[
              {
                label: loading ? "正在刷新…" : "刷新版本",
                icon: loading ? <Loader2 size={12} className="animate-spin" /> : undefined,
                onClick: load,
                disabled: loading || restoring != null,
              },
            ]}
          />
          {picked.length === 2 ? (
            <BookButton
              variant="secondary"
              size="sm"
              onClick={handleDiff}
              disabled={diffing}
              className="min-w-0 flex-1 !text-xs sm:flex-none"
            >
              {diffing ? (
                <span className="inline-flex items-center gap-2">
                  <Loader2 size={12} className="animate-spin" /> 对比中…
                </span>
              ) : (
                "对比已选版本"
              )}
            </BookButton>
          ) : null}
        </div>
      </div>

      {loading && versions.length === 0 ? (
        <div className="text-xs text-book-text-muted inline-flex items-center gap-2">
          <Loader2 size={12} className="animate-spin" /> 加载版本…
        </div>
      ) : versions.length === 0 ? (
        <div className="text-xs text-book-text-muted py-3">
          尚无蓝图版本记录。每次 AI 优化 / 保存蓝图会自动创建新版本。
        </div>
      ) : (
        <>
          <ul className="a4n-flat-surface divide-y divide-book-border/35 overflow-hidden">
            {versions.map((version, index) => {
              const isPicked = picked.includes(version.version);
              const versionLabel = blueprintVersionLabel(version.version, index);
              return (
                <li
                  key={version.id}
                  className={`flex items-start gap-2.5 px-3 py-2.5 text-xs transition-colors ${
                    isPicked ? "bg-book-primary/10" : ""
                  }`}
                >
                  <button
                    type="button"
                    aria-pressed={isPicked}
                    aria-label={`选择${versionLabel}`}
                    onClick={() => togglePick(version.version)}
                    className={`mt-0.5 inline-flex h-6 w-6 shrink-0 items-center justify-center rounded-full border transition-colors focus-visible:outline-2 focus-visible:outline-book-primary/35 ${
                      isPicked
                        ? "border-book-primary bg-book-primary text-white"
                        : "border-book-border/70 bg-book-bg-paper/45 text-transparent hover:border-book-primary/55"
                    }`}
                  >
                    <Check size={12} />
                  </button>
                  <span className="min-w-0 flex-1" title={version.version}>
                    <span className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
                      <span className="font-medium text-book-text-main">{versionLabel}</span>
                      <time className="text-[10px] tabular-nums text-book-text-muted">
                        {formatUiDateTime(version.created_at)}
                      </time>
                      {version.created_by ? (
                        <span className="text-[10px] text-book-text-muted">
                          {version.created_by}
                        </span>
                      ) : null}
                    </span>
                    <span className="mt-0.5 block truncate text-[11px] text-book-text-sub">
                      {version.change_description || "保存蓝图"}
                    </span>
                  </span>
                  {restoring != null ? (
                    restoring === version.version ? (
                      <span className="inline-flex h-8 w-8 shrink-0 items-center justify-center text-book-text-muted">
                        <Loader2 size={11} className="animate-spin" />
                      </span>
                    ) : (
                      <span className="h-8 w-8 shrink-0" />
                    )
                  ) : (
                    <Dropdown
                      items={[
                        {
                          label: "恢复此版本",
                          danger: true,
                          onClick: () => void handleRestore(version),
                        },
                      ]}
                    />
                  )}
                </li>
              );
            })}
          </ul>
          {diff != null && (
            <div className="mt-3 pt-3 border-t border-book-border/30">
              <div className="text-xs text-book-text-muted mb-1.5">
                已选版本差异
              </div>
              <BlueprintDiffSummary diff={diff} />
            </div>
          )}
        </>
      )}
    </div>
  );
};

const BLUEPRINT_DIFF_LABELS: Record<string, string> = {
  title: "标题",
  genre: "类型",
  style: "写作风格",
  tone: "情感基调",
  target_audience: "目标读者",
  full_synopsis: "完整梗概",
  world_setting: "世界观",
  total_chapters: "目标章节",
  characters: "角色",
  added: "新增",
  removed: "删除",
  changed: "修改",
  old: "原内容",
  new: "新内容",
};

const BlueprintDiffSummary: React.FC<{ diff: unknown }> = ({ diff }) => {
  if (typeof diff === "string") {
    return <div className="a4n-flat-surface whitespace-pre-wrap break-words p-3 text-[11px] leading-relaxed text-book-text-sub">{diff}</div>;
  }
  const entries =
    diff && typeof diff === "object" && !Array.isArray(diff)
      ? Object.entries(diff as Record<string, unknown>)
      : [];
  if (entries.length === 0) {
    return <div className="text-[11px] text-book-text-muted">没有可展示的差异</div>;
  }
  return (
    <div className="a4n-flat-surface divide-y divide-book-border/40">
        {entries.slice(0, 16).map(([key, value], index) => (
          <div key={key} className="grid gap-1 px-3 py-2 text-[11px] sm:grid-cols-[88px_minmax(0,1fr)] sm:gap-3">
            <span className="font-medium text-book-text-muted">
              {BLUEPRINT_DIFF_LABELS[key] ?? `变化 ${index + 1}`}
            </span>
            <span className="min-w-0 whitespace-pre-wrap break-words text-book-text-sub">
              {formatBlueprintDiffValue(value)}
            </span>
          </div>
        ))}
    </div>
  );
};

function formatBlueprintDiffValue(value: unknown): string {
  if (value == null) return "未记录";
  if (typeof value === "string") return value.trim() || "未记录";
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  if (Array.isArray(value)) {
    return value.map(formatBlueprintDiffValue).filter(Boolean).join("、") || "未记录";
  }
  if (typeof value === "object") {
    return Object.entries(value as Record<string, unknown>)
      .slice(0, 8)
      .map(([key, item]) => `${BLUEPRINT_DIFF_LABELS[key] ?? "内容"}：${formatBlueprintDiffValue(item)}`)
      .join("；");
  }
  return String(value);
}
