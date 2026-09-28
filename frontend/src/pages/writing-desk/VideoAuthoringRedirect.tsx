import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { listAllProjects, type NovelProject } from "../../api/project";
import { extractErrorMessage } from "../../api";
import { BookButton } from "../../components/ui/BookButton";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { useI18n } from "../../i18n";
import { getProjectWorkspaceRoute } from "../../utils/projectRouting";

/** Old bookmarks remain explainable after film authoring moves to video projects. */
export function VideoAuthoringRedirect({ projectId }: { projectId: number }) {
  const { t } = useI18n();
  const navigate = useNavigate();
  const [projects, setProjects] = useState<NovelProject[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [reload, setReload] = useState(0);
  useEffect(() => {
    let active = true;
    setLoading(true);
    setError(null);
    listAllProjects({ project_kind: "video", root_project_id: projectId, access_scope: "all" })
      .then((items) => { if (active) setProjects(items); })
      .catch((failure) => { if (active) setError(extractErrorMessage(failure, t("video.loadProjectFailed"))); })
      .finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, [projectId, reload, t]);
  return (
    <section className="a4n-flat-surface min-w-0 space-y-3 p-4">
      <h3 className="font-semibold">{t("common.mediaAuthoring.movedTitle")}</h3>
      <p className="text-sm text-book-text-muted">{t("common.mediaAuthoring.movedDescription")}</p>
      {loading ? <LoadingSpinner size="sm" text={t("common.loading")} /> : error ? (
        <div role="alert" className="space-y-2 text-sm text-error">
          <p>{error}</p>
          <BookButton variant="secondary" onClick={() => setReload((value) => value + 1)}>{t("video.chapter.retryLoad")}</BookButton>
        </div>
      ) : projects.length ? (
        <ul className="space-y-2">
          {projects.map((project) => <li key={project.id}>
            <BookButton variant="secondary" onClick={() => navigate(getProjectWorkspaceRoute(project))}>
              {project.title}
            </BookButton>
          </li>)}
        </ul>
      ) : (
        <p className="text-sm text-book-text-muted">{t("common.mediaAuthoring.noVideoProject")}</p>
      )}
      <BookButton variant="secondary" onClick={() => navigate("/")}>{t("common.mediaAuthoring.openProjects")}</BookButton>
    </section>
  );
}
