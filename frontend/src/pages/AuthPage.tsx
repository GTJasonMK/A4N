import React, { useEffect, useRef, useState } from "react";
import { BookOpen, Globe } from "lucide-react";
import { LoadingSpinner } from "../components/ui/LoadingSpinner";
import { BookButton } from "../components/ui/BookButton";
import { BookInput } from "../components/ui/BookInput";
import { BookCard } from "../components/ui/BookCard";
import { SegmentedControl } from "../components/ui/SegmentedControl";
import { useAuthStore } from "../store/auth";
import { useToast } from "../components/feedback/Toast";
import { extractErrorMessage } from "../api";
import { oauth2AuthorizeUrl } from "../api/auth";
import { copyTextToClipboard } from "../utils/clipboard";

const OAUTH_PROVIDERS = ["github", "google"] as const;
const OAUTH_PROVIDER_LABELS: Record<(typeof OAUTH_PROVIDERS)[number], string> = {
  github: "GitHub",
  google: "Google",
};

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
  const [oauthLinkCopied, setOauthLinkCopied] = useState(false);
  const [oauthBusy, setOauthBusy] = useState(false);
  const [showOauth, setShowOauth] = useState(false);
  const submitBusyRef = useRef(false);
  const oauthBusyRef = useRef(false);
  const oauthProviderRef = useRef(oauthProvider);
  const oauthSeqRef = useRef(0);
  const mountedRef = useRef(false);
  const submitReady =
    username.trim().length > 0 &&
    password.length > 0 &&
    (mode === "login" ||
      (email.trim().length > 0 && inviteCode.trim().length > 0));

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
    setOauthLinkCopied(false);
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
      addToast("请选择登录服务", "error");
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
      setOauthLinkCopied(false);
      try {
        await copyTextToClipboard(url);
        setOauthLinkCopied(true);
        addToast("授权链接已复制，请在浏览器打开", "success");
      } catch {
        setOauthLinkCopied(false);
        addToast("授权链接已生成，但自动复制失败，请展开链接后手动复制", "error");
      }
    } catch (err) {
      if (
        !mountedRef.current ||
        oauthSeqRef.current !== requestSeq ||
        oauthProviderRef.current.trim() !== requestProvider
      ) {
        return;
      }
      addToast(extractErrorMessage(err, "获取授权链接失败"), "error");
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
      addToast("请填写授权码", "error");
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
      addToast(`第三方登录成功：${user.username}`, "success");
    } catch (err) {
      if (!mountedRef.current || oauthSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, "第三方登录失败"), "error");
    } finally {
      if (oauthSeqRef.current === requestSeq) oauthBusyRef.current = false;
      if (mountedRef.current && oauthSeqRef.current === requestSeq)
        setOauthBusy(false);
    }
  };

  return (
    <div className="flex h-full w-full items-start justify-center overflow-y-auto bg-book-bg p-4 sm:items-center sm:p-8">
      <BookCard className="my-auto w-full max-w-[440px] rounded-3xl p-5 sm:p-6">
        <header className="mb-5 flex items-center gap-3">
          <span className="inline-flex h-10 w-10 shrink-0 items-center justify-center rounded-2xl bg-book-primary/10 text-book-primary">
            <BookOpen size={18} />
          </span>
          <div className="min-w-0">
            <h1 className="font-serif text-lg font-semibold text-book-text-main">A4N</h1>
            <p className="text-small text-book-text-muted">AI 长篇创作助手</p>
          </div>
        </header>

        {!showOauth ? (
          <>
            <SegmentedControl
              ariaLabel="账户操作"
              className="mb-4 w-full [&_.a4n-seg__btn]:min-h-9 [&_.a4n-seg__btn]:flex-1"
              value={mode}
              onChange={setMode}
              options={[
                { id: "login", label: "登录", disabled: submitting },
                { id: "register", label: "注册", disabled: submitting },
              ]}
            />

            <form onSubmit={handleSubmit} className="flex flex-col gap-3">
              <label className="flex flex-col gap-1">
                <span className="text-body-sm font-medium text-book-text-sub">用户名</span>
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
                  <span className="text-body-sm font-medium text-book-text-sub">邮箱</span>
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
                  <span className="text-body-sm font-medium text-book-text-sub">邀请码</span>
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
                <span className="text-body-sm font-medium text-book-text-sub">密码</span>
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
                disabled={submitting || !submitReady}
                className="mt-1 w-full"
              >
                {submitting ? (
                  <span className="inline-flex items-center gap-1.5">
                    <LoadingSpinner size="sm" />
                    {mode === "login" ? "登录中…" : "创建中…"}
                  </span>
                ) : mode === "login" ? (
                  "登录"
                ) : (
                  "创建账户"
                )}
              </BookButton>
            </form>

            <p className="mt-4 text-center text-small leading-relaxed text-book-text-muted">
              {mode === "login"
                ? "你的创作内容优先保存在本地。"
                : "创建成功后会自动登录当前账户。"}
            </p>

            {mode === "login" ? (
              <>
                <hr className="story-divider my-4" />
                <button
                  type="button"
                  onClick={() => setShowOauth(true)}
                  disabled={submitting}
                  aria-expanded={false}
                  aria-controls="oauth-login-panel"
                  className="inline-flex min-h-10 w-full items-center justify-center gap-1.5 text-xs text-book-text-sub hover:text-book-text-main disabled:cursor-not-allowed disabled:opacity-45"
                >
                  <Globe size={12} /> 使用 GitHub 或 Google 登录
                </button>
              </>
            ) : null}
          </>
        ) : (
          <section id="oauth-login-panel" aria-label="第三方账号登录" className="space-y-4 text-xs">
            <div>
              <h2 className="text-sm font-semibold text-book-text-main">第三方账号登录</h2>
              <p className="mt-1 text-small leading-relaxed text-book-text-muted">
                先获取授权链接，在浏览器完成授权后粘贴返回的授权码。
              </p>
            </div>
            <label className="flex flex-col gap-1">
              <span className="text-body-sm font-medium text-book-text-sub">登录服务</span>
              <select
                value={oauthProvider}
                onChange={(e) => handleOauthProviderChange(e.target.value)}
                disabled={oauthBusy || oauthAuthorizeUrl !== null}
                className="pyqt-input min-w-0"
              >
                {OAUTH_PROVIDERS.map((provider) => (
                  <option key={provider} value={provider}>
                    {OAUTH_PROVIDER_LABELS[provider]}
                  </option>
                ))}
              </select>
            </label>
            {oauthAuthorizeUrl == null ? (
              <BookButton
                variant="primary"
                onClick={handleGetAuthorizeUrl}
                disabled={oauthBusy}
                className="w-full"
              >
                {oauthBusy ? (
                  <span className="inline-flex items-center gap-1.5">
                    <LoadingSpinner size="sm" /> 正在生成…
                  </span>
                ) : (
                  "获取授权链接"
                )}
              </BookButton>
            ) : (
              <>
                <div className="rounded-xl bg-book-primary/8 px-3 py-2.5 text-small leading-relaxed text-book-text-sub">
                  {oauthLinkCopied
                    ? "授权链接已复制。请在浏览器完成授权，再粘贴返回的授权码。"
                    : "授权链接已生成，但自动复制失败。请展开链接后手动复制。"}
                </div>
                <details className="text-caption text-book-text-muted">
                  <summary className="w-fit cursor-pointer hover:text-book-text-main">查看授权链接</summary>
                  <div className="mt-1 break-all rounded-xl bg-book-bg-tertiary/50 p-2.5 font-mono text-small leading-relaxed text-book-text-sub">
                    {oauthAuthorizeUrl}
                  </div>
                </details>
                <label className="flex flex-col gap-1">
                  <span className="text-body-sm font-medium text-book-text-sub">授权码</span>
                  <BookInput
                    value={oauthCode}
                    onChange={(e) => setOauthCode(e.target.value)}
                    placeholder="粘贴浏览器返回的授权码"
                    disabled={oauthBusy}
                    autoComplete="one-time-code"
                  />
                </label>
                <BookButton
                  variant="primary"
                  onClick={handleOauthCallback}
                  disabled={oauthBusy || !oauthCode.trim()}
                  className="w-full"
                >
                  {oauthBusy ? (
                    <span className="inline-flex items-center gap-1.5">
                      <LoadingSpinner size="sm" /> 正在登录…
                    </span>
                  ) : (
                    "完成登录"
                  )}
                </BookButton>
              </>
            )}
            <button
              type="button"
              onClick={() => {
                setShowOauth(false);
                handleOauthProviderChange(oauthProvider);
              }}
              disabled={oauthBusy}
              className="inline-flex min-h-9 w-full items-center justify-center text-xs text-book-text-muted hover:text-book-text-main disabled:cursor-not-allowed disabled:opacity-45"
            >
              返回用户名登录
            </button>
          </section>
        )}
      </BookCard>
    </div>
  );
};
