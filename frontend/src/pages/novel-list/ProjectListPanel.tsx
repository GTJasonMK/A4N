import React from "react";
import { Check, UserPlus, X } from "lucide-react";
import type { NovelProject, ProjectAccessScope, ProjectInvite } from "../../api/project";
import { BookButton } from "../../components/ui/BookButton";
import { Dropdown } from "../../components/ui/Dropdown";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { SegmentedControl } from "../../components/ui/SegmentedControl";
import { PROJECT_ACCESS_ROLE_LABELS } from "../../utils/projectRouting";
import {
  AllProjectList,
  RecentProjectList,
  type ProjectActionItems,
} from "./ProjectLists";
import type { ProjectFamily, ProjectKindFilter, ProjectTab } from "./novelListUtils";
import {
  PROJECT_KIND_FILTER_LABELS,
  PROJECT_KIND_FILTERS,
  PROJECT_TAB_LABELS,
} from "./novelListUtils";

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
  onAccessScopeChange,
  onProjectKindFilterChange,
  onAcceptInvite,
  onDeclineInvite,
  onOpenProject,
  onOpenWorkspace,
  getActions,
}) => (
  <section aria-label="项目列表" className="flex min-w-0 flex-1 flex-col bg-book-bg px-3 py-3 sm:px-5 md:overflow-hidden md:px-6 md:py-5 lg:px-8 lg:py-6">
    <nav aria-label="项目筛选" className="content-wide mb-3 border-b border-book-border/60 pb-2.5">
      <div className="flex flex-col gap-2.5 sm:flex-row sm:items-center sm:justify-between">
        <SegmentedControl
          ariaLabel="项目视图"
          value={tab}
          onChange={onTabChange}
          options={[
            { id: "recent", label: PROJECT_TAB_LABELS.recent },
            { id: "all", label: PROJECT_TAB_LABELS.all },
          ]}
        />
        <div className="grid min-w-0 grid-cols-2 items-center gap-2 sm:flex sm:justify-end">
            <label className="inline-flex min-w-0 items-center gap-1.5 text-small font-semibold text-book-text-muted max-md:flex-1">
              <span className="hidden xl:inline">项目类型</span>
              <select
                aria-label="项目类型"
                value={projectKindFilter}
                onChange={(event) =>
                  onProjectKindFilterChange(event.target.value as ProjectKindFilter)
                }
                className="pyqt-input a4n-compact-select h-8 min-w-0 flex-1 sm:min-w-[6.75rem] sm:flex-none max-md:w-full"
              >
                {PROJECT_KIND_FILTERS.map((item) => (
                  <option key={item} value={item}>
                    {PROJECT_KIND_FILTER_LABELS[item]}
                  </option>
                ))}
              </select>
            </label>
            <label className="inline-flex min-w-0 items-center gap-1.5 text-small font-semibold text-book-text-muted max-md:flex-1">
              <span className="hidden xl:inline">访问权限</span>
              <select
                aria-label="访问权限"
                value={accessScope}
                onChange={(event) =>
                  onAccessScopeChange(event.target.value as ProjectAccessScope)
                }
                className="pyqt-input a4n-compact-select h-8 min-w-0 flex-1 sm:min-w-[6.75rem] sm:flex-none max-md:w-full"
              >
                {ACCESS_SCOPE_OPTIONS.map((item) => (
                  <option key={item.value} value={item.value}>
                    {item.label}
                  </option>
                ))}
              </select>
            </label>
            {loading && (
              <span className="col-span-2 inline-flex items-center justify-end sm:col-span-1">
                <LoadingSpinner size="sm" text="加载中" />
              </span>
            )}
        </div>
      </div>
    </nav>

    <div className="content-wide min-h-0 flex-1 overflow-visible pr-0 md:overflow-y-auto md:pr-2 custom-scrollbar">
      {pendingInvites.length > 0 && (
        <ProjectInviteBanner
          invites={pendingInvites}
          inviteActionIds={inviteActionIds}
          onAccept={onAcceptInvite}
          onDecline={onDeclineInvite}
        />
      )}
      {loadError ? (
        <div className="a4n-flat-surface border-dashed px-6 py-12 text-center text-sm text-error">
          {loadError}
        </div>
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

const ACCESS_SCOPE_OPTIONS: Array<{ value: ProjectAccessScope; label: string }> = [
  { value: "all", label: "全部可访问" },
  { value: "owned", label: "我创建的" },
  { value: "collaborating", label: "我协作的" },
];

const ProjectInviteBanner: React.FC<{
  invites: ProjectInvite[];
  inviteActionIds: ReadonlySet<number>;
  onAccept: (invite: ProjectInvite) => void;
  onDecline: (invite: ProjectInvite) => void;
}> = ({ invites, inviteActionIds, onAccept, onDecline }) => (
  <div className="mb-4">
    <div className="mb-2 flex items-center gap-2 px-1 text-xs font-semibold text-book-text-main">
      <UserPlus size={14} className="text-book-primary" />
      待处理项目邀请
    </div>
    <div className="space-y-2">
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

const ProjectInviteRow: React.FC<{
  invite: ProjectInvite;
  busy: boolean;
  onAccept: (invite: ProjectInvite) => void;
  onDecline: (invite: ProjectInvite) => void;
}> = ({ invite, busy, onAccept, onDecline }) => (
  <div className="a4n-flat-surface flex items-center gap-3 px-3 py-2.5 sm:px-4">
    <div className="min-w-0 flex-1">
      <div className="truncate text-xs font-semibold text-book-text-main">
        {invite.project_title}
      </div>
      <div className="mt-1 truncate text-small text-book-text-muted">
        {invite.inviter_username} 邀请 · {PROJECT_ACCESS_ROLE_LABELS[invite.role]}
      </div>
    </div>
    {!busy ? (
      <Dropdown
        items={[
          {
            label: "拒绝邀请",
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
      接受
    </BookButton>
  </div>
);
