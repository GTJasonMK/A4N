import { ThemedSelect } from "../../../components/ui/ThemedSelect";
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
import { formatDateTime, formatNumber, useI18n, type MessageKey } from "../../../i18n";
import { useAuthStore } from "../../../store/auth";

const emptyCreateUserForm: CreateUserInput = {
  username: "",
  email: "",
  password: "",
  role: "user",
};
const USER_ROLES = ["user", "admin", "read_only"] as const;
const USER_ROLE_LABEL_KEYS: Record<(typeof USER_ROLES)[number], MessageKey> = {
  user: "settingsAdmin.roleUser",
  admin: "settingsAdmin.roleAdmin",
  read_only: "settingsAdmin.roleReadOnly",
};

type UserRole = (typeof USER_ROLES)[number];

const isUserRole = (value: string): value is UserRole =>
  USER_ROLES.includes(value as UserRole);

const userRoleLabel = (value: string, t: ReturnType<typeof useI18n>["t"]): string =>
  isUserRole(value) ? t(USER_ROLE_LABEL_KEYS[value]) : t("settingsAdmin.unknownRole");

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
  const { t, locale } = useI18n();
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
  const { loading, error, load, isCurrentScope: isCurrentUser } =
    useAdminSectionLoader(
      currentUserId,
      loadUsers,
      applyUsers,
      t("settingsAdmin.loadUsersFailed"),
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

  const resetCreate = () => {
    setCreateOpen(false);
    setCreateForm(emptyCreateUserForm);
    createFormVersionRef.current += 1;
  };

  const createDirty =
    createForm.username.trim().length > 0 ||
    createForm.email.trim().length > 0 ||
    createForm.password.length > 0 ||
    createForm.role !== emptyCreateUserForm.role;

  const closeCreate = async () => {
    if (createBusy) return;
    if (createDirty) {
      const ok = await confirmDialog({
        title: t("common.unsavedChangesTitle"),
        message: t("common.unsavedChangesMessage", {
          name: t("settingsAdmin.createUser"),
          action: t("common.close"),
        }),
        confirmText: t("common.discardDraft"),
        dialogType: "warning",
      });
      if (!ok) return;
    }
    resetCreate();
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
      addToast(t("settingsAdmin.userFieldsRequired"), "error");
      return;
    }
    const requestCurrentUserId = currentUserId;
    const actionKey = "create-user";
    if (!tryStartAction(actionKey)) return;
    try {
      await createAdminUser(input);
      if (!isCurrentUser(requestCurrentUserId)) return;
      if (createFormVersionRef.current === requestFormVersion && createOpen) {
        addToast(t("settingsAdmin.userCreated"), "success");
        resetCreate();
      } else {
        addToast(t("settingsAdmin.userCreatedWithDraft"), "info");
      }
      await load();
    } catch (err) {
      if (!isCurrentUser(requestCurrentUserId)) return;
      addToast(extractErrorMessage(err, t("settingsAdmin.createUserFailed")), "error");
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
        title: t("settingsAdmin.createInviteCode"),
        message: t("settingsAdmin.inviteCodeLabel"),
        defaultValue: generateInviteCode(),
        placeholder: "A4N-XXXX-XXXX-XXXX-XXXX",
        required: true,
        confirmText: t("settingsAdmin.createInviteCode"),
      });
      if (!code) return;
      const trimmedCode = code.trim();
      if (!trimmedCode) {
        addToast(t("settingsAdmin.inviteCodeRequired"), "error");
        return;
      }
      if (!isCurrentUser(requestCurrentUserId)) return;
      await createInviteCode({ code: trimmedCode });
      if (!isCurrentUser(requestCurrentUserId)) return;
      addToast(t("settingsAdmin.inviteCreated"), "success");
      await load();
    } catch (err) {
      if (!isCurrentUser(requestCurrentUserId)) return;
      addToast(extractErrorMessage(err, t("settingsAdmin.createInviteFailed")), "error");
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
        title: t("settingsAdmin.changeUserRole"),
        message: t("settingsAdmin.changeUserRoleMessage", { name: u.username }),
        defaultValue: userRoleLabel(u.role, t),
        placeholder: t("settingsAdmin.rolePlaceholder"),
        required: true,
        confirmText: t("settingsAdmin.updateRole"),
      });
      if (!next || next === u.role) return;
      if (!isCurrentUser(requestCurrentUserId)) return;
      const inputRole = next.trim();
      const roleValuesByLabel = Object.fromEntries(
        USER_ROLES.map((role) => [t(USER_ROLE_LABEL_KEYS[role]), role]),
      ) as Record<string, UserRole>;
      const role = roleValuesByLabel[inputRole] ?? inputRole;
      if (!isUserRole(role)) {
        addToast(t("settingsAdmin.invalidRole"), "error");
        return;
      }
      const updated = await updateUserRole(u.id, role);
      if (!isCurrentUser(requestCurrentUserId)) return;
      addToast(t("settingsAdmin.roleUpdated"), "success");
      if (updated.id === requestCurrentUserId) {
        useAuthStore.setState({ user: updated });
        if (updated.role !== "admin") return;
      }
      await load();
    } catch (err) {
      if (!isCurrentUser(requestCurrentUserId)) return;
      addToast(extractErrorMessage(err, t("settingsAdmin.updateFailed")), "error");
    } finally {
      if (isCurrentUser(requestCurrentUserId)) finishAction(actionKey);
    }
  };

  const handleDel = async (u: UserSummary) => {
    if (u.id === currentUserId) {
      addToast(t("settingsAdmin.cannotDisableSelf"), "error");
      return;
    }
    const requestCurrentUserId = currentUserId;
    const actionKey = `del-${u.id}`;
    if (!tryStartAction(actionKey)) return;
    try {
      const ok = await confirmDialog({
        title: t("settingsAdmin.disableUser"),
        message: t("settingsAdmin.disableUserMessage", { name: u.username }),
        confirmText: t("settingsAdmin.disable"),
        dialogType: "danger",
      });
      if (!ok) return;
      if (!isCurrentUser(requestCurrentUserId)) return;
      await disableUser(u.id);
      if (!isCurrentUser(requestCurrentUserId)) return;
      addToast(t("settingsAdmin.disabled"), "success");
      await load();
    } catch (err) {
      if (!isCurrentUser(requestCurrentUserId)) return;
      addToast(extractErrorMessage(err, t("settingsAdmin.disableFailed")), "error");
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
      addToast(extractErrorMessage(err, t("settingsAdmin.loadTokenFailed")), "error");
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
      addToast(extractErrorMessage(err, t("settingsAdmin.loadUsageFailed")), "error");
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

  const tokenUserLabel = users.find((user) => user.id === tokenFor)?.username ?? t("settingsAdmin.userFallback");
  const usageUserLabel = users.find((user) => user.id === usageFor)?.username ?? t("settingsAdmin.userFallback");
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
        label: t("settingsAdmin.tokenSummary"),
        icon: <KeyRound size={12} />,
        onClick: () => void handleToken(u),
      },
      {
        label: t("settingsAdmin.viewUsage"),
        icon: <BarChart3 size={12} />,
        onClick: () => void handleUsage(u),
      },
      { type: "divider" },
      {
        label: t("settingsAdmin.adjustRole"),
        icon: <Shield size={12} />,
        onClick: () => void handleRole(u),
      },
    ];
    if (u.id !== currentUserId && u.is_active !== false) {
      items.push({
        label: t("settingsAdmin.disableUser"),
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
    <div
      className="flex flex-col gap-3"
      data-settings-dirty={createDirty ? "true" : undefined}
    >
      <div className="flex min-h-9 flex-wrap items-center gap-2">
        <span className="text-xs text-book-text-muted">{t("settingsAdmin.userCount", { count: users.length })}</span>
        <span className="flex-1" />
        <BookButton size="sm" onClick={openCreate}>
          <Plus size={13} />
          {t("settingsAdmin.createUser")}
        </BookButton>
        <Dropdown
          label={t("settingsAdmin.more")}
          items={[
            {
              label: isActing("create-invite-code") ? t("settingsAdmin.creatingInviteCode") : t("settingsAdmin.createInviteCode"),
              icon: <Plus size={12} />,
              disabled: isActing("create-invite-code"),
              onClick: () => void handleCreateInviteCode(),
            },
            {
              label: loading ? t("settingsAdmin.refreshing") : t("settingsAdmin.refreshList"),
              icon: <RefreshCw size={12} className={loading ? "animate-spin" : undefined} />,
              disabled: loading,
              onClick: () => void load(),
            },
          ]}
        />
      </div>
      {error ? <p role="alert" className="text-xs text-error">{error}</p> : null}
      <ul className="space-y-2.5">
        {loading && users.length === 0 ? (
          <li className="a4n-flat-surface inline-flex w-full items-center gap-1.5 p-3 text-xs text-book-text-muted">
            <LoadingSpinner size="sm" />
            {t("settingsAdmin.loadingUsers")}
          </li>
        ) : error && users.length === 0 ? null : users.length === 0 ? (
          <li className="a4n-empty-state a4n-empty-state--framed p-3 text-xs text-book-text-muted">
            {t("settingsAdmin.noUsers")}
          </li>
        ) : (
          users.map((u) => (
            <li key={u.id} className="a4n-flat-surface grid grid-cols-[minmax(0,1fr)_auto] items-start gap-3 p-3 text-xs">
              <div className="min-w-0">
                <div className="truncate font-medium text-book-text-main">{u.username}</div>
                <div className="mt-1 truncate text-book-text-muted">{u.email}</div>
                <div className="mt-0.5 text-small text-book-text-muted">
                  {userRoleLabel(u.role, t)}
                  {u.is_active === false ? ` · ${t("settingsAdmin.userDisabled")}` : ""}
                  {u.last_login_at
                    ? ` · ${t("settingsAdmin.lastLogin", { date: formatDateTime(u.last_login_at, locale, t("settingsAdmin.unknownDate")) })}`
                    : ` · ${t("settingsAdmin.neverLoggedIn")}`}
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
          <span className="font-medium text-book-text-main">{t("settingsAdmin.inviteCodes")}</span>
          <span className="min-w-0 flex-1 truncate text-right text-book-text-muted">
            {t("settingsAdmin.inviteCodeSummary", { available: availableInviteCount, total: inviteCodes.length })}
          </span>
        </summary>
        <ul className="divide-y divide-book-border/30 border-t border-book-border/30">
          {loading && inviteCodes.length === 0 ? (
            <li className="inline-flex w-full items-center gap-1.5 p-3 text-xs text-book-text-muted">
              <LoadingSpinner size="sm" />
              {t("settingsAdmin.loadingInviteCodes")}
            </li>
          ) : error && inviteCodes.length === 0 ? null : inviteCodes.length === 0 ? (
            <li className="p-3 text-xs text-book-text-muted">{t("settingsAdmin.noInviteCodes")}</li>
          ) : (
            inviteCodes.map((code) => (
              <li key={code.id} className="grid gap-1 px-3 py-2.5 text-xs sm:grid-cols-[minmax(0,1fr)_auto] sm:items-center sm:gap-3">
                <span className="min-w-0 break-all font-mono text-book-text-main">{code.code}</span>
                <span className={code.used_at ? "text-book-text-muted" : "text-book-primary"}>
                  {code.used_at ? t("settingsAdmin.used") : t("settingsAdmin.available")} · {formatDateTime(code.created_at, locale, t("settingsAdmin.unknownDate"))}
                </span>
              </li>
            ))
          )}
        </ul>
      </details>
      <Modal
        isOpen={tokenFor != null}
        onClose={closeTokenSummary}
        title={`${tokenUserLabel} · ${t("settingsAdmin.tokenSummary")}`}
        maxWidthClassName="max-w-xl"
      >
        <TokenSummaryPanel data={tokenInfo} loading={tokenLoadingFor === tokenFor} />
      </Modal>
      <Modal
        isOpen={usageFor != null}
        onClose={closeUsageRecords}
        title={`${usageUserLabel} · ${t("settingsAdmin.recentUsage")}`}
        maxWidthClassName="max-w-2xl"
      >
        <UsageRecordsPanel data={usageInfo} loading={usageLoadingFor === usageFor} />
      </Modal>
      <Modal
        isOpen={createOpen}
        onClose={() => void closeCreate()}
        title={t("settingsAdmin.createUser")}
        subtitle={t("settingsAdmin.createUserSubtitle")}
        icon={<Plus size={15} />}
        maxWidthClassName="max-w-lg"
        closeOnBackdrop={!createBusy}
        closeOnEscape={!createBusy}
        showCloseButton={!createBusy}
        footer={
          <BookButton
            variant="primary"
            type="submit"
            form="admin-create-user-form"
            disabled={createBusy || !createReady}
          >
            {createBusy ? t("settingsAdmin.creating") : t("settingsAdmin.createUser")}
          </BookButton>
        }
      >
        <form
          id="admin-create-user-form"
          className="grid gap-3 text-xs"
          onSubmit={(event) => { event.preventDefault(); void handleCreate(); }}
        >
          <label className="grid gap-1">
            <span className="text-book-text-muted">{t("settingsAdmin.username")}</span>
            <input
              className="pyqt-input"
              value={createForm.username}
              onChange={(e) => updateCreateForm({ username: e.target.value })}
              autoComplete="off"
              disabled={createBusy}
            />
          </label>
          <label className="grid gap-1">
            <span className="text-book-text-muted">{t("settingsAdmin.email")}</span>
            <input
              className="pyqt-input"
              value={createForm.email}
              onChange={(e) => updateCreateForm({ email: e.target.value })}
              autoComplete="off"
              disabled={createBusy}
            />
          </label>
          <label className="grid gap-1">
            <span className="text-book-text-muted">{t("settingsAdmin.initialPassword")}</span>
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
            <span className="text-book-text-muted">{t("settingsAdmin.role")}</span>
            <ThemedSelect
              className="pyqt-input"
              value={createForm.role}
              onChange={(selectedValue) => updateCreateForm({ role: selectedValue })}
              disabled={createBusy}
            >
              {USER_ROLES.map((role) => (
                <option key={role} value={role}>
                  {t(USER_ROLE_LABEL_KEYS[role])}
                </option>
              ))}
            </ThemedSelect>
          </label>
        </form>
      </Modal>
    </div>
  );
};

const TokenSummaryPanel: React.FC<{
  data: unknown;
  loading: boolean;
}> = ({ data, loading }) => {
  const { t, locale } = useI18n();
  const record = data && typeof data === "object" && !Array.isArray(data)
    ? data as Record<string, unknown>
    : null;
  const value = (key: string): number | null => {
    const item = record?.[key];
    return typeof item === "number" && Number.isFinite(item) ? item : null;
  };
  const items = [
    [t("settingsAdmin.tokenCalls"), value("total_calls")],
    [t("settingsAdmin.inputTokens"), value("total_prompt_tokens")],
    [t("settingsAdmin.outputTokens"), value("total_completion_tokens")],
    [t("settingsAdmin.totalTokens"), value("total_tokens")],
    [t("settingsAdmin.success"), value("success_count")],
    [t("settingsAdmin.failure"), value("fail_count")],
  ] as const;

  return (
    <div className="overflow-hidden">
      {loading && !record ? (
        <div className="inline-flex items-center gap-1.5 px-3 py-3 text-small text-book-text-muted">
          <LoadingSpinner size="sm" /> {t("settingsAdmin.loadingData")}
        </div>
      ) : !record ? (
        <div className="px-3 py-3 text-small text-book-text-muted">{t("settingsAdmin.noData")}</div>
      ) : (
        <>
          <dl className="grid grid-cols-2 divide-x divide-y divide-book-border/30 sm:grid-cols-3">
            {items.map(([label, item]) => (
              <div key={label} className="px-3 py-2.5">
                <dt className="text-caption text-book-text-muted">{label}</dt>
                <dd className="mt-0.5 font-mono text-sm font-semibold text-book-text-main">
                  {item == null ? "—" : formatNumber(item, locale)}
                </dd>
              </div>
            ))}
          </dl>
          <details className="border-t border-book-border/30 px-3 py-2 text-caption text-book-text-muted">
            <summary className="cursor-pointer">{t("settingsAdmin.diagnostics")}</summary>
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
}> = ({ data, loading }) => {
  const { t, locale } = useI18n();
  return (
  <div className="overflow-hidden">
    {loading && data == null ? (
      <div className="inline-flex items-center gap-1.5 px-3 py-3 text-small text-book-text-muted">
        <LoadingSpinner size="sm" /> {t("settingsAdmin.loadingData")}
      </div>
    ) : !data?.length ? (
      <div className="px-3 py-3 text-small text-book-text-muted">{t("settingsAdmin.noCallRecords")}</div>
    ) : (
      <ul className="divide-y divide-book-border/30">
        {data.map((item, index) => (
          <li key={item.id} className="px-3 py-2.5 text-small">
            <div className="flex min-w-0 items-center gap-2">
              <span className="min-w-0 flex-1 truncate font-medium text-book-text-main">
                {item.model || t("settingsAdmin.callNumber", { count: index + 1 })}
              </span>
              <span className="shrink-0 font-mono text-book-text-sub">
                {t("settingsAdmin.tokens", { count: formatNumber(item.total_tokens, locale) })}
              </span>
            </div>
            <div className="mt-1 flex flex-wrap gap-x-3 gap-y-0.5 text-book-text-muted">
              <span>{t("settingsAdmin.inputValue", { count: formatNumber(item.prompt_tokens, locale) })}</span>
              <span>{t("settingsAdmin.outputValue", { count: formatNumber(item.completion_tokens, locale) })}</span>
              <span>{formatDateTime(item.created_at, locale, t("settingsAdmin.unknownDate"))}</span>
            </div>
          </li>
        ))}
      </ul>
    )}
  </div>
  );
};
