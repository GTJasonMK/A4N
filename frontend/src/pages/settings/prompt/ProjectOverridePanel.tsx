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
import { PromptProjectSelect } from "./PromptProjectSelect";
import { formatDateTime, useI18n } from "../../../i18n";

interface ProjectOverridePanelProps {
  prompt: PromptEntry;
  content: string;
  dirty: boolean;
}

export const ProjectOverridePanel: React.FC<ProjectOverridePanelProps> = ({ prompt, content, dirty }) => {
  const addToast = useToast((state) => state.addToast);
  const { t, locale } = useI18n();
  const [projectId, setProjectId] = useState<number | null>(null);
  const [overrides, setOverrides] = useState<ProjectPromptOverrideSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<{ reason: unknown } | null>(null);
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

  const loadOverridesForProject = async (targetProjectId: number): Promise<boolean> => {
    const requestSeq = listSeqRef.current + 1;
    listSeqRef.current = requestSeq;
    setLoading(true);
    setLoadError(null);
    try {
      const next = await promptListProjectOverrides(targetProjectId);
      if (
        !mountedRef.current ||
        listSeqRef.current !== requestSeq ||
        projectIdRef.current !== targetProjectId
      ) {
        return false;
      }
      setOverrides(next);
      return true;
    } catch (error) {
      if (mountedRef.current && listSeqRef.current === requestSeq) {
        setLoadError({ reason: error });
      }
      return false;
    } finally {
      if (mountedRef.current && listSeqRef.current === requestSeq) setLoading(false);
    }
  };

  const handleProjectChange = (nextProjectId: number | null) => {
    projectIdRef.current = nextProjectId;
    setProjectId(nextProjectId);
    listSeqRef.current += 1;
    setOverrides([]);
    setLoadError(null);
    setLoading(false);
    if (nextProjectId != null) void loadOverridesForProject(nextProjectId);
  };

  const currentOverride = overrides.find((item) => item.prompt_name === prompt.name) ?? null;
  const loadErrorMessage = loadError
    ? extractErrorMessage(loadError.reason, t("settingsPrompt.loadOverrideFailed"))
    : null;

  const handleSetOverride = async () => {
    const targetProjectId = projectIdRef.current;
    if (targetProjectId == null || mutationBusyRef.current) return;
    const ok = await confirmDialog({
      title: currentOverride ? t("settingsPrompt.updateOverride") : t("settingsPrompt.setOverride"),
      message: t("settingsPrompt.overrideMessage"),
      confirmText: currentOverride ? t("common.update") : t("common.set"),
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
      if (projectIdRef.current === targetProjectId && !await loadOverridesForProject(targetProjectId)) return;
      addToast(currentOverride ? t("settingsPrompt.overrideUpdated") : t("settingsPrompt.overrideSet"), "success");
    } catch (error) {
      if (mountedRef.current && mutationSeqRef.current === requestSeq) {
        addToast(extractErrorMessage(error, t("settingsPrompt.setOverrideFailed")), "error");
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
      title: t("settingsPrompt.removeOverride"),
      message: t("settingsPrompt.removeOverrideMessage"),
      confirmText: t("common.remove"),
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
      if (projectIdRef.current === targetProjectId && !await loadOverridesForProject(targetProjectId)) return;
      addToast(t("settingsPrompt.overrideRemoved"), "success");
    } catch (error) {
      if (mountedRef.current && mutationSeqRef.current === requestSeq) {
        addToast(extractErrorMessage(error, t("settingsPrompt.removeOverrideFailed")), "error");
      }
    } finally {
      if (mutationSeqRef.current === requestSeq) mutationBusyRef.current = false;
      if (mountedRef.current && mutationSeqRef.current === requestSeq) setRemoving(false);
    }
  };

  return (
    <details className="py-2.5">
      <summary className="inline-flex cursor-pointer items-center gap-1.5 text-xs text-book-text-sub">
        <GitBranch size={11} /> {t("settingsPrompt.projectOverride")}
      </summary>
      <div className="mt-2 grid gap-2 text-xs">
        <PromptProjectSelect
          value={projectId}
          onChange={handleProjectChange}
          ariaLabel={t("settingsPrompt.selectOverrideProject")}
          className="w-full"
        />
        {projectId == null ? (
          <div className="text-small text-book-text-muted">{t("settingsPrompt.selectProjectHint")}</div>
        ) : loading ? (
          <div className="inline-flex items-center gap-1.5 text-small text-book-text-muted">
            <LoadingSpinner size="sm" /> {t("settingsPrompt.loadingProjectOverride")}
          </div>
        ) : loadErrorMessage ? (
          <p role="alert" className="border-l border-error/40 pl-2.5 text-small text-error">{loadErrorMessage}</p>
        ) : dirty ? (
          <div className="border-l border-warning/45 pl-2.5 text-small text-warning">
            {t("settingsPrompt.saveBeforeOverride")}
          </div>
        ) : (
          <div className="flex flex-wrap items-center gap-2 border-l border-book-border/45 pl-2.5">
            <div className="min-w-0 flex-1 text-small text-book-text-muted">
              {currentOverride
                ? t("settingsPrompt.usingOverride", { date: formatDateTime(currentOverride.updated_at, locale) })
                : t("settingsPrompt.usingGlobal")}
            </div>
            <BookButton size="sm" onClick={() => void handleSetOverride()} disabled={setting || removing}>
              {setting ? <LoadingSpinner size="sm" /> : null}
              {setting ? t("settingsPrompt.saving") : currentOverride ? t("settingsPrompt.updateWithCurrent") : t("settingsPrompt.useCurrent")}
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
                      label: t("settingsPrompt.restoreGlobal"),
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
