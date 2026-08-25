import React from "react";
import { BookOpen, ChevronRight, Gamepad2, Image, Video } from "lucide-react";
import { Dropdown } from "../../components/ui/Dropdown";
import { useContextMenu, type ContextMenuEntry } from "../../components/ui/ContextMenu";
import type { NovelProject } from "../../api/project";
import {
  getProjectLifecycleBlockedReason,
  getProjectProgressText,
} from "../../utils/projectStatus";
import {
  getProjectHomeEntryRoute,
  getProjectWorkspaceRoute,
} from "../../utils/projectRouting";
import { prefetchProjectRouteByStatus } from "../../utils/projectRoutePrefetch";
import { type ProjectFamily } from "./novelListUtils";
import { formatDate, useI18n } from "../../i18n";

export type ProjectActionItems = React.ComponentProps<typeof Dropdown>["items"];

interface ProjectListProps {
  families: ProjectFamily[];
  groups: [string, ProjectFamily[]][];
  empty: boolean;
  onOpen: (project: NovelProject) => void;
  onWrite: (project: NovelProject) => void;
  getActions: (project: NovelProject) => ProjectActionItems;
}

export const RecentProjectList: React.FC<
  Pick<ProjectListProps, "families" | "empty" | "onOpen" | "onWrite" | "getActions">
> = ({ families, empty, onOpen, onWrite, getActions }) => {
  const { t } = useI18n();
  if (empty) {
    return (
      <EmptyHint text={`${t("novel.noProjects")}。${t("novel.createProject")}。`} />
    );
  }
  if (families.length === 0) {
    return null;
  }
  return (
    <ul className="a4n-motion-stagger space-y-3">
      {families.map((family) => (
        <ProjectFamilyCard
          key={family.key}
          family={family}
          onOpen={onOpen}
          onWrite={onWrite}
          getActions={getActions}
        />
      ))}
    </ul>
  );
};

export const AllProjectList: React.FC<
  Pick<ProjectListProps, "groups" | "empty" | "onOpen" | "onWrite" | "getActions">
> = ({ groups, empty, onOpen, onWrite, getActions }) => {
  const { t } = useI18n();
  if (empty) {
    return <EmptyHint text={t("novel.noProjects")} />;
  }
  return (
    <div className="a4n-motion-stagger space-y-5">
      {groups.map(([letter, families]) => (
        <div key={letter}>
          <div className="text-xs font-semibold text-book-text-muted uppercase tracking-wide mb-1.5 px-1">
            {letter}
          </div>
          <ul className="space-y-3">
            {families.map((family) => (
              <ProjectFamilyCard
                key={family.key}
                family={family}
                onOpen={onOpen}
                onWrite={onWrite}
                getActions={getActions}
              />
            ))}
          </ul>
        </div>
      ))}
    </div>
  );
};

const ProjectFamilyCard: React.FC<{
  family: ProjectFamily;
  onOpen: (project: NovelProject) => void;
  onWrite: (project: NovelProject) => void;
  getActions: (project: NovelProject) => ProjectActionItems;
}> = ({ family, onOpen, onWrite, getActions }) => {
  const { t } = useI18n();
  const derivedProjects = family.projects.filter(
    (project) => project.id !== family.root?.id,
  );

  return (
    <li
      className="a4n-flat-surface a4n-project-family overflow-hidden"
      data-project-family={family.key}
      data-project-family-kind={derivedProjects.length > 0 ? "root-with-derived" : "standalone"}
    >
      {family.root ? (
        <ul>
          <ProjectRow
            project={family.root}
            relation={derivedProjects.length > 0 ? "root" : "standalone"}
            onOpen={onOpen}
            onWrite={onWrite}
            actions={getActions(family.root)}
          />
        </ul>
      ) : (
        <div className="border-b border-book-border/35 px-3 py-2 text-small text-book-text-muted sm:px-4">
          {t("novel.originatedFrom")} <span className="font-medium text-book-text-sub">{family.title}</span>
        </div>
      )}

      {derivedProjects.length > 0 ? (
        <section
          aria-label={`${family.title} ${t("novel.derivedProjects", { count: derivedProjects.length })}`}
          className={`${family.root ? "border-t " : ""}border-book-border/35 bg-book-bg-tertiary/20`}
        >
          <div className="px-3 pb-1 pt-2 text-caption font-medium text-book-text-muted sm:px-4">
            {t("novel.derivedProjects", { count: derivedProjects.length })}
          </div>
          <ul className="divide-y divide-book-border/35">
            {derivedProjects.map((project) => (
              <ProjectRow
                key={project.id}
                project={project}
                relation="derived"
                onOpen={onOpen}
                onWrite={onWrite}
                actions={getActions(project)}
              />
            ))}
          </ul>
        </section>
      ) : null}
    </li>
  );
};

const ProjectRow: React.FC<{
  project: NovelProject;
  relation?: "standalone" | "root" | "derived";
  onOpen: (project: NovelProject) => void;
  onWrite: (project: NovelProject) => void;
  actions: ProjectActionItems;
}> = ({ project, relation = "standalone", onOpen, onWrite, actions }) => {
  const { locale, t } = useI18n();
  const menu = useContextMenu();
  const ctxItems: ContextMenuEntry[] = actions.map((a) => {
    if ("type" in a && a.type === "divider") return { divider: true };
    const it = a as {
      label: string;
      onClick: () => void;
      danger?: boolean;
      disabled?: boolean;
      icon?: React.ReactNode;
    };
    return {
      label: it.label,
      onClick: it.onClick,
      danger: it.danger,
      disabled: it.disabled,
      icon: it.icon,
    };
  });
  const hasActionItems = actions.some((item) => !("type" in item && item.type === "divider"));
  const kindLabel = {
    text: t("novel.textProject"),
    video: t("novel.videoProject"),
    comic: t("novel.comicProject"),
    galgame: t("novel.galgameProject"),
  }[project.project_kind];
  const ProjectKindIcon = {
    text: BookOpen,
    video: Video,
    comic: Image,
    galgame: Gamepad2,
  }[project.project_kind];
  const roleLabel = {
    owner: t("novel.ownerRole"),
    manager: t("novel.managerRole"),
    editor: t("novel.editorRole"),
    viewer: t("novel.viewerRole"),
  }[project.my_role];
  const entryLabel = project.project_kind === "video"
    ? t("novel.enterVideoProject")
    : project.project_kind === "comic"
      ? t("novel.enterComicProject")
      : project.project_kind === "galgame"
        ? t("novel.enterGalgameProject")
        : project.status === "writing" || project.status === "completed"
          ? t("novel.enterWritingDesk")
          : t("novel.enterProjectDetails");
  const entryRoute = getProjectHomeEntryRoute(project);
  const workspaceLabel = project.project_kind === "video"
    ? t("novel.enterVideoWorkspace")
    : project.project_kind === "comic"
      ? t("novel.enterComicWorkspace")
      : project.project_kind === "galgame"
        ? t("novel.enterGameWorkspace")
        : t("novel.enterWritingDesk");
  const workspaceRoute = getProjectWorkspaceRoute(project);
  const lifecycleBlockedReason = getProjectLifecycleBlockedReason(project, t);
  const lifecycleLabel =
    project.lifecycle_gate === "resetting"
      ? t("novel.resetFinishing")
      : project.lifecycle_gate === "deleting"
        ? t("novel.deleting")
        : "";
  const showWorkspaceEntry = !lifecycleBlockedReason && workspaceRoute !== entryRoute;
  const progressText = getProjectProgressText(project, t);
  const projectMeta = [
    kindLabel,
    lifecycleLabel,
    progressText,
    project.my_role !== "owner" ? roleLabel : "",
  ].filter(Boolean).join(" · ");
  const fullItems: ContextMenuEntry[] = [
    { label: entryLabel, onClick: () => onOpen(project), shortcut: "Enter" },
    ...(showWorkspaceEntry ? [{ label: workspaceLabel, onClick: () => onWrite(project) }] : []),
    ...(hasActionItems ? [{ divider: true as const }, ...ctxItems] : []),
  ];

  return (
    <li className="group">
      <div
        data-project-relation={relation}
        role="button"
        tabIndex={0}
        onClick={() => onOpen(project)}
        onContextMenu={menu.handle(fullItems)}
        onMouseEnter={() => prefetchProjectRouteByStatus(project, { immediate: true })}
        onFocus={() => prefetchProjectRouteByStatus(project, { immediate: true })}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            onOpen(project);
          }
        }}
        className={`a4n-project-row w-full cursor-pointer px-3 py-3 text-left transition-colors hover:bg-book-bg-tertiary/55 focus-visible:bg-book-bg-tertiary/55 focus-visible:outline-none sm:px-4 ${
          relation === "derived" ? "pl-5 sm:pl-6" : ""
        }`}
      >
        <div className="flex items-start gap-3">
          <span
            className="a4n-project-kind mt-0.5"
            data-project-kind={project.project_kind}
            aria-hidden="true"
          >
            <ProjectKindIcon size={17} strokeWidth={1.8} />
          </span>
          <div className="min-w-0 flex-1">
            <h3 className="truncate text-body-lg font-semibold text-book-text-main sm:text-base">
              {project.title}
            </h3>
            <p className="mt-1 truncate text-small text-book-text-muted">
              {project.owner_username} · {formatDate(project.updated_at, locale)}
            </p>
            <p
              className="mt-1 truncate text-small text-book-text-sub"
              title={progressText}
            >
              {projectMeta}
            </p>
          </div>
          <div className="flex shrink-0 items-center gap-2">
            {lifecycleBlockedReason ? (
              <span className="hidden text-body-sm font-semibold text-warning sm:inline">
                {lifecycleLabel}
              </span>
            ) : showWorkspaceEntry ? (
              <button
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  onWrite(project);
                }}
                className="inline-flex min-h-8 cursor-pointer items-center bg-transparent px-1 text-body-sm font-semibold text-book-primary hover:opacity-80"
              >
                <span className="hidden sm:inline">{workspaceLabel}</span>
                <span className="sm:hidden">{t("common.open")}</span>
              </button>
            ) : (
              <span className="text-body-sm font-semibold text-book-primary">
                <span className="hidden sm:inline">{entryLabel}</span>
                <span className="sm:hidden">{t("common.open")}</span>
              </span>
            )}
            {hasActionItems ? (
              <span onClick={(e) => e.stopPropagation()}>
                <Dropdown items={actions} />
              </span>
            ) : null}
            <span className="a4n-project-row__disclosure" aria-hidden="true">
              <ChevronRight size={15} strokeWidth={1.8} />
            </span>
          </div>
        </div>
      </div>
    </li>
  );
};

const EmptyHint: React.FC<{ text: string }> = ({ text }) => (
  <div className="a4n-empty-state a4n-empty-state--framed text-sm">{text}</div>
);
