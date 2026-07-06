import React, { useCallback, useEffect, useRef, useState } from "react";
import { Loader2, Plus, RefreshCw } from "lucide-react";
import { extractErrorMessage } from "../../../api";
import {
  createInviteCode,
  createAdminUser,
  deleteUser,
  getTokenSummary,
  getUserUsage,
  listInviteCodes,
  listAllUsers,
  updateUserRole,
  type AdminInviteCode,
  type AdminUser,
  type CreateAdminUserInput,
} from "../../../api/admin";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { promptDialog } from "../../../components/feedback/PromptDialog";
import { useToast } from "../../../components/feedback/Toast";
import { BookButton, BookIconButton } from "../../../components/ui/BookButton";
import { Modal } from "../../../components/ui/Modal";
import { JsonCard } from "./JsonCard";
import { useAdminSectionLoader } from "./useAdminSectionLoader";

const emptyCreateUserForm: CreateAdminUserInput = {
  username: "",
  email: "",
  password: "",
  role: "user",
};
const USER_ROLES = ["user", "admin", "read_only"] as const;

type UserRole = (typeof USER_ROLES)[number];

const isUserRole = (value: string): value is UserRole =>
  USER_ROLES.includes(value as UserRole);

const inviteAlphabet = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

function generateInviteCode(): string {
  if (!globalThis.crypto?.getRandomValues) {
    return `A4N-${Date.now().toString(36).toUpperCase()}`;
  }
  const bytes = new Uint8Array(16);
  globalThis.crypto.getRandomValues(bytes);
  const raw = Array.from(
    bytes,
    (byte) => inviteAlphabet[byte % inviteAlphabet.length],
  ).join("");
  return `A4N-${raw.match(/.{1,4}/g)?.join("-") ?? raw}`;
}

export const UsersSection: React.FC<{ callerUserId: number }> = ({
  callerUserId,
}) => {
  const { addToast } = useToast();
  const [users, setUsers] = useState<AdminUser[]>([]);
  const [inviteCodes, setInviteCodes] = useState<AdminInviteCode[]>([]);
  const [actingKeys, setActingKeys] = useState<Set<string>>(() => new Set());
  const [createOpen, setCreateOpen] = useState(false);
  const [createForm, setCreateForm] =
    useState<CreateAdminUserInput>(emptyCreateUserForm);
  const [tokenFor, setTokenFor] = useState<number | null>(null);
  const [tokenInfo, setTokenInfo] = useState<unknown>(null);
  const [tokenLoadingFor, setTokenLoadingFor] = useState<number | null>(null);
  const [usageFor, setUsageFor] = useState<number | null>(null);
  const [usageInfo, setUsageInfo] = useState<unknown[] | null>(null);
  const [usageLoadingFor, setUsageLoadingFor] = useState<number | null>(null);
  const activeCallerUserIdRef = useRef(callerUserId);
  const mountedRef = useRef(true);
  const actingKeysRef = useRef<Set<string>>(new Set());
  const loadSeqRef = useRef(0);
  const tokenSeqRef = useRef(0);
  const usageSeqRef = useRef(0);
  const tokenInFlightForRef = useRef<number | null>(null);
  const usageInFlightForRef = useRef<number | null>(null);
  const createFormVersionRef = useRef(0);

  activeCallerUserIdRef.current = callerUserId;

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const isCurrentCaller = useCallback((requestCallerUserId: number) => {
    return (
      mountedRef.current &&
      activeCallerUserIdRef.current === requestCallerUserId
    );
  }, []);

  const isCurrentLoad = useCallback(
    (requestCallerUserId: number, seq: number) => {
      return isCurrentCaller(requestCallerUserId) && loadSeqRef.current === seq;
    },
    [isCurrentCaller],
  );

  const isActing = useCallback(
    (key: string) => actingKeys.has(key),
    [actingKeys],
  );

  const tryStartAction = useCallback((key: string) => {
    if (actingKeysRef.current.has(key)) return false;
    const next = new Set(actingKeysRef.current);
    next.add(key);
    actingKeysRef.current = next;
    setActingKeys(next);
    return true;
  }, []);

  const finishAction = useCallback((key: string) => {
    if (!actingKeysRef.current.has(key)) return;
    const next = new Set(actingKeysRef.current);
    next.delete(key);
    actingKeysRef.current = next;
    setActingKeys(next);
  }, []);

  const loadUsers = useCallback(async () => {
    const requestCallerUserId = callerUserId;
    const requestSeq = loadSeqRef.current + 1;
    loadSeqRef.current = requestSeq;
    try {
      const [nextUsers, nextInviteCodes] = await Promise.all([
        listAllUsers(requestCallerUserId),
        listInviteCodes(requestCallerUserId),
      ]);
      if (!isCurrentLoad(requestCallerUserId, requestSeq)) return;
      setUsers(nextUsers);
      setInviteCodes(nextInviteCodes);
    } catch (err) {
      if (!isCurrentLoad(requestCallerUserId, requestSeq)) return;
      throw err;
    }
  }, [callerUserId, isCurrentLoad]);
  const { loading, load } = useAdminSectionLoader(loadUsers, "加载用户失败");

  useEffect(() => {
    actingKeysRef.current = new Set();
    setActingKeys(new Set());
    setCreateOpen(false);
    setCreateForm(emptyCreateUserForm);
    createFormVersionRef.current += 1;
    setTokenFor(null);
    setTokenInfo(null);
    setTokenLoadingFor(null);
    tokenInFlightForRef.current = null;
    setUsageFor(null);
    setUsageInfo(null);
    setUsageLoadingFor(null);
    usageInFlightForRef.current = null;
  }, [callerUserId]);

  useEffect(() => {
    if (tokenFor != null && !users.some((user) => user.id === tokenFor)) {
      tokenSeqRef.current += 1;
      setTokenFor(null);
      setTokenInfo(null);
      setTokenLoadingFor(null);
      tokenInFlightForRef.current = null;
    }
    if (usageFor != null && !users.some((user) => user.id === usageFor)) {
      usageSeqRef.current += 1;
      setUsageFor(null);
      setUsageInfo(null);
      setUsageLoadingFor(null);
      usageInFlightForRef.current = null;
    }
  }, [tokenFor, usageFor, users]);

  const closeCreate = () => {
    setCreateOpen(false);
    setCreateForm(emptyCreateUserForm);
    createFormVersionRef.current += 1;
  };

  const openCreate = () => {
    setCreateOpen(true);
    createFormVersionRef.current += 1;
  };

  const updateCreateForm = (patch: Partial<CreateAdminUserInput>) => {
    createFormVersionRef.current += 1;
    setCreateForm((prev) => ({ ...prev, ...patch }));
  };

  const handleCreate = async () => {
    const requestFormVersion = createFormVersionRef.current;
    const input = {
      username: createForm.username.trim(),
      email: createForm.email.trim(),
      password: createForm.password,
      role: createForm.role,
    };
    if (!input.username || !input.email || !input.password.trim()) {
      addToast("用户名、邮箱、密码不能为空", "error");
      return;
    }
    const requestCallerUserId = callerUserId;
    const actionKey = "create-user";
    if (!tryStartAction(actionKey)) return;
    try {
      await createAdminUser(requestCallerUserId, input);
      if (!isCurrentCaller(requestCallerUserId)) return;
      if (createFormVersionRef.current === requestFormVersion && createOpen) {
        addToast("用户已创建", "success");
        closeCreate();
      } else {
        addToast("已创建提交时的用户，当前保留后续编辑内容", "info");
      }
      await load();
    } catch (err) {
      if (!isCurrentCaller(requestCallerUserId)) return;
      addToast(extractErrorMessage(err, "创建用户失败"), "error");
    } finally {
      if (isCurrentCaller(requestCallerUserId)) finishAction(actionKey);
    }
  };

  const handleCreateInviteCode = async () => {
    const requestCallerUserId = callerUserId;
    const actionKey = "create-invite-code";
    if (!tryStartAction(actionKey)) return;
    try {
      const code = await promptDialog({
        title: "新增邀请码",
        message: "邀请码：",
        defaultValue: generateInviteCode(),
        placeholder: "A4N-XXXX-XXXX-XXXX-XXXX",
        required: true,
        confirmText: "创建邀请码",
      });
      if (!code) return;
      const trimmedCode = code.trim();
      if (!trimmedCode) {
        addToast("邀请码不能为空", "error");
        return;
      }
      if (!isCurrentCaller(requestCallerUserId)) return;
      await createInviteCode(requestCallerUserId, { code: trimmedCode });
      if (!isCurrentCaller(requestCallerUserId)) return;
      addToast("邀请码已创建", "success");
      await load();
    } catch (err) {
      if (!isCurrentCaller(requestCallerUserId)) return;
      addToast(extractErrorMessage(err, "创建邀请码失败"), "error");
    } finally {
      if (isCurrentCaller(requestCallerUserId)) finishAction(actionKey);
    }
  };

  const handleRole = async (u: AdminUser) => {
    const requestCallerUserId = callerUserId;
    const actionKey = `role-${u.id}`;
    if (!tryStartAction(actionKey)) return;
    try {
      const next = await promptDialog({
        title: "修改用户角色",
        message: `将用户「${u.username}」的角色改为：`,
        defaultValue: u.role,
        placeholder: "admin / user / read_only",
        required: true,
        confirmText: "更新角色",
      });
      if (!next || next === u.role) return;
      if (!isCurrentCaller(requestCallerUserId)) return;
      const role = next.trim();
      if (!isUserRole(role)) {
        addToast(`角色必须是 ${USER_ROLES.join(" / ")}`, "error");
        return;
      }
      await updateUserRole(requestCallerUserId, u.id, role);
      if (!isCurrentCaller(requestCallerUserId)) return;
      addToast("角色已更新", "success");
      await load();
    } catch (err) {
      if (!isCurrentCaller(requestCallerUserId)) return;
      addToast(extractErrorMessage(err, "更新失败"), "error");
    } finally {
      if (isCurrentCaller(requestCallerUserId)) finishAction(actionKey);
    }
  };

  const handleDel = async (u: AdminUser) => {
    if (u.id === callerUserId) {
      addToast("不能删除自己", "error");
      return;
    }
    const requestCallerUserId = callerUserId;
    const actionKey = `del-${u.id}`;
    if (!tryStartAction(actionKey)) return;
    try {
      const ok = await confirmDialog({
        title: "禁用用户",
        message: `禁用账户「${u.username}」？（软删除：is_active=false）`,
        confirmText: "禁用",
        dialogType: "danger",
      });
      if (!ok) return;
      if (!isCurrentCaller(requestCallerUserId)) return;
      await deleteUser(requestCallerUserId, u.id);
      if (!isCurrentCaller(requestCallerUserId)) return;
      addToast("已禁用", "success");
      await load();
    } catch (err) {
      if (!isCurrentCaller(requestCallerUserId)) return;
      addToast(extractErrorMessage(err, "禁用失败"), "error");
    } finally {
      if (isCurrentCaller(requestCallerUserId)) finishAction(actionKey);
    }
  };

  const handleToken = async (u: AdminUser) => {
    if (tokenInFlightForRef.current === u.id) return;
    const requestCallerUserId = callerUserId;
    const requestSeq = tokenSeqRef.current + 1;
    tokenSeqRef.current = requestSeq;
    tokenInFlightForRef.current = u.id;
    setTokenFor(u.id);
    setTokenInfo(null);
    setTokenLoadingFor(u.id);
    try {
      const nextTokenInfo = await getTokenSummary(requestCallerUserId, u.id);
      if (
        !isCurrentCaller(requestCallerUserId) ||
        tokenSeqRef.current !== requestSeq
      )
        return;
      setTokenInfo(nextTokenInfo);
    } catch (err) {
      if (
        !isCurrentCaller(requestCallerUserId) ||
        tokenSeqRef.current !== requestSeq
      )
        return;
      addToast(extractErrorMessage(err, "加载 Token 失败"), "error");
    } finally {
      if (
        isCurrentCaller(requestCallerUserId) &&
        tokenSeqRef.current === requestSeq
      ) {
        tokenInFlightForRef.current = null;
        setTokenLoadingFor(null);
      }
    }
  };

  const handleUsage = async (u: AdminUser) => {
    if (usageInFlightForRef.current === u.id) return;
    const requestCallerUserId = callerUserId;
    const requestSeq = usageSeqRef.current + 1;
    usageSeqRef.current = requestSeq;
    usageInFlightForRef.current = u.id;
    setUsageFor(u.id);
    setUsageInfo(null);
    setUsageLoadingFor(u.id);
    try {
      const data = await getUserUsage(requestCallerUserId, u.id, 20);
      if (
        !isCurrentCaller(requestCallerUserId) ||
        usageSeqRef.current !== requestSeq
      )
        return;
      setUsageInfo(data);
    } catch (err) {
      if (
        !isCurrentCaller(requestCallerUserId) ||
        usageSeqRef.current !== requestSeq
      )
        return;
      addToast(extractErrorMessage(err, "加载用量失败"), "error");
    } finally {
      if (
        isCurrentCaller(requestCallerUserId) &&
        usageSeqRef.current === requestSeq
      ) {
        usageInFlightForRef.current = null;
        setUsageLoadingFor(null);
      }
    }
  };

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center justify-between">
        <h4 className="text-sm font-serif text-book-text-main">
          邀请码（{inviteCodes.length}）
        </h4>
        <BookIconButton
          label="新增邀请码"
          onClick={() => void handleCreateInviteCode()}
          variant="secondary"
          size="sm"
          disabled={isActing("create-invite-code")}
        >
          <Plus size={14} />
        </BookIconButton>
      </div>
      <ul className="pyqt-card divide-y divide-book-border/30">
        {loading && inviteCodes.length === 0 ? (
          <li className="p-3 text-xs text-book-text-muted inline-flex items-center gap-1.5">
            <Loader2 size={12} className="animate-spin" />
            正在加载邀请码…
          </li>
        ) : inviteCodes.length === 0 ? (
          <li className="p-3 text-xs text-book-text-muted">暂无邀请码</li>
        ) : (
          inviteCodes.map((code) => (
            <li key={code.id} className="p-3 flex items-center gap-2 text-xs">
              <span className="font-mono text-book-text-muted shrink-0 w-8">
                #{code.id}
              </span>
              <span className="font-mono text-book-text-main shrink-0">
                {code.code}
              </span>
              <span
                className={`pyqt-pill !text-[10px] shrink-0 ${
                  code.used_at
                    ? "!text-book-text-muted"
                    : "!border-book-primary !text-book-primary"
                }`}
              >
                {code.used_at ? "已使用" : "未使用"}
              </span>
              <span className="text-book-text-muted shrink-0">
                创建：{code.created_at ?? "-"}
              </span>
              {code.used_by_user_id != null && (
                <span className="text-book-text-muted shrink-0">
                  使用者：#{code.used_by_user_id}
                </span>
              )}
            </li>
          ))
        )}
      </ul>
      <div className="flex items-center justify-between">
        <h4 className="text-sm font-serif text-book-text-main">
          用户列表（{users.length}）
        </h4>
        <div className="flex items-center gap-2">
          <BookIconButton
            label="新增用户"
            onClick={openCreate}
            variant="secondary"
            size="sm"
          >
            <Plus size={14} />
          </BookIconButton>
          <BookIconButton
            label="刷新"
            onClick={load}
            variant="secondary"
            size="sm"
            disabled={loading}
          >
            <RefreshCw
              size={14}
              className={loading ? "animate-spin" : undefined}
            />
          </BookIconButton>
        </div>
      </div>
      <ul className="pyqt-card divide-y divide-book-border/30">
        {loading && users.length === 0 ? (
          <li className="p-3 text-xs text-book-text-muted inline-flex items-center gap-1.5">
            <Loader2 size={12} className="animate-spin" />
            正在加载用户…
          </li>
        ) : users.length === 0 ? (
          <li className="p-3 text-xs text-book-text-muted">暂无用户</li>
        ) : (
          users.map((u) => (
            <li key={u.id} className="p-3 flex flex-col gap-2 text-xs">
              <div className="flex min-w-0 items-center gap-2">
                <span className="font-mono text-book-text-muted shrink-0 w-8">
                  #{u.id}
                </span>
                <span className="font-semibold text-book-text-main shrink-0">
                  {u.username}
                </span>
                <span className="min-w-0 truncate text-book-text-muted">
                  {u.email}
                </span>
                <span
                  className={`pyqt-pill !text-[10px] shrink-0 ${
                    u.role === "admin"
                      ? "!border-book-primary !text-book-primary"
                      : u.role === "read_only"
                        ? "!text-book-text-muted"
                        : ""
                  }`}
                >
                  {u.role}
                </span>
                {u.is_active === false && (
                  <span className="pyqt-pill !text-[10px] !border-error/50 !text-error shrink-0">
                    已禁用
                  </span>
                )}
                <span className="flex-1" />
                <button
                  type="button"
                  onClick={() => handleToken(u)}
                  className="text-[11px] text-book-text-muted hover:text-book-text-main px-2 py-0.5 rounded hover:bg-book-bg-tertiary"
                >
                  Token
                </button>
                <button
                  type="button"
                  onClick={() => handleUsage(u)}
                  className="text-[11px] text-book-text-muted hover:text-book-text-main px-2 py-0.5 rounded hover:bg-book-bg-tertiary"
                >
                  用量
                </button>
                <button
                  type="button"
                  onClick={() => void handleRole(u)}
                  disabled={isActing(`role-${u.id}`)}
                  className="text-[11px] text-book-primary px-2 py-0.5 rounded hover:bg-book-primary/10"
                >
                  {isActing(`role-${u.id}`) ? "…" : "改角色"}
                </button>
                <button
                  type="button"
                  onClick={() => void handleDel(u)}
                  disabled={isActing(`del-${u.id}`)}
                  className="text-[11px] text-error px-2 py-0.5 rounded hover:bg-error-bg"
                >
                  {isActing(`del-${u.id}`) ? "…" : "禁用"}
                </button>
              </div>
            </li>
          ))
        )}
      </ul>
      {tokenFor != null &&
        (tokenLoadingFor === tokenFor || tokenInfo != null) && (
          <JsonCard
            title={`#${tokenFor} Token 摘要`}
            data={tokenInfo}
            loading={tokenLoadingFor === tokenFor}
          />
        )}
      {usageFor != null &&
        (usageLoadingFor === usageFor || usageInfo != null) && (
          <JsonCard
            title={`#${usageFor} 最近 20 条 Token 记录`}
            data={usageInfo}
            loading={usageLoadingFor === usageFor}
          />
        )}
      <Modal
        isOpen={createOpen}
        onClose={closeCreate}
        title="新增用户"
        subtitle="管理员创建账号后，用户使用用户名和密码登录。"
        icon={<Plus size={15} />}
        maxWidthClassName="max-w-lg"
        footer={
          <>
            <BookButton
              variant="secondary"
              onClick={closeCreate}
              disabled={isActing("create-user")}
            >
              取消
            </BookButton>
            <BookButton
              variant="primary"
              onClick={() => void handleCreate()}
              disabled={isActing("create-user")}
            >
              {isActing("create-user") ? "创建中…" : "创建用户"}
            </BookButton>
          </>
        }
      >
        <div className="grid gap-3 text-xs">
          <label className="grid gap-1">
            <span className="text-book-text-muted">用户名</span>
            <input
              className="pyqt-input"
              value={createForm.username}
              onChange={(e) => updateCreateForm({ username: e.target.value })}
              autoComplete="off"
            />
          </label>
          <label className="grid gap-1">
            <span className="text-book-text-muted">邮箱</span>
            <input
              className="pyqt-input"
              value={createForm.email}
              onChange={(e) => updateCreateForm({ email: e.target.value })}
              autoComplete="off"
            />
          </label>
          <label className="grid gap-1">
            <span className="text-book-text-muted">初始密码</span>
            <input
              className="pyqt-input"
              type="password"
              value={createForm.password}
              onChange={(e) => updateCreateForm({ password: e.target.value })}
              autoComplete="new-password"
            />
          </label>
          <label className="grid gap-1">
            <span className="text-book-text-muted">角色</span>
            <select
              className="pyqt-input"
              value={createForm.role}
              onChange={(e) => updateCreateForm({ role: e.target.value })}
            >
              {USER_ROLES.map((role) => (
                <option key={role} value={role}>
                  {role}
                </option>
              ))}
            </select>
          </label>
        </div>
      </Modal>
    </div>
  );
};
