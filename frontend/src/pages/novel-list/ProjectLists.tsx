import React from "react";
import { Dropdown } from "../../components/ui/Dropdown";
import { useContextMenu, type ContextMenuEntry } from "../../components/ui/ContextMenu";
import type { NovelProject } from "../../api/project";
import { getProjectProgressText } from "../../utils/projectStatus";
import {
  getProjectHomeEntryLabel,
  getProjectHomeEntryRoute,
  getProjectPrimaryLabel,
  getProjectPrimaryRoute,
  getProjectWorkspaceLabel,
  getProjectWorkspaceRoute,
} from "../../utils/projectRouting";
import { prefetchProjectRouteByStatus } from "../../utils/projectRoutePrefetch";
import { formatUiDate } from "../../utils/dateTime";
import {
  PROJECT_KIND_LABELS,
  PROJECT_ROLE_LABELS,
  type ProjectFamily,
} from "./novelListUtils";

export type ProjectActionItems = React.ComponentProps<typeof Dropdown>["items"];

export const TabButton: React.FC<{
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
}> = ({ active, onClick, children }) => (
  <button
    type="button"
    onClick={onClick}
    aria-pressed={active}
    data-active={active ? "1" : undefined}
    className="pyqt-tab"
  >
    {children}
  </button>
);

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
  if (empty) {
    return (
      <EmptyHint text={'还没有项目。点击左侧“创建项目”开始你的第一部作品。'} />
    );
  }
  if (families.length === 0) {
    return null;
  }
  return (
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
  );
};

export const AllProjectList: React.FC<
  Pick<ProjectListProps, "groups" | "empty" | "onOpen" | "onWrite" | "getActions">
> = ({ groups, empty, onOpen, onWrite, getActions }) => {
  if (empty) {
    return <EmptyHint text="还没有项目。" />;
  }
  return (
    <div className="space-y-5">
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
  const derivedProjects = family.projects.filter(
    (project) => project.id !== family.root?.id,
  );

  return (
    <li
      className="a4n-flat-surface overflow-hidden"
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
        <div className="border-b border-book-border/35 px-3 py-2 text-[11px] text-book-text-muted sm:px-4">
          源自 <span className="font-medium text-book-text-sub">{family.title}</span>
        </div>
      )}

      {derivedProjects.length > 0 ? (
        <section
          aria-label={`${family.title}的派生项目`}
          className={`${family.root ? "border-t " : ""}border-book-border/35 bg-book-bg-tertiary/20`}
        >
          <div className="px-3 pb-1 pt-2 text-[10px] font-medium text-book-text-muted sm:px-4">
            派生项目 · {derivedProjects.length}
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
  const menu = useContextMenu();
  const ctxItems: ContextMenuEntry[] = actions.map((a) => {
    if ("type" in a && a.type === "divider") return { divider: true };
    const it = a as { label: string; onClick: () => void; danger?: boolean; icon?: React.ReactNode };
    return {
      label: it.label,
      onClick: it.onClick,
      danger: it.danger,
      icon: it.icon,
    };
  });
  const hasActionItems = actions.some((item) => !("type" in item && item.type === "divider"));
  const entryLabel = getProjectHomeEntryLabel(project);
  const entryRoute = getProjectHomeEntryRoute(project);
  const workspaceLabel = getProjectWorkspaceLabel(project) || getProjectPrimaryLabel(project);
  const workspaceRoute = getProjectWorkspaceRoute(project) || getProjectPrimaryRoute(project);
  const showWorkspaceEntry = workspaceRoute !== entryRoute;
  const progressText = getProjectProgressText(project);
  const progressDetail = project.progress.detail;
  const kindLabel = PROJECT_KIND_LABELS[project.project_kind] ?? "项目";
  const roleLabel = project.my_role ? PROJECT_ROLE_LABELS[project.my_role] : null;
  const projectMeta = [
    kindLabel,
    progressText,
    roleLabel && project.my_role !== "owner" ? roleLabel : "",
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
        className={`w-full cursor-pointer px-3 py-3 text-left transition-colors hover:bg-book-bg-tertiary/55 focus-visible:bg-book-bg-tertiary/55 focus-visible:outline-none sm:px-4 ${
          relation === "derived" ? "pl-5 sm:pl-6" : ""
        }`}
      >
        <div className="flex items-start gap-3">
          <div className="min-w-0 flex-1">
            <h3 className="truncate text-[15px] font-semibold text-book-text-main sm:text-base">
              {project.title}
            </h3>
            <p className="mt-1 truncate text-[11px] text-book-text-muted">
              {project.owner_username ? `${project.owner_username} · ` : ""}
              {formatUiDate(project.updated_at)}
            </p>
            <p
              className="mt-1 truncate text-[11px] text-book-text-sub"
              title={progressDetail ?? progressText}
            >
              {projectMeta}
            </p>
          </div>
          <div className="flex shrink-0 items-center gap-2">
            {showWorkspaceEntry ? (
              <button
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  onWrite(project);
                }}
                className="hidden min-h-8 cursor-pointer items-center bg-transparent px-1 text-[12px] font-semibold text-book-primary hover:opacity-80 sm:inline-flex"
              >
                {workspaceLabel}
              </button>
            ) : (
              <span className="hidden text-[12px] font-semibold text-book-primary sm:inline">
                {entryLabel}
              </span>
            )}
            {hasActionItems ? (
              <span onClick={(e) => e.stopPropagation()}>
                <Dropdown items={actions} />
              </span>
            ) : null}
          </div>
        </div>
      </div>
    </li>
  );
};

const EmptyHint: React.FC<{ text: string }> = ({ text }) => (
  <div className="a4n-empty-state a4n-empty-state--framed text-sm">{text}</div>
);
