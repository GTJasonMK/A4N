import { useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { extractErrorMessage } from "../api";
import { BookCard } from "../components/ui/BookCard";
import { BookButton } from "../components/ui/BookButton";
import { useI18n } from "../i18n";
import { useAuthStore } from "../store/auth";

export function OAuthCallbackPage() {
  const [callbackUrl] = useState(() => window.location.href);
  const [error, setError] = useState<string | null>(null);
  const oauthLogin = useAuthStore((state) => state.oauthLogin);
  const navigate = useNavigate();
  const { t } = useI18n();
  const startedRef = useRef(false);

  useEffect(() => {
    if (startedRef.current) return;
    startedRef.current = true;
    let mounted = true;
    window.history.replaceState(window.history.state, "", new URL(callbackUrl).pathname);
    void oauthLogin(callbackUrl).then(({ returnTo }) => {
      if (mounted) navigate(returnTo, { replace: true });
    }).catch((failure: unknown) => {
      if (mounted) setError(extractErrorMessage(failure, t("auth.oauthLoginFailed")));
    });
    return () => { mounted = false; };
  }, [callbackUrl, navigate, oauthLogin, t]);

  return (
    <div className="a4n-auth-page">
      <BookCard>
        <h1 className="text-lg font-semibold">{t("auth.oauthLogin")}</h1>
        {error ? (
          <>
            <p role="alert" className="my-4 text-error">{error}</p>
            <BookButton onClick={() => navigate("/", { replace: true })}>{t("auth.backToUsernameLogin")}</BookButton>
          </>
        ) : (
          <p role="status" className="my-4">{t("auth.completingLogin")}</p>
        )}
      </BookCard>
    </div>
  );
}
