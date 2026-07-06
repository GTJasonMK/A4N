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
import { PROJECT_KIND_LABELS, PROJECT_ROLE_LABELS } from "./novelListUtils";

export type ProjectActionItems = React.ComponentProps<typeof Dropdown>["items"];

export const TabButton: React.FC<{
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
}> = ({ active, onClick, children }) => (
  <button
    type="button"
    onClick={onClick}
    data-active={active ? "1" : undefined}
    className="pyqt-tab"
  >
    {children}
  </button>
);

interface ProjectListProps {
  projects: NovelProject[];
  groups: [string, NovelProject[]][];
  empty: boolean;
  onOpen: (project: NovelProject) => void;
  onWrite: (project: NovelProject) => void;
  getActions: (project: NovelProject) => ProjectActionItems;
}

export const RecentProjectList: React.FC<
  Pick<ProjectListProps, "projects" | "empty" | "onOpen" | "onWrite" | "getActions">
> = ({ projects, empty, onOpen, onWrite, getActions }) => {
  if (empty) {
    return (
      <EmptyHint text={'还没有项目。点击左侧"创建小说"开始你的第一部作品。'} />
    );
  }
  if (projects.length === 0) {
    return null;
  }
  return (
    <ul className="space-y-2.5">
      {projects.map((p) => (
        <ProjectRow
          key={p.id}
          project={p}
          onOpen={onOpen}
          onWrite={onWrite}
          actions={getActions(p)}
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
      {groups.map(([letter, items]) => (
        <div key={letter}>
          <div className="text-xs font-semibold text-book-text-muted uppercase tracking-wide mb-1.5 px-1">
            {letter}
          </div>
          <ul className="space-y-2.5">
            {items.map((p) => (
              <ProjectRow
                key={p.id}
                project={p}
                onOpen={onOpen}
                onWrite={onWrite}
                actions={getActions(p)}
              />
            ))}
          </ul>
        </div>
      ))}
    </div>
  );
};

const ProjectRow: React.FC<{
  project: NovelProject;
  onOpen: (project: NovelProject) => void;
  onWrite: (project: NovelProject) => void;
  actions: ProjectActionItems;
}> = ({ project, onOpen, onWrite, actions }) => {
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
  const fullItems: ContextMenuEntry[] = [
    { label: entryLabel, onClick: () => onOpen(project), shortcut: "Enter" },
    ...(showWorkspaceEntry ? [{ label: workspaceLabel, onClick: () => onWrite(project) }] : []),
    ...(hasActionItems ? [{ divider: true as const }, ...ctxItems] : []),
  ];

  return (
    <li>
      <div
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
        className="a4n-interactive-row flex min-h-[76px] w-full cursor-pointer flex-col justify-between px-3 py-2.5 text-left sm:min-h-[68px]"
      >
        <div className="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
          <span className="min-w-0 text-sm font-semibold text-book-text-main truncate font-serif">
            {project.title}
          </span>
          <div className="flex max-w-full shrink-0 flex-wrap items-center gap-1 sm:max-w-[58%] sm:justify-end">
            {project.my_role ? (
              <span className="pyqt-pill whitespace-nowrap !text-[10px]">
                {PROJECT_ROLE_LABELS[project.my_role]}
              </span>
            ) : null}
            <span className="pyqt-pill whitespace-nowrap !text-[10px]">
              {PROJECT_KIND_LABELS[project.project_kind]}
            </span>
            <span className="pyqt-pill whitespace-nowrap !text-[10px]" title={progressDetail ?? progressText}>
              {progressText}
            </span>
            {hasActionItems ? (
              <span onClick={(e) => e.stopPropagation()}>
                <Dropdown items={actions} label="项目操作" />
              </span>
            ) : null}
          </div>
        </div>
        <div className="mt-2 flex flex-col gap-1 text-[10px] text-book-text-muted sm:flex-row sm:items-center sm:justify-between sm:gap-3">
          <span className="min-w-0 break-words sm:truncate">
            {project.owner_username ? `${project.owner_username} · ` : ""}
            {project.updated_at ? new Date(project.updated_at).toLocaleDateString() : "—"}
            {progressDetail ? ` · ${progressDetail}` : ""}
          </span>
          {showWorkspaceEntry ? (
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                onWrite(project);
              }}
              className="bg-transparent p-0 text-book-primary hover:underline cursor-pointer"
            >
              {workspaceLabel} →
            </button>
          ) : null}
        </div>
      </div>
    </li>
  );
};

const EmptyHint: React.FC<{ text: string }> = ({ text }) => (
  <div className="a4n-page-surface border-dashed text-center text-sm text-book-text-muted py-16 px-6">{text}</div>
);
