import React, { useState } from "react";
import {
  Check,
  CircleAlert,
  Clipboard,
  Link2,
  Pause,
  Play,
  RefreshCw,
  ShieldCheck,
  Square,
  UserRound,
  WifiOff,
  X,
} from "lucide-react";
import { BookButton } from "../ui/BookButton";
import { LoadingSpinner } from "../ui/LoadingSpinner";
import { useToast } from "../feedback/Toast";
import { formatNumber, useI18n } from "../../i18n";
import { copyTextToClipboard } from "../../utils/clipboard";
import type { PageSessionState } from "../../hooks/usePageSession";

interface PageSessionControlBarProps {
  state: PageSessionState;
}

const statusKey = {
  awaiting_attach: "awaitingAttach",
  attached: "attached",
  running: "running",
  paused: "paused",
  offline: "offline",
  idle: "idle",
} as const;

export const PageSessionControlBar: React.FC<PageSessionControlBarProps> = ({ state }) => {
  const { locale, t } = useI18n();
  const { addToast } = useToast();
  const [copied, setCopied] = useState(false);
  const { session, authorizationCode, loading, busy, error, locked } = state;

  if (!loading && !session && !error) return null;

  const status = session?.status ?? "awaiting_attach";
  const operation = session?.last_operation;
  const activityVisible = loading || busy;
  const statusText = t(`pageSession.${statusKey[status]}`);
  const copyCode = async () => {
    if (!authorizationCode) return;
    try {
      await copyTextToClipboard(authorizationCode);
      setCopied(true);
      addToast(t("pageSession.codeCopied"), "success");
      window.setTimeout(() => setCopied(false), 1400);
    } catch (err) {
      addToast(err instanceof Error ? err.message : t("pageSession.copyCodeFailed"), "error");
    }
  };
  const run = (action: Parameters<PageSessionState["control"]>[0]) => {
    void state.control(action).catch(() => undefined);
  };

  return (
    <section
      aria-label={t("pageSession.ariaLabel")}
      aria-live="polite"
      data-page-session-busy={activityVisible ? "true" : undefined}
      className="shrink-0 border-t border-book-border/65 bg-book-bg-paper/95 px-3 py-2 shadow-[0_-4px_18px_rgba(27,39,37,0.06)]"
      data-page-session-status={status}
    >
      <div className="mx-auto flex min-w-0 max-w-[1400px] flex-wrap items-center gap-x-3 gap-y-1.5">
        <div className="flex min-w-0 items-center gap-2">
          <span
            className={`inline-flex h-7 w-7 shrink-0 items-center justify-center rounded-[10px] border ${
              locked
                ? "border-book-primary/35 bg-book-primary/10 text-book-primary"
                : status === "offline" || error
                  ? "border-error/35 bg-error-bg text-error"
                  : "border-book-border/60 bg-book-bg-tertiary/55 text-book-text-muted"
            }`}
            aria-hidden="true"
          >
            {status === "offline" ? <WifiOff size={13} /> : locked ? <ShieldCheck size={13} /> : <Link2 size={13} />}
          </span>
          <div className="min-w-0">
            <div className="flex min-w-0 items-center gap-2 text-xs font-semibold text-book-text-main">
              <span className="truncate">{t("pageSession.title")}</span>
              <span className="a4n-page-session__status shrink-0 rounded-full bg-book-bg-tertiary px-1.5 py-0.5 text-[10px] font-medium text-book-text-muted">
                {statusText}
              </span>
              {activityVisible ? (
                <span
                  className="a4n-page-session__activity"
                  role="status"
                  aria-label={t("common.loading")}
                >
                  <LoadingSpinner size="sm" />
                </span>
              ) : null}
            </div>
            <div className="max-w-[min(68vw,38rem)] truncate text-[11px] text-book-text-muted">
              {loading && !session
                ? t("pageSession.connecting")
                : error
                  ? error
                  : operation && status === "running"
                    ? `${operation.command} · ${operation.phase}${operation.progress != null ? ` · ${formatNumber(Math.round(operation.progress), locale)}%` : ""}`
                    : session?.route ?? t("pageSession.currentPage")}
            </div>
          </div>
        </div>

        {authorizationCode && status === "awaiting_attach" ? (
          <div className="flex min-w-0 flex-1 flex-wrap items-center gap-2 rounded-[10px] border border-book-primary/25 bg-book-primary/5 px-2 py-1">
            <span className="shrink-0 text-[11px] text-book-text-muted">{t("pageSession.authorizationCode")}</span>
            <code className="min-w-0 break-all font-mono text-xs font-semibold tracking-wide text-book-primary">
              {authorizationCode}
            </code>
            <BookButton
              variant="ghost"
              size="sm"
              className="ml-auto shrink-0 !min-h-7 !px-2"
              onClick={() => void copyCode()}
              aria-label={t("pageSession.copyCode")}
            >
              {copied ? <Check size={12} /> : <Clipboard size={12} />}
              <span className="hidden sm:inline">{copied ? t("pageSession.copied") : t("pageSession.copyCode")}</span>
            </BookButton>
            <BookButton
              variant="ghost"
              size="sm"
              className="shrink-0 !min-h-7 !px-2"
              onClick={() => void state.authorize().catch(() => undefined)}
              disabled={busy}
              aria-label={t("pageSession.refreshCode")}
            >
              <RefreshCw size={12} className={busy ? "animate-spin" : undefined} />
              <span className="hidden sm:inline">{t("pageSession.refreshCode")}</span>
            </BookButton>
          </div>
        ) : null}

        <div className="ml-auto flex flex-wrap items-center justify-end gap-1.5">
          {status === "running" ? (
            <>
              <BookButton variant="secondary" size="sm" onClick={() => run("pause")} disabled={busy}>
                <Pause size={12} /> {t("pageSession.pause")}
              </BookButton>
              <BookButton variant="ghost" size="sm" onClick={() => run("cancel")} disabled={busy}>
                <Square size={12} /> {t("pageSession.cancel")}
              </BookButton>
            </>
          ) : null}
          {status === "paused" ? (
            <BookButton variant="secondary" size="sm" onClick={() => run("resume")} disabled={busy}>
              <Play size={12} /> {t("pageSession.resume")}
            </BookButton>
          ) : null}
          {locked ? (
            <BookButton variant="warning" size="sm" onClick={() => run("takeover")} disabled={busy}>
              <UserRound size={12} /> {t("pageSession.takeover")}
            </BookButton>
          ) : null}
          {status === "idle" || (status === "awaiting_attach" && !authorizationCode) ? (
            <BookButton variant="secondary" size="sm" onClick={() => void state.authorize().catch(() => undefined)} disabled={busy}>
              <Link2 size={12} /> {t("pageSession.authorize")}
            </BookButton>
          ) : null}
          {status === "offline" ? (
            <BookButton variant="secondary" size="sm" onClick={() => void state.restart()} disabled={busy || loading}>
              <RefreshCw size={12} /> {t("pageSession.reconnect")}
            </BookButton>
          ) : null}
          {error && !session ? (
            <BookButton variant="secondary" size="sm" onClick={() => void state.restart()} disabled={busy || loading}>
              <RefreshCw size={12} /> {t("common.retry")}
            </BookButton>
          ) : null}
          {session && status !== "offline" ? (
            <BookButton variant="ghost" size="sm" onClick={() => run("detach")} disabled={busy}>
              <X size={12} /> {t("pageSession.detach")}
            </BookButton>
          ) : null}
        </div>
      </div>
      {error && session ? (
        <div className="mx-auto mt-1 flex max-w-[1400px] items-center gap-1.5 text-[11px] text-error">
          <CircleAlert size={12} aria-hidden="true" />
          <span className="min-w-0 break-words">{error}</span>
        </div>
      ) : null}
    </section>
  );
};
