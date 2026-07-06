import React, { useEffect, useRef, useState } from "react";
import { Loader2, BookOpen, Globe } from "lucide-react";
import { BookButton } from "../components/ui/BookButton";
import { BookInput } from "../components/ui/BookInput";
import { BookCard } from "../components/ui/BookCard";
import { useAuthStore } from "../store/auth";
import { useToast } from "../components/feedback/Toast";
import { extractErrorMessage } from "../api";
import { oauth2AuthorizeUrl } from "../api/auth";
import { copyTextToClipboard } from "../utils/clipboard";

const OAUTH_PROVIDERS = ["github", "google"] as const;

export const AuthPage: React.FC = () => {
  const { login, oauthLogin, register } = useAuthStore();
  const { addToast } = useToast();

  const [mode, setMode] = useState<"login" | "register">("login");
  const [username, setUsername] = useState("");
  const [email, setEmail] = useState("");
  const [inviteCode, setInviteCode] = useState("");
  const [password, setPassword] = useState("");
  const [submitting, setSubmitting] = useState(false);

  const [oauthProvider, setOauthProvider] = useState("github");
  const [oauthCode, setOauthCode] = useState("");
  const [oauthAuthorizeUrl, setOauthAuthorizeUrl] = useState<string | null>(
    null,
  );
  const [oauthBusy, setOauthBusy] = useState(false);
  const [showOauth, setShowOauth] = useState(false);
  const submitBusyRef = useRef(false);
  const oauthBusyRef = useRef(false);
  const oauthProviderRef = useRef(oauthProvider);
  const oauthSeqRef = useRef(0);
  const mountedRef = useRef(false);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      submitBusyRef.current = false;
      oauthBusyRef.current = false;
      oauthSeqRef.current += 1;
    };
  }, []);

  const handleOauthProviderChange = (nextProvider: string) => {
    oauthProviderRef.current = nextProvider;
    oauthSeqRef.current += 1;
    setOauthProvider(nextProvider);
    setOauthAuthorizeUrl(null);
    setOauthCode("");
    oauthBusyRef.current = false;
    setOauthBusy(false);
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (submitBusyRef.current) return;
    if (!username.trim() || !password) {
      addToast("请填写用户名和密码", "error");
      return;
    }
    if (mode === "register" && !email.trim()) {
      addToast("请填写邮箱", "error");
      return;
    }
    if (mode === "register" && !inviteCode.trim()) {
      addToast("请填写邀请码", "error");
      return;
    }
    submitBusyRef.current = true;
    setSubmitting(true);
    const requestMode = mode;
    const requestUsername = username.trim();
    const requestEmail = email.trim();
    const requestPassword = password;
    const requestInviteCode = inviteCode.trim();
    try {
      if (requestMode === "login") {
        await login(requestUsername, requestPassword);
        if (!mountedRef.current) return;
        addToast(`欢迎回来，${requestUsername}`, "success");
      } else {
        await register(
          requestUsername,
          requestEmail,
          requestPassword,
          requestInviteCode,
        );
        if (!mountedRef.current) return;
        addToast(`账户已创建：${requestUsername}`, "success");
      }
    } catch (err) {
      if (!mountedRef.current) return;
      addToast(
        extractErrorMessage(
          err,
          requestMode === "login" ? "登录失败" : "注册失败",
        ),
        "error",
      );
    } finally {
      submitBusyRef.current = false;
      if (mountedRef.current) setSubmitting(false);
    }
  };

  const handleGetAuthorizeUrl = async () => {
    const requestProvider = oauthProvider.trim();
    if (!requestProvider) {
      addToast("请输入 provider 名", "error");
      return;
    }
    if (oauthBusyRef.current) return;
    const requestSeq = oauthSeqRef.current + 1;
    oauthSeqRef.current = requestSeq;
    oauthBusyRef.current = true;
    setOauthBusy(true);
    try {
      const url = await oauth2AuthorizeUrl(requestProvider);
      if (
        !mountedRef.current ||
        oauthSeqRef.current !== requestSeq ||
        oauthProviderRef.current.trim() !== requestProvider
      ) {
        return;
      }
      setOauthAuthorizeUrl(url);
      try {
        await copyTextToClipboard(url);
        addToast("授权 URL 已复制到剪贴板，请在浏览器打开", "success");
      } catch (copyErr) {
        addToast(
          extractErrorMessage(copyErr, "授权 URL 已生成，但复制到剪贴板失败"),
          "error",
        );
      }
    } catch (err) {
      if (
        !mountedRef.current ||
        oauthSeqRef.current !== requestSeq ||
        oauthProviderRef.current.trim() !== requestProvider
      ) {
        return;
      }
      addToast(extractErrorMessage(err, "获取授权 URL 失败"), "error");
    } finally {
      if (oauthSeqRef.current === requestSeq) oauthBusyRef.current = false;
      if (mountedRef.current && oauthSeqRef.current === requestSeq)
        setOauthBusy(false);
    }
  };

  const handleOauthCallback = async () => {
    const requestProvider = oauthProvider.trim();
    const requestCode = oauthCode.trim();
    if (!requestProvider || !requestCode) {
      addToast("请填写 provider 与回调 code", "error");
      return;
    }
    if (oauthBusyRef.current) return;
    const requestSeq = oauthSeqRef.current + 1;
    oauthSeqRef.current = requestSeq;
    oauthBusyRef.current = true;
    setOauthBusy(true);
    try {
      const user = await oauthLogin(requestProvider, requestCode);
      if (!mountedRef.current || oauthSeqRef.current !== requestSeq) return;
      addToast(`OAuth 登录成功：${user.username}`, "success");
    } catch (err) {
      if (!mountedRef.current || oauthSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, "OAuth 登录失败"), "error");
    } finally {
      if (oauthSeqRef.current === requestSeq) oauthBusyRef.current = false;
      if (mountedRef.current && oauthSeqRef.current === requestSeq)
        setOauthBusy(false);
    }
  };

  return (
    <div className="flex h-full w-full items-start justify-center overflow-y-auto bg-book-bg p-3 sm:items-center">
      <BookCard className="my-auto w-full max-w-sm p-4">
        <div className="flex items-center gap-2 mb-4">
          <BookOpen className="text-book-primary" size={20} />
          <div>
            <h1 className="text-base font-serif font-semibold text-book-text-main">
              A4N
            </h1>
            <p className="text-[10px] text-book-text-muted">AI 长篇创作助手</p>
          </div>
        </div>

        <div className="flex gap-0 mb-4 border border-book-border bg-book-bg-paper text-xs font-semibold rounded-sm overflow-hidden">
          {(["login", "register"] as const).map((m) => (
            <button
              key={m}
              type="button"
              onClick={() => setMode(m)}
              className={`flex-1 min-h-10 px-3 py-1.5 transition-colors ${
                mode === m
                  ? "bg-book-primary/10 text-book-primary"
                  : "text-book-text-muted hover:text-book-text-main hover:bg-book-bg-tertiary"
              }`}
            >
              {m === "login" ? "登录" : "注册"}
            </button>
          ))}
        </div>

        <form onSubmit={handleSubmit} className="flex flex-col gap-2">
          <label className="flex flex-col gap-1">
            <span className="text-[11px] text-book-text-sub">用户名</span>
            <BookInput
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              placeholder="输入用户名"
              autoComplete="username"
              disabled={submitting}
            />
          </label>
          {mode === "register" && (
            <label className="flex flex-col gap-1">
              <span className="text-[11px] text-book-text-sub">邮箱</span>
              <BookInput
                type="email"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                placeholder="输入邮箱"
                autoComplete="email"
                disabled={submitting}
              />
            </label>
          )}
          {mode === "register" && (
            <label className="flex flex-col gap-1">
              <span className="text-[11px] text-book-text-sub">邀请码</span>
              <BookInput
                value={inviteCode}
                onChange={(e) => setInviteCode(e.target.value)}
                placeholder="输入邀请码"
                autoComplete="one-time-code"
                disabled={submitting}
              />
            </label>
          )}
          <label className="flex flex-col gap-1">
            <span className="text-[11px] text-book-text-sub">密码</span>
            <BookInput
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="输入密码"
              autoComplete={
                mode === "login" ? "current-password" : "new-password"
              }
              disabled={submitting}
            />
          </label>

          <BookButton
            type="submit"
            variant="primary"
            disabled={submitting}
            className="mt-1"
          >
            {submitting ? (
              <span className="inline-flex items-center gap-1.5">
                <Loader2 size={12} className="animate-spin" />
                处理中…
              </span>
            ) : mode === "login" ? (
              "登录"
            ) : (
              "创建账户"
            )}
          </BookButton>
        </form>

        <p className="text-xs text-book-text-muted text-center mt-6 leading-relaxed">
          A4N 是本地优先的 Rust + Tauri 桌面应用，也可私有化 Web 部署。
          <br />
          注册后会自动登录当前账户。
        </p>

        <hr className="story-divider my-4" />
        <button
          type="button"
          onClick={() => setShowOauth((v) => !v)}
          className="inline-flex min-h-10 w-full items-center justify-center gap-1.5 text-xs text-book-text-sub hover:text-book-text-main"
        >
          <Globe size={12} />{" "}
          {showOauth ? "收起 OAuth 登录" : "使用第三方账号登录（OAuth）"}
        </button>

        {showOauth && (
          <div className="mt-3 space-y-2 text-xs">
            <div className="grid gap-2 sm:grid-cols-[4rem_minmax(0,1fr)_auto] sm:items-center">
              <span className="text-book-text-muted sm:w-16">
                Provider
              </span>
              <select
                value={oauthProvider}
                onChange={(e) => handleOauthProviderChange(e.target.value)}
                disabled={oauthBusy}
                className="pyqt-input !py-1 min-w-0"
              >
                {OAUTH_PROVIDERS.map((provider) => (
                  <option key={provider} value={provider}>
                    {provider}
                  </option>
                ))}
              </select>
              <BookButton
                variant="secondary"
                onClick={handleGetAuthorizeUrl}
                disabled={oauthBusy}
                className="w-full !px-3 sm:w-auto"
              >
                {oauthBusy ? (
                  <Loader2 size={11} className="animate-spin" />
                ) : (
                  "获取授权链接"
                )}
              </BookButton>
            </div>
            {oauthAuthorizeUrl && (
              <div className="rounded border border-book-border/40 bg-book-bg-paper/50 p-2 break-all text-[10px] font-mono">
                {oauthAuthorizeUrl}
              </div>
            )}
            <div className="grid gap-2 sm:grid-cols-[4rem_minmax(0,1fr)_auto] sm:items-center">
              <span className="text-book-text-muted sm:w-16">
                回调 code
              </span>
              <input
                value={oauthCode}
                onChange={(e) => setOauthCode(e.target.value)}
                placeholder="粘贴从浏览器回调拿到的 code"
                disabled={oauthBusy}
                className="pyqt-input !py-1 min-w-0 text-[11px] font-mono"
              />
              <BookButton
                variant="primary"
                onClick={handleOauthCallback}
                disabled={oauthBusy}
                className="w-full !px-3 sm:w-auto"
              >
                {oauthBusy ? (
                  <Loader2 size={11} className="animate-spin" />
                ) : (
                  "完成登录"
                )}
              </BookButton>
            </div>
            <p className="text-[10px] text-book-text-muted">
              请在浏览器完成授权流程，再从回调地址复制 code 并粘贴到这里。
            </p>
          </div>
        )}
      </BookCard>
    </div>
  );
};
