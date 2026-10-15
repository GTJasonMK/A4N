import { ThemedSelect } from "../components/ui/ThemedSelect";
import React, { useCallback, useEffect, useRef, useState } from "react";
import { BookOpen, Globe, RefreshCw } from "lucide-react";
import { LoadingSpinner } from "../components/ui/LoadingSpinner";
import { StatusBarToggle } from "../components/ui/StatusBar";
import { BookButton } from "../components/ui/BookButton";
import { BookInput } from "../components/ui/BookInput";
import { BookCard } from "../components/ui/BookCard";
import { SegmentedControl } from "../components/ui/SegmentedControl";
import { useAuthStore } from "../store/auth";
import { useToast } from "../components/feedback/Toast";
import { extractErrorMessage } from "../api";
import { beginOAuthLogin, cancelOAuthLogin, getOAuth2AvailableProviders } from "../api/oauth";
import { useI18n } from "../i18n";
import { logger } from "../utils/logger";

const OAUTH_PROVIDER_LABELS = {
  github: "GitHub",
  google: "Google",
} as const;
type OAuthProvider = keyof typeof OAUTH_PROVIDER_LABELS;

const isOAuthProvider = (value: string): value is OAuthProvider =>
  Object.prototype.hasOwnProperty.call(OAUTH_PROVIDER_LABELS, value);

export const AuthPage: React.FC = () => {
  const { login, oauthLogin, register } = useAuthStore();
  const { addToast } = useToast();
  const { t } = useI18n();

  const [mode, setMode] = useState<"login" | "register">("login");
  const [username, setUsername] = useState("");
  const [email, setEmail] = useState("");
  const [inviteCode, setInviteCode] = useState("");
  const [password, setPassword] = useState("");
  const [submitting, setSubmitting] = useState(false);

  const [oauthProviders, setOauthProviders] = useState<OAuthProvider[] | null>(null);
  const [oauthProvidersLoading, setOauthProvidersLoading] = useState(true);
  const [oauthProviderLoadFailed, setOauthProviderLoadFailed] = useState(false);
  const [oauthProvider, setOauthProvider] = useState<OAuthProvider | "">("");
  const [oauthBusy, setOauthBusy] = useState(false);
  const [showOauth, setShowOauth] = useState(false);
  const submitBusyRef = useRef(false);
  const oauthBusyRef = useRef(false);
  const oauthProviderRef = useRef<OAuthProvider | "">("");
  const oauthProviderLoadSeqRef = useRef(0);
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
      oauthProviderLoadSeqRef.current += 1;
      void cancelOAuthLogin().catch((error: unknown) => {
        logger.scoped("OAuth").error("Failed to cancel browser authorization", { error });
      });
    };
  }, []);

  const loadOauthProviders = useCallback(async () => {
    const requestSeq = oauthProviderLoadSeqRef.current + 1;
    oauthProviderLoadSeqRef.current = requestSeq;
    setOauthProvidersLoading(true);
    setOauthProviderLoadFailed(false);
    try {
      const providers = await getOAuth2AvailableProviders();
      if (providers.some((provider) => !isOAuthProvider(provider))) {
        throw new Error("OAuth provider availability response is invalid");
      }
      if (!mountedRef.current || oauthProviderLoadSeqRef.current !== requestSeq) return;
      const available = providers as OAuthProvider[];
      const initialProvider = available[0] ?? "";
      setOauthProviders(available);
      oauthProviderRef.current = initialProvider;
      setOauthProvider(initialProvider);
    } catch (error) {
      if (!mountedRef.current || oauthProviderLoadSeqRef.current !== requestSeq) return;
      logger.scoped("OAuth").error("Failed to load available OAuth providers", { error });
      setOauthProviderLoadFailed(true);
      setOauthProviders([]);
    } finally {
      if (mountedRef.current && oauthProviderLoadSeqRef.current === requestSeq) {
        setOauthProvidersLoading(false);
      }
    }
  }, []);

  useEffect(() => {
    void loadOauthProviders();
  }, [loadOauthProviders]);

  const handleOauthProviderChange = (nextProvider: string) => {
    if (!isOAuthProvider(nextProvider) || !oauthProviders?.includes(nextProvider)) return;
    oauthProviderRef.current = nextProvider;
    oauthSeqRef.current += 1;
    setOauthProvider(nextProvider);
    oauthBusyRef.current = false;
    setOauthBusy(false);
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (submitBusyRef.current) return;
    if (!username.trim() || !password) {
      addToast(t("auth.fillUsernamePassword"), "error");
      return;
    }
    if (mode === "register" && !email.trim()) {
      addToast(t("auth.fillEmail"), "error");
      return;
    }
    if (mode === "register" && !inviteCode.trim()) {
      addToast(t("auth.fillInviteCode"), "error");
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
        addToast(t("auth.welcomeUser", { name: requestUsername }), "success");
      } else {
        await register(
          requestUsername,
          requestEmail,
          requestPassword,
          requestInviteCode,
        );
        if (!mountedRef.current) return;
        addToast(t("auth.accountCreated", { name: requestUsername }), "success");
      }
    } catch (err) {
      if (!mountedRef.current) return;
      addToast(
        extractErrorMessage(
          err,
          requestMode === "login" ? t("auth.loginFailed") : t("auth.registerFailed"),
        ),
        "error",
      );
    } finally {
      submitBusyRef.current = false;
      if (mountedRef.current) setSubmitting(false);
    }
  };

  const handleOauthLogin = async () => {
    const requestProvider = oauthProvider.trim();
    if (!requestProvider || !oauthProviders?.includes(requestProvider as OAuthProvider)) {
      addToast(t("auth.oauthUnavailable"), "error");
      return;
    }
    if (oauthBusyRef.current) return;
    const requestSeq = oauthSeqRef.current + 1;
    oauthSeqRef.current = requestSeq;
    oauthBusyRef.current = true;
    setOauthBusy(true);
    try {
      const callbackUrl = await beginOAuthLogin(requestProvider);
      if (
        !mountedRef.current ||
        oauthSeqRef.current !== requestSeq ||
        oauthProviderRef.current.trim() !== requestProvider
      ) {
        return;
      }
      if (callbackUrl) {
        const { user } = await oauthLogin(callbackUrl);
        if (
          !mountedRef.current ||
          oauthSeqRef.current !== requestSeq ||
          oauthProviderRef.current.trim() !== requestProvider
        ) {
          return;
        }
        addToast(t("auth.oauthLoginSuccess", { name: user.username }), "success");
      }
    } catch (err) {
      if (
        !mountedRef.current ||
        oauthSeqRef.current !== requestSeq ||
        oauthProviderRef.current.trim() !== requestProvider
      ) {
        return;
      }
      addToast(extractErrorMessage(err, t("auth.oauthLoginFailed")), "error");
    } finally {
      if (oauthSeqRef.current === requestSeq) oauthBusyRef.current = false;
      if (mountedRef.current && oauthSeqRef.current === requestSeq)
        setOauthBusy(false);
    }
  };

  const handleCancelOauth = async () => {
    oauthSeqRef.current += 1;
    try {
      await cancelOAuthLogin();
      if (!mountedRef.current) return;
      setShowOauth(false);
      handleOauthProviderChange(oauthProvider);
    } catch (err) {
      if (!mountedRef.current) return;
      oauthBusyRef.current = false;
      setOauthBusy(false);
      addToast(extractErrorMessage(err, t("auth.oauthLoginFailed")), "error");
    }
  };

  return (
    <div className="a4n-auth-page">
      <div className="a4n-auth-shell">
        <aside className="a4n-auth-intro" aria-label={t("auth.productIntro")}>
          <div className="a4n-auth-brand-lockup">
            <span className="a4n-auth-brand-mark" aria-hidden="true">
              <BookOpen size={19} />
            </span>
            <span>
              <strong>A4N</strong>
              <small>AI Narrative Studio</small>
            </span>
          </div>

          <div className="a4n-auth-intro-copy">
            <p className="a4n-auth-eyebrow">{t("auth.tagline")}</p>
            <h1>
              {t("auth.heroLineOne")}
              <span>{t("auth.heroLineTwo")}</span>
            </h1>
            <p>{t("auth.heroDescription")}</p>
          </div>

          <div className="a4n-auth-intro-footer">
            <span>{t("auth.localFirst")}</span>
            <span>{t("auth.structuredCreation")}</span>
            <span>{t("auth.endToEnd")}</span>
          </div>
        </aside>

        <BookCard className="a4n-auth-card">
          <header className="a4n-auth-form-header">
            <div className="flex items-center justify-between gap-2">
              <p className="a4n-kicker">{t("auth.continueCreation")}</p>
              <StatusBarToggle />
            </div>
            <h2>
              {showOauth
                ? t("auth.oauthLogin")
                : mode === "login"
                  ? t("auth.welcomeBack")
                  : t("auth.createAccount")}
            </h2>
            <p>
              {showOauth
                ? t("auth.oauthDescription")
                : mode === "login"
                  ? t("auth.loginDescription")
                  : t("auth.registerDescription")}
            </p>
          </header>

          {!showOauth ? (
            <>
              <SegmentedControl
                ariaLabel={t("auth.accountActions")}
                className="mb-5 w-full [&_.a4n-seg__btn]:min-h-9 [&_.a4n-seg__btn]:flex-1"
                value={mode}
                onChange={setMode}
                options={[
                  { id: "login", label: t("auth.login"), disabled: submitting },
                  { id: "register", label: t("auth.register"), disabled: submitting },
                ]}
              />

              <form onSubmit={handleSubmit} className="flex flex-col gap-3.5">
                <label className="flex flex-col gap-1">
                  <span className="text-body-sm font-medium text-book-text-sub">{t("auth.username")}</span>
                  <BookInput
                    value={username}
                    onChange={(e) => setUsername(e.target.value)}
                    placeholder={t("auth.usernamePlaceholder")}
                    autoComplete="username"
                    disabled={submitting}
                  />
                </label>
                {mode === "register" && (
                  <label className="flex flex-col gap-1">
                    <span className="text-body-sm font-medium text-book-text-sub">{t("auth.email")}</span>
                    <BookInput
                      type="email"
                      value={email}
                      onChange={(e) => setEmail(e.target.value)}
                      placeholder={t("auth.emailPlaceholder")}
                      autoComplete="email"
                      disabled={submitting}
                    />
                  </label>
                )}
                {mode === "register" && (
                  <label className="flex flex-col gap-1">
                    <span className="text-body-sm font-medium text-book-text-sub">{t("auth.inviteCode")}</span>
                    <BookInput
                      value={inviteCode}
                      onChange={(e) => setInviteCode(e.target.value)}
                      placeholder={t("auth.inviteCodePlaceholder")}
                      autoComplete="one-time-code"
                      disabled={submitting}
                    />
                  </label>
                )}
                <label className="flex flex-col gap-1">
                  <span className="text-body-sm font-medium text-book-text-sub">{t("auth.password")}</span>
                  <BookInput
                    type="password"
                    value={password}
                    onChange={(e) => setPassword(e.target.value)}
                    placeholder={t("auth.passwordPlaceholder")}
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
                      {mode === "login" ? t("auth.loggingIn") : t("auth.creatingAccount")}
                    </span>
                  ) : mode === "login" ? (
                    t("auth.login")
                  ) : (
                    t("auth.createAccountButton")
                  )}
                </BookButton>
              </form>

              <p className="a4n-auth-privacy">
                {mode === "login"
                  ? t("auth.localPrivacy")
                  : t("auth.registerPrivacy")}
              </p>

              {mode === "login" && oauthProvidersLoading ? (
                <p role="status" className="a4n-auth-privacy">
                  {t("auth.oauthProvidersLoading")}
                </p>
              ) : mode === "login" && oauthProviders && oauthProviders.length > 0 ? (
                <>
                  <hr className="story-divider my-5" />
                  <button
                    type="button"
                    onClick={() => setShowOauth(true)}
                    disabled={submitting}
                    aria-expanded={showOauth}
                    aria-controls="oauth-login-panel"
                    className="a4n-auth-oauth-button"
                  >
                    <Globe size={13} /> {t("auth.useOauth")}
                  </button>
                </>
              ) : null}
              {mode === "login" && !oauthProvidersLoading && oauthProviders !== null && oauthProviders.length === 0 ? (
                <div className="flex flex-wrap items-center gap-2">
                  <p role="status" className="a4n-auth-privacy">
                    {t(oauthProviderLoadFailed ? "auth.oauthProviderLoadFailed" : "auth.oauthUnavailable")}
                  </p>
                  {oauthProviderLoadFailed ? (
                    <BookButton
                      type="button"
                      variant="ghost"
                      size="sm"
                      onClick={() => void loadOauthProviders()}
                      disabled={oauthProvidersLoading}
                    >
                      <RefreshCw size={12} /> {t("common.retry")}
                    </BookButton>
                  ) : null}
                </div>
              ) : null}
            </>
          ) : (
            <section
              id="oauth-login-panel"
              aria-label={t("auth.oauthLogin")}
              className="space-y-4 text-xs"
            >
              <label className="flex flex-col gap-1">
                <span className="text-body-sm font-medium text-book-text-sub">{t("auth.loginService")}</span>
                <ThemedSelect
                  value={oauthProvider}
                  onChange={handleOauthProviderChange}
                  disabled={oauthBusy}
                  className="pyqt-input min-w-0"
                >
                  {oauthProviders?.map((provider) => (
                    <option key={provider} value={provider}>
                      {OAUTH_PROVIDER_LABELS[provider]}
                    </option>
                  ))}
                </ThemedSelect>
              </label>
              <BookButton
                variant="primary"
                onClick={handleOauthLogin}
                disabled={oauthBusy}
                className="w-full"
              >
                {oauthBusy ? t("auth.awaitingAuthorization") : t("auth.continueWithProvider", {
                  provider: oauthProvider ? OAUTH_PROVIDER_LABELS[oauthProvider] : "",
                })}
              </BookButton>
              <button
                type="button"
                onClick={() => void handleCancelOauth()}
                className="a4n-auth-oauth-button"
              >
                {t("auth.backToUsernameLogin")}
              </button>
            </section>
          )}
        </BookCard>
      </div>
    </div>
  );
};
