import React, { useCallback, useEffect, useRef, useState } from "react";
import { Loader2 } from "lucide-react";
import { BookButton } from "../../../components/ui/BookButton";
import { BookInput } from "../../../components/ui/BookInput";
import { useToast } from "../../../components/feedback/Toast";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { extractErrorMessage } from "../../../api";
import {
  changeUserPassword,
  getUser,
  listSessions,
  refreshToken,
  revokeSession,
  updateUserProfile,
} from "../../../api/auth";
import { useAuthStore } from "../../../store/auth";
import { nonEmptyText, textInputValue } from "../../../utils/text";

export const ProfileEditor: React.FC<{ userId: number }> = ({ userId }) => {
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
        editVersionRef.current += 1;
        setEmail(textInputValue(user.email));
        setAvatar(textInputValue(user.avatar_url));
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
    if (busyRef.current) return;
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
      if (editVersionRef.current === requestEditVersion) {
        editVersionRef.current += 1;
        setEmail(textInputValue(user.email));
        setAvatar(textInputValue(user.avatar_url));
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
    <div className="mt-5 pyqt-card p-3 space-y-2 text-xs">
      <div className="font-serif text-sm text-book-text-main">编辑资料</div>
      {loading ? (
        <div className="text-book-text-muted inline-flex items-center gap-1.5">
          <Loader2 size={11} className="animate-spin" /> 加载中…
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
            label="头像 URL"
            value={avatar}
            onChange={(e) => updateAvatar(e.target.value)}
            placeholder="https://…/avatar.png"
          />
          <div className="flex justify-end">
            <BookButton variant="primary" onClick={handleSave} disabled={busy}>
              {busy ? <Loader2 size={11} className="animate-spin" /> : "保存"}
            </BookButton>
          </div>
        </>
      )}
    </div>
  );
};

export const PasswordChanger: React.FC = () => {
  const { addToast } = useToast();
  const [oldPwd, setOldPwd] = useState("");
  const [newPwd, setNewPwd] = useState("");
  const [busy, setBusy] = useState(false);
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
    <div className="mt-3 pyqt-card p-3 space-y-2 text-xs">
      <div className="font-serif text-sm text-book-text-main">修改密码</div>
      <BookInput
        label="旧密码"
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
        <BookButton variant="primary" onClick={handleChange} disabled={busy}>
          {busy ? <Loader2 size={11} className="animate-spin" /> : "修改密码"}
        </BookButton>
      </div>
    </div>
  );
};

export const SessionManager: React.FC = () => {
  const { addToast } = useToast();
  const onUnauthorized = useAuthStore((state) => state.onUnauthorized);
  const [sessions, setSessions] = useState<unknown[]>([]);
  const [loading, setLoading] = useState(false);
  const [revoking, setRevoking] = useState(false);
  const [refreshTokenInput, setRefreshTokenInput] = useState("");
  const [refreshBusy, setRefreshBusy] = useState(false);
  const [tokenResult, setTokenResult] = useState<string | null>(null);
  const mountedRef = useRef(false);
  const loadSeqRef = useRef(0);
  const revokeSeqRef = useRef(0);
  const refreshSeqRef = useRef(0);
  const refreshInputVersionRef = useRef(0);
  const revokingRef = useRef(false);
  const refreshBusyRef = useRef(false);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      loadSeqRef.current += 1;
      revokeSeqRef.current += 1;
      refreshSeqRef.current += 1;
      revokingRef.current = false;
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

  const handleRevoke = async () => {
    if (revokingRef.current) return;
    revokingRef.current = true;
    const ok = await confirmDialog({
      title: "撤销当前会话",
      message: "撤销当前会话？",
      confirmText: "撤销",
      dialogType: "warning",
    });
    if (!ok) {
      revokingRef.current = false;
      return;
    }
    const requestSeq = revokeSeqRef.current + 1;
    revokeSeqRef.current = requestSeq;
    setRevoking(true);
    try {
      await revokeSession();
      if (!mountedRef.current || revokeSeqRef.current !== requestSeq) return;
      addToast("已撤销", "success");
      onUnauthorized();
    } catch (err) {
      if (!mountedRef.current || revokeSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, "撤销失败"), "error");
    } finally {
      if (revokeSeqRef.current === requestSeq) revokingRef.current = false;
      if (mountedRef.current && revokeSeqRef.current === requestSeq) setRevoking(false);
    }
  };

  const handleRefresh = async () => {
    if (refreshBusyRef.current) return;
    const requestInput = refreshTokenInput.trim();
    if (!requestInput) {
      addToast("请填写 refresh token", "error");
      return;
    }
    const requestSeq = refreshSeqRef.current + 1;
    const requestInputVersion = refreshInputVersionRef.current;
    refreshSeqRef.current = requestSeq;
    refreshBusyRef.current = true;
    setRefreshBusy(true);
    try {
      const pair = await refreshToken(requestInput);
      if (!mountedRef.current || refreshSeqRef.current !== requestSeq) return;
      if (refreshInputVersionRef.current === requestInputVersion) {
        setTokenResult(JSON.stringify(pair, null, 2));
        addToast("已刷新令牌", "success");
      } else {
        setTokenResult(null);
        addToast("已刷新提交时的令牌，当前输入已变化", "info");
      }
    } catch (err) {
      if (!mountedRef.current || refreshSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, "刷新失败"), "error");
    } finally {
      if (refreshSeqRef.current === requestSeq) refreshBusyRef.current = false;
      if (mountedRef.current && refreshSeqRef.current === requestSeq) setRefreshBusy(false);
    }
  };

  return (
    <div className="mt-3 pyqt-card p-3 space-y-2 text-xs">
      <div className="font-serif text-sm text-book-text-main inline-flex items-center justify-between w-full">
        会话管理
        <span className="pyqt-pill !text-[10px]">{sessions.length} 个</span>
      </div>
      {loading ? (
        <div className="text-book-text-muted inline-flex items-center gap-1.5">
          <Loader2 size={11} className="animate-spin" /> 加载…
        </div>
      ) : sessions.length === 0 ? (
        <div className="text-book-text-muted italic">（无活动会话）</div>
      ) : (
        <ul className="space-y-1 max-h-32 overflow-y-auto custom-scrollbar">
          {sessions.map((session, index) => (
            <li
              key={index}
              className="a4n-stat-tile p-1.5 font-mono text-[10px] break-all"
            >
              {JSON.stringify(session)}
            </li>
          ))}
        </ul>
      )}
      <div className="flex justify-end">
        <BookButton variant="secondary" onClick={handleRevoke} disabled={revoking}>
          {revoking ? <Loader2 size={11} className="animate-spin" /> : "撤销当前会话"}
        </BookButton>
      </div>
      <div className="border-t border-book-border/30 pt-2">
        <div className="text-book-text-muted mb-1">刷新令牌</div>
        <div className="flex items-center gap-1.5">
          <input
            value={refreshTokenInput}
            onChange={(e) => {
              refreshInputVersionRef.current += 1;
              setRefreshTokenInput(e.target.value);
              setTokenResult(null);
            }}
            placeholder="粘贴 refresh_token"
            className="pyqt-input !py-1 flex-1 text-[11px] font-mono"
          />
          <BookButton variant="secondary" onClick={handleRefresh} disabled={refreshBusy}>
            {refreshBusy ? <Loader2 size={11} className="animate-spin" /> : "刷新"}
          </BookButton>
        </div>
        {tokenResult && (
          <pre className="mt-1 rounded border border-book-border/40 bg-book-bg-tertiary/40 p-2 text-[10px] font-mono whitespace-pre-wrap break-words max-h-32 overflow-y-auto custom-scrollbar">
            {tokenResult}
          </pre>
        )}
      </div>
    </div>
  );
};
