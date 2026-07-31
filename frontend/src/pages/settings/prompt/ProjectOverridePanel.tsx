import React, { useEffect, useRef, useState } from "react";
import { GitBranch, Trash2 } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { extractErrorMessage } from "../../../api";
import {
  promptListProjectOverrides,
  promptRemoveProjectOverride,
  promptSetProjectOverride,
  type ProjectPromptOverrideSummary,
  type PromptEntry,
} from "../../../api/prompt";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { useToast } from "../../../components/feedback/Toast";
import { BookButton } from "../../../components/ui/BookButton";
import { Dropdown } from "../../../components/ui/Dropdown";
import { formatUiDateTime } from "../../../utils/dateTime";
import { PromptProjectSelect } from "./PromptProjectSelect";

interface ProjectOverridePanelProps {
  prompt: PromptEntry;
  content: string;
  dirty: boolean;
}

export const ProjectOverridePanel: React.FC<ProjectOverridePanelProps> = ({ prompt, content, dirty }) => {
  const addToast = useToast((state) => state.addToast);
  const [projectId, setProjectId] = useState<number | null>(null);
  const [overrides, setOverrides] = useState<ProjectPromptOverrideSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [setting, setSetting] = useState(false);
  const [removing, setRemoving] = useState(false);
  const mountedRef = useRef(true);
  const projectIdRef = useRef<number | null>(null);
  const listSeqRef = useRef(0);
  const mutationSeqRef = useRef(0);
  const mutationBusyRef = useRef(false);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      listSeqRef.current += 1;
      mutationSeqRef.current += 1;
      mutationBusyRef.current = false;
    };
  }, []);

  const loadOverridesForProject = async (targetProjectId: number) => {
    const requestSeq = listSeqRef.current + 1;
    listSeqRef.current = requestSeq;
    setLoading(true);
    try {
      const next = await promptListProjectOverrides(targetProjectId);
      if (
        !mountedRef.current ||
        listSeqRef.current !== requestSeq ||
        projectIdRef.current !== targetProjectId
      ) {
        return;
      }
      setOverrides(next);
    } catch (error) {
      if (mountedRef.current && listSeqRef.current === requestSeq) {
        addToast(extractErrorMessage(error, "加载项目覆盖失败"), "error");
      }
    } finally {
      if (mountedRef.current && listSeqRef.current === requestSeq) setLoading(false);
    }
  };

  const handleProjectChange = (nextProjectId: number | null) => {
    projectIdRef.current = nextProjectId;
    setProjectId(nextProjectId);
    listSeqRef.current += 1;
    setOverrides([]);
    setLoading(false);
    if (nextProjectId != null) void loadOverridesForProject(nextProjectId);
  };

  const currentOverride = overrides.find((item) => item.prompt_name === prompt.name) ?? null;

  const handleSetOverride = async () => {
    const targetProjectId = projectIdRef.current;
    if (targetProjectId == null || mutationBusyRef.current) return;
    const ok = await confirmDialog({
      title: currentOverride ? "更新项目独立版本" : "设置项目独立版本",
      message: "所选项目将使用当前编辑器中的提示词内容，其他项目不受影响。是否继续？",
      confirmText: currentOverride ? "更新" : "设置",
      dialogType: "warning",
    });
    if (!ok || !mountedRef.current || projectIdRef.current !== targetProjectId) return;

    const requestSeq = mutationSeqRef.current + 1;
    mutationSeqRef.current = requestSeq;
    mutationBusyRef.current = true;
    setSetting(true);
    try {
      await promptSetProjectOverride({
        project_id: targetProjectId,
        name: prompt.name,
        content,
      });
      if (!mountedRef.current || mutationSeqRef.current !== requestSeq) return;
      addToast(currentOverride ? "项目独立版本已更新" : "项目独立版本已设置", "success");
      if (projectIdRef.current === targetProjectId) await loadOverridesForProject(targetProjectId);
    } catch (error) {
      if (mountedRef.current && mutationSeqRef.current === requestSeq) {
        addToast(extractErrorMessage(error, "设置项目独立版本失败"), "error");
      }
    } finally {
      if (mutationSeqRef.current === requestSeq) mutationBusyRef.current = false;
      if (mountedRef.current && mutationSeqRef.current === requestSeq) setSetting(false);
    }
  };

  const handleRemoveOverride = async () => {
    const targetProjectId = projectIdRef.current;
    if (targetProjectId == null || !currentOverride || mutationBusyRef.current) return;
    const ok = await confirmDialog({
      title: "移除项目独立版本",
      message: "移除后，所选项目会重新使用全局提示词内容。",
      confirmText: "移除",
      dialogType: "warning",
    });
    if (!ok || !mountedRef.current || projectIdRef.current !== targetProjectId) return;

    const requestSeq = mutationSeqRef.current + 1;
    mutationSeqRef.current = requestSeq;
    mutationBusyRef.current = true;
    setRemoving(true);
    try {
      await promptRemoveProjectOverride({ project_id: targetProjectId, name: prompt.name });
      if (!mountedRef.current || mutationSeqRef.current !== requestSeq) return;
      addToast("项目已恢复使用全局内容", "success");
      if (projectIdRef.current === targetProjectId) await loadOverridesForProject(targetProjectId);
    } catch (error) {
      if (mountedRef.current && mutationSeqRef.current === requestSeq) {
        addToast(extractErrorMessage(error, "移除项目独立版本失败"), "error");
      }
    } finally {
      if (mutationSeqRef.current === requestSeq) mutationBusyRef.current = false;
      if (mountedRef.current && mutationSeqRef.current === requestSeq) setRemoving(false);
    }
  };

  return (
    <details className="py-2.5">
      <summary className="inline-flex cursor-pointer items-center gap-1.5 text-xs text-book-text-sub">
        <GitBranch size={11} /> 项目独立版本
      </summary>
      <div className="mt-2 grid gap-2 text-xs">
        <PromptProjectSelect
          value={projectId}
          onChange={handleProjectChange}
          ariaLabel="选择使用独立提示词的项目"
          className="w-full"
        />
        {projectId == null ? (
          <div className="text-small text-book-text-muted">选择项目后可查看或设置独立内容。</div>
        ) : loading ? (
          <div className="inline-flex items-center gap-1.5 text-small text-book-text-muted">
            <LoadingSpinner size="sm" /> 正在读取项目设置…
          </div>
        ) : dirty ? (
          <div className="border-l border-warning/45 pl-2.5 text-small text-warning">
            请先保存当前提示词，再更新项目独立版本。
          </div>
        ) : (
          <div className="flex flex-wrap items-center gap-2 border-l border-book-border/45 pl-2.5">
            <div className="min-w-0 flex-1 text-small text-book-text-muted">
              {currentOverride
                ? `正在使用独立内容 · 更新于 ${formatUiDateTime(currentOverride.updated_at)}`
                : "当前使用全局内容"}
            </div>
            <BookButton size="sm" onClick={() => void handleSetOverride()} disabled={setting || removing}>
              {setting ? <LoadingSpinner size="sm" /> : null}
              {setting ? "保存中…" : currentOverride ? "更新为当前内容" : "使用当前内容"}
            </BookButton>
            {currentOverride && !setting ? (
              removing ? (
                <span className="inline-flex h-8 w-8 items-center justify-center text-book-text-muted">
                  <LoadingSpinner size="sm" />
                </span>
              ) : (
                <Dropdown
                  items={[
                    {
                      label: "恢复全局内容",
                      icon: <Trash2 size={12} />,
                      danger: true,
                      onClick: () => void handleRemoveOverride(),
                    },
                  ]}
                />
              )
            ) : null}
          </div>
        )}
      </div>
    </details>
  );
};
