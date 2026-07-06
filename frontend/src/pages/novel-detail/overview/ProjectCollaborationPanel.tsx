import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { UserPlus, Users } from "lucide-react";
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
import { BookCard } from "../../../components/ui/BookCard";

interface ProjectCollaborationPanelProps {
  project: NovelProject;
  onRefresh: () => Promise<void>;
}

const ROLE_LABELS = {
  owner: "所有者",
  manager: "管理员",
  editor: "编辑者",
  viewer: "只读成员",
} as const;

const MEMBER_ROLE_OPTIONS: Array<{ value: ProjectMemberRole; label: string }> = [
  { value: "manager", label: "管理员" },
  { value: "editor", label: "编辑者" },
  { value: "viewer", label: "只读成员" },
];

export const ProjectCollaborationPanel: React.FC<ProjectCollaborationPanelProps> = ({
  project,
  onRefresh,
}) => {
  const { addToast } = useToast();
  const [members, setMembers] = useState<ProjectMember[]>([]);
  const [invites, setInvites] = useState<ProjectInvite[]>([]);
  const [identity, setIdentity] = useState("");
  const [role, setRole] = useState<ProjectMemberRole>("editor");
  const [loading, setLoading] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [busyActionKeys, setBusyActionKeys] = useState<ReadonlySet<string>>(new Set());
  const loadSeqRef = useRef(0);
  const busyActionKeysRef = useRef<Set<string>>(new Set());
  const submittingRef = useRef(false);
  const mountedRef = useRef(false);

  const canManage = project.my_role === "owner" || project.my_role === "manager";
  const canLeave = project.my_role != null && project.my_role !== "owner";
  const rootProjectId = project.collaboration_root_project_id;
  const visibleInvites = useMemo(
    () => invites.filter((invite) => invite.status === "pending"),
    [invites],
  );

  const load = useCallback(async () => {
    const requestSeq = loadSeqRef.current + 1;
    loadSeqRef.current = requestSeq;
    setLoading(true);
    try {
      const [nextMembers, nextInvites] = await Promise.all([
        listProjectMembers(rootProjectId),
        canManage ? listProjectInvites(rootProjectId) : Promise.resolve([]),
      ]);
      if (!mountedRef.current || loadSeqRef.current !== requestSeq) return;
      setMembers(nextMembers);
      setInvites(nextInvites);
    } catch (err) {
      if (mountedRef.current && loadSeqRef.current === requestSeq) {
        addToast(extractErrorMessage(err, "加载协作成员失败"), "error");
      }
    } finally {
      if (mountedRef.current && loadSeqRef.current === requestSeq) {
        setLoading(false);
      }
    }
  }, [addToast, canManage, rootProjectId]);

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
      addToast("邀请已发送", "success");
      await load();
      await onRefresh();
    } catch (err) {
      if (mountedRef.current) {
        addToast(extractErrorMessage(err, "邀请失败"), "error");
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
      addToast("成员角色已更新", "success");
      await load();
      await onRefresh();
    } catch (err) {
      if (mountedRef.current) {
        addToast(extractErrorMessage(err, "更新成员角色失败"), "error");
      }
    } finally {
      finishAction(actionKey);
    }
  };

  const handleRemove = async (member: ProjectMember) => {
    const actionKey = `remove:${member.user_id}`;
    if (!beginAction(actionKey)) return;
    const ok = await confirmDialog({
      title: "移除协作者",
      message: `确定要移除「${member.username}」吗？`,
      confirmText: "移除",
      dialogType: "danger",
    });
    if (!ok) {
      finishAction(actionKey);
      return;
    }
    try {
      await removeProjectMember(rootProjectId, member.user_id);
      if (!mountedRef.current) return;
      addToast("协作者已移除", "success");
      await load();
      await onRefresh();
    } catch (err) {
      if (mountedRef.current) {
        addToast(extractErrorMessage(err, "移除协作者失败"), "error");
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
      addToast("邀请已撤销", "success");
      await load();
      await onRefresh();
    } catch (err) {
      if (mountedRef.current) {
        addToast(extractErrorMessage(err, "撤销邀请失败"), "error");
      }
    } finally {
      finishAction(actionKey);
    }
  };

  const handleLeave = async () => {
    const actionKey = "leave";
    if (!beginAction(actionKey)) return;
    const ok = await confirmDialog({
      title: "离开项目",
      message: "离开后你将无法再访问该项目，除非项目成员重新邀请你。",
      confirmText: "离开项目",
      dialogType: "danger",
    });
    if (!ok) {
      finishAction(actionKey);
      return;
    }
    try {
      await leaveProject(rootProjectId);
      if (!mountedRef.current) return;
      addToast("已离开项目", "success");
      await onRefresh();
    } catch (err) {
      if (mountedRef.current) {
        addToast(extractErrorMessage(err, "离开项目失败"), "error");
      }
    } finally {
      finishAction(actionKey);
    }
  };

  return (
    <BookCard className="p-4">
      <div className="mb-3 flex items-center justify-between gap-3">
        <div className="flex items-center gap-2 min-w-0">
          <Users size={14} className="text-book-primary shrink-0" />
          <h3 className="text-sm font-serif font-semibold text-book-text-main">协作成员</h3>
          {loading && <span className="text-[11px] text-book-text-muted">加载中</span>}
        </div>
        {canLeave && (
          <BookButton
            variant="ghost"
            size="sm"
            onClick={() => void handleLeave()}
            disabled={busyActionKeys.has("leave")}
          >
            离开项目
          </BookButton>
        )}
      </div>

      {canManage && (
        <div className="mb-3 flex flex-wrap items-center gap-2">
          <input
            value={identity}
            onChange={(event) => setIdentity(event.target.value)}
            placeholder="用户名或邮箱"
            className="pyqt-input h-9 min-w-0 flex-1 sm:min-w-[220px]"
          />
          <select
            value={role}
            onChange={(event) => setRole(event.target.value as ProjectMemberRole)}
            className="pyqt-input h-9 w-28"
          >
            {MEMBER_ROLE_OPTIONS.map((item) => (
              <option key={item.value} value={item.value}>
                {item.label}
              </option>
            ))}
          </select>
          <BookButton
            variant="primary"
            size="sm"
            disabled={submitting || !identity.trim()}
            onClick={() => void handleInvite()}
          >
            <span className="inline-flex items-center gap-1.5">
              <UserPlus size={13} /> 邀请
            </span>
          </BookButton>
        </div>
      )}

      <div className="divide-y divide-book-border/40 border-y border-book-border/40">
        {members.map((member) => (
          <div key={`${member.user_id}:${member.role}`} className="flex items-center gap-3 py-2">
            <div className="min-w-0 flex-1">
              <div className="truncate text-xs font-semibold text-book-text-main">
                {member.username}
              </div>
              <div className="truncate text-[11px] text-book-text-muted">{member.email}</div>
            </div>
            {canManage && !member.is_owner ? (
              <select
                value={member.role}
                disabled={busyActionKeys.has(`role:${member.user_id}`)}
                onChange={(event) =>
                  void handleRoleChange(member, event.target.value as ProjectMemberRole)
                }
                className="pyqt-input h-8 w-28 text-[11px]"
              >
                {MEMBER_ROLE_OPTIONS.map((item) => (
                  <option key={item.value} value={item.value}>
                    {item.label}
                  </option>
                ))}
              </select>
            ) : (
              <span className="pyqt-pill !text-[10px]">
                {ROLE_LABELS[member.role] || member.role}
              </span>
            )}
            {canManage && !member.is_owner && (
              <button
                type="button"
                className="text-[11px] text-red-500 hover:underline"
                onClick={() => void handleRemove(member)}
                disabled={busyActionKeys.has(`remove:${member.user_id}`)}
              >
                移除
              </button>
            )}
          </div>
        ))}
      </div>

      {canManage && visibleInvites.length > 0 && (
        <div className="mt-3 space-y-2">
          <div className="text-[11px] font-semibold text-book-text-muted">待处理邀请</div>
          {visibleInvites.map((invite) => (
            <div
              key={invite.id}
              className="flex items-center justify-between gap-2 text-[11px] text-book-text-sub"
            >
              <span className="truncate">
                {invite.invitee_username || invite.invitee_email || `用户 ${invite.invitee_user_id}`} ·{" "}
                {ROLE_LABELS[invite.role]}
              </span>
              <button
                type="button"
                className="text-book-primary hover:underline"
                onClick={() => void handleRevoke(invite)}
                disabled={busyActionKeys.has(`revoke:${invite.id}`)}
              >
                撤销
              </button>
            </div>
          ))}
        </div>
      )}
    </BookCard>
  );
};
