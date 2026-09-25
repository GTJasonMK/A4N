import { ThemedSelect } from "../../components/ui/ThemedSelect";
import React from "react";
import { AlertCircle, Check, Plus, RefreshCw, Settings as SettingsIcon, Upload, UserPlus, X } from "lucide-react";
import type { NovelProject, ProjectAccessScope, ProjectInvite } from "../../api/project";
import { BookButton, BookIconButton } from "../../components/ui/BookButton";
import { Dropdown } from "../../components/ui/Dropdown";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { SegmentedControl } from "../../components/ui/SegmentedControl";
import { StatusBarToggle } from "../../components/ui/StatusBar";
import {
  AllProjectList,
  RecentProjectList,
  type ProjectActionItems,
} from "./ProjectLists";
import type { ProjectFamily, ProjectKindFilter, ProjectTab } from "./novelListUtils";
import { PROJECT_KIND_FILTERS } from "./novelListUtils";
import { useI18n } from "../../i18n";
import { usePackagePreferences } from "../../store/packagePreferences";
import { getProjectAccessRoleLabel } from "../../utils/projectRouting";

interface ProjectListPanelProps {
  tab: ProjectTab;
  loading: boolean;
  loadError: string | null;
  empty: boolean;
  recentProjectFamilies: ProjectFamily[];
  groupedProjectFamilies: [string, ProjectFamily[]][];
  accessScope: ProjectAccessScope;
  projectKindFilter: ProjectKindFilter;
  pendingInvites: ProjectInvite[];
  inviteActionIds: ReadonlySet<number>;
  onTabChange: (tab: ProjectTab) => void;
  onCreateProject: () => void;
  onImportProject: () => void;
  onOpenSettings: () => void;
  onRetry: () => void;
  onAccessScopeChange: (scope: ProjectAccessScope) => void;
  onProjectKindFilterChange: (kind: ProjectKindFilter) => void;
  onAcceptInvite: (invite: ProjectInvite) => void;
  onDeclineInvite: (invite: ProjectInvite) => void;
  onOpenProject: (project: NovelProject) => void;
  onOpenWorkspace: (project: NovelProject) => void;
  getActions: (project: NovelProject) => ProjectActionItems;
}

export const ProjectListPanel: React.FC<ProjectListPanelProps> = ({
  tab,
  loading,
  loadError,
  empty,
  recentProjectFamilies,
  groupedProjectFamilies,
  accessScope,
  projectKindFilter,
  pendingInvites,
  inviteActionIds,
  onTabChange,
  onCreateProject,
  onImportProject,
  onOpenSettings,
  onRetry,
  onAccessScopeChange,
  onProjectKindFilterChange,
  onAcceptInvite,
  onDeclineInvite,
  onOpenProject,
  onOpenWorkspace,
  getActions,
}) => {
  const { t } = useI18n();
  const { isVisible } = usePackagePreferences();
  const projectKindLabels: Record<ProjectKindFilter, string> = {
    all: t("common.all"),
    text: t("novel.textProject"),
    comic: t("novel.comicProject"),
    video: t("novel.videoProject"),
    galgame: t("novel.galgameProject"),
  };
  const accessScopeLabels: Record<ProjectAccessScope, string> = {
    all: t("novel.allAccessible"),
    owned: t("novel.owned"),
    collaborating: t("novel.collaborating"),
  };
  return (
  <section aria-label={t("novel.projectList")} className="a4n-home-content flex min-w-0 flex-1 flex-col md:overflow-hidden">
    <header className="a4n-home-heading content-wide">
      <p className="a4n-kicker">{t("auth.continueCreation")}</p>
      <h1>{t("novel.projectWorkbench")}</h1>
    </header>
    <nav aria-label={t("novel.projectFilter")} className="a4n-home-filter-bar content-wide mb-3 border-b border-book-border/60 pb-2.5">
      <div className="flex flex-col gap-2.5 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex min-w-0 flex-wrap items-center gap-2">
          <SegmentedControl
            ariaLabel={t("novel.projectView")}
            value={tab}
            onChange={onTabChange}
            options={[
              { id: "recent", label: t("novel.recentProjects") },
              { id: "all", label: t("novel.allProjects") },
            ]}
          />
          <div className="ml-auto flex shrink-0 items-center gap-1 md:hidden">
            <BookButton
              variant="primary"
              size="lg"
              onClick={onCreateProject}
              aria-label={t("novel.createProject")}
              title={t("novel.createProject")}
              className="!h-11 !min-h-11 !w-11 !min-w-11 !p-0"
            >
              <Plus size={18} aria-hidden="true" />
            </BookButton>
            {isVisible("external-import") ? <BookIconButton
              label={t("novel.importProjectButton")}
              onClick={onImportProject}
              variant="secondary"
              className="!h-11 !w-11"
            >
              <Upload size={17} aria-hidden="true" />
            </BookIconButton> : null}
            <BookIconButton
              label={t("nav.openSettings")}
              onClick={onOpenSettings}
              variant="secondary"
              className="!h-11 !w-11"
            >
              <SettingsIcon size={17} aria-hidden="true" />
            </BookIconButton>
          </div>
        </div>
        <div className="grid min-w-0 grid-cols-[minmax(0,1fr)_minmax(0,1fr)_auto] items-center gap-2 sm:flex sm:justify-end">
            <label className="inline-flex min-w-0 flex-col items-stretch gap-1.5 text-small font-semibold text-book-text-muted max-md:flex-1 xl:flex-row xl:items-center">
              <span>{t("novel.projectTypeFilter")}</span>
              <ThemedSelect
                aria-label={t("novel.projectTypeFilter")}
                value={projectKindFilter}
                onChange={(selectedValue) =>
                  onProjectKindFilterChange(selectedValue as ProjectKindFilter)
                }
                className="pyqt-input a4n-compact-select h-8 min-w-0 flex-1 sm:min-w-[6.75rem] sm:flex-none max-md:w-full"
              >
                {PROJECT_KIND_FILTERS.map((item) => (
                  <option key={item} value={item}>
                    {projectKindLabels[item]}
                  </option>
                ))}
              </ThemedSelect>
            </label>
            <label className="inline-flex min-w-0 flex-col items-stretch gap-1.5 text-small font-semibold text-book-text-muted max-md:flex-1 xl:flex-row xl:items-center">
              <span>{t("novel.accessFilter")}</span>
              <ThemedSelect
                aria-label={t("novel.accessFilter")}
                value={accessScope}
                onChange={(selectedValue) =>
                  onAccessScopeChange(selectedValue as ProjectAccessScope)
                }
                className="pyqt-input a4n-compact-select h-8 min-w-0 flex-1 sm:min-w-[6.75rem] sm:flex-none max-md:w-full"
              >
                {ACCESS_SCOPE_OPTIONS.map((item) => (
                  <option key={item.value} value={item.value}>
                    {accessScopeLabels[item.value]}
                  </option>
                ))}
              </ThemedSelect>
            </label>
            <StatusBarToggle />
            {loading && (
              <span className="col-span-2 inline-flex items-center justify-end sm:col-span-1">
                <LoadingSpinner size="sm" text={t("common.loading")} />
              </span>
            )}
        </div>
      </div>
    </nav>

    <div
      key={`${tab}:${accessScope}:${projectKindFilter}`}
      className="content-wide min-h-0 flex-1 overflow-visible pr-0 md:overflow-y-auto md:pr-2 custom-scrollbar"
    >
      {pendingInvites.length > 0 && (
        <ProjectInviteBanner
          invites={pendingInvites}
          inviteActionIds={inviteActionIds}
          onAccept={onAcceptInvite}
          onDecline={onDeclineInvite}
        />
      )}
      {loadError ? (
        <div role="alert" className="a4n-flat-surface flex flex-col items-center gap-3 px-6 py-10 text-center">
          <AlertCircle size={24} className="text-error" aria-hidden="true" />
          <h2 className="text-base font-semibold text-book-text-main">{t("novel.projectLoadFailed")}</h2>
          {loadError !== t("novel.projectLoadFailed") ? (
            <p className="max-w-lg text-sm text-book-text-sub">{loadError}</p>
          ) : null}
          <BookButton variant="secondary" onClick={onRetry} disabled={loading}>
            <RefreshCw size={15} aria-hidden="true" />
            {t("common.retry")}
          </BookButton>
        </div>
      ) : loading && !hasVisibleProjects(tab, recentProjectFamilies, groupedProjectFamilies) ? (
        <ProjectListSkeleton />
      ) : tab === "recent" ? (
        <RecentProjectList
          families={recentProjectFamilies}
          empty={empty}
          onOpen={onOpenProject}
          onWrite={onOpenWorkspace}
          getActions={getActions}
        />
      ) : (
        <AllProjectList
          groups={groupedProjectFamilies}
          empty={empty}
          onOpen={onOpenProject}
          onWrite={onOpenWorkspace}
          getActions={getActions}
        />
      )}
      </div>
  </section>
  );
};

function hasVisibleProjects(
  tab: ProjectTab,
  recentProjectFamilies: ProjectFamily[],
  groupedProjectFamilies: [string, ProjectFamily[]][],
): boolean {
  return tab === "recent"
    ? recentProjectFamilies.length > 0
    : groupedProjectFamilies.length > 0;
}

const ProjectListSkeleton: React.FC = () => {
  const { t } = useI18n();
  return (
    <div role="status" aria-live="polite" className="space-y-3">
      <span className="sr-only">{t("common.loading")}</span>
      <div className="a4n-flat-surface overflow-hidden" aria-hidden="true">
        {[0, 1, 2].map((item) => (
          <div
            key={item}
            className="a4n-project-skeleton-row border-b border-book-border/35 last:border-b-0"
          >
            <span className="a4n-project-skeleton-icon" />
            <span className="min-w-0 flex-1 space-y-2">
              <span className="a4n-project-skeleton-line a4n-project-skeleton-line--title" />
              <span className="a4n-project-skeleton-line a4n-project-skeleton-line--meta" />
              <span className="a4n-project-skeleton-line a4n-project-skeleton-line--detail" />
            </span>
            <span className="a4n-project-skeleton-action" />
          </div>
        ))}
      </div>
    </div>
  );
};

const ACCESS_SCOPE_OPTIONS: Array<{ value: ProjectAccessScope }> = [
  { value: "all" },
  { value: "owned" },
  { value: "collaborating" },
];

const ProjectInviteBanner: React.FC<{
  invites: ProjectInvite[];
  inviteActionIds: ReadonlySet<number>;
  onAccept: (invite: ProjectInvite) => void;
  onDecline: (invite: ProjectInvite) => void;
}> = ({ invites, inviteActionIds, onAccept, onDecline }) => {
  const { t } = useI18n();
  return (
  <div className="mb-4">
    <div className="mb-2 flex items-center gap-2 px-1 text-xs font-semibold text-book-text-main">
      <UserPlus size={14} className="text-book-primary" />
      {t("novel.pendingInvites")}
    </div>
    <div className="a4n-motion-stagger space-y-2">
      {invites.map((invite) => (
        <ProjectInviteRow
          key={invite.id}
          invite={invite}
          busy={inviteActionIds.has(invite.id)}
          onAccept={onAccept}
          onDecline={onDecline}
        />
      ))}
    </div>
  </div>
  );
};

const ProjectInviteRow: React.FC<{
  invite: ProjectInvite;
  busy: boolean;
  onAccept: (invite: ProjectInvite) => void;
  onDecline: (invite: ProjectInvite) => void;
}> = ({ invite, busy, onAccept, onDecline }) => {
  const { t } = useI18n();
  return (
  <div className="a4n-flat-surface flex items-center gap-3 px-3 py-2.5 sm:px-4">
    <div className="min-w-0 flex-1">
      <div className="break-words text-xs font-semibold text-book-text-main">
        {invite.project_title}
      </div>
      <div className="mt-1 break-words text-small text-book-text-muted">
        {invite.inviter_username} {t("novel.invited")} · {getProjectAccessRoleLabel(invite.role, t)}
      </div>
    </div>
    {!busy ? (
      <Dropdown
        items={[
          {
            label: t("novel.declineInvite"),
            icon: <X size={12} />,
            onClick: () => onDecline(invite),
          },
        ]}
      />
    ) : null}
    <BookButton
      variant="primary"
      size="sm"
      onClick={() => onAccept(invite)}
      disabled={busy}
    >
      {busy ? <LoadingSpinner size="sm" /> : <Check size={12} />}
      {t("novel.acceptInvite")}
    </BookButton>
  </div>
  );
};
