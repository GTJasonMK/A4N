import React, { useCallback, useEffect, useRef, useState } from "react";
import { BarChart3, KeyRound, Plus, RefreshCw, Shield, UserX } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { extractErrorMessage } from "../../../api";
import {
  createInviteCode,
  createAdminUser,
  disableUser,
  getTokenSummary,
  getUserUsage,
  listInviteCodes,
  listAllUsers,
  updateUserRole,
  type CreateUserInput,
  type InviteCodeSummary,
  type UserSummary,
  type UsageLogSummary,
} from "../../../api/admin";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { promptDialog } from "../../../components/feedback/PromptDialog";
import { useToast } from "../../../components/feedback/Toast";
import { BookButton } from "../../../components/ui/BookButton";
import { Dropdown, type DropdownItem } from "../../../components/ui/Dropdown";
import { Modal } from "../../../components/ui/Modal";
import { useAdminSectionLoader } from "./useAdminSectionLoader";

const emptyCreateUserForm: CreateUserInput = {
  username: "",
  email: "",
  password: "",
  role: "user",
};
const USER_ROLES = ["user", "admin", "read_only"] as const;
const USER_ROLE_LABELS: Record<(typeof USER_ROLES)[number], string> = {
  user: "普通用户",
  admin: "管理员",
  read_only: "只读用户",
};

type UserRole = (typeof USER_ROLES)[number];

const isUserRole = (value: string): value is UserRole =>
  USER_ROLES.includes(value as UserRole);

const userRoleLabel = (value: string): string =>
  isUserRole(value) ? USER_ROLE_LABELS[value] : "未知角色";

const USER_ROLE_VALUES_BY_LABEL = Object.fromEntries(
  Object.entries(USER_ROLE_LABELS).map(([value, label]) => [label, value]),
) as Record<string, UserRole>;

const formatAdminDate = (value: string | null): string => {
  if (!value) return "—";
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) return "日期未知";
  return parsed.toLocaleString("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  });
};

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

export const UsersSection: React.FC<{ currentUserId: number }> = ({
  currentUserId,
}) => {
  const { addToast } = useToast();
  const [users, setUsers] = useState<UserSummary[]>([]);
  const [inviteCodes, setInviteCodes] = useState<InviteCodeSummary[]>([]);
  const [actingKeys, setActingKeys] = useState<Set<string>>(() => new Set());
  const [createOpen, setCreateOpen] = useState(false);
  const [createForm, setCreateForm] =
    useState<CreateUserInput>(emptyCreateUserForm);
  const [tokenFor, setTokenFor] = useState<number | null>(null);
  const [tokenInfo, setTokenInfo] = useState<unknown>(null);
  const [tokenLoadingFor, setTokenLoadingFor] = useState<number | null>(null);
  const [usageFor, setUsageFor] = useState<number | null>(null);
  const [usageInfo, setUsageInfo] = useState<UsageLogSummary[] | null>(null);
  const [usageLoadingFor, setUsageLoadingFor] = useState<number | null>(null);
  const actingKeysRef = useRef<Set<string>>(new Set());
  const tokenSeqRef = useRef(0);
  const usageSeqRef = useRef(0);
  const tokenInFlightForRef = useRef<number | null>(null);
  const usageInFlightForRef = useRef<number | null>(null);
  const createFormVersionRef = useRef(0);

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

  const loadUsers = useCallback(
    () => Promise.all([listAllUsers(), listInviteCodes()]),
    [],
  );
  const applyUsers = useCallback(
    ([nextUsers, nextInviteCodes]: [UserSummary[], InviteCodeSummary[]]) => {
      setUsers(nextUsers);
      setInviteCodes(nextInviteCodes);
    },
    [],
  );
  const { loading, load, isCurrentScope: isCurrentUser } =
    useAdminSectionLoader(
      currentUserId,
      loadUsers,
      applyUsers,
      "加载用户失败",
    );

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
  }, [currentUserId]);

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

  const updateCreateForm = (patch: Partial<CreateUserInput>) => {
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
    const requestCurrentUserId = currentUserId;
    const actionKey = "create-user";
    if (!tryStartAction(actionKey)) return;
    try {
      await createAdminUser(input);
      if (!isCurrentUser(requestCurrentUserId)) return;
      if (createFormVersionRef.current === requestFormVersion && createOpen) {
        addToast("用户已创建", "success");
        closeCreate();
      } else {
        addToast("已创建提交时的用户，当前保留后续编辑内容", "info");
      }
      await load();
    } catch (err) {
      if (!isCurrentUser(requestCurrentUserId)) return;
      addToast(extractErrorMessage(err, "创建用户失败"), "error");
    } finally {
      if (isCurrentUser(requestCurrentUserId)) finishAction(actionKey);
    }
  };

  const handleCreateInviteCode = async () => {
    const requestCurrentUserId = currentUserId;
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
      if (!isCurrentUser(requestCurrentUserId)) return;
      await createInviteCode({ code: trimmedCode });
      if (!isCurrentUser(requestCurrentUserId)) return;
      addToast("邀请码已创建", "success");
      await load();
    } catch (err) {
      if (!isCurrentUser(requestCurrentUserId)) return;
      addToast(extractErrorMessage(err, "创建邀请码失败"), "error");
    } finally {
      if (isCurrentUser(requestCurrentUserId)) finishAction(actionKey);
    }
  };

  const handleRole = async (u: UserSummary) => {
    const requestCurrentUserId = currentUserId;
    const actionKey = `role-${u.id}`;
    if (!tryStartAction(actionKey)) return;
    try {
      const next = await promptDialog({
        title: "修改用户角色",
        message: `将用户「${u.username}」的角色改为：`,
        defaultValue: userRoleLabel(u.role),
        placeholder: "管理员 / 普通用户 / 只读用户",
        required: true,
        confirmText: "更新角色",
      });
      if (!next || next === u.role) return;
      if (!isCurrentUser(requestCurrentUserId)) return;
      const inputRole = next.trim();
      const role = USER_ROLE_VALUES_BY_LABEL[inputRole] ?? inputRole;
      if (!isUserRole(role)) {
        addToast("角色必须是管理员、普通用户或只读用户", "error");
        return;
      }
      await updateUserRole(u.id, role);
      if (!isCurrentUser(requestCurrentUserId)) return;
      addToast("角色已更新", "success");
      await load();
    } catch (err) {
      if (!isCurrentUser(requestCurrentUserId)) return;
      addToast(extractErrorMessage(err, "更新失败"), "error");
    } finally {
      if (isCurrentUser(requestCurrentUserId)) finishAction(actionKey);
    }
  };

  const handleDel = async (u: UserSummary) => {
    if (u.id === currentUserId) {
      addToast("不能删除自己", "error");
      return;
    }
    const requestCurrentUserId = currentUserId;
    const actionKey = `del-${u.id}`;
    if (!tryStartAction(actionKey)) return;
    try {
      const ok = await confirmDialog({
        title: "禁用用户",
        message: `禁用账户「${u.username}」？该账户将无法继续登录，已有数据会保留。`,
        confirmText: "禁用",
        dialogType: "danger",
      });
      if (!ok) return;
      if (!isCurrentUser(requestCurrentUserId)) return;
      await disableUser(u.id);
      if (!isCurrentUser(requestCurrentUserId)) return;
      addToast("已禁用", "success");
      await load();
    } catch (err) {
      if (!isCurrentUser(requestCurrentUserId)) return;
      addToast(extractErrorMessage(err, "禁用失败"), "error");
    } finally {
      if (isCurrentUser(requestCurrentUserId)) finishAction(actionKey);
    }
  };

  const handleToken = async (u: UserSummary) => {
    if (tokenInFlightForRef.current === u.id) return;
    const requestCurrentUserId = currentUserId;
    const requestSeq = tokenSeqRef.current + 1;
    tokenSeqRef.current = requestSeq;
    tokenInFlightForRef.current = u.id;
    setTokenFor(u.id);
    setTokenInfo(null);
    setTokenLoadingFor(u.id);
    try {
      const nextTokenInfo = await getTokenSummary(u.id);
      if (
        !isCurrentUser(requestCurrentUserId) ||
        tokenSeqRef.current !== requestSeq
      )
        return;
      setTokenInfo(nextTokenInfo);
    } catch (err) {
      if (
        !isCurrentUser(requestCurrentUserId) ||
        tokenSeqRef.current !== requestSeq
      )
        return;
      addToast(extractErrorMessage(err, "加载 Token 失败"), "error");
    } finally {
      if (
        isCurrentUser(requestCurrentUserId) &&
        tokenSeqRef.current === requestSeq
      ) {
        tokenInFlightForRef.current = null;
        setTokenLoadingFor(null);
      }
    }
  };

  const handleUsage = async (u: UserSummary) => {
    if (usageInFlightForRef.current === u.id) return;
    const requestCurrentUserId = currentUserId;
    const requestSeq = usageSeqRef.current + 1;
    usageSeqRef.current = requestSeq;
    usageInFlightForRef.current = u.id;
    setUsageFor(u.id);
    setUsageInfo(null);
    setUsageLoadingFor(u.id);
    try {
      const data = await getUserUsage(u.id, 20);
      if (
        !isCurrentUser(requestCurrentUserId) ||
        usageSeqRef.current !== requestSeq
      )
        return;
      setUsageInfo(data);
    } catch (err) {
      if (
        !isCurrentUser(requestCurrentUserId) ||
        usageSeqRef.current !== requestSeq
      )
        return;
      addToast(extractErrorMessage(err, "加载用量失败"), "error");
    } finally {
      if (
        isCurrentUser(requestCurrentUserId) &&
        usageSeqRef.current === requestSeq
      ) {
        usageInFlightForRef.current = null;
        setUsageLoadingFor(null);
      }
    }
  };

  const tokenUserLabel = users.find((user) => user.id === tokenFor)?.username ?? "用户";
  const usageUserLabel = users.find((user) => user.id === usageFor)?.username ?? "用户";
  const availableInviteCount = inviteCodes.filter((code) => !code.used_at).length;

  const closeTokenSummary = () => {
    tokenSeqRef.current += 1;
    tokenInFlightForRef.current = null;
    setTokenFor(null);
    setTokenInfo(null);
    setTokenLoadingFor(null);
  };

  const closeUsageRecords = () => {
    usageSeqRef.current += 1;
    usageInFlightForRef.current = null;
    setUsageFor(null);
    setUsageInfo(null);
    setUsageLoadingFor(null);
  };

  const userActions = (u: UserSummary): DropdownItem[] => {
    const items: DropdownItem[] = [
      {
        label: "令牌摘要",
        icon: <KeyRound size={12} />,
        onClick: () => void handleToken(u),
      },
      {
        label: "查看用量",
        icon: <BarChart3 size={12} />,
        onClick: () => void handleUsage(u),
      },
      { type: "divider" },
      {
        label: "调整角色",
        icon: <Shield size={12} />,
        onClick: () => void handleRole(u),
      },
    ];
    if (u.id !== currentUserId && u.is_active !== false) {
      items.push({
        label: "禁用用户",
        icon: <UserX size={12} />,
        danger: true,
        onClick: () => void handleDel(u),
      });
    }
    return items;
  };
  const createBusy = isActing("create-user");
  const createReady =
    createForm.username.trim().length > 0 &&
    createForm.email.trim().length > 0 &&
    createForm.password.trim().length > 0;

  return (
    <div className="flex flex-col gap-3">
      <div className="flex min-h-9 flex-wrap items-center gap-2">
        <span className="text-xs text-book-text-muted">{users.length} 位用户</span>
        <span className="flex-1" />
        <BookButton size="sm" onClick={openCreate}>
          <Plus size={13} />
          新增用户
        </BookButton>
        <Dropdown
          label="更多"
          items={[
            {
              label: isActing("create-invite-code") ? "正在新增邀请码" : "新增邀请码",
              icon: <Plus size={12} />,
              disabled: isActing("create-invite-code"),
              onClick: () => void handleCreateInviteCode(),
            },
            {
              label: loading ? "正在刷新" : "刷新列表",
              icon: <RefreshCw size={12} className={loading ? "animate-spin" : undefined} />,
              disabled: loading,
              onClick: () => void load(),
            },
          ]}
        />
      </div>
      <ul className="space-y-2.5">
        {loading && users.length === 0 ? (
          <li className="a4n-flat-surface inline-flex w-full items-center gap-1.5 p-3 text-xs text-book-text-muted">
            <LoadingSpinner size="sm" />
            正在加载用户…
          </li>
        ) : users.length === 0 ? (
          <li className="a4n-empty-state a4n-empty-state--framed p-3 text-xs text-book-text-muted">
            暂无用户
          </li>
        ) : (
          users.map((u) => (
            <li key={u.id} className="a4n-flat-surface grid grid-cols-[minmax(0,1fr)_auto] items-start gap-3 p-3 text-xs">
              <div className="min-w-0">
                <div className="truncate font-medium text-book-text-main">{u.username}</div>
                <div className="mt-1 truncate text-book-text-muted">{u.email}</div>
                <div className="mt-0.5 text-small text-book-text-muted">
                  {userRoleLabel(u.role)}
                  {u.is_active === false ? " · 已禁用" : ""}
                  {u.last_login_at ? ` · 最近登录 ${formatAdminDate(u.last_login_at)}` : " · 尚未登录"}
                </div>
              </div>
              <div>
                  {isActing(`role-${u.id}`) || isActing(`del-${u.id}`) ? (
                    <LoadingSpinner size="sm" className="text-book-text-muted" />
                  ) : (
                    <Dropdown items={userActions(u)} />
                  )}
              </div>
            </li>
          ))
        )}
      </ul>
      <details className="a4n-flat-surface group overflow-hidden">
        <summary className="flex min-h-11 cursor-pointer list-none items-center gap-3 px-3 py-2.5 text-xs marker:content-none">
          <span className="font-medium text-book-text-main">邀请码</span>
          <span className="min-w-0 flex-1 truncate text-right text-book-text-muted">
            {availableInviteCount} 个可用 · 共 {inviteCodes.length} 个
          </span>
        </summary>
        <ul className="divide-y divide-book-border/30 border-t border-book-border/30">
          {loading && inviteCodes.length === 0 ? (
            <li className="inline-flex w-full items-center gap-1.5 p-3 text-xs text-book-text-muted">
              <LoadingSpinner size="sm" />
              正在加载邀请码…
            </li>
          ) : inviteCodes.length === 0 ? (
            <li className="p-3 text-xs text-book-text-muted">暂无邀请码</li>
          ) : (
            inviteCodes.map((code) => (
              <li key={code.id} className="grid gap-1 px-3 py-2.5 text-xs sm:grid-cols-[minmax(0,1fr)_auto] sm:items-center sm:gap-3">
                <span className="min-w-0 break-all font-mono text-book-text-main">{code.code}</span>
                <span className={code.used_at ? "text-book-text-muted" : "text-book-primary"}>
                  {code.used_at ? "已使用" : "可使用"} · {formatAdminDate(code.created_at)}
                </span>
              </li>
            ))
          )}
        </ul>
      </details>
      <Modal
        isOpen={tokenFor != null}
        onClose={closeTokenSummary}
        title={`${tokenUserLabel} · 令牌摘要`}
        maxWidthClassName="max-w-xl"
      >
        <TokenSummaryPanel data={tokenInfo} loading={tokenLoadingFor === tokenFor} />
      </Modal>
      <Modal
        isOpen={usageFor != null}
        onClose={closeUsageRecords}
        title={`${usageUserLabel} · 最近用量`}
        maxWidthClassName="max-w-2xl"
      >
        <UsageRecordsPanel data={usageInfo} loading={usageLoadingFor === usageFor} />
      </Modal>
      <Modal
        isOpen={createOpen}
        onClose={() => {
          if (!createBusy) closeCreate();
        }}
        title="新增用户"
        subtitle="管理员创建账号后，用户使用用户名和密码登录。"
        icon={<Plus size={15} />}
        maxWidthClassName="max-w-lg"
        closeOnBackdrop={!createBusy}
        closeOnEscape={!createBusy}
        showCloseButton={!createBusy}
        footer={
          <BookButton
            variant="primary"
            onClick={() => void handleCreate()}
            disabled={createBusy || !createReady}
          >
            {createBusy ? "创建中…" : "创建用户"}
          </BookButton>
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
              disabled={createBusy}
            />
          </label>
          <label className="grid gap-1">
            <span className="text-book-text-muted">邮箱</span>
            <input
              className="pyqt-input"
              value={createForm.email}
              onChange={(e) => updateCreateForm({ email: e.target.value })}
              autoComplete="off"
              disabled={createBusy}
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
              disabled={createBusy}
            />
          </label>
          <label className="grid gap-1">
            <span className="text-book-text-muted">角色</span>
            <select
              className="pyqt-input"
              value={createForm.role}
              onChange={(e) => updateCreateForm({ role: e.target.value })}
              disabled={createBusy}
            >
              {USER_ROLES.map((role) => (
                <option key={role} value={role}>
                  {USER_ROLE_LABELS[role]}
                </option>
              ))}
            </select>
          </label>
        </div>
      </Modal>
    </div>
  );
};

const TokenSummaryPanel: React.FC<{
  data: unknown;
  loading: boolean;
}> = ({ data, loading }) => {
  const record = data && typeof data === "object" && !Array.isArray(data)
    ? data as Record<string, unknown>
    : null;
  const value = (key: string): number | null => {
    const item = record?.[key];
    return typeof item === "number" && Number.isFinite(item) ? item : null;
  };
  const items = [
    ["调用", value("total_calls")],
    ["输入令牌", value("total_prompt_tokens")],
    ["输出令牌", value("total_completion_tokens")],
    ["总令牌", value("total_tokens")],
    ["成功", value("success_count")],
    ["失败", value("fail_count")],
  ] as const;

  return (
    <div className="overflow-hidden">
      {loading && !record ? (
        <div className="inline-flex items-center gap-1.5 px-3 py-3 text-small text-book-text-muted">
          <LoadingSpinner size="sm" /> 正在加载数据…
        </div>
      ) : !record ? (
        <div className="px-3 py-3 text-small text-book-text-muted">暂无数据</div>
      ) : (
        <>
          <dl className="grid grid-cols-2 divide-x divide-y divide-book-border/30 sm:grid-cols-3">
            {items.map(([label, item]) => (
              <div key={label} className="px-3 py-2.5">
                <dt className="text-caption text-book-text-muted">{label}</dt>
                <dd className="mt-0.5 font-mono text-sm font-semibold text-book-text-main">
                  {item == null ? "—" : item.toLocaleString()}
                </dd>
              </div>
            ))}
          </dl>
          <details className="border-t border-book-border/30 px-3 py-2 text-caption text-book-text-muted">
            <summary className="cursor-pointer">诊断信息</summary>
            <pre className="mt-2 whitespace-pre-wrap break-words font-mono">
              {JSON.stringify(data, null, 2)}
            </pre>
          </details>
        </>
      )}
    </div>
  );
};

const UsageRecordsPanel: React.FC<{
  data: UsageLogSummary[] | null;
  loading: boolean;
}> = ({ data, loading }) => (
  <div className="overflow-hidden">
    {loading && data == null ? (
      <div className="inline-flex items-center gap-1.5 px-3 py-3 text-small text-book-text-muted">
        <LoadingSpinner size="sm" /> 正在加载数据…
      </div>
    ) : !data?.length ? (
      <div className="px-3 py-3 text-small text-book-text-muted">暂无调用记录</div>
    ) : (
      <ul className="divide-y divide-book-border/30">
        {data.map((item, index) => (
          <li key={item.id} className="px-3 py-2.5 text-small">
            <div className="flex min-w-0 items-center gap-2">
              <span className="min-w-0 flex-1 truncate font-medium text-book-text-main">
                {item.model || `调用 ${index + 1}`}
              </span>
              <span className="shrink-0 font-mono text-book-text-sub">
                {item.total_tokens.toLocaleString()} 令牌
              </span>
            </div>
            <div className="mt-1 flex flex-wrap gap-x-3 gap-y-0.5 text-book-text-muted">
              <span>输入 {item.prompt_tokens.toLocaleString()}</span>
              <span>输出 {item.completion_tokens.toLocaleString()}</span>
              <span>{formatAdminDate(item.created_at)}</span>
            </div>
          </li>
        ))}
      </ul>
    )}
  </div>
);
