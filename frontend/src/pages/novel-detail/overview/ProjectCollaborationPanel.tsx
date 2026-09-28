import { ThemedSelect } from "../../../components/ui/ThemedSelect";
import React, { useCallback, useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { UserPlus } from "lucide-react";
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
  const navigate = useNavigate();
  const dirtyRegistry = useNovelDetailDirtyRegistry();
  const memberRoleOptions: Array<{ value: ProjectMemberRole; label: string }> = [
    { value: "manager", label: t("novel.managerRole") },
    { value: "editor", label: t("novel.editorRole") },
    { value: "viewer", label: t("novel.viewerRole") },
  ];
  const [members, setMembers] = useState<ProjectMember[]>([]);
  const [invites, setInvites] = useState<ProjectInvite[]>([]);
  const [membersLoadError, setMembersLoadError] = useState<string | null>(null);
  const [invitesLoadError, setInvitesLoadError] = useState<string | null>(null);
  const [identity, setIdentity] = useState("");
  const [role, setRole] = useState<ProjectMemberRole>("editor");
  const [loading, setLoading] = useState(true);
  const [submitting, setSubmitting] = useState(false);
  const [busyActionKeys, setBusyActionKeys] = useState<ReadonlySet<string>>(new Set());
  const loadSeqRef = useRef(0);
  const busyActionKeysRef = useRef<Set<string>>(new Set());
  const submittingRef = useRef(false);
  const mountedRef = useRef(false);

  const canManage = project.my_role === "owner" || project.my_role === "manager";
  const canLeave = project.my_role !== "owner";
  const rootProjectId = project.collaboration_root_project_id;
  const dirtyKey = `collaboration:${project.id}`;
  useNovelDetailDirty(dirtyKey, Boolean(identity.trim()), () => setIdentity(""), () => !submittingRef.current);

  const load = useCallback(async (): Promise<boolean> => {
    const requestSeq = loadSeqRef.current + 1;
    loadSeqRef.current = requestSeq;
    setLoading(true);
    try {
      const [membersResult, invitesResult] = await Promise.allSettled([
        listProjectMembers(rootProjectId),
        canManage ? listProjectInvites(rootProjectId) : Promise.resolve([]),
      ]);
      if (!mountedRef.current || loadSeqRef.current !== requestSeq) return false;
      const errors: string[] = [];
      if (membersResult.status === "fulfilled") {
        setMembers(membersResult.value);
        setMembersLoadError(null);
      } else {
        const message = extractErrorMessage(membersResult.reason, t("novel.collaborationActions.membersLoadFailed"));
        setMembersLoadError(message);
        errors.push(message);
      }
      if (invitesResult.status === "fulfilled") {
        setInvites(invitesResult.value);
        setInvitesLoadError(null);
      } else {
        const message = extractErrorMessage(invitesResult.reason, t("novel.collaborationActions.invitesLoadFailed"));
        setInvitesLoadError(message);
        errors.push(message);
      }
      return errors.length === 0;
    } finally {
      if (mountedRef.current && loadSeqRef.current === requestSeq) {
        setLoading(false);
      }
    }
  }, [addToast, canManage, rootProjectId, t]);

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

  const beginAction = (key: string): boolean => {
    if (!mountedRef.current || busyActionKeysRef.current.has(key)) return false;
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

  const handleInvite = async () => {
    const value = identity.trim();
    if (!value || submittingRef.current) return;
    submittingRef.current = true;
    setSubmitting(true);
    try {
      await inviteProjectMember(rootProjectId, {
        username_or_email: value,
        role,
      });
      if (!mountedRef.current) return;
      setIdentity("");
      await load();
      await onRefresh();
      addToast(t("novel.collaborationActions.inviteSent"), "success");
    } catch (err) {
      if (mountedRef.current) {
        addToast(extractErrorMessage(err, t("novel.collaborationActions.inviteFailed")), "error");
      }
    } finally {
      submittingRef.current = false;
      if (mountedRef.current) setSubmitting(false);
    }
  };

  const handleRoleChange = async (member: ProjectMember, nextRole: ProjectMemberRole) => {
    const actionKey = `role:${member.user_id}`;
    if (!beginAction(actionKey)) return;
    try {
      await updateProjectMemberRole(rootProjectId, member.user_id, nextRole);
      if (!mountedRef.current) return;
      await load();
      await onRefresh();
      addToast(t("novel.collaborationActions.roleUpdated"), "success");
    } catch (err) {
      if (mountedRef.current) {
        addToast(extractErrorMessage(err, t("novel.collaborationActions.roleUpdateFailed")), "error");
      }
    } finally {
      finishAction(actionKey);
    }
  };

  const handleRemove = async (member: ProjectMember) => {
    const actionKey = `remove:${member.user_id}`;
    if (!beginAction(actionKey)) return;
    const ok = await confirmDialog({
      title: t("novel.collaborationActions.removeTitle"),
      message: t("novel.collaborationActions.removeMessage", { username: member.username }),
      confirmText: t("common.remove"),
      dialogType: "danger",
    });
    if (!ok) {
      finishAction(actionKey);
      return;
    }
    try {
      await removeProjectMember(rootProjectId, member.user_id);
      if (!mountedRef.current) return;
      await load();
      await onRefresh();
      addToast(t("novel.collaborationActions.removed"), "success");
    } catch (err) {
      if (mountedRef.current) {
        addToast(extractErrorMessage(err, t("novel.collaborationActions.removeFailed")), "error");
      }
    } finally {
      finishAction(actionKey);
    }
  };

  const handleRevoke = async (invite: ProjectInvite) => {
    const actionKey = `revoke:${invite.id}`;
    if (!beginAction(actionKey)) return;
    try {
      await revokeProjectInvite(rootProjectId, invite.id);
      if (!mountedRef.current) return;
      await load();
      await onRefresh();
      addToast(t("novel.collaborationActions.inviteRevoked"), "success");
    } catch (err) {
      if (mountedRef.current) {
        addToast(extractErrorMessage(err, t("novel.collaborationActions.revokeFailed")), "error");
      }
    } finally {
      finishAction(actionKey);
    }
  };

  const handleLeave = async () => {
    const actionKey = "leave";
    if (!beginAction(actionKey)) return;
    const ok = await confirmDialog({
      title: t("novel.collaborationActions.leaveTitle"),
      message: t("novel.collaborationActions.leaveMessage"),
      confirmText: t("novel.collaborationActions.leave"),
      dialogType: "danger",
    });
    if (!ok) {
      finishAction(actionKey);
      return;
    }
    try {
      await leaveProject(rootProjectId);
      if (!mountedRef.current) return;
      dirtyRegistry.set(dirtyKey, false);
      setIdentity("");
      addToast(t("novel.collaborationActions.left"), "success");
      navigate("/", { replace: true });
    } catch (err) {
      if (mountedRef.current) {
        addToast(extractErrorMessage(err, t("novel.collaborationActions.leaveFailed")), "error");
      }
    } finally {
      finishAction(actionKey);
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
                disabled: busyActionKeys.has("leave"),
                danger: true,
              },
            ]}
          />
        ) : null}
      </div>
      {membersLoadError ? <p role="alert" className="text-small text-error">{membersLoadError}</p> : null}
      {canManage && invitesLoadError ? <p role="alert" className="text-small text-error">{invitesLoadError}</p> : null}

      {canManage && (
        <div className="mb-3 grid grid-cols-[minmax(0,1fr)_auto] items-center gap-2 sm:grid-cols-[minmax(220px,1fr)_7rem_auto]">
          <input
            value={identity}
            onChange={(event) => setIdentity(event.target.value)}
            placeholder={t("novel.collaborationActions.identityPlaceholder")}
            className="pyqt-input col-span-2 h-9 min-w-0 sm:col-span-1"
          />
          <ThemedSelect
            value={role}
            onChange={(selectedValue) => setRole(selectedValue as ProjectMemberRole)}
            className="pyqt-input h-9 w-full"
          >
                {memberRoleOptions.map((item) => (
              <option key={item.value} value={item.value}>
                {item.label}
              </option>
            ))}
          </ThemedSelect>
          <BookButton
            variant="primary"
            size="sm"
            disabled={submitting || !identity.trim()}
            onClick={() => void handleInvite()}
          >
            <span className="inline-flex items-center gap-1.5">
              <UserPlus size={13} /> {t("novel.collaborationActions.invite")}
            </span>
          </BookButton>
        </div>
      )}

      <ul aria-label={t("novel.collaborationActions.membersAria")} className="space-y-2">
        {members.map((member) => (
          <li
            key={`${member.user_id}:${member.role}`}
            className="a4n-flat-surface grid grid-cols-[minmax(0,1fr)_auto] items-center gap-3 px-3 py-2.5"
          >
            <div className="min-w-0 flex-1">
              <div className="truncate text-xs font-semibold text-book-text-main">
                {member.username}
              </div>
              <div className="truncate text-small text-book-text-muted">{member.email}</div>
            </div>
            <div className="flex shrink-0 items-center gap-2">
              {canManage && member.role !== "owner" ? (
                <ThemedSelect
                  value={member.role}
                  aria-label={t("novel.collaborationActions.adjustRole", { username: member.username })}
                  disabled={busyActionKeys.has(`role:${member.user_id}`)}
                  onChange={(selectedValue) =>
                    void handleRoleChange(member, selectedValue as ProjectMemberRole)
                  }
                  className="pyqt-input h-8 w-32 text-small"
                >
                  {memberRoleOptions.map((item) => (
                    <option key={item.value} value={item.value}>
                      {item.label}
                    </option>
                  ))}
                </ThemedSelect>
              ) : (
                <span className="text-small text-book-text-muted">
                  {getProjectAccessRoleLabel(member.role, t)}
                </span>
              )}
              {canManage && member.role !== "owner" ? (
                <Dropdown
                  items={[
                    {
                      label: t("novel.collaborationActions.removeMember"),
                      onClick: () => void handleRemove(member),
                      disabled: busyActionKeys.has(`remove:${member.user_id}`),
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
                      disabled: busyActionKeys.has(`revoke:${invite.id}`),
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
