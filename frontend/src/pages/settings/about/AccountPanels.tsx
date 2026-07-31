import React, { useCallback, useEffect, useRef, useState } from "react";
import { KeyRound, RefreshCw, UserRound } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { BookButton } from "../../../components/ui/BookButton";
import { BookInput } from "../../../components/ui/BookInput";
import { useToast } from "../../../components/feedback/Toast";
import { extractErrorMessage } from "../../../api";
import {
  changeUserPassword,
  getUser,
  listSessions,
  refreshToken,
  updateUserProfile,
  type SessionSummary,
} from "../../../api/auth";
import { formatUiDateTime } from "../../../utils/dateTime";
import { nonEmptyText, textInputValue } from "../../../utils/text";

export const ProfileEditor: React.FC<{ userId: number; embedded?: boolean }> = ({
  userId,
  embedded = false,
}) => {
  const { addToast } = useToast();
  const [email, setEmail] = useState("");
  const [avatar, setAvatar] = useState("");
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const mountedRef = useRef(false);
  const activeUserIdRef = useRef(userId);
  const loadSeqRef = useRef(0);
  const saveSeqRef = useRef(0);
  const editVersionRef = useRef(0);
  const busyRef = useRef(false);
  const savedProfileRef = useRef({ email: "", avatar: "" });
  const profileDirty =
    email !== savedProfileRef.current.email ||
    avatar !== savedProfileRef.current.avatar;

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

  useEffect(() => {
    const requestUserId = userId;
    const requestSeq = loadSeqRef.current + 1;
    loadSeqRef.current = requestSeq;
    setLoading(true);
    setLoadError(null);
    busyRef.current = false;
    setBusy(false);
    void (async () => {
      try {
        const user = await getUser(requestUserId);
        if (!isCurrentUser(requestUserId) || loadSeqRef.current !== requestSeq) return;
        savedProfileRef.current = {
          email: textInputValue(user.email),
          avatar: textInputValue(user.avatar_url),
        };
        editVersionRef.current += 1;
        setEmail(savedProfileRef.current.email);
        setAvatar(savedProfileRef.current.avatar);
      } catch (err) {
        if (!isCurrentUser(requestUserId) || loadSeqRef.current !== requestSeq) return;
        const message = extractErrorMessage(err, "加载资料失败");
        setLoadError(message);
        addToast(message, "error");
      } finally {
        if (isCurrentUser(requestUserId) && loadSeqRef.current === requestSeq) setLoading(false);
      }
    })();
  }, [addToast, isCurrentUser, userId]);

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
      const user = await updateUserProfile(requestUserId, {
        email: nonEmptyText(requestEmail),
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
        addToast("已更新资料", "success");
      } else {
        addToast("已保存提交时的资料，当前保留后续编辑内容", "info");
      }
    } catch (err) {
      if (!isCurrentUser(requestUserId) || saveSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, "更新失败"), "error");
    } finally {
      if (saveSeqRef.current === requestSeq) busyRef.current = false;
      if (isCurrentUser(requestUserId) && saveSeqRef.current === requestSeq) setBusy(false);
    }
  };

  return (
    <details
      className={`${embedded ? "border-t border-book-border/35" : "a4n-flat-surface"} overflow-hidden text-xs`}
    >
      <summary className="flex cursor-pointer list-none items-center gap-2 px-3 py-3 text-book-text-sub transition-colors hover:bg-book-bg-tertiary/40 [&::-webkit-details-marker]:hidden">
        <UserRound size={13} className="text-book-text-muted" />
        <span className="font-medium text-book-text-main">个人资料</span>
        <span className="text-caption text-book-text-muted">邮箱与头像</span>
      </summary>
      <div className="space-y-2 border-t border-book-border/35 p-3">
        {loading ? (
          <div className="inline-flex items-center gap-1.5 text-book-text-muted">
            <LoadingSpinner size="sm" /> 加载中…
          </div>
        ) : loadError ? (
          <div className="text-error">{loadError}</div>
        ) : (
          <>
            <BookInput
              label="邮箱"
              value={email}
              onChange={(e) => updateEmail(e.target.value)}
              placeholder="user@example.com"
            />
            <BookInput
              label="头像地址"
              value={avatar}
              onChange={(e) => updateAvatar(e.target.value)}
              placeholder="https://…/avatar.png"
            />
            <div className="flex justify-end">
              <BookButton
                variant="primary"
                size="sm"
                onClick={handleSave}
                disabled={busy || !profileDirty}
              >
                {busy ? (
                  <span className="inline-flex items-center gap-1.5">
                    <LoadingSpinner size="sm" /> 保存中…
                  </span>
                ) : (
                  "保存资料"
                )}
              </BookButton>
            </div>
          </>
        )}
      </div>
    </details>
  );
};

export const PasswordChanger: React.FC<{ embedded?: boolean }> = ({ embedded = false }) => {
  const { addToast } = useToast();
  const [oldPwd, setOldPwd] = useState("");
  const [newPwd, setNewPwd] = useState("");
  const [busy, setBusy] = useState(false);
  const passwordReady = oldPwd.length > 0 && newPwd.length > 0;
  const mountedRef = useRef(false);
  const changeSeqRef = useRef(0);
  const editVersionRef = useRef(0);
  const busyRef = useRef(false);

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
    setOldPwd(value);
  };

  const updateNewPwd = (value: string) => {
    editVersionRef.current += 1;
    setNewPwd(value);
  };

  const handleChange = async () => {
    if (busyRef.current) return;
    if (!oldPwd || !newPwd) {
      addToast("请填写两个密码字段", "error");
      return;
    }
    const requestSeq = changeSeqRef.current + 1;
    const requestEditVersion = editVersionRef.current;
    const requestOldPwd = oldPwd;
    const requestNewPwd = newPwd;
    changeSeqRef.current = requestSeq;
    busyRef.current = true;
    setBusy(true);
    try {
      await changeUserPassword(requestOldPwd, requestNewPwd);
      if (!mountedRef.current || changeSeqRef.current !== requestSeq) return;
      if (editVersionRef.current === requestEditVersion) {
        editVersionRef.current += 1;
        addToast("密码已修改", "success");
        setOldPwd("");
        setNewPwd("");
      } else {
        addToast("已修改提交时的密码，当前保留后续输入内容", "info");
      }
    } catch (err) {
      if (!mountedRef.current || changeSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, "修改失败"), "error");
    } finally {
      if (changeSeqRef.current === requestSeq) busyRef.current = false;
      if (mountedRef.current && changeSeqRef.current === requestSeq) setBusy(false);
    }
  };

  return (
    <details
      className={`${embedded ? "border-t border-book-border/35" : "a4n-flat-surface"} overflow-hidden text-xs`}
    >
      <summary className="flex cursor-pointer list-none items-center gap-2 px-3 py-3 text-book-text-sub transition-colors hover:bg-book-bg-tertiary/40 [&::-webkit-details-marker]:hidden">
        <KeyRound size={13} className="text-book-text-muted" />
        <span className="font-medium text-book-text-main">修改密码</span>
      </summary>
      <div className="space-y-2 border-t border-book-border/35 p-3">
        <BookInput
          label="当前密码"
          type="password"
          value={oldPwd}
          onChange={(e) => updateOldPwd(e.target.value)}
        />
        <BookInput
          label="新密码"
          type="password"
          value={newPwd}
          onChange={(e) => updateNewPwd(e.target.value)}
        />
        <div className="flex justify-end">
          <BookButton
            variant="primary"
            size="sm"
            onClick={handleChange}
            disabled={busy || !passwordReady}
          >
            {busy ? (
              <span className="inline-flex items-center gap-1.5">
                <LoadingSpinner size="sm" /> 更新中…
              </span>
            ) : (
              "更新密码"
            )}
          </BookButton>
        </div>
      </div>
    </details>
  );
};

export const SessionManager: React.FC = () => {
  const { addToast } = useToast();
  const [sessions, setSessions] = useState<SessionSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [refreshBusy, setRefreshBusy] = useState(false);
  const mountedRef = useRef(false);
  const loadSeqRef = useRef(0);
  const refreshSeqRef = useRef(0);
  const refreshBusyRef = useRef(false);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      loadSeqRef.current += 1;
      refreshSeqRef.current += 1;
      refreshBusyRef.current = false;
    };
  }, []);

  const load = async () => {
    const requestSeq = loadSeqRef.current + 1;
    loadSeqRef.current = requestSeq;
    setLoading(true);
    try {
      const nextSessions = await listSessions();
      if (!mountedRef.current || loadSeqRef.current !== requestSeq) return;
      setSessions(nextSessions);
    } catch (err) {
      if (!mountedRef.current || loadSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, "加载会话失败"), "error");
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
    const requestSeq = refreshSeqRef.current + 1;
    refreshSeqRef.current = requestSeq;
    refreshBusyRef.current = true;
    setRefreshBusy(true);
    try {
      await refreshToken("");
      if (!mountedRef.current || refreshSeqRef.current !== requestSeq) return;
      addToast("登录凭证已更新", "success");
    } catch (err) {
      if (!mountedRef.current || refreshSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, "刷新失败"), "error");
    } finally {
      if (refreshSeqRef.current === requestSeq) refreshBusyRef.current = false;
      if (mountedRef.current && refreshSeqRef.current === requestSeq) setRefreshBusy(false);
    }
  };

  return (
    <section className="space-y-2 text-xs">
      <div className="flex min-w-0 items-center gap-2">
        <div className="min-w-0 flex-1">
          <h3 className="font-medium text-book-text-main">登录设备</h3>
          <p className="mt-0.5 text-caption text-book-text-muted">
            {loading ? "正在检查会话…" : `${sessions.length} 个会话`}
          </p>
        </div>
      </div>
      {loading ? (
        <div className="a4n-flat-surface flex items-center gap-1.5 p-3 text-book-text-muted">
          <LoadingSpinner size="sm" /> 加载…
        </div>
      ) : sessions.length === 0 ? (
        <div className="a4n-flat-surface p-3 text-book-text-muted">没有其他活动会话。</div>
      ) : (
        <ul className="grid items-start gap-2 sm:grid-cols-2">
          {sessions.map((session) => (
            <SessionRow key={session.id} session={session} />
          ))}
        </ul>
      )}
      <details className="a4n-flat-surface overflow-hidden text-small text-book-text-muted">
        <summary className="flex cursor-pointer list-none items-center gap-2 px-3 py-2.5 transition-colors hover:bg-book-bg-tertiary/40 [&::-webkit-details-marker]:hidden">
          <RefreshCw size={11} className="shrink-0" />
          <span className="font-medium text-book-text-main">登录修复</span>
          <span className="text-caption">凭证异常时使用</span>
        </summary>
        <div className="flex flex-wrap items-center justify-between gap-2 border-t border-book-border/35 p-3">
          <p className="min-w-0 flex-1 leading-relaxed">
            如果登录状态异常，可使用本机已保存的凭证重新建立会话。
          </p>
          <BookButton variant="secondary" size="sm" onClick={handleRefresh} disabled={refreshBusy}>
            {refreshBusy ? (
              <LoadingSpinner size="sm" />
            ) : (
              <RefreshCw size={11} />
            )}
            重新获取凭证
          </BookButton>
        </div>
      </details>
    </section>
  );
};

const SessionRow: React.FC<{ session: SessionSummary }> = ({ session }) => {
  const expiresAt = session.expires_at ? new Date(session.expires_at) : null;
  const expired = Boolean(expiresAt && !Number.isNaN(expiresAt.getTime()) && expiresAt <= new Date());
  const status = session.is_revoked ? "已撤销" : expired ? "已过期" : "活跃";
  return (
    <li className="a4n-flat-surface p-3 text-small">
      <div className="flex min-w-0 items-center gap-2">
        <div className="min-w-0 flex-1">
          <div className="truncate font-medium text-book-text-main" title={session.user_agent ?? ""}>
            {session.user_agent || "未知设备"}
          </div>
          <div className="mt-0.5 text-book-text-muted">
            <span className={status === "活跃" ? "text-success" : undefined}>{status}</span>
            {` · 最近使用 ${formatSessionDate(session.last_used_at ?? session.created_at)}`}
          </div>
        </div>
      </div>
      <details className="mt-1 text-caption text-book-text-muted">
        <summary className="w-fit cursor-pointer hover:text-book-text-main">诊断信息</summary>
        <div className="mt-1 grid gap-0.5 break-all font-mono">
          <span>会话 {session.id}</span>
          <span>创建 {formatSessionDate(session.created_at)}</span>
          <span>到期 {formatSessionDate(session.expires_at)}</span>
        </div>
      </details>
    </li>
  );
};

function formatSessionDate(value: string | null): string {
  return formatUiDateTime(value, "未知");
}
