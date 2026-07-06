import React, { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { Loader2 } from "lucide-react";
import { BookButton } from "../../../components/ui/BookButton";
import { useToast } from "../../../components/feedback/Toast";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { extractErrorMessage } from "../../../api";
import {
  diffBlueprintVersions,
  listBlueprintVersions,
  restoreBlueprintVersion,
  type BlueprintVersion,
} from "../../../api/blueprint";

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
    restoringRef.current = version.version;
    const ok = await confirmDialog({
      title: "恢复蓝图版本",
      message: `恢复到蓝图版本 ${version.version}？\n\n当前蓝图会被覆盖，但版本仓库保留所有历史。`,
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
      addToast(`已恢复到版本 ${version.version}`, "success");
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
      <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
        <h3 className="text-sm font-serif text-book-text-main">
          蓝图版本 <span className="text-xs text-book-text-muted">({versions.length})</span>
        </h3>
        <div className="flex flex-wrap items-center justify-end gap-2">
          {picked.length === 2 && (
            <BookButton variant="secondary" onClick={handleDiff} disabled={diffing}>
              {diffing ? (
                <span className="inline-flex items-center gap-2">
                  <Loader2 size={14} className="animate-spin" /> 对比中…
                </span>
              ) : (
                `对比 ${picked[0]} ↔ ${picked[1]}`
              )}
            </BookButton>
          )}
          <BookButton variant="secondary" onClick={load} disabled={loading || restoring != null}>
            {loading ? <Loader2 size={14} className="animate-spin" /> : "刷新"}
          </BookButton>
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
          <ul className="space-y-1 max-h-72 overflow-y-auto custom-scrollbar">
            {versions.map((version) => {
              const isPicked = picked.includes(version.version);
              return (
                <li
                  key={version.id}
                  className={`flex flex-wrap items-center gap-2 rounded px-2 py-1.5 text-xs transition-colors ${
                    isPicked
                      ? "bg-book-primary/10 border border-book-primary/40"
                      : "border border-transparent hover:bg-book-bg-tertiary"
                  }`}
                >
                  <input
                    type="checkbox"
                    checked={isPicked}
                    onChange={() => togglePick(version.version)}
                    className="accent-book-primary"
                  />
                  <span className="shrink-0 font-mono text-book-text-main">
                    {version.version}
                  </span>
                  <span className="shrink-0 text-[10px] text-book-text-muted">
                    {new Date(version.created_at).toLocaleString()}
                  </span>
                  {version.created_by && (
                    <span className="pyqt-pill !text-[10px] shrink-0">
                      {version.created_by}
                    </span>
                  )}
                  <span className="min-w-0 flex-1 basis-40 truncate text-book-text-sub">
                    {version.change_description || "（无说明）"}
                  </span>
                  <button
                    type="button"
                    onClick={() => void handleRestore(version)}
                    disabled={restoring !== null}
                    className="pyqt-btn-secondary shrink-0 !min-h-9 !px-2 text-[10px] sm:!min-h-[24px]"
                  >
                    {restoring === version.version ? (
                      <Loader2 size={10} className="animate-spin" />
                    ) : (
                      "恢复"
                    )}
                  </button>
                </li>
              );
            })}
          </ul>
          {diff != null && (
            <div className="mt-3 pt-3 border-t border-book-border/30">
              <div className="text-xs text-book-text-muted mb-1.5">
                Diff（{picked[0]} → {picked[1]}）
              </div>
              <pre className="rounded border border-book-border/40 bg-book-bg-tertiary/40 p-3 text-[10px] font-mono whitespace-pre-wrap break-words max-h-72 overflow-y-auto custom-scrollbar">
                {typeof diff === "string" ? diff : JSON.stringify(diff, null, 2)}
              </pre>
            </div>
          )}
        </>
      )}
    </div>
  );
};
