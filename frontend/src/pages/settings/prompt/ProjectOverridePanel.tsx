import React, { useEffect, useRef, useState } from "react";
import { ChevronRight, GitBranch, Loader2 } from "lucide-react";
import { BookButton } from "../../../components/ui/BookButton";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { useToast } from "../../../components/feedback/Toast";
import { extractErrorMessage } from "../../../api";
import {
  promptListProjectOverrides,
  promptRemoveProjectOverride,
  promptSetProjectOverride,
  type PromptEntry,
} from "../../../api/prompt";

interface ProjectOverridePanelProps {
  prompt: PromptEntry;
  content: string;
}

export const ProjectOverridePanel: React.FC<ProjectOverridePanelProps> = ({ prompt, content }) => {
  const { addToast } = useToast();
  const [projectId, setProjectId] = useState("");
  const [overrides, setOverrides] = useState<unknown[]>([]);
  const [loadedProjectId, setLoadedProjectId] = useState<number | null>(null);
  const [loading, setLoading] = useState(false);
  const [setting, setSetting] = useState(false);
  const [removing, setRemoving] = useState<Set<string>>(() => new Set());
  const mountedRef = useRef(true);
  const projectIdInputRef = useRef("");
  const listSeqRef = useRef(0);
  const setOverrideSeqRef = useRef(0);
  const settingRef = useRef(false);
  const removingRef = useRef<Set<string>>(new Set());

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      listSeqRef.current += 1;
      setOverrideSeqRef.current += 1;
      settingRef.current = false;
      removingRef.current.clear();
    };
  }, []);

  const parseProjectId = (raw: string) => {
    const value = Number(raw);
    return Number.isInteger(value) && value > 0 ? value : null;
  };

  const currentProjectId = () => parseProjectId(projectIdInputRef.current);

  const handleProjectIdChange = (value: string) => {
    projectIdInputRef.current = value;
    setProjectId(value);
    listSeqRef.current += 1;
    setLoading(false);
    setOverrides([]);
    setLoadedProjectId(null);
  };

  const loadOverridesForProject = async (pid: number) => {
    const requestSeq = listSeqRef.current + 1;
    listSeqRef.current = requestSeq;
    setLoading(true);
    try {
      const nextOverrides = await promptListProjectOverrides(pid);
      if (
        !mountedRef.current ||
        listSeqRef.current !== requestSeq ||
        currentProjectId() !== pid
      ) {
        return;
      }
      setOverrides(nextOverrides);
      setLoadedProjectId(pid);
    } catch (err) {
      if (!mountedRef.current || listSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, "加载覆盖失败"), "error");
    } finally {
      if (mountedRef.current && listSeqRef.current === requestSeq) setLoading(false);
    }
  };

  const loadOverrides = async () => {
    const pid = currentProjectId();
    if (pid == null) {
      addToast("请输入有效的项目 ID", "error");
      return;
    }
    await loadOverridesForProject(pid);
  };

  const handleSetOverride = async () => {
    if (settingRef.current) return;
    const pid = currentProjectId();
    if (pid == null) {
      addToast("请输入有效的项目 ID", "error");
      return;
    }
    const requestPromptName = prompt.name;
    const requestContent = content;
    settingRef.current = true;
    const ok = await confirmDialog({
      title: "设置项目级覆盖",
      message: `把当前提示词「${requestPromptName}」的内容作为项目 #${pid} 的覆盖？此后该项目用本版本而非默认。`,
      confirmText: "设置覆盖",
      dialogType: "warning",
    });
    if (!ok) {
      settingRef.current = false;
      return;
    }
    const requestSeq = setOverrideSeqRef.current + 1;
    setOverrideSeqRef.current = requestSeq;
    setSetting(true);
    try {
      await promptSetProjectOverride({
        project_id: pid,
        name: requestPromptName,
        content: requestContent,
      });
      if (!mountedRef.current || setOverrideSeqRef.current !== requestSeq) return;
      addToast("已设置项目级覆盖", "success");
      if (currentProjectId() === pid) await loadOverridesForProject(pid);
    } catch (err) {
      if (!mountedRef.current || setOverrideSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, "设置覆盖失败"), "error");
    } finally {
      if (setOverrideSeqRef.current === requestSeq) settingRef.current = false;
      if (mountedRef.current && setOverrideSeqRef.current === requestSeq) setSetting(false);
    }
  };

  const handleRemoveOverride = async (pid: number, name: string) => {
    const removingKey = `${pid}:${name}`;
    if (removingRef.current.has(removingKey)) return;
    removingRef.current.add(removingKey);
    const ok = await confirmDialog({
      title: "移除项目级覆盖",
      message: `移除项目 #${pid} 对「${name}」的覆盖？`,
      confirmText: "移除",
      dialogType: "warning",
    });
    if (!ok) {
      removingRef.current.delete(removingKey);
      return;
    }
    setRemoving((current) => new Set(current).add(removingKey));
    try {
      await promptRemoveProjectOverride({ project_id: pid, name });
      if (!mountedRef.current) return;
      addToast("已移除覆盖", "success");
      if (currentProjectId() === pid) await loadOverridesForProject(pid);
    } catch (err) {
      if (!mountedRef.current) return;
      addToast(extractErrorMessage(err, "移除失败"), "error");
    } finally {
      removingRef.current.delete(removingKey);
      if (mountedRef.current) {
        setRemoving((current) => {
          const next = new Set(current);
          next.delete(removingKey);
          return next;
        });
      }
    }
  };

  const visibleOverrides = loadedProjectId === currentProjectId() ? overrides : [];

  return (
    <details className="pyqt-card p-2.5">
      <summary className="cursor-pointer text-xs text-book-text-sub inline-flex items-center gap-1.5">
        <GitBranch size={11} /> 项目级覆盖
      </summary>
      <div className="mt-2 flex flex-col gap-2">
        <div className="flex items-center gap-1.5 text-xs">
          <span className="text-book-text-muted">项目 ID</span>
          <input
            type="number"
            value={projectId}
            onChange={(event) => handleProjectIdChange(event.target.value)}
            placeholder="如 1"
            min={1}
            className="pyqt-input !py-1 !px-2 w-20"
          />
          <BookButton variant="secondary" onClick={loadOverrides} disabled={loading}>
            {loading ? <Loader2 size={11} className="animate-spin" /> : "查看覆盖"}
          </BookButton>
          <BookButton
            variant="primary"
            onClick={handleSetOverride}
            disabled={setting}
            title="把当前编辑器内容作为该项目的覆盖（不影响其它项目）"
          >
            {setting ? <Loader2 size={11} className="animate-spin" /> : "覆盖为当前内容"}
          </BookButton>
        </div>
        {visibleOverrides.length > 0 && loadedProjectId != null && (
          <ul className="space-y-0.5">
            {visibleOverrides.map((override, index) => {
              const obj = override as Record<string, unknown>;
              const name = String(obj.prompt_name ?? obj.name ?? `#${index}`);
              const removingKey = `${loadedProjectId}:${name}`;
              const isRemoving = removing.has(removingKey);
              return (
                <li
                  key={index}
                  className="flex items-center gap-2 px-2 py-1 rounded text-[11px] bg-book-bg-paper/40 border border-book-border/40"
                >
                  <ChevronRight size={10} className="text-book-text-muted" />
                  <span className="font-mono text-book-text-main">{name}</span>
                  {obj.updated_at != null && (
                    <span className="text-book-text-muted text-[10px]">
                      {String(obj.updated_at)}
                    </span>
                  )}
                  <span className="flex-1" />
                  <button
                    type="button"
                    onClick={() => void handleRemoveOverride(loadedProjectId, name)}
                    disabled={isRemoving}
                    className="text-[10px] text-error px-1.5 py-0.5 rounded hover:bg-error-bg"
                  >
                    {isRemoving ? "移除中…" : "移除"}
                  </button>
                </li>
              );
            })}
          </ul>
        )}
      </div>
    </details>
  );
};
