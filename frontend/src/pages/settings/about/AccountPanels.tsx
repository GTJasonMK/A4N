import React, { useCallback, useEffect, useRef, useState } from "react";
import { ChevronRight, KeyRound, RefreshCw, UserRound } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { BookButton } from "../../../components/ui/BookButton";
import { BookInput } from "../../../components/ui/BookInput";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { useToast } from "../../../components/feedback/Toast";
import { extractErrorMessage } from "../../../api";
import {
  changeUserPassword,
  getCurrentUser,
  listSessions,
  refreshToken,
  updateUserProfile,
  type SessionSummary,
} from "../../../api/auth";
import { nonEmptyText, textInputValue } from "../../../utils/text";
import { formatDateTime, useI18n } from "../../../i18n";
import { useAuthStore } from "../../../store/auth";
import { shouldUseHttpBackend } from "../../../utils/runtimeMode";
import { sessionDeviceLabel } from "./aboutUtils";
import { useDirty } from "../../../components/navigation/dirtyRegistry";

export const PageCliPanel: React.FC = () => {
  const { user, updateProfile } = useAuthStore();
  const { addToast } = useToast();
  const { t } = useI18n();
  const [busy, setBusy] = useState(false);

  if (!user || !shouldUseHttpBackend()) return null;

  const handleChange = async (enabled: boolean) => {
    if (busy || enabled === user.page_cli_enabled) return;
    setBusy(true);
    try {
      await updateProfile({ page_cli_enabled: enabled });
      addToast(t(enabled ? "pageCli.enabledToast" : "pageCli.disabledToast"), "success");
    } catch (err) {
      addToast(extractErrorMessage(err, t("pageCli.updateFailed")), "error");
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="a4n-flat-surface overflow-hidden text-xs" aria-label={t("pageCli.title")}>
      <div className="flex flex-wrap items-start justify-between gap-3 px-3 py-3">
        <div className="min-w-0 flex-1">
          <h3 className="font-medium text-book-text-main">{t("pageCli.title")}</h3>
        </div>
        <label className="inline-flex min-h-9 shrink-0 cursor-pointer items-center gap-2 rounded-xl border border-book-border/60 bg-book-bg-tertiary/45 px-2.5 text-book-text-sub has-[:focus-visible]:outline has-[:focus-visible]:outline-2 has-[:focus-visible]:outline-book-primary">
          <input
            type="checkbox"
            className="h-4 w-4 accent-book-primary"
            checked={user.page_cli_enabled}
            disabled={busy}
            onChange={(event) => void handleChange(event.target.checked)}
          />
          <span>{user.page_cli_enabled ? t("pageCli.enabled") : t("pageCli.disabled")}</span>
        </label>
      </div>
    </section>
  );
};

export const ProfileEditor: React.FC<{ userId: number; embedded?: boolean }> = ({
  userId,
  embedded = false,
}) => {
  const { addToast } = useToast();
  const { t } = useI18n();
  const [email, setEmail] = useState("");
  const [avatar, setAvatar] = useState("");
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<{ reason: unknown } | null>(null);
  const mountedRef = useRef(false);
  const activeUserIdRef = useRef(userId);
  const loadSeqRef = useRef(0);
  const saveSeqRef = useRef(0);
  const editVersionRef = useRef(0);
  const busyRef = useRef(false);
  const discardingDraftRef = useRef(false);
  const savedProfileRef = useRef({ email: "", avatar: "" });
  const profileDirty =
    email !== savedProfileRef.current.email ||
    avatar !== savedProfileRef.current.avatar;
  useDirty(`settings-account-profile:${userId}`, profileDirty, undefined, () => !busyRef.current);

  activeUserIdRef.current = userId;

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      loadSeqRef.current += 1;
      saveSeqRef.current += 1;
      busyRef.current = false;
    };
  }, []);

  const isCurrentUser = useCallback((requestUserId: number) => {
    return mountedRef.current && activeUserIdRef.current === requestUserId;
  }, []);

  const updateEmail = (value: string) => {
    editVersionRef.current += 1;
    setEmail(value);
  };

  const updateAvatar = (value: string) => {
    editVersionRef.current += 1;
    setAvatar(value);
  };

  const loadProfile = useCallback(async () => {
    const requestUserId = userId;
    const requestSeq = loadSeqRef.current + 1;
    loadSeqRef.current = requestSeq;
    setLoading(true);
    setLoadError(null);
    busyRef.current = false;
    setBusy(false);
    try {
      const user = await getCurrentUser();
      if (!isCurrentUser(requestUserId) || loadSeqRef.current !== requestSeq) return;
      setLoadError(null);
      savedProfileRef.current = {
        email: textInputValue(user.email),
        avatar: textInputValue(user.avatar_url),
      };
      editVersionRef.current += 1;
      setEmail(savedProfileRef.current.email);
      setAvatar(savedProfileRef.current.avatar);
    } catch (err) {
      if (!isCurrentUser(requestUserId) || loadSeqRef.current !== requestSeq) return;
      setLoadError({ reason: err });
    } finally {
      if (isCurrentUser(requestUserId) && loadSeqRef.current === requestSeq) setLoading(false);
    }
  }, [isCurrentUser, userId]);

  useEffect(() => {
    void loadProfile();
  }, [loadProfile]);

  const handleSave = async () => {
    if (busyRef.current || !profileDirty) return;
    const requestUserId = userId;
    const requestSeq = saveSeqRef.current + 1;
    const requestEditVersion = editVersionRef.current;
    const requestEmail = email;
    const requestAvatar = avatar;
    saveSeqRef.current = requestSeq;
    busyRef.current = true;
    setBusy(true);
    try {
      const user = await updateUserProfile({
        email: requestEmail.trim(),
        avatar_url: nonEmptyText(requestAvatar),
      });
      if (!isCurrentUser(requestUserId) || saveSeqRef.current !== requestSeq) return;
      savedProfileRef.current = {
        email: textInputValue(user.email),
        avatar: textInputValue(user.avatar_url),
      };
      if (editVersionRef.current === requestEditVersion) {
        editVersionRef.current += 1;
        setEmail(savedProfileRef.current.email);
        setAvatar(savedProfileRef.current.avatar);
        addToast(t("settingsAbout.updatedProfile"), "success");
      } else {
        addToast(t("settingsAbout.savedSubmittedProfile"), "info");
      }
    } catch (err) {
      if (!isCurrentUser(requestUserId) || saveSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, t("settingsAbout.updateFailed")), "error");
    } finally {
      if (saveSeqRef.current === requestSeq) busyRef.current = false;
      if (isCurrentUser(requestUserId) && saveSeqRef.current === requestSeq) setBusy(false);
    }
  };

  return (
    <details
      data-settings-dirty={profileDirty ? "true" : undefined}
      onToggle={(event) => {
        if (discardingDraftRef.current) {
          discardingDraftRef.current = false;
          return;
        }
        if (event.currentTarget.open || !profileDirty) return;
        const details = event.currentTarget;
        details.open = true;
        if (busyRef.current) {
          addToast(t("common.operationInProgress"), "info");
          return;
        }
        void confirmDialog({
          title: t("common.unsavedChangesTitle"),
          message: t("common.unsavedChangesMessage", { name: t("settingsAbout.profile"), action: t("common.close") }),
          confirmText: t("common.discardDraft"),
          dialogType: "warning",
        }).then((discard) => {
          if (!discard || !mountedRef.current) return;
          editVersionRef.current += 1;
          setEmail(savedProfileRef.current.email);
          setAvatar(savedProfileRef.current.avatar);
          discardingDraftRef.current = true;
          details.open = false;
        });
      }}
      className={`${embedded ? "border-t border-book-border/35" : "a4n-flat-surface"} group overflow-hidden text-xs`}
    >
      <summary className="flex cursor-pointer list-none items-center gap-2 px-3 py-3 text-book-text-sub transition-colors hover:bg-book-bg-tertiary/40 [&::-webkit-details-marker]:hidden">
        <UserRound size={13} className="text-book-text-muted" />
        <span className="font-medium text-book-text-main">{t("settingsAbout.profile")}</span>
        <span className="text-caption text-book-text-muted">{t("settingsAbout.emailAndAvatar")}</span>
        <ChevronRight size={14} className="ml-auto shrink-0 transition-transform group-open:rotate-90" />
      </summary>
      <form className="space-y-2 border-t border-book-border/35 p-3" onSubmit={(event) => { event.preventDefault(); void handleSave(); }}>
        {loading ? (
          <div className="inline-flex items-center gap-1.5 text-book-text-muted">
            <LoadingSpinner size="sm" /> {t("settingsAbout.loading")}
          </div>
        ) : loadError ? (
          <div role="alert" className="flex flex-wrap items-center gap-2 text-error">
            <span className="min-w-0 flex-1">
              {extractErrorMessage(loadError.reason, t("settingsAbout.loadProfileFailed"))}
            </span>
            <BookButton variant="secondary" size="sm" onClick={() => void loadProfile()} disabled={loading || busy}>
              <RefreshCw size={12} className={loading ? "animate-spin" : undefined} />
              {t("common.retry")}
            </BookButton>
          </div>
        ) : (
          <>
            <BookInput
              label={t("settingsAbout.email")}
              type="email"
              required
              autoComplete="email"
              value={email}
              onChange={(e) => updateEmail(e.target.value)}
              placeholder="user@example.com"
            />
            <BookInput
              label={t("settingsAbout.avatarUrl")}
              type="url"
              autoComplete="url"
              value={avatar}
              onChange={(e) => updateAvatar(e.target.value)}
              placeholder="https://…/avatar.png"
            />
            <div className="flex justify-end">
              <BookButton
                variant="primary"
                size="sm"
                type="submit"
                disabled={busy || !profileDirty}
              >
                {busy ? (
                  <span className="inline-flex items-center gap-1.5">
                  <LoadingSpinner size="sm" /> {t("settingsAbout.saving")}
                  </span>
                ) : (
                  t("settingsAbout.saveProfile")
                )}
              </BookButton>
            </div>
          </>
        )}
      </form>
    </details>
  );
};

export const PasswordChanger: React.FC<{ embedded?: boolean }> = ({ embedded = false }) => {
  const { addToast } = useToast();
  const { t } = useI18n();
  const [oldPwd, setOldPwd] = useState("");
  const [newPwd, setNewPwd] = useState("");
  const [busy, setBusy] = useState(false);
  const [saveError, setSaveError] = useState<{ reason: unknown } | null>(null);
  const passwordReady = oldPwd.length > 0 && newPwd.length > 0;
  const mountedRef = useRef(false);
  const changeSeqRef = useRef(0);
  const editVersionRef = useRef(0);
  const busyRef = useRef(false);
  const discardingDraftRef = useRef(false);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      changeSeqRef.current += 1;
      busyRef.current = false;
    };
  }, []);

  const updateOldPwd = (value: string) => {
    editVersionRef.current += 1;
    setSaveError(null);
    setOldPwd(value);
  };

  const updateNewPwd = (value: string) => {
    editVersionRef.current += 1;
    setSaveError(null);
    setNewPwd(value);
  };
  const passwordDirty = oldPwd.length > 0 || newPwd.length > 0;
  useDirty("settings-account-password", passwordDirty, undefined, () => !busyRef.current);

  const handleChange = async () => {
    if (busyRef.current) return;
    if (!oldPwd || !newPwd) {
      addToast(t("settingsAbout.fillPasswords"), "error");
      return;
    }
    const requestSeq = changeSeqRef.current + 1;
    const requestEditVersion = editVersionRef.current;
    const requestOldPwd = oldPwd;
    const requestNewPwd = newPwd;
    changeSeqRef.current = requestSeq;
    busyRef.current = true;
    setBusy(true);
    setSaveError(null);
    try {
      await changeUserPassword(requestOldPwd, requestNewPwd);
      if (!mountedRef.current || changeSeqRef.current !== requestSeq) return;
      if (editVersionRef.current === requestEditVersion) {
        editVersionRef.current += 1;
        addToast(t("settingsAbout.passwordChanged"), "success");
        setOldPwd("");
        setNewPwd("");
      } else {
        addToast(t("settingsAbout.savedSubmittedPassword"), "info");
      }
    } catch (err) {
      if (!mountedRef.current || changeSeqRef.current !== requestSeq) return;
      if (editVersionRef.current === requestEditVersion) {
        setSaveError({ reason: err });
      } else {
        addToast(extractErrorMessage(err, t("settingsAbout.updatePasswordFailed")), "error");
      }
    } finally {
      if (changeSeqRef.current === requestSeq) busyRef.current = false;
      if (mountedRef.current && changeSeqRef.current === requestSeq) setBusy(false);
    }
  };

  return (
    <details
      data-settings-dirty={passwordDirty ? "true" : undefined}
      onToggle={(event) => {
        if (discardingDraftRef.current) {
          discardingDraftRef.current = false;
          return;
        }
        if (event.currentTarget.open || !passwordDirty) return;
        const details = event.currentTarget;
        details.open = true;
        if (busyRef.current) {
          addToast(t("common.operationInProgress"), "info");
          return;
        }
        void confirmDialog({
          title: t("common.unsavedChangesTitle"),
          message: t("common.unsavedChangesMessage", { name: t("settingsAbout.changePassword"), action: t("common.close") }),
          confirmText: t("common.discardDraft"),
          dialogType: "warning",
        }).then((discard) => {
          if (!discard || !mountedRef.current) return;
          editVersionRef.current += 1;
          setOldPwd("");
          setNewPwd("");
          setSaveError(null);
          discardingDraftRef.current = true;
          details.open = false;
        });
      }}
      className={`${embedded ? "border-t border-book-border/35" : "a4n-flat-surface"} group overflow-hidden text-xs`}
    >
      <summary className="flex cursor-pointer list-none items-center gap-2 px-3 py-3 text-book-text-sub transition-colors hover:bg-book-bg-tertiary/40 [&::-webkit-details-marker]:hidden">
        <KeyRound size={13} className="text-book-text-muted" />
        <span className="font-medium text-book-text-main">{t("settingsAbout.changePassword")}</span>
        <ChevronRight size={14} className="ml-auto shrink-0 transition-transform group-open:rotate-90" />
      </summary>
      <form className="space-y-2 border-t border-book-border/35 p-3" onSubmit={(event) => { event.preventDefault(); void handleChange(); }}>
        <BookInput
          label={t("settingsAbout.currentPassword")}
          type="password"
          autoComplete="current-password"
          value={oldPwd}
          onChange={(e) => updateOldPwd(e.target.value)}
        />
        <BookInput
          label={t("settingsAbout.newPassword")}
          type="password"
          autoComplete="new-password"
          value={newPwd}
          onChange={(e) => updateNewPwd(e.target.value)}
        />
        {saveError ? (
          <p role="alert" className="text-error">
            {extractErrorMessage(saveError.reason, t("settingsAbout.updatePasswordFailed"))}
          </p>
        ) : null}
        <div className="flex justify-end">
          <BookButton
            variant="primary"
            size="sm"
            type="submit"
            disabled={busy || !passwordReady}
          >
            {busy ? (
              <span className="inline-flex items-center gap-1.5">
                <LoadingSpinner size="sm" /> {t("settingsAbout.updating")}
              </span>
            ) : (
              t("settingsAbout.changePasswordAction")
            )}
          </BookButton>
        </div>
      </form>
    </details>
  );
};

export const SessionManager: React.FC = () => {
  const { addToast } = useToast();
  const { t } = useI18n();
  const [sessions, setSessions] = useState<SessionSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<{ reason: unknown } | null>(null);
  const [refreshBusy, setRefreshBusy] = useState(false);
  const mountedRef = useRef(false);
  const refreshBusyRef = useRef(false);
  const loadSeqRef = useRef(0);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      refreshBusyRef.current = false;
    };
  }, []);

  const load = async (allowDuringRefresh = false): Promise<void> => {
    if (refreshBusyRef.current && !allowDuringRefresh) {
      addToast(t("common.operationInProgress"), "info");
      return;
    }
    const requestSeq = ++loadSeqRef.current;
    setLoading(true);
    setLoadError(null);
    try {
      const nextSessions = await listSessions();
      if (!mountedRef.current || loadSeqRef.current !== requestSeq) return;
      setSessions(nextSessions);
    } catch (err) {
      if (!mountedRef.current || loadSeqRef.current !== requestSeq) return;
      setLoadError({ reason: err });
    } finally {
      if (mountedRef.current && loadSeqRef.current === requestSeq) setLoading(false);
    }
  };

  useEffect(() => {
    void load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handleRefresh = async () => {
    if (refreshBusyRef.current) return;
    refreshBusyRef.current = true;
    setRefreshBusy(true);
    try {
      await refreshToken();
      addToast(t("settingsAbout.credentialUpdated"), "success");
      if (!mountedRef.current) return;
      await load(true);
    } catch (err) {
      addToast(extractErrorMessage(err, t("settingsAbout.refreshFailed")), "error");
    } finally {
      refreshBusyRef.current = false;
      if (mountedRef.current) setRefreshBusy(false);
    }
  };

  return (
    <section className="space-y-2 text-xs">
      <div className="flex min-w-0 items-center gap-2">
        <div className="min-w-0 flex-1">
          <h3 className="font-medium text-book-text-main">{t("settingsAbout.sessions")}</h3>
          <p className="mt-0.5 text-caption text-book-text-muted">
            {loading ? t("settingsAbout.checkingSessions") : loadError ? t("settingsAbout.loadSessionsFailed") : t("settingsAbout.sessionCount", { count: sessions.length })}
          </p>
        </div>
      </div>
      {loadError ? <div className="flex items-start justify-between gap-3">
        <p role="alert" className="text-error">
          {extractErrorMessage(loadError.reason, t("settingsAbout.loadSessionsFailed"))}
        </p>
        <BookButton size="sm" variant="secondary" onClick={() => void load()} disabled={loading}>{t("common.retry")}</BookButton>
      </div> : null}
      {loading ? (
        <div className="a4n-flat-surface flex items-center gap-1.5 p-3 text-book-text-muted">
          <LoadingSpinner size="sm" /> {t("settingsAbout.loadSessions")}
        </div>
      ) : loadError && sessions.length === 0 ? null : sessions.length === 0 ? (
        <div className="a4n-flat-surface p-3 text-book-text-muted">{t("settingsAbout.noOtherSessions")}</div>
      ) : (
        <ul className="grid grid-cols-1 items-start gap-2 sm:grid-cols-2">
          {sessions.map((session) => (
            <SessionRow key={session.id} session={session} />
          ))}
        </ul>
      )}
      <details className="a4n-flat-surface group overflow-hidden text-small text-book-text-muted">
        <summary className="flex cursor-pointer list-none items-center gap-2 px-3 py-2.5 transition-colors hover:bg-book-bg-tertiary/40 [&::-webkit-details-marker]:hidden">
          <RefreshCw size={11} className="shrink-0" />
          <span className="font-medium text-book-text-main">{t("settingsAbout.loginRepair")}</span>
          <span className="text-caption">{t("settingsAbout.loginRepairHint")}</span>
          <ChevronRight size={14} className="ml-auto shrink-0 transition-transform group-open:rotate-90" />
        </summary>
        <div className="flex flex-wrap items-center justify-between gap-2 border-t border-book-border/35 p-3">
          <p className="min-w-0 flex-1 leading-relaxed">
            {t("settingsAbout.loginRepairMessage")}
          </p>
          <BookButton variant="secondary" size="sm" onClick={handleRefresh} disabled={refreshBusy}>
            {refreshBusy ? (
              <LoadingSpinner size="sm" />
            ) : (
              <RefreshCw size={11} />
            )}
            {t("settingsAbout.refreshCredentials")}
          </BookButton>
        </div>
      </details>
    </section>
  );
};

const SessionRow: React.FC<{ session: SessionSummary }> = ({ session }) => {
  const { t, locale } = useI18n();
  const expiresAt = new Date(session.expires_at);
  const expired = !Number.isNaN(expiresAt.getTime()) && expiresAt <= new Date();
  return (
    <li className="a4n-flat-surface p-3 text-small">
      <div className="flex min-w-0 items-center gap-2">
        <div className="min-w-0 flex-1">
          <div className="truncate font-medium text-book-text-main" title={session.user_agent ?? ""}>
            {sessionDeviceLabel(session.user_agent) || t("settingsAbout.unknownDevice")}
          </div>
          <div className="mt-0.5 text-book-text-muted">
            <span className={!expired ? "text-success" : undefined}>{expired ? t("settingsAbout.sessionExpired") : t("settingsAbout.active")}</span>
            {` · ${t("settingsAbout.recentlyUsed", { date: formatDateTime(session.last_used_at ?? session.created_at, locale, t("settingsAbout.unknownDate")) })}`}
          </div>
        </div>
      </div>
      <details className="mt-1 text-caption text-book-text-muted">
        <summary className="w-fit cursor-pointer hover:text-book-text-main">{t("settingsAbout.diagnostic")}</summary>
        <div className="mt-1 grid gap-0.5 break-all font-mono">
          <span>{session.user_agent}</span>
          <span>{t("settingsAbout.session", { id: session.id })}</span>
          <span>{t("settingsAbout.created", { date: formatDateTime(session.created_at, locale, t("settingsAbout.unknownDate")) })}</span>
          <span>{t("settingsAbout.expires", { date: formatDateTime(session.expires_at, locale, t("settingsAbout.unknownDate")) })}</span>
        </div>
      </details>
    </li>
  );
};
