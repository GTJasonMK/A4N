import { ThemedSelect } from "../../../components/ui/ThemedSelect";
import React, { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { RefreshCw, UserPlus } from "lucide-react";
import { extractErrorMessage } from "../../../api";
import {
  inviteProjectMember,
  leaveProject,
  listProjectInvites,
  listProjectMembers,
  removeProjectMember,
  revokeProjectInvite,
  updateProjectMemberRole,
  type NovelProject,
  type ProjectInvite,
  type ProjectMember,
  type ProjectMemberRole,
} from "../../../api/project";
import { useToast } from "../../../components/feedback/Toast";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { BookButton } from "../../../components/ui/BookButton";
import { Dropdown } from "../../../components/ui/Dropdown";
import { getProjectAccessRoleLabel } from "../../../utils/projectRouting";
import { useI18n } from "../../../i18n";
import { useNovelDetailDirty, useNovelDetailDirtyRegistry } from "../novelDetailDirty";

interface ProjectCollaborationPanelProps {
  project: NovelProject;
  onRefresh: () => Promise<boolean>;
}

export const ProjectCollaborationPanel: React.FC<ProjectCollaborationPanelProps> = ({
  project,
  onRefresh,
}) => {
  const { addToast } = useToast();
  const { t } = useI18n();
  const tRef = useRef(t);
  tRef.current = t;
  const navigate = useNavigate();
  const dirtyRegistry = useNovelDetailDirtyRegistry();
  const memberRoleOptions: Array<{ value: ProjectMemberRole; label: string; disabled?: boolean }> = [
    {
      value: "manager",
      label: project.my_role === "owner"
        ? t("novel.managerRole")
        : t("novel.collaborationActions.managerRoleOwnerOnly"),
      disabled: project.my_role !== "owner",
    },
    { value: "editor", label: t("novel.editorRole") },
    { value: "viewer", label: t("novel.viewerRole") },
  ];
  const [members, setMembers] = useState<ProjectMember[]>([]);
  const [invites, setInvites] = useState<ProjectInvite[]>([]);
  const [membersLoadError, setMembersLoadError] = useState<unknown | null>(null);
  const [invitesLoadError, setInvitesLoadError] = useState<unknown | null>(null);
  const [membersReady, setMembersReady] = useState(false);
  const [invitesReady, setInvitesReady] = useState(false);
  const [identity, setIdentity] = useState("");
  const [role, setRole] = useState<ProjectMemberRole>("editor");
  const [loading, setLoading] = useState(true);
  const [submitting, setSubmitting] = useState(false);
  const [busyActionKeys, setBusyActionKeys] = useState<ReadonlySet<string>>(new Set());
  const loadSeqRef = useRef(0);
  const busyActionKeysRef = useRef<Set<string>>(new Set());
  const submittingRef = useRef(false);
  const mountedRef = useRef(false);
  const scopeRef = useRef("");

  const canManage = project.my_role === "owner" || project.my_role === "manager";
  const canManageMember = (member: ProjectMember): boolean =>
    canManage && member.role !== "owner" &&
    (project.my_role === "owner" || member.role !== "manager");
  const canLeave = project.my_role !== "owner";
  const rootProjectId = project.collaboration_root_project_id;
  const scopeKey = `${project.id}:${rootProjectId}`;
  scopeRef.current = scopeKey;
  const dirtyKey = `collaboration:${project.id}`;
  useNovelDetailDirty(
    dirtyKey,
    Boolean(identity.trim()) || role !== "editor",
    () => {
      setIdentity("");
      setRole("editor");
    },
    () => !submittingRef.current,
  );
  useNovelDetailDirty(
    `collaboration-operations:${scopeKey}`,
    submitting || busyActionKeys.size > 0,
    undefined,
    () => !submittingRef.current && busyActionKeysRef.current.size === 0,
  );

  const isCurrentScope = (requestScope: string): boolean =>
    mountedRef.current && scopeRef.current === requestScope;

  const load = useCallback(async (): Promise<boolean> => {
    const requestScope = scopeRef.current;
    const requestSeq = loadSeqRef.current + 1;
    loadSeqRef.current = requestSeq;
    setLoading(true);
    try {
      const [membersResult, invitesResult] = await Promise.allSettled([
        listProjectMembers(rootProjectId),
        canManage ? listProjectInvites(rootProjectId) : Promise.resolve([]),
      ]);
      if (!isCurrentScope(requestScope)) return false;
      if (loadSeqRef.current !== requestSeq) return true;
      const errors: string[] = [];
      if (membersResult.status === "fulfilled") {
        setMembers(membersResult.value);
        setMembersLoadError(null);
        setMembersReady(true);
      } else {
        setMembersLoadError(membersResult.reason);
        errors.push("members");
      }
      if (invitesResult.status === "fulfilled") {
        setInvites(invitesResult.value);
        setInvitesLoadError(null);
        setInvitesReady(true);
      } else {
        setInvitesLoadError(invitesResult.reason);
        errors.push("invites");
      }
      return errors.length === 0;
    } finally {
      if (isCurrentScope(requestScope) && loadSeqRef.current === requestSeq) {
        setLoading(false);
      }
    }
  }, [canManage, rootProjectId]);

  const refreshAfterMutation = async (requestScope: string): Promise<boolean> => {
    if (!isCurrentScope(requestScope)) return true;
    let refreshed = true;
    try {
      refreshed = await onRefresh();
    } catch {
      refreshed = false;
    }
    if (!isCurrentScope(requestScope)) return true;
    return refreshed;
  };

  const refreshAfterRejectedMutation = (requestScope: string) => {
    if (!isCurrentScope(requestScope)) return;
    void load();
    void refreshAfterMutation(requestScope).then((refreshed) => {
      if (isCurrentScope(requestScope) && !refreshed) {
        addToast(tRef.current("novel.refreshFailed"), "error");
      }
    });
  };

  useEffect(() => {
    mountedRef.current = true;
    void load();
    return () => {
      mountedRef.current = false;
      loadSeqRef.current += 1;
      busyActionKeysRef.current.clear();
      submittingRef.current = false;
    };
  }, [load]);

  useLayoutEffect(() => {
    setIdentity("");
    setRole("editor");
    setMembers([]);
    setInvites([]);
    setMembersLoadError(null);
    setInvitesLoadError(null);
    setLoading(true);
    setMembersReady(false);
    setInvitesReady(false);
    submittingRef.current = false;
    setSubmitting(false);
    busyActionKeysRef.current.clear();
    setBusyActionKeys(new Set());
  }, [scopeKey]);

  const beginAction = (key: string): boolean => {
    if (
      !mountedRef.current ||
      busyActionKeysRef.current.has(key) ||
      busyActionKeysRef.current.has("leave") ||
      (key === "leave" && (submittingRef.current || busyActionKeysRef.current.size > 0))
    ) return false;
    const next = new Set(busyActionKeysRef.current);
    next.add(key);
    busyActionKeysRef.current = next;
    setBusyActionKeys(next);
    return true;
  };

  const finishAction = (key: string) => {
    if (!busyActionKeysRef.current.has(key)) return;
    const next = new Set(busyActionKeysRef.current);
    next.delete(key);
    busyActionKeysRef.current = next;
    if (mountedRef.current) setBusyActionKeys(next);
  };

  const memberActionBusy = (userId: number): boolean =>
    busyActionKeysRef.current.has("leave") ||
    busyActionKeysRef.current.has(`role:${userId}`) ||
    busyActionKeysRef.current.has(`remove:${userId}`);

  const handleInvite = async () => {
    const value = identity.trim();
    if (!value || submittingRef.current || busyActionKeysRef.current.has("leave")) return;
    const requestScope = scopeRef.current;
    submittingRef.current = true;
    setSubmitting(true);
    try {
      const createdInvite = await inviteProjectMember(rootProjectId, {
        username_or_email: value,
        role,
      });
      if (!isCurrentScope(requestScope)) return;
      setIdentity("");
      if (invitesReady) {
        setInvites((current) => [
          createdInvite,
          ...current.filter((invite) => invite.id !== createdInvite.id),
        ]);
      }
      void load();
      addToast(tRef.current("novel.collaborationActions.inviteSent"), "success");
      void refreshAfterMutation(requestScope).then((refreshed) => {
        if (isCurrentScope(requestScope) && !refreshed) {
          addToast(tRef.current("novel.refreshFailed"), "error");
        }
      });
    } catch (err) {
      if (isCurrentScope(requestScope)) {
        addToast(extractErrorMessage(err, tRef.current("novel.collaborationActions.inviteFailed")), "error");
        refreshAfterRejectedMutation(requestScope);
      }
    } finally {
      if (isCurrentScope(requestScope)) {
        submittingRef.current = false;
        setSubmitting(false);
      }
    }
  };

  const handleRoleChange = async (member: ProjectMember, nextRole: ProjectMemberRole) => {
    if (!membersReady || loading || membersLoadError != null) {
      addToast(
        membersLoadError
          ? extractErrorMessage(membersLoadError, t("novel.collaborationActions.membersLoadFailed"))
          : t("common.operationInProgress"),
        "error",
      );
      return;
    }
    const actionKey = `role:${member.user_id}`;
    if (memberActionBusy(member.user_id)) return;
    if (!beginAction(actionKey)) return;
    const requestScope = scopeRef.current;
    const requestLoadSeq = loadSeqRef.current;
    try {
      await updateProjectMemberRole(rootProjectId, member.user_id, nextRole);
      if (!isCurrentScope(requestScope)) return;
      if (loadSeqRef.current !== requestLoadSeq) {
        void load();
        return;
      }
      setMembers((current) => current.map((item) =>
        item.user_id === member.user_id ? { ...item, role: nextRole } : item,
      ));
      setMembersLoadError(null);
      addToast(tRef.current("novel.collaborationActions.roleUpdated"), "success");
      void refreshAfterMutation(requestScope).then((refreshed) => {
        if (isCurrentScope(requestScope) && !refreshed) {
          addToast(tRef.current("novel.refreshFailed"), "error");
        }
      });
    } catch (err) {
      if (isCurrentScope(requestScope)) {
        addToast(extractErrorMessage(err, tRef.current("novel.collaborationActions.roleUpdateFailed")), "error");
        refreshAfterRejectedMutation(requestScope);
      }
    } finally {
      if (isCurrentScope(requestScope)) finishAction(actionKey);
    }
  };

  const handleRemove = async (member: ProjectMember) => {
    if (!membersReady || loading || membersLoadError != null) {
      addToast(
        membersLoadError
          ? extractErrorMessage(membersLoadError, t("novel.collaborationActions.membersLoadFailed"))
          : t("common.operationInProgress"),
        "error",
      );
      return;
    }
    const actionKey = `remove:${member.user_id}`;
    if (memberActionBusy(member.user_id)) return;
    const requestScope = scopeRef.current;
    const requestLoadSeq = loadSeqRef.current;
    const ok = await confirmDialog({
      title: t("novel.collaborationActions.removeTitle"),
      message: t("novel.collaborationActions.removeMessage", { username: member.username }),
      confirmText: t("common.remove"),
      dialogType: "danger",
    });
    if (!isCurrentScope(requestScope)) return;
    if (!ok) return;
    if (loadSeqRef.current !== requestLoadSeq) {
      addToast(tRef.current("common.listChangedRetry"), "info");
      return;
    }
    if (!beginAction(actionKey)) return;
    try {
      await removeProjectMember(rootProjectId, member.user_id);
      if (!isCurrentScope(requestScope)) return;
      setMembers((current) => current.filter((item) => item.user_id !== member.user_id));
      setMembersLoadError(null);
      addToast(tRef.current("novel.collaborationActions.removed"), "success");
      void refreshAfterMutation(requestScope).then((refreshed) => {
        if (isCurrentScope(requestScope) && !refreshed) {
          addToast(tRef.current("novel.refreshFailed"), "error");
        }
      });
    } catch (err) {
      if (isCurrentScope(requestScope)) {
        addToast(extractErrorMessage(err, tRef.current("novel.collaborationActions.removeFailed")), "error");
        refreshAfterRejectedMutation(requestScope);
      }
    } finally {
      if (isCurrentScope(requestScope)) finishAction(actionKey);
    }
  };

  const handleRevoke = async (invite: ProjectInvite) => {
    if (!invitesReady || loading) {
      addToast(
        invitesLoadError
          ? extractErrorMessage(invitesLoadError, t("novel.collaborationActions.invitesLoadFailed"))
          : t("common.operationInProgress"),
        "error",
      );
      return;
    }
    const actionKey = `revoke:${invite.id}`;
    if (!beginAction(actionKey)) return;
    const requestScope = scopeRef.current;
    const requestLoadSeq = loadSeqRef.current;
    try {
      await revokeProjectInvite(rootProjectId, invite.id);
      if (!isCurrentScope(requestScope)) return;
      if (loadSeqRef.current !== requestLoadSeq) {
        void load();
        return;
      }
      setInvites((current) => current.filter((item) => item.id !== invite.id));
      setInvitesLoadError(null);
      addToast(tRef.current("novel.collaborationActions.inviteRevoked"), "success");
      void refreshAfterMutation(requestScope).then((refreshed) => {
        if (isCurrentScope(requestScope) && !refreshed) {
          addToast(tRef.current("novel.refreshFailed"), "error");
        }
      });
    } catch (err) {
      if (isCurrentScope(requestScope)) {
        addToast(extractErrorMessage(err, tRef.current("novel.collaborationActions.revokeFailed")), "error");
        refreshAfterRejectedMutation(requestScope);
      }
    } finally {
      if (isCurrentScope(requestScope)) finishAction(actionKey);
    }
  };

  const handleLeave = async () => {
    const actionKey = "leave";
    if (!beginAction(actionKey)) return;
    const requestScope = scopeRef.current;
    const hasInviteDraft = dirtyRegistry.hasDirty(dirtyKey);
    if (hasInviteDraft && !dirtyRegistry.canDiscardDirty(dirtyKey)) {
      addToast(t("common.operationInProgress"), "info");
      finishAction(actionKey);
      return;
    }
    const ok = await confirmDialog({
      title: t(hasInviteDraft ? "common.unsavedChangesTitle" : "novel.collaborationActions.leaveTitle"),
      message: hasInviteDraft
        ? t("common.unsavedChangesMessage", {
            name: t("novel.collaborationActions.invite"),
            action: t("novel.collaborationActions.leave"),
          })
        : t("novel.collaborationActions.leaveMessage"),
      confirmText: t("novel.collaborationActions.leave"),
      dialogType: "danger",
    });
    if (!isCurrentScope(requestScope)) return;
    if (!ok) {
      finishAction(actionKey);
      return;
    }
    try {
      await leaveProject(rootProjectId);
      if (!isCurrentScope(requestScope)) return;
      dirtyRegistry.set(dirtyKey, false);
      setIdentity("");
      setRole("editor");
      addToast(tRef.current("novel.collaborationActions.left"), "success");
      dirtyRegistry.set(`collaboration-operations:${requestScope}`, false);
      finishAction(actionKey);
      navigate("/", { replace: true });
    } catch (err) {
      if (isCurrentScope(requestScope)) {
        addToast(extractErrorMessage(err, tRef.current("novel.collaborationActions.leaveFailed")), "error");
        refreshAfterRejectedMutation(requestScope);
      }
    } finally {
      if (isCurrentScope(requestScope)) finishAction(actionKey);
    }
  };

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between gap-2">
        {loading || !membersLoadError || members.length > 0 ? (
          <p className="text-small leading-relaxed text-book-text-muted">
            {loading ? t("novel.collaborationActions.loading") : t("novel.collaborationActions.memberCount", { count: members.length })}
          </p>
        ) : null}
        {canLeave ? (
          <Dropdown
            items={[
              {
                label: t("novel.collaborationActions.leave"),
                onClick: () => void handleLeave(),
                disabled: submitting || busyActionKeys.size > 0,
                danger: true,
              },
            ]}
          />
        ) : null}
      </div>
      {membersLoadError ? (
        <p role="alert" className="text-small text-error">
          {extractErrorMessage(membersLoadError, t("novel.collaborationActions.membersLoadFailed"))}
        </p>
      ) : null}
      {canManage && invitesLoadError ? (
        <p role="alert" className="text-small text-error">
          {extractErrorMessage(invitesLoadError, t("novel.collaborationActions.invitesLoadFailed"))}
        </p>
      ) : null}
      {(membersLoadError || (canManage && invitesLoadError)) ? (
        <BookButton variant="secondary" size="sm" onClick={() => void load()} disabled={loading}>
          <RefreshCw size={14} aria-hidden="true" />
          {t("common.retry")}
        </BookButton>
      ) : null}

      {canManage && (
        <form
          className="mb-3 grid grid-cols-[minmax(0,1fr)_auto] items-center gap-2 sm:grid-cols-[minmax(220px,1fr)_7rem_auto]"
          onSubmit={(event) => {
            event.preventDefault();
            void handleInvite();
          }}
        >
          <input
            value={identity}
            onChange={(event) => setIdentity(event.target.value)}
            disabled={submitting || busyActionKeys.has("leave")}
            aria-label={t("novel.collaborationActions.identityPlaceholder")}
            placeholder={t("novel.collaborationActions.identityPlaceholder")}
            className="pyqt-input col-span-2 h-9 min-w-0 sm:col-span-1"
          />
          <ThemedSelect
            value={role}
            aria-label={t("novel.collaborationActions.inviteRole")}
            onChange={(selectedValue) => setRole(selectedValue as ProjectMemberRole)}
            disabled={submitting || busyActionKeys.has("leave")}
            className="pyqt-input h-9 w-full"
          >
            {memberRoleOptions.map((item) => (
              <option key={item.value} value={item.value} disabled={item.disabled}>
                {item.label}
              </option>
            ))}
          </ThemedSelect>
          <BookButton
            variant="primary"
            size="sm"
            type="submit"
            disabled={submitting || busyActionKeys.has("leave") || !identity.trim()}
          >
            <span className="inline-flex items-center gap-1.5">
              <UserPlus size={13} /> {t("novel.collaborationActions.invite")}
            </span>
          </BookButton>
        </form>
      )}

      <ul aria-label={t("novel.collaborationActions.membersAria")} className="space-y-2">
        {members.map((member) => (
          <li
            key={member.user_id}
            className="a4n-flat-surface grid grid-cols-[minmax(0,1fr)_auto] items-center gap-3 px-3 py-2.5"
          >
            <div className="min-w-0 flex-1">
              <div className="truncate text-xs font-semibold text-book-text-main">
                {member.username}
              </div>
              <div className="truncate text-small text-book-text-muted">{member.email}</div>
            </div>
            <div className="flex shrink-0 items-center gap-2">
              {canManageMember(member) ? (
                <ThemedSelect
                  value={member.role}
                  aria-label={t("novel.collaborationActions.adjustRole", { username: member.username })}
                  disabled={memberActionBusy(member.user_id)}
                  onChange={(selectedValue) =>
                    void handleRoleChange(member, selectedValue as ProjectMemberRole)
                  }
                  className="pyqt-input h-8 w-32 text-small"
                >
                  {memberRoleOptions.map((item) => (
                    <option key={item.value} value={item.value} disabled={item.disabled}>
                      {item.label}
                    </option>
                  ))}
                </ThemedSelect>
              ) : (
                <span className="text-small text-book-text-muted">
                  {getProjectAccessRoleLabel(member.role, t)}
                </span>
              )}
              {canManageMember(member) ? (
                <Dropdown
                  items={[
                    {
                      label: t("novel.collaborationActions.removeMember"),
                      onClick: () => void handleRemove(member),
                      disabled: memberActionBusy(member.user_id),
                      danger: true,
                    },
                  ]}
                />
              ) : null}
            </div>
          </li>
        ))}
      </ul>

      {canManage && invites.length > 0 && (
        <section className="mt-4">
          <div className="text-small font-semibold text-book-text-muted">{t("novel.collaborationActions.pendingInvites")}</div>
          <ul aria-label={t("novel.collaborationActions.pendingInvites")} className="mt-2 space-y-2">
            {invites.map((invite) => (
              <li
                key={invite.id}
                className="a4n-flat-surface grid grid-cols-[minmax(0,1fr)_auto] items-center gap-3 px-3 py-2.5 text-small text-book-text-sub"
              >
                <span className="min-w-0">
                  <span className="block truncate font-medium text-book-text-main">
                    {invite.invitee_username}
                  </span>
                  <span className="mt-0.5 block text-book-text-muted">
                    {getProjectAccessRoleLabel(invite.role, t)}
                  </span>
                </span>
                <Dropdown
                  items={[
                    {
                      label: t("novel.collaborationActions.revokeInvite"),
                      onClick: () => void handleRevoke(invite),
                      disabled: busyActionKeys.has("leave") || busyActionKeys.has(`revoke:${invite.id}`),
                      danger: true,
                    },
                  ]}
                />
              </li>
            ))}
          </ul>
        </section>
      )}
    </div>
  );
};
