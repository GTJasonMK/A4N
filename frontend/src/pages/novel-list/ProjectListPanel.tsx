import React from "react";
import { Check, Loader2, UserPlus, X } from "lucide-react";
import type { NovelProject, ProjectAccessScope, ProjectInvite } from "../../api/project";
import {
  AllProjectList,
  RecentProjectList,
  TabButton,
  type ProjectActionItems,
} from "./ProjectLists";
import type { ProjectKindFilter, ProjectTab } from "./novelListUtils";
import {
  PROJECT_KIND_FILTER_LABELS,
  PROJECT_KIND_FILTERS,
  PROJECT_ROLE_LABELS,
  PROJECT_TAB_LABELS,
} from "./novelListUtils";

interface ProjectListPanelProps {
  tab: ProjectTab;
  loading: boolean;
  loadError: string | null;
  empty: boolean;
  recentProjects: NovelProject[];
  groupedProjects: [string, NovelProject[]][];
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
  recentProjects,
  groupedProjects,
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
  <section className="flex min-w-0 flex-1 flex-col bg-book-bg px-3 py-4 sm:px-5 md:overflow-hidden md:px-5 md:py-6 lg:px-8 lg:py-8">
    <nav className="content-wide mb-3 flex flex-wrap items-center gap-2 border-b border-book-border/60 pb-2">
      <div className="flex flex-wrap items-center gap-1.5">
        <TabButton active={tab === "recent"} onClick={() => onTabChange("recent")}>
          {PROJECT_TAB_LABELS.recent}
        </TabButton>
        <TabButton active={tab === "all"} onClick={() => onTabChange("all")}>
          {PROJECT_TAB_LABELS.all}
        </TabButton>
      </div>
      <div className="flex w-full flex-wrap items-center gap-2 sm:ml-auto sm:w-auto sm:justify-end">
        <label className="inline-flex items-center gap-1.5 text-[11px] font-semibold text-book-text-muted">
          项目类型
          <select
            value={projectKindFilter}
            onChange={(event) => onProjectKindFilterChange(event.target.value as ProjectKindFilter)}
            className="pyqt-input a4n-compact-select h-8 min-w-[7.5rem] flex-1 sm:flex-none"
          >
            {PROJECT_KIND_FILTERS.map((item) => (
              <option key={item} value={item}>
                {PROJECT_KIND_FILTER_LABELS[item]}
              </option>
            ))}
          </select>
        </label>
        <label className="inline-flex items-center gap-1.5 text-[11px] font-semibold text-book-text-muted">
          访问权限
          <select
            value={accessScope}
            onChange={(event) => onAccessScopeChange(event.target.value as ProjectAccessScope)}
            className="pyqt-input a4n-compact-select h-8 min-w-[7.5rem] flex-1 sm:flex-none"
          >
            {ACCESS_SCOPE_OPTIONS.map((item) => (
              <option key={item.value} value={item.value}>
                {item.label}
              </option>
            ))}
          </select>
        </label>
        {loading && (
          <span className="inline-flex items-center gap-1 text-xs text-book-text-muted">
            <Loader2 size={12} className="animate-spin" /> 加载中
          </span>
        )}
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
        <div className="a4n-page-surface border-dashed px-6 py-12 text-center text-sm text-error">
          {loadError}
        </div>
      ) : tab === "recent" ? (
        <RecentProjectList
          projects={recentProjects}
          empty={empty}
          onOpen={onOpenProject}
          onWrite={onOpenWorkspace}
          getActions={getActions}
        />
      ) : (
        <AllProjectList
          groups={groupedProjects}
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
  <div className="mb-4 rounded-md border border-book-primary/25 bg-book-primary/5 p-3">
    <div className="mb-2 flex items-center gap-2 text-xs font-semibold text-book-text-main">
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
  <div className="flex flex-wrap items-center justify-between gap-2 text-xs text-book-text-sub">
    <span>
      {invite.inviter_username || `用户 ${invite.inviter_user_id}`} 邀请你以{" "}
      {PROJECT_ROLE_LABELS[invite.role]} 身份加入
      <span className="font-semibold text-book-text-main">
        「{invite.project_title || `项目 #${invite.project_id}`}」
      </span>
    </span>
    <span className="flex flex-wrap items-center gap-1.5">
      <button
        type="button"
        onClick={() => onAccept(invite)}
        disabled={busy}
        className="pyqt-btn-primary min-h-9 px-2 text-[11px] sm:min-h-7"
      >
        <Check size={12} /> 接受
      </button>
      <button
        type="button"
        onClick={() => onDecline(invite)}
        disabled={busy}
        className="pyqt-btn-secondary min-h-9 px-2 text-[11px] sm:min-h-7"
      >
        <X size={12} /> 拒绝
      </button>
    </span>
  </div>
);
