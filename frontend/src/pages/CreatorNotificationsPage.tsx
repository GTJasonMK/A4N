import { useCallback, useEffect, useState } from "react";
import { Check, Inbox } from "lucide-react";
import { listCreatorNotifications, markCreatorNotificationRead, NOTIFICATIONS_UPDATED_EVENT, type CreatorNotificationInbox } from "../api/notifications";
import { extractErrorMessage } from "../api";
import { LoadingSpinner } from "../components/ui/LoadingSpinner";
import { useToast } from "../components/feedback/Toast";
import { formatDateTime, useI18n } from "../i18n";
import { useAuthStore } from "../store/auth";
import { BackButton, useSmartBack } from "../components/ui/BackButton";
import { PageHeader } from "../components/shell/PageHeader";

export function CreatorNotificationsPage() {
  const { t, locale } = useI18n();
  const smartBack = useSmartBack();
  const addToast = useToast((state) => state.addToast);
  const { user } = useAuthStore();
  const [inbox, setInbox] = useState<CreatorNotificationInbox | null>(null);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [busyId, setBusyId] = useState<number | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setLoadError(null);
    try {
      if (user?.role !== "user") return;
      setInbox(await listCreatorNotifications());
    } catch (error) {
      setLoadError(extractErrorMessage(error, t("notifications.loadFailed")));
    } finally {
      setLoading(false);
    }
  }, [t, user?.id, user?.role]);

  useEffect(() => {
    void load();
  }, [load]);

  const markRead = async (notificationId: number) => {
    if (busyId != null) return;
    setBusyId(notificationId);
    try {
      await markCreatorNotificationRead(notificationId);
      setInbox((current) => current == null ? current : {
        unread_count: Math.max(0, current.unread_count - 1),
        items: current.items.map((item) =>
          item.id === notificationId ? { ...item, is_read: true } : item,
        ),
      });
      window.dispatchEvent(new Event(NOTIFICATIONS_UPDATED_EVENT));
    } catch (error) {
      addToast(extractErrorMessage(error, t("notifications.markReadFailed")), "error");
    } finally {
      setBusyId(null);
    }
  };

  if (user?.role !== "user") {
    return (
      <div className="a4n-page-container flex h-full items-center justify-center px-4 text-sm text-book-text-muted">
        {t("notifications.accessDenied")}
      </div>
    );
  }

  return (
    <div className="a4n-page-container h-full min-h-0 overflow-y-auto">
      <div className="mx-auto w-full max-w-3xl px-4 md:px-8">
        <PageHeader
          leading={<BackButton iconOnly title={t("nav.backToPrevious")} onClick={smartBack} />}
          eyebrow={t("notifications.title")}
          title={t("notifications.inbox")}
          showNotificationButton={false}
          meta={inbox && inbox.unread_count > 0 ? (
            <span className="shrink-0 text-xs font-medium text-book-primary">
              {t("notifications.unreadCount", { count: inbox.unread_count })}
            </span>
          ) : null}
          className="mb-4 !px-0 !py-4"
        />
        {loading ? (
          <div className="flex min-h-40 items-center justify-center">
            <LoadingSpinner size="md" text={t("common.loading")} />
          </div>
        ) : loadError ? (
          <div className="flex min-h-48 flex-col items-center justify-center gap-3 border-y border-book-border/60 text-center">
            <p className="text-sm text-error">{loadError}</p>
            <button type="button" className="a4n-btn-ghost min-h-9 px-3 text-xs" onClick={() => void load()}>
              {t("common.retry")}
            </button>
          </div>
        ) : inbox?.items.length ? (
          <div className="divide-y divide-book-border/60 border-y border-book-border/60">
            {inbox.items.map((item) => (
              <article
                key={item.id}
                className={`relative px-1 py-5 ${item.is_read ? "" : "bg-book-primary/5"}`}
              >
                <div className="flex items-start gap-3">
                  <Inbox size={16} className="mt-1 shrink-0 text-book-primary" aria-hidden="true" />
                  <div className="min-w-0 flex-1">
                    <div className="flex flex-wrap items-baseline gap-x-3 gap-y-1">
                      <h2 className="text-sm font-semibold text-book-text-main">{item.title}</h2>
                      <time className="text-caption text-book-text-muted">
                        {formatDateTime(item.created_at, locale, item.created_at)}
                      </time>
                      <span className={`text-caption ${item.is_read ? "text-book-text-muted" : "font-medium text-book-primary"}`}>
                        {t(item.is_read ? "notifications.read" : "notifications.unread")}
                      </span>
                    </div>
                    <p className="mt-2 whitespace-pre-wrap break-words text-sm leading-6 text-book-text-sub">
                      {item.body}
                    </p>
                  </div>
                  {!item.is_read ? (
                    <button
                      type="button"
                      aria-label={t("notifications.markRead")}
                      title={t("notifications.markRead")}
                      disabled={busyId === item.id}
                      onClick={() => void markRead(item.id)}
                      className="a4n-status-bar-action shrink-0"
                    >
                      <Check size={14} />
                    </button>
                  ) : null}
                </div>
              </article>
            ))}
          </div>
        ) : (
          <div className="flex min-h-48 flex-col items-center justify-center border-y border-book-border/60 text-center">
            <Inbox size={24} className="mb-3 text-book-text-muted" aria-hidden="true" />
            <p className="text-sm text-book-text-muted">{t("notifications.empty")}</p>
          </div>
        )}
      </div>
    </div>
  );
}
