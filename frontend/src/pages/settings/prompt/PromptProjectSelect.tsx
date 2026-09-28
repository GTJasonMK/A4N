import { ThemedSelect } from "../../../components/ui/ThemedSelect";
import React, { useEffect, useState } from "react";
import { extractErrorMessage } from "../../../api";
import { listAllProjects, type NovelProject } from "../../../api/project";
import { useI18n } from "../../../i18n";

interface PromptProjectSelectProps {
  value: number | null;
  onChange: (projectId: number | null) => void;
  allowEmpty?: boolean;
  ariaLabel: string;
  className?: string;
}

export const PromptProjectSelect: React.FC<PromptProjectSelectProps> = ({
  value,
  onChange,
  allowEmpty = false,
  ariaLabel,
  className = "",
}) => {
  const { t } = useI18n();
  const [projects, setProjects] = useState<NovelProject[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    setLoading(true);
    setLoadError(null);
    void listAllProjects({ access_scope: "all" })
      .then((items) => {
        if (active) setProjects(items);
      })
      .catch((error) => {
        if (active) setLoadError(extractErrorMessage(error, t("settingsPrompt.loadProjectsFailed")));
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, []);

  return (
    <>
    <ThemedSelect
      value={value ?? ""}
      onChange={(selectedValue) => {
        const next = Number(selectedValue);
        onChange(Number.isInteger(next) && next > 0 ? next : null);
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
    {loadError ? <span role="alert" className="block text-small text-error">{loadError}</span> : null}
    </>
  );
};
