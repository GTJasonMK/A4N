import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  AlertTriangle,
  ArrowLeft,
  Check,
  CircleHelp,
  Clock3,
  ExternalLink,
  FlaskConical,
  Play,
  RefreshCw,
  Square,
  Trash2,
} from "lucide-react";
import { useNavigate, useSearchParams } from "react-router-dom";
import type {
  TestRunDetail,
  TestRunStatus,
  TestRunSummary,
  TestScenarioDescriptor,
  TestTarget,
} from "../../api/generated/host";
import {
  cancelTestRun,
  deleteTestRun,
  getTestRun,
  listAllProjects,
  listTestRuns,
  listTestScenarios,
  startTestRun,
  type AdminProjectSummary,
} from "../../api/admin";
import { extractErrorMessage } from "../../api";
import { BookButton, BookIconButton } from "../../components/ui/BookButton";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import { useAuthStore } from "../../store/auth";
import { formatDateTime, useI18n } from "../../i18n";

type TargetMode = "test_project" | "current_project" | "repository";
type MessageKey = Parameters<ReturnType<typeof useI18n>["t"]>[0];
type ErrorScope = "load" | "projects" | "run" | "delete" | "project";

const PROJECT_ROUTES: Record<AdminProjectSummary["project_kind"], string> = {
  text: "novel",
  video: "video-project",
  canvas: "canvas-project",
  comic: "comic-project",
  galgame: "galgame-project",
};

const STATUS_KEYS: Record<TestRunStatus, MessageKey> = {
  queued: "settingsAdmin.testCenter.queued",
  running: "settingsAdmin.testCenter.running",
  passed: "settingsAdmin.testCenter.passed",
  failed: "settingsAdmin.testCenter.failed",
  cancelled: "settingsAdmin.testCenter.cancelled",
  timed_out: "settingsAdmin.testCenter.timedOut",
};

const STATUS_STYLES: Record<TestRunStatus, string> = {
  queued: "text-book-text-muted",
  running: "text-book-primary",
  passed: "text-success",
  failed: "text-error",
  cancelled: "text-book-text-muted",
  timed_out: "text-warning",
};

const isActive = (status: TestRunStatus): boolean =>
  status === "queued" || status === "running";

function upsertRunSummary(
  runs: TestRunSummary[],
  summary: TestRunSummary,
): TestRunSummary[] {
  const index = runs.findIndex((run) => run.id === summary.id);
  if (index < 0) return [summary, ...runs].slice(0, 30);
  return runs.map((run) => run.id === summary.id ? mergeRunSummary(run, summary) : run);
}

function mergeRunSummary(current: TestRunSummary, incoming: TestRunSummary): TestRunSummary {
  const rank = (status: TestRunStatus) =>
    status === "queued" ? 0 : status === "running" ? 1 : 2;
  const currentRank = rank(current.status);
  const incomingRank = rank(incoming.status);
  const keepCurrentStatus = currentRank > incomingRank
    || (currentRank === 2 && incomingRank === 2 && current.status !== incoming.status);
  return {
    ...incoming,
    status: keepCurrentStatus ? current.status : incoming.status,
    target_project_id: incoming.target_project_id ?? current.target_project_id,
    started_at: incoming.started_at ?? current.started_at,
    finished_at: incoming.finished_at ?? current.finished_at,
    duration_ms: incoming.duration_ms ?? current.duration_ms,
    message: keepCurrentStatus ? current.message ?? incoming.message : incoming.message ?? current.message,
    impact: incoming.impact.length >= current.impact.length ? incoming.impact : current.impact,
  };
}

function mergeRunList(
  current: TestRunSummary[],
  incoming: TestRunSummary[],
  selectedRunId: string,
): TestRunSummary[] {
  const next = incoming.map((run) => {
    const previous = current.find((item) => item.id === run.id);
    return previous ? mergeRunSummary(previous, run) : run;
  });
  const present = new Set(next.map((run) => run.id));
  for (const run of current) {
    if (!present.has(run.id) && (isActive(run.status) || run.id === selectedRunId)) next.push(run);
  }
  return next.slice(0, 30);
}

function runCreatedProject(run: TestRunSummary): boolean {
  return run.impact.some((impact) =>
    impact.action === "create" && impact.resource === "project" && impact.project_id != null,
  );
}

const SCENARIO_COPY_KEYS: Record<string, { title: MessageKey; description: MessageKey }> = {
  admin_health_check: {
    title: "settingsAdmin.testCenter.scenarioAdminHealthTitle",
    description: "settingsAdmin.testCenter.scenarioAdminHealthDescription",
  },
  admin_overview: {
    title: "settingsAdmin.testCenter.scenarioAdminOverviewTitle",
    description: "settingsAdmin.testCenter.scenarioAdminOverviewDescription",
  },
  project_list: {
    title: "settingsAdmin.testCenter.scenarioProjectListTitle",
    description: "settingsAdmin.testCenter.scenarioProjectListDescription",
  },
  project_create_update: {
    title: "settingsAdmin.testCenter.scenarioProjectCreateUpdateTitle",
    description: "settingsAdmin.testCenter.scenarioProjectCreateUpdateDescription",
  },
  project_duplicate: {
    title: "settingsAdmin.testCenter.scenarioProjectDuplicateTitle",
    description: "settingsAdmin.testCenter.scenarioProjectDuplicateDescription",
  },
  project_status_read: {
    title: "settingsAdmin.testCenter.scenarioProjectStatusTitle",
    description: "settingsAdmin.testCenter.scenarioProjectStatusDescription",
  },
  "repository_test_core-functional": {
    title: "settingsAdmin.testCenter.scenarioRepositoryCoreTitle",
    description: "settingsAdmin.testCenter.scenarioRepositoryCoreDescription",
  },
  "repository_test_browser-functional": {
    title: "settingsAdmin.testCenter.scenarioRepositoryBrowserTitle",
    description: "settingsAdmin.testCenter.scenarioRepositoryBrowserDescription",
  },
  "repository_test_authoring-workflows": {
    title: "settingsAdmin.testCenter.scenarioRepositoryAuthoringTitle",
    description: "settingsAdmin.testCenter.scenarioRepositoryAuthoringDescription",
  },
  "repository_test_desktop-workflows": {
    title: "settingsAdmin.testCenter.scenarioRepositoryDesktopTitle",
    description: "settingsAdmin.testCenter.scenarioRepositoryDesktopDescription",
  },
  "repository_test_all-functional": {
    title: "settingsAdmin.testCenter.scenarioRepositoryAllTitle",
    description: "settingsAdmin.testCenter.scenarioRepositoryAllDescription",
  },
};

const CATEGORY_COPY_KEYS: Record<string, MessageKey> = {
  system: "settingsAdmin.testCenter.categorySystem",
  project: "settingsAdmin.testCenter.categoryProject",
  "核心功能": "settingsAdmin.testCenter.categoryCoreFunctional",
  "浏览器功能": "settingsAdmin.testCenter.categoryBrowserFunctional",
  "完整流程": "settingsAdmin.testCenter.categoryAuthoringWorkflows",
  "桌面功能": "settingsAdmin.testCenter.categoryDesktopWorkflows",
  "全部功能": "settingsAdmin.testCenter.categoryAllFunctional",
};

const RESOURCE_COPY_KEYS: Record<string, MessageKey> = {
  project: "settingsAdmin.testCenter.resourceProject",
};

const numberParameter = (value: string): number | null => {
  const parsed = Number(value);
  return Number.isInteger(parsed) && parsed > 0 ? parsed : null;
};

export const TestCenterPage: React.FC = () => {
  const { user } = useAuthStore();
  const { t, locale } = useI18n();
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();
  const selectedRunId = searchParams.get("runId") ?? "";
  const [scenarios, setScenarios] = useState<TestScenarioDescriptor[]>([]);
  const [runs, setRuns] = useState<TestRunSummary[]>([]);
  const [projects, setProjects] = useState<AdminProjectSummary[]>([]);
  const [selectedScenarioId, setSelectedScenarioId] = useState("");
  const [detail, setDetail] = useState<TestRunDetail | null>(null);
  const [category, setCategory] = useState("all");
  const [targetMode, setTargetMode] = useState<TargetMode>("test_project");
  const [sourceProjectId, setSourceProjectId] = useState("");
  const [currentProjectId, setCurrentProjectId] = useState(
    searchParams.get("projectId") ?? "",
  );
  const [projectTitle, setProjectTitle] = useState("");
  const [projectDescription, setProjectDescription] = useState("");
  const [loading, setLoading] = useState(true);
  const [projectLoading, setProjectLoading] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<{ scope: ErrorScope; key?: string; message: string } | null>(null);
  const activeRunId = runs.find((run) => isActive(run.status))?.id ?? null;
  const syncedRunId = useRef("");
  const runRequestsInFlight = useRef(new Set<string>());
  const projectLoadRequestId = useRef(0);
  const refreshedProjectIds = useRef(new Set<number>());
  const mutationInFlight = useRef(false);
  const selectedRunIdRef = useRef(selectedRunId);
  selectedRunIdRef.current = selectedRunId;

  const updateError = useCallback((scope: ErrorScope, message: string | null, key?: string) => {
    setError((current) => message
      ? { scope, key, message }
      : current?.scope === scope && (key === undefined || current.key === key) ? null : current);
  }, []);

  const selectRun = useCallback((runId: string) => {
    setSearchParams((current) => {
      if ((current.get("runId") ?? "") === runId) return current;
      if (runId) current.set("runId", runId);
      else current.delete("runId");
      return current;
    }, { replace: true });
  }, [setSearchParams]);

  const selectedScenario = scenarios.find((item) => item.id === selectedScenarioId) ?? null;
  const categories = useMemo(
    () => Array.from(new Set(scenarios.map((item) => item.category))),
    [scenarios],
  );
  const visibleScenarios = category === "all"
    ? scenarios
    : scenarios.filter((item) => item.category === category);
  const scenarioCopy = (scenario: TestScenarioDescriptor) => {
    const keys = SCENARIO_COPY_KEYS[scenario.id];
    return keys
      ? { title: t(keys.title), description: t(keys.description) }
      : { title: scenario.title, description: scenario.description };
  };
  const categoryCopy = (value: string) => {
    const key = CATEGORY_COPY_KEYS[value];
    return key ? t(key) : value;
  };

  const selectScenario = (scenarioId: string) => {
    const scenario = scenarios.find((item) => item.id === scenarioId);
    setSelectedScenarioId(scenarioId);
    if (scenario && !scenario.target_modes.includes(targetMode) && scenario.target_modes[0]) {
      setTargetMode(scenario.target_modes[0]);
    }
  };

  const load = useCallback(async () => {
    setLoading(true);
    updateError("load", null);
    try {
      const [nextScenarios, nextRuns] = await Promise.all([
        listTestScenarios(),
        listTestRuns(30),
      ]);
      const selectedRunSummary = nextRuns.find((run) => run.id === selectedRunIdRef.current);
      setScenarios(nextScenarios);
      setRuns((current) => mergeRunList(current, nextRuns, selectedRunIdRef.current));
      const activeRun = nextRuns.find((run) => isActive(run.status));
      if (activeRun && !selectedRunIdRef.current) selectRun(activeRun.id);
      setSelectedScenarioId((current) =>
        selectedRunIdRef.current
          ? selectedRunSummary && nextScenarios.some((item) => item.id === selectedRunSummary.scenario_id)
            ? selectedRunSummary.scenario_id
            : ""
          : current && nextScenarios.some((item) => item.id === current)
            ? current
            : nextScenarios[0]?.id ?? "",
      );
    } catch (cause) {
      updateError("load", extractErrorMessage(cause, t("settingsAdmin.testCenter.loadFailed")));
    } finally {
      setLoading(false);
    }
  }, [selectRun, t, updateError]);

  const loadProjects = useCallback(async (): Promise<AdminProjectSummary[] | null> => {
    const requestId = projectLoadRequestId.current + 1;
    projectLoadRequestId.current = requestId;
    setProjectLoading(true);
    updateError("projects", null);
    try {
      const nextProjects = await listAllProjects();
      if (projectLoadRequestId.current !== requestId) return null;
      setProjects(nextProjects);
      setCurrentProjectId((current) =>
        current && nextProjects.some((project) => String(project.id) === current) ? current : "",
      );
      setSourceProjectId((current) =>
        current && nextProjects.some((project) => String(project.id) === current) ? current : "",
      );
      return nextProjects;
    } catch (cause) {
      if (projectLoadRequestId.current === requestId) {
        updateError("projects", extractErrorMessage(cause, t("settingsAdmin.testCenter.projectLoadFailed")));
      }
      return null;
    } finally {
      if (projectLoadRequestId.current === requestId) setProjectLoading(false);
    }
  }, [t, updateError]);

  useEffect(() => {
    void load();
    void loadProjects();
  }, [load, loadProjects]);

  useEffect(() => {
    if (!selectedScenario || selectedScenario.target_modes.includes(targetMode)) return;
    const fallback = selectedScenario.target_modes[0];
    if (fallback) setTargetMode(fallback);
  }, [selectedScenario, targetMode]);

  const loadRun = useCallback(async (runId: string) => {
    if (!runId || runRequestsInFlight.current.has(runId)) return;
    runRequestsInFlight.current.add(runId);
    if (selectedRunIdRef.current === runId) updateError("run", null, runId);
    try {
      const next = await getTestRun(runId);
      if (selectedRunIdRef.current === runId) setDetail(next);
      setRuns((current) => upsertRunSummary(current, next.summary));
      if (selectedRunIdRef.current === runId || activeRunId === runId) {
        updateError("run", null, runId);
      }
      const createdProjectId = next.summary.target_project_id;
      if (
        createdProjectId != null
        && runCreatedProject(next.summary)
        && !projects.some((project) => project.id === createdProjectId)
        && !refreshedProjectIds.current.has(createdProjectId)
      ) {
        refreshedProjectIds.current.add(createdProjectId);
        void loadProjects();
      }
    } catch (cause) {
      if (selectedRunIdRef.current === runId || activeRunId === runId) {
        updateError("run", extractErrorMessage(cause, t("settingsAdmin.testCenter.loadFailed")), runId);
      }
    } finally {
      runRequestsInFlight.current.delete(runId);
    }
  }, [activeRunId, loadProjects, projects, t, updateError]);

  useEffect(() => {
    if (!selectedRunId) {
      setDetail(null);
      return;
    }
    void loadRun(selectedRunId);
  }, [loadRun, selectedRunId]);

  useEffect(() => {
    if (!selectedRunId) {
      syncedRunId.current = "";
      return;
    }
    if (
      !detail
      || detail.summary.id !== selectedRunId
      || syncedRunId.current === selectedRunId
      || !scenarios.some((scenario) => scenario.id === detail.summary.scenario_id)
    ) return;
    syncedRunId.current = selectedRunId;
    setSelectedScenarioId(detail.summary.scenario_id);
  }, [detail, scenarios, selectedRunId]);

  useEffect(() => {
    if (!activeRunId) return undefined;
    const timer = window.setInterval(() => void loadRun(activeRunId), 800);
    return () => window.clearInterval(timer);
  }, [activeRunId, loadRun]);

  const selectedRunDetail = detail?.summary.id === selectedRunId ? detail : null;
  const selectedRunError = Boolean(
    selectedRunId && !selectedRunDetail && error?.scope === "run" && error.key === selectedRunId,
  );
  const canCancelSelectedRun = Boolean(
    selectedRunDetail
    && selectedRunDetail.summary.id === activeRunId
    && selectedRunDetail.summary.scenario_id === selectedScenario?.id
    && isActive(selectedRunDetail.summary.status),
  );

  const showActiveRun = () => {
    if (!activeRunId) return;
    const activeRun = runs.find((run) => run.id === activeRunId);
    const scenario = scenarios.find((item) => item.id === activeRun?.scenario_id);
    if (scenario) {
      setCategory(scenario.category);
      selectScenario(scenario.id);
    }
    selectRun(activeRunId);
  };

  const runScenario = async () => {
    if (!selectedScenario || busy || mutationInFlight.current) return;
    if (!selectedScenario.target_modes.includes(targetMode)) {
      updateError("run", t("settingsAdmin.testCenter.targetModeUnsupported"), "start");
      return;
    }
    let target: TestTarget;
    if (targetMode === "repository") {
      target = { mode: "repository" };
    } else if (targetMode === "current_project") {
      const projectId = numberParameter(currentProjectId);
      if (projectId == null || !projects.some((project) => project.id === projectId)) {
        updateError("run", t("settingsAdmin.testCenter.currentProjectRequired"), "start");
        return;
      }
      target = { mode: "current_project", project_id: projectId };
    } else {
      const sourceProjectIdValue = numberParameter(sourceProjectId);
      if (sourceProjectId && (
        sourceProjectIdValue == null
        || !projects.some((project) => project.id === sourceProjectIdValue)
      )) {
        updateError("run", t("settingsAdmin.testCenter.sourceProjectRequired"), "start");
        return;
      }
      target = {
        mode: "test_project",
        source_project_id: sourceProjectIdValue,
      };
    }
    const mutatesProject = selectedScenario.effect !== "read";
    mutationInFlight.current = true;
    try {
      if (targetMode === "current_project" && mutatesProject) {
        const confirmed = await confirmDialog({
          title: t("settingsAdmin.testCenter.impactTitle"),
          message: `${t("settingsAdmin.testCenter.impactMessage")}\n\n${scenarioCopy(selectedScenario).description}`,
          confirmText: t("settingsAdmin.testCenter.confirmRun"),
          dialogType: selectedScenario.effect === "destructive" ? "danger" : "warning",
        });
        if (!confirmed) return;
      }

      let parameters: Record<string, string | number> = {};
      if (selectedScenario.id === "project_create_update") {
        parameters = {
          ...(projectTitle.trim() ? { title: projectTitle.trim() } : {}),
          ...(projectDescription.trim() ? { description: projectDescription.trim() } : {}),
        };
      }

      setBusy(true);
      updateError("run", null, "start");
      const run = await startTestRun({
        scenario_id: selectedScenario.id,
        target,
        parameters,
        confirm_impact: targetMode === "current_project" && mutatesProject,
      });
      selectRun(run.id);
      setRuns((current) => upsertRunSummary(current, run));
    } catch (cause) {
      updateError("run", extractErrorMessage(cause, t("settingsAdmin.testCenter.runFailed")), "start");
    } finally {
      mutationInFlight.current = false;
      setBusy(false);
    }
  };

  const cancelRun = async (runId: string) => {
    if (
      !selectedRunDetail
      || selectedRunDetail.summary.id !== runId
      || selectedRunDetail.summary.scenario_id !== selectedScenario?.id
      || activeRunId !== runId
      || !isActive(selectedRunDetail.summary.status)
      || mutationInFlight.current
    ) return;
    mutationInFlight.current = true;
    setBusy(true);
    updateError("run", null, runId);
    try {
      await cancelTestRun(runId);
      await loadRun(runId);
    } catch (cause) {
      updateError("run", extractErrorMessage(cause, t("settingsAdmin.testCenter.runFailed")), runId);
    } finally {
      mutationInFlight.current = false;
      setBusy(false);
    }
  };

  const removeRun = async (run: TestRunSummary) => {
    if (busy || mutationInFlight.current) return;
    mutationInFlight.current = true;
    try {
      const removeProject = runCreatedProject(run)
        ? await confirmDialog({
          title: t("settingsAdmin.testCenter.deleteRunTitle"),
          message: t("settingsAdmin.testCenter.deleteRunMessage"),
          confirmText: t("settingsAdmin.testCenter.deleteRunAndProject"),
          cancelText: t("settingsAdmin.testCenter.keepTestProject"),
          dialogType: "warning",
        })
        : false;
      if (!removeProject && !(await confirmDialog({
        title: t("settingsAdmin.testCenter.deleteRunTitle"),
        message: t("settingsAdmin.testCenter.deleteRunMessage"),
        confirmText: t("settingsAdmin.testCenter.deleteRun"),
        dialogType: "warning",
      }))) return;

      setBusy(true);
      updateError("delete", null, run.id);
      await deleteTestRun({ run_id: run.id, delete_test_project: removeProject });
      setRuns((current) => current.filter((item) => item.id !== run.id));
      if (selectedRunId === run.id) selectRun("");
      if (removeProject) {
        run.impact.forEach((impact) => {
          if (impact.action === "create" && impact.resource === "project" && impact.project_id != null) {
            refreshedProjectIds.current.delete(impact.project_id);
          }
        });
        await loadProjects();
      }
      updateError("delete", null, run.id);
    } catch (cause) {
      updateError("delete", extractErrorMessage(cause, t("settingsAdmin.testCenter.deleteFailed")), run.id);
      setRuns((current) => current.filter((item) => item.id !== run.id));
      if (selectedRunIdRef.current === run.id) {
        setDetail(null);
        selectRun("");
      }
      void load();
    } finally {
      mutationInFlight.current = false;
      setBusy(false);
    }
  };

  const statusLabel = (status: TestRunStatus) => t(STATUS_KEYS[status]);
  const formatDuration = (milliseconds: number | null) =>
    milliseconds == null ? "" : t("settingsAdmin.testCenter.duration", {
      seconds: Math.max(0.1, Math.round(milliseconds / 100) / 10),
    });
  const openProject = (projectId: number) => {
    const projectKey = String(projectId);
    updateError("project", null, projectKey);
    void (async () => {
      let project = projects.find((item) => item.id === projectId);
      if (!project) {
        const latestProjects = await loadProjects();
        if (!latestProjects) return;
        project = latestProjects?.find((item) => item.id === projectId);
      }
      if (!project) {
        updateError("project", t("settingsAdmin.testCenter.projectUnavailable"), projectKey);
        return;
      }
      navigate(`/${PROJECT_ROUTES[project.project_kind]}/${projectId}`);
    })().catch((cause) => {
      updateError("project", extractErrorMessage(cause, t("settingsAdmin.testCenter.projectLoadFailed")), projectKey);
    });
  };

  if (!user || user.role !== "admin") {
    return (
      <main className="flex min-h-screen items-center justify-center bg-book-bg p-6">
        <div className="max-w-md border-l-2 border-warning py-2 pl-4">
          <h1 className="text-sm font-semibold text-book-text-main">
            {t("settingsAdmin.permissionRequired")}
          </h1>
          <p className="mt-2 text-xs text-book-text-muted">{t("settingsAdmin.adminOnly")}</p>
        </div>
      </main>
    );
  }

  if (loading) {
    return (
      <main className="flex min-h-screen items-center justify-center bg-book-bg p-6">
        <LoadingSpinner text={t("settingsAdmin.testCenter.loading")} />
      </main>
    );
  }

  return (
    <main className="flex min-h-0 flex-1 flex-col overflow-hidden bg-book-bg text-book-text-main">
      <header className="flex flex-wrap items-center gap-3 border-b border-book-border/70 px-4 py-3 md:px-6">
        <BookIconButton
          label={t("nav.backToPrevious")}
          onClick={() => navigate("/settings#admin")}
        >
          <ArrowLeft size={15} />
        </BookIconButton>
        <div className="flex min-w-0 flex-1 items-center gap-3">
          <span className="flex size-9 shrink-0 items-center justify-center border border-book-border/70 bg-book-bg-paper text-book-primary">
            <FlaskConical size={17} />
          </span>
          <div className="min-w-0">
            <h1 className="truncate text-base font-semibold">{t("settingsAdmin.testCenter.title")}</h1>
            <p className="text-xs text-book-text-muted">{t("settingsAdmin.testCenter.subtitle")}</p>
          </div>
        </div>
        <BookIconButton
          label={t("settingsAdmin.testCenter.refresh")}
          onClick={() => {
            void load();
            void loadProjects();
            if (selectedRunId) void loadRun(selectedRunId);
          }}
          disabled={loading || busy}
        >
          <RefreshCw size={15} />
        </BookIconButton>
      </header>

      {error ? (
        <div role="alert" className="flex items-start gap-2 border-b border-error/30 bg-error/5 px-4 py-2 text-xs text-error md:px-6">
          <AlertTriangle size={14} className="mt-0.5 shrink-0" />
          <span className="min-w-0 break-words">{error.message}</span>
        </div>
      ) : null}

      <div className="grid min-h-0 flex-1 grid-cols-1 overflow-y-auto xl:grid-cols-[260px_minmax(0,1fr)_300px] xl:overflow-hidden">
        <aside className="border-b border-book-border/70 p-3 xl:overflow-y-auto xl:border-b-0 xl:border-r xl:p-4">
          <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
            <h2 className="text-xs font-semibold uppercase text-book-text-muted">
              {t("settingsAdmin.testCenter.scenarios")}
            </h2>
            <select
              aria-label={t("settingsAdmin.testCenter.category")}
              value={category}
              onChange={(event) => {
                const nextCategory = event.target.value;
                const nextVisible = nextCategory === "all"
                  ? scenarios
                  : scenarios.filter((scenario) => scenario.category === nextCategory);
                setCategory(nextCategory);
                if (!nextVisible.some((scenario) => scenario.id === selectedScenarioId)) {
                  const nextScenario = nextVisible[0];
                  if (nextScenario) selectScenario(nextScenario.id);
                  else setSelectedScenarioId("");
                }
              }}
              className="max-w-full border border-book-border/70 bg-book-bg-paper px-2 py-1.5 text-xs"
            >
              <option value="all">{t("settingsAdmin.testCenter.allCategories")}</option>
              {categories.map((item) => <option key={item} value={item}>{categoryCopy(item)}</option>)}
            </select>
          </div>
          <div className="flex flex-col gap-1.5">
            {visibleScenarios.map((scenario) => (
              <button
                key={scenario.id}
                type="button"
                aria-pressed={selectedScenarioId === scenario.id}
                onClick={() => selectScenario(scenario.id)}
                className={`min-w-0 border-l-2 px-3 py-2.5 text-left transition-colors ${
                  selectedScenarioId === scenario.id
                    ? "border-book-primary bg-book-bg-paper"
                    : "border-transparent hover:bg-book-bg-paper/70"
                }`}
              >
                <span className="block break-words text-xs font-medium">{scenarioCopy(scenario).title}</span>
                <span className="mt-1 flex flex-wrap items-center gap-2 text-caption text-book-text-muted">
                  <span>{categoryCopy(scenario.category)}</span>
                  <span aria-hidden="true">·</span>
                  <span>{t(`settingsAdmin.testCenter.effect${scenario.effect[0].toUpperCase()}${scenario.effect.slice(1)}` as MessageKey)}</span>
                </span>
              </button>
            ))}
            {visibleScenarios.length === 0 ? (
              <p className="py-5 text-center text-xs text-book-text-muted">
                {t("settingsAdmin.testCenter.noScenarios")}
              </p>
            ) : null}
          </div>
        </aside>

        <section className="min-w-0 p-4 md:p-6 xl:overflow-y-auto">
          {selectedRunId && !selectedRunDetail ? (
            <div
              role="status"
              aria-busy={!selectedRunError}
              className="flex min-h-64 items-center justify-center text-xs text-book-text-muted"
            >
              {selectedRunError
                ? t("settingsAdmin.testCenter.runDetailUnavailable")
                : <LoadingSpinner text={t("settingsAdmin.testCenter.loadingRun")} />}
            </div>
          ) : selectedScenario ? (
            <>
              <div className="flex flex-wrap items-start justify-between gap-3 border-b border-book-border/60 pb-4">
                <div className="min-w-0">
                  <div className="mb-1 flex flex-wrap items-center gap-2 text-caption text-book-text-muted">
                    <span>{categoryCopy(selectedScenario.category)}</span>
                    <span aria-hidden="true">·</span>
                    <span>{t(`settingsAdmin.testCenter.effect${selectedScenario.effect[0].toUpperCase()}${selectedScenario.effect.slice(1)}` as MessageKey)}</span>
                    <span>{t("settingsAdmin.testCenter.scenarioVersion", { version: selectedScenario.version })}</span>
                  </div>
                  <h2 className="break-words text-lg font-semibold">{scenarioCopy(selectedScenario).title}</h2>
                  <p className="mt-1 max-w-3xl text-xs leading-5 text-book-text-sub">{scenarioCopy(selectedScenario).description}</p>
                </div>
                <div className="flex shrink-0 items-center gap-2">
                  {canCancelSelectedRun && selectedRunDetail ? (
                    <BookButton
                      variant="warning"
                      size="sm"
                      disabled={busy}
                      onClick={() => void cancelRun(selectedRunDetail.summary.id)}
                    >
                      <Square size={13} className="mr-1.5 inline" />{t("settingsAdmin.testCenter.cancelRun")}
                    </BookButton>
                  ) : activeRunId ? (
                    <BookButton size="sm" variant="secondary" disabled={busy} onClick={showActiveRun}>
                      <ExternalLink size={13} className="mr-1.5 inline" />
                      {t("settingsAdmin.testCenter.viewActiveRun")}
                    </BookButton>
                  ) : (
                    <BookButton
                      size="sm"
                      disabled={busy
                        || !selectedScenario
                        || (!!selectedRunId && !selectedRunDetail)
                        || (projectLoading && targetMode === "current_project")}
                      onClick={() => void runScenario()}
                    >
                      {busy ? <LoadingSpinner size="sm" /> : <Play size={13} className="mr-1.5 inline" />}
                      {selectedRunDetail?.summary.scenario_id === selectedScenario.id
                        ? t("settingsAdmin.testCenter.rerun")
                        : t("settingsAdmin.testCenter.run")}
                    </BookButton>
                  )}
                </div>
              </div>

              <section className="border-b border-book-border/60 py-4">
                <h3 className="mb-3 text-xs font-semibold">{t("settingsAdmin.testCenter.target")}</h3>
                <div className="flex flex-wrap gap-2" role="group" aria-label={t("settingsAdmin.testCenter.target")}>
                  {selectedScenario.target_modes.includes("test_project") ? (
                    <button
                      type="button"
                      aria-pressed={targetMode === "test_project"}
                      onClick={() => setTargetMode("test_project")}
                      className={`min-h-9 border px-3 text-xs ${targetMode === "test_project" ? "border-book-primary bg-book-bg-paper text-book-primary" : "border-book-border/70 text-book-text-sub"}`}
                    >
                      {t("settingsAdmin.testCenter.testProject")}
                    </button>
                  ) : null}
                  {selectedScenario.target_modes.includes("current_project") ? (
                    <button
                      type="button"
                      aria-pressed={targetMode === "current_project"}
                      onClick={() => setTargetMode("current_project")}
                      className={`min-h-9 border px-3 text-xs ${targetMode === "current_project" ? "border-book-primary bg-book-bg-paper text-book-primary" : "border-book-border/70 text-book-text-sub"}`}
                    >
                      {t("settingsAdmin.testCenter.currentProject")}
                    </button>
                  ) : null}
                  {selectedScenario.target_modes.includes("repository") ? (
                    <button
                      type="button"
                      aria-pressed={targetMode === "repository"}
                      onClick={() => setTargetMode("repository")}
                      className={`min-h-9 border px-3 text-xs ${targetMode === "repository" ? "border-book-primary bg-book-bg-paper text-book-primary" : "border-book-border/70 text-book-text-sub"}`}
                    >
                      {t("settingsAdmin.testCenter.repositoryTests")}
                    </button>
                  ) : null}
                </div>
                <div className="mt-3 max-w-xl">
                  {targetMode === "repository" ? (
                    <p className="text-xs text-book-text-muted">
                      {t("settingsAdmin.testCenter.repositoryTestsHint")}
                    </p>
                  ) : null}
                  {targetMode !== "repository" && (targetMode === "current_project" || selectedScenario.id === "project_duplicate") ? (
                    <label className="block text-xs text-book-text-sub">
                      <span className="mb-1.5 block">
                        {targetMode === "current_project"
                          ? t("settingsAdmin.testCenter.selectProject")
                          : t("settingsAdmin.testCenter.sourceProject")}
                      </span>
                      <select
                        value={targetMode === "current_project" ? currentProjectId : sourceProjectId}
                        onChange={(event) => targetMode === "current_project"
                          ? setCurrentProjectId(event.target.value)
                          : setSourceProjectId(event.target.value)}
                        disabled={projectLoading}
                        className="min-h-9 w-full border border-book-border/70 bg-book-bg-paper px-2 text-xs text-book-text-main"
                      >
                        <option value="">
                          {projectLoading
                            ? t("settingsAdmin.testCenter.projectLoading")
                            : t("settingsAdmin.testCenter.selectProject")}
                        </option>
                        {projects.map((project) => (
                          <option key={project.id} value={String(project.id)}>
                            {project.title} · #{project.id}
                          </option>
                        ))}
                      </select>
                    </label>
                  ) : null}
                  {targetMode === "test_project" && selectedScenario.id !== "project_duplicate" ? (
                    <p className="text-xs text-book-text-muted">{t("settingsAdmin.testCenter.testProjectHint")}</p>
                  ) : null}
                  {targetMode === "current_project" && selectedScenario.effect !== "read" ? (
                    <p className="mt-2 flex items-start gap-1.5 text-xs text-warning">
                      <AlertTriangle size={13} className="mt-0.5 shrink-0" />
                      <span>{t("settingsAdmin.testCenter.currentProjectMutationHint")}</span>
                    </p>
                  ) : null}
                </div>
              </section>

              {selectedScenario.id === "project_create_update" ? (
                <section className="grid max-w-3xl gap-3 border-b border-book-border/60 py-4 md:grid-cols-2">
                  <label className="text-xs text-book-text-sub">
                    <span className="mb-1.5 block">{t("settingsAdmin.testCenter.titleLabel")}</span>
                    <input
                      value={projectTitle}
                      onChange={(event) => setProjectTitle(event.target.value)}
                      maxLength={120}
                      className="min-h-9 w-full border border-book-border/70 bg-book-bg-tertiary/45 px-2.5 text-xs text-book-text-main"
                    />
                  </label>
                  <label className="text-xs text-book-text-sub">
                    <span className="mb-1.5 block">{t("settingsAdmin.testCenter.descriptionLabel")}</span>
                    <input
                      value={projectDescription}
                      onChange={(event) => setProjectDescription(event.target.value)}
                      maxLength={2000}
                      className="min-h-9 w-full border border-book-border/70 bg-book-bg-tertiary/45 px-2.5 text-xs text-book-text-main"
                    />
                  </label>
                </section>
              ) : null}

              <RunResult
                detail={selectedRunDetail?.summary.scenario_id === selectedScenario.id ? selectedRunDetail : null}
                statusLabel={statusLabel}
                formatDuration={formatDuration}
                locale={locale}
                onOpenProject={openProject}
                onDelete={() => selectedRunDetail && void removeRun(selectedRunDetail.summary)}
                busy={busy}
                t={t}
              />
            </>
          ) : (
            <div className="flex min-h-64 items-center justify-center text-xs text-book-text-muted">
              {t("settingsAdmin.testCenter.noSelection")}
            </div>
          )}
        </section>

        <aside className="border-t border-book-border/70 p-3 xl:overflow-y-auto xl:border-l xl:border-t-0 xl:p-4">
          <div className="mb-3 flex items-center justify-between gap-2">
            <h2 className="text-xs font-semibold uppercase text-book-text-muted">
              {t("settingsAdmin.testCenter.history")}
            </h2>
            <span className="tabular-nums text-caption text-book-text-muted">{runs.length}</span>
          </div>
          {runs.length === 0 ? (
            <p className="border-l-2 border-book-border/70 py-1 pl-3 text-xs text-book-text-muted">
              {t("settingsAdmin.testCenter.noHistory")}
            </p>
          ) : (
            <ul className="flex flex-col gap-1.5">
              {runs.map((run) => (
                <li key={run.id} className="flex min-w-0 items-stretch border-b border-book-border/45">
                  <button
                    type="button"
                    aria-current={selectedRunId === run.id ? "true" : undefined}
                    onClick={() => {
                      const scenario = scenarios.find((item) => item.id === run.scenario_id);
                      if (scenario) {
                        setCategory(scenario.category);
                        selectScenario(scenario.id);
                      }
                      selectRun(run.id);
                    }}
                    className={`min-w-0 flex-1 px-2 py-2 text-left hover:bg-book-bg-paper ${selectedRunId === run.id ? "bg-book-bg-paper" : ""}`}
                  >
                    <span className="block truncate text-xs font-medium">
                      {(() => {
                        const scenario = scenarios.find((item) => item.id === run.scenario_id);
                        return scenario ? scenarioCopy(scenario).title : run.scenario_id;
                      })()}
                    </span>
                    <span className="mt-1 flex flex-wrap items-center gap-x-2 gap-y-1 text-caption text-book-text-muted">
                      <span className={STATUS_STYLES[run.status]}>{statusLabel(run.status)}</span>
                      <span>{formatDateTime(run.created_at, locale)}</span>
                    </span>
                  </button>
                  <BookIconButton
                    label={t("settingsAdmin.testCenter.deleteRun")}
                    size="sm"
                    disabled={busy || isActive(run.status)}
                    onClick={() => void removeRun(run)}
                  >
                    <Trash2 size={13} />
                  </BookIconButton>
                </li>
              ))}
            </ul>
          )}
        </aside>
      </div>
    </main>
  );
};

function RunResult({
  detail,
  statusLabel,
  formatDuration,
  locale,
  onOpenProject,
  onDelete,
  busy,
  t,
}: {
  detail: TestRunDetail | null;
  statusLabel: (status: TestRunStatus) => string;
  formatDuration: (duration: number | null) => string;
  locale: Parameters<typeof formatDateTime>[1];
  onOpenProject: (projectId: number) => void;
  onDelete: () => void;
  busy: boolean;
  t: ReturnType<typeof useI18n>["t"];
}) {
  if (!detail) {
    return (
      <div className="flex min-h-40 items-center gap-2 py-5 text-xs text-book-text-muted">
        <CircleHelp size={14} />{t("settingsAdmin.testCenter.noSteps")}
      </div>
    );
  }
  const { summary } = detail;
  return (
    <section className="pt-4">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-2 text-xs">
            <span className={`font-semibold ${STATUS_STYLES[summary.status]}`}>
              {statusLabel(summary.status)}
            </span>
            {formatDuration(summary.duration_ms) ? (
              <span className="inline-flex items-center gap-1 text-book-text-muted">
                <Clock3 size={12} />{formatDuration(summary.duration_ms)}
              </span>
            ) : null}
            {summary.started_at ? (
              <span className="text-book-text-muted">
                {t("settingsAdmin.testCenter.createdAt", {
                  date: formatDateTime(summary.started_at, locale),
                })}
              </span>
            ) : null}
          </div>
          {summary.message ? <p className="mt-2 break-words text-xs text-book-text-sub">{summary.message}</p> : null}
          <p className="mt-1 font-mono text-caption text-book-text-muted">{summary.id}</p>
        </div>
        <div className="flex flex-wrap gap-2">
          {summary.target_project_id != null ? (
            <BookButton size="sm" variant="secondary" onClick={() => onOpenProject(summary.target_project_id!)}>
              <ExternalLink size={13} className="mr-1.5 inline" />{t("settingsAdmin.testCenter.openProject")}
            </BookButton>
          ) : null}
          {!isActive(summary.status) ? (
            <BookIconButton
              label={t("settingsAdmin.testCenter.deleteRun")}
              size="sm"
              disabled={busy}
              onClick={onDelete}
            >
              <Trash2 size={13} />
            </BookIconButton>
          ) : null}
        </div>
      </div>

      <div className="mt-5 grid min-w-0 gap-5 2xl:grid-cols-[minmax(0,1fr)_minmax(240px,0.7fr)]">
        <div className="min-w-0">
          <h3 className="mb-2 text-xs font-semibold">{t("settingsAdmin.testCenter.steps")}</h3>
          {detail.steps.length === 0 ? (
            <div className="flex min-h-12 items-center gap-2 text-xs text-book-text-muted">
              {isActive(summary.status) ? <LoadingSpinner size="sm" /> : null}
              {t("settingsAdmin.testCenter.noSteps")}
            </div>
          ) : (
            <ol className="border-l border-book-border/70 pl-3">
              {detail.steps.map((step) => (
                <li key={step.index} className="relative pb-3 last:pb-0">
                  <span className="absolute -left-[17px] top-1 flex size-4 items-center justify-center bg-book-bg">
                    {step.status === "passed"
                      ? <Check size={13} className="text-success" />
                      : isActive(step.status)
                        ? <LoadingSpinner size="sm" />
                        : <AlertTriangle size={12} className="text-error" />}
                  </span>
                  <p className="text-xs font-medium">{step.name}</p>
                  {step.message ? <p className="mt-1 break-words text-caption text-book-text-muted">{step.message}</p> : null}
                </li>
              ))}
            </ol>
          )}

          <h3 className="mb-2 mt-5 text-xs font-semibold">{t("settingsAdmin.testCenter.logs")}</h3>
          {detail.logs.length === 0 ? (
            <p className="text-xs text-book-text-muted">{t("settingsAdmin.testCenter.noLogs")}</p>
          ) : (
            <pre className="max-h-64 overflow-auto border border-book-border/60 bg-book-bg-tertiary/40 p-3 text-caption leading-5 text-book-text-sub">
              {detail.logs.join("\n")}
            </pre>
          )}
        </div>

        <div className="min-w-0">
          <h3 className="mb-2 text-xs font-semibold">{t("settingsAdmin.testCenter.impact")}</h3>
          {detail.summary.impact.length === 0 ? (
            <p className="text-xs text-book-text-muted">{t("settingsAdmin.testCenter.noData")}</p>
          ) : (
            <ul className="divide-y divide-book-border/50 border-y border-book-border/50">
              {detail.summary.impact.map((impact, index) => (
                <li key={`${impact.action}-${impact.project_id}-${index}`} className="py-2">
                  <p className="flex flex-wrap items-center gap-x-2 text-xs">
                    <span className="font-medium">{t(`settingsAdmin.testCenter.effect${impact.action[0].toUpperCase()}${impact.action.slice(1)}` as MessageKey)}</span>
                    <span className="text-book-text-muted">
                      {RESOURCE_COPY_KEYS[impact.resource]
                        ? t(RESOURCE_COPY_KEYS[impact.resource])
                        : impact.resource}
                    </span>
                    {impact.project_id != null ? <span className="font-mono text-caption">#{impact.project_id}</span> : null}
                  </p>
                  <p className="mt-1 break-words text-caption text-book-text-muted">{impact.detail}</p>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>
    </section>
  );
}
