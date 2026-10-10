import { ThemedSelect } from "../../../components/ui/ThemedSelect";
import React, { useCallback, useEffect, useRef, useState } from "react";
import { RefreshCw } from "lucide-react";
import { extractErrorMessage } from "../../../api";
import { hasProjectEditorRole, listAllProjects, type NovelProject } from "../../../api/project";
import { BookButton } from "../../../components/ui/BookButton";
import { useI18n } from "../../../i18n";
import { isPositiveSafeInteger } from "../../../utils/numberRange";

interface PromptProjectSelectProps {
  value: number | null;
  onChange: (projectId: number | null) => void;
  allowEmpty?: boolean;
  requireEditor?: boolean;
  ariaLabel: string;
  className?: string;
}

export const PromptProjectSelect: React.FC<PromptProjectSelectProps> = ({
  value,
  onChange,
  allowEmpty = false,
  requireEditor = false,
  ariaLabel,
  className = "",
}) => {
  const { t } = useI18n();
  const [projects, setProjects] = useState<NovelProject[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<{ reason: unknown } | null>(null);
  const requestSeqRef = useRef(0);

  const loadProjects = useCallback(async () => {
    const requestSeq = requestSeqRef.current + 1;
    requestSeqRef.current = requestSeq;
    setLoading(true);
    setLoadError(null);
    try {
      const items = await listAllProjects({ access_scope: "all" });
      if (requestSeqRef.current !== requestSeq) return;
      setProjects(requireEditor ? items.filter(hasProjectEditorRole) : items);
    } catch (error) {
      if (requestSeqRef.current === requestSeq) setLoadError({ reason: error });
    } finally {
      if (requestSeqRef.current === requestSeq) setLoading(false);
    }
  }, [requireEditor]);

  useEffect(() => {
    void loadProjects();
    return () => {
      requestSeqRef.current += 1;
    };
  }, [loadProjects]);

  return (
    <>
    <ThemedSelect
      value={value ?? ""}
      onChange={(selectedValue) => {
        const next = Number(selectedValue);
        onChange(isPositiveSafeInteger(next) ? next : null);
      }}
      className={`pyqt-input min-w-0 ${className}`}
      aria-label={ariaLabel}
      disabled={loading || (loadError != null && !allowEmpty)}
    >
      <option value="">
        {loadError ? t("settingsPrompt.loadProjectsFailed") : loading ? t("settingsPrompt.loadingProjects") : allowEmpty ? t("settingsPrompt.noProjectContext") : t("settingsPrompt.selectProject")}
      </option>
      {projects.map((project) => (
        <option key={project.id} value={project.id}>
          {project.title}
        </option>
      ))}
    </ThemedSelect>
    {loadError ? (
      <div role="alert" className="flex flex-wrap items-center gap-2 text-small text-error">
        <span className="min-w-0 flex-1">
          {extractErrorMessage(loadError.reason, t("settingsPrompt.loadProjectsFailed"))}
        </span>
        <BookButton variant="secondary" size="sm" onClick={() => void loadProjects()} disabled={loading}>
          <RefreshCw size={12} className={loading ? "animate-spin" : undefined} />
          {t("common.retry")}
        </BookButton>
      </div>
    ) : null}
    </>
  );
};
