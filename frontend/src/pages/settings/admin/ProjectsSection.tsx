import React, { useCallback, useState } from "react";
import { RefreshCw } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { listAllProjects, type AdminProjectSummary } from "../../../api/admin";
import { BookIconButton } from "../../../components/ui/BookButton";
import { useAdminSectionLoader } from "./useAdminSectionLoader";
import { getAdminProjectTypeLabel } from "../../../utils/projectRouting";
import { projectStatusLabel } from "../../status-machine-dialog/statusLabels";
import { useI18n } from "../../../i18n";

export const ProjectsSection: React.FC = () => {
  const { t } = useI18n();
  const [projects, setProjects] = useState<AdminProjectSummary[]>([]);
  const loadProjects = useCallback(() => listAllProjects(), []);
  const { loading, load } = useAdminSectionLoader(
    "admin-projects",
    loadProjects,
    setProjects,
    t("settingsAdmin.loadFailed"),
  );

  return (
    <div className="flex flex-col gap-3">
      <div className="flex min-h-8 items-center gap-2 text-xs text-book-text-muted">
        <span>{t("settingsAdmin.projectCount", { count: projects.length })}</span>
        <span className="flex-1" />
        <BookIconButton label={t("settingsAdmin.refreshProjects")} onClick={load} size="sm" disabled={loading}>
          <RefreshCw size={14} className={loading ? "animate-spin" : undefined} />
        </BookIconButton>
      </div>
      {loading && projects.length === 0 ? (
        <div className="a4n-empty-state a4n-empty-state--compact inline-flex items-center justify-center gap-1.5 text-xs">
          <LoadingSpinner size="sm" />
          {t("settingsAdmin.loadingProjects")}
        </div>
      ) : projects.length === 0 ? (
        <div className="a4n-empty-state a4n-empty-state--compact text-xs">
          {t("settingsAdmin.noProjects")}
        </div>
      ) : (
        <ul className="space-y-2.5">
          {projects.map((p) => (
            <li key={p.id} className="a4n-flat-surface p-3 text-xs">
              <div className="grid grid-cols-[minmax(0,1fr)_auto] items-start gap-3">
                <div className="min-w-0">
                  <div className="truncate font-medium text-book-text-main">{p.title}</div>
                  <div className="mt-1 truncate text-small text-book-text-muted">
                    {getAdminProjectTypeLabel(p, t)} · {projectStatusLabel(p.status, t)}
                  </div>
                </div>
                {(p.total_chapters ?? p.target_chapters) > 0 ? (
                  <span className="shrink-0 tabular-nums text-book-text-sub">
                    {t("settingsAdmin.chapterCount", { current: p.current_chapter, total: p.total_chapters ?? p.target_chapters })}
                  </span>
                ) : null}
              </div>
              <details className="mt-1 text-caption text-book-text-muted">
                <summary className="cursor-pointer">{t("settingsAdmin.diagnostics")}</summary>
                <div className="mt-1 flex flex-wrap gap-x-4 gap-y-1 font-mono">
                  <span>{t("settingsAdmin.projectId", { id: p.id })}</span>
                  <span>{t("settingsAdmin.userId", { id: p.user_id })}</span>
                </div>
              </details>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
};
