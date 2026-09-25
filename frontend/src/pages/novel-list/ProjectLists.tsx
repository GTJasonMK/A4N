import React from "react";
import { BookOpen, ChevronRight, Gamepad2, Image, Video } from "lucide-react";
import { Dropdown } from "../../components/ui/Dropdown";
import { BookCardAction } from "../../components/ui/BookCard";
import { useContextMenu, type ContextMenuEntry } from "../../components/ui/ContextMenu";
import { canEditProjectMaterials, type NovelProject } from "../../api/project";
import {
  getProjectLifecycleBlockedReason,
  getProjectProgressText,
} from "../../utils/projectStatus";
import {
  getProjectHomeEntryLabel,
  getProjectHomeEntryRoute,
  getProjectWorkspaceLabel,
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
          <div className="text-xs font-semibold text-book-text-muted uppercase mb-1.5 px-1">
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
      className="a4n-project-family space-y-2"
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
        <div className="px-1 text-small text-book-text-muted">
          {t("novel.originatedFrom")} <span className="font-medium text-book-text-sub">{family.title}</span>
        </div>
      )}

      {derivedProjects.length > 0 ? (
        <section
          aria-label={`${family.title} ${t("novel.derivedProjects", { count: derivedProjects.length })}`}
          className="ml-3 border-l border-book-border/70 pl-3 sm:ml-5 sm:pl-4"
        >
          <div className="pb-2 text-small font-medium text-book-text-muted">
            {t("novel.derivedProjects", { count: derivedProjects.length })}
          </div>
          <ul className="space-y-2">
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
  const entryLabel = getProjectHomeEntryLabel(project, t);
  const entryRoute = getProjectHomeEntryRoute(project);
  const workspaceLabel = getProjectWorkspaceLabel(project, t);
  const workspaceRoute = getProjectWorkspaceRoute(project);
  const lifecycleBlockedReason = getProjectLifecycleBlockedReason(project, t);
  const lifecycleLabel =
    project.lifecycle_gate === "resetting"
      ? t("novel.resetFinishing")
      : project.lifecycle_gate === "deleting"
        ? t("novel.deleting")
        : "";
  const showWorkspaceEntry = canEditProjectMaterials(project) && workspaceRoute !== entryRoute;
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
    <li className="group a4n-project-list-entry">
      <div
        data-project-relation={relation}
        onContextMenu={menu.handle(fullItems)}
        onMouseEnter={() => prefetchProjectRouteByStatus(project, { immediate: true })}
        onFocus={() => prefetchProjectRouteByStatus(project, { immediate: true })}
        className="a4n-project-row relative w-full px-3 py-4 text-left sm:px-4"
      >
        <BookCardAction label={`${project.title} · ${entryLabel}`} onClick={() => onOpen(project)} />
        <div className="flex items-start gap-3">
          <span
            className="a4n-project-kind mt-0.5"
            data-project-kind={project.project_kind}
            aria-hidden="true"
          >
            <ProjectKindIcon size={17} strokeWidth={1.8} />
          </span>
          <div className="min-w-0 flex-1">
            <h3 className="text-base font-semibold text-book-text-main sm:text-lg">
              <span className="block truncate">{project.title}</span>
            </h3>
            <p className="mt-1 truncate text-small text-book-text-muted">
              {project.owner_username} · {formatDate(project.updated_at, locale)}
            </p>
            <p
              className="mt-1 break-words text-small text-book-text-sub sm:truncate"
              title={progressText}
            >
              {projectMeta}
            </p>
          </div>
          <div className="pointer-events-none flex shrink-0 items-center gap-2">
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
                aria-label={workspaceLabel}
                title={workspaceLabel}
                className="pointer-events-auto relative z-20 inline-flex min-h-11 cursor-pointer items-center bg-transparent px-1 text-body-sm font-semibold text-book-primary hover:underline sm:min-h-8"
              >
                <span className="hidden sm:inline">{workspaceLabel}</span>
                <span className="sm:hidden">{t("novel.writeShortcut")}</span>
              </button>
            ) : (
              <span className="hidden text-body-sm font-semibold text-book-primary sm:inline">
                {entryLabel}
              </span>
            )}
            {hasActionItems ? (
              <span className="pointer-events-auto relative z-20">
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
