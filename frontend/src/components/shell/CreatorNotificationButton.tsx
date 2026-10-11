import { useEffect, useState } from "react";
import { Bell } from "lucide-react";
import { useNavigate } from "react-router-dom";
import { listCreatorNotifications, NOTIFICATIONS_UPDATED_EVENT } from "../../api/notifications";
import { useI18n } from "../../i18n";
import { useAuthStore } from "../../store/auth";
import { BookIconButton } from "../ui/BookButton";

export function CreatorNotificationButton() {
  const user = useAuthStore((state) => state.user);
  const navigate = useNavigate();
  const { t } = useI18n();
  const [unreadCount, setUnreadCount] = useState(0);

  useEffect(() => {
    if (user?.role !== "user") {
      setUnreadCount(0);
      return undefined;
    }
    let active = true;
    const refresh = () => {
      void listCreatorNotifications()
        .then((inbox) => {
          if (active) setUnreadCount(inbox.unread_count);
        })
        .catch(() => undefined);
    };
    refresh();
    const timer = window.setInterval(refresh, 60_000);
    window.addEventListener(NOTIFICATIONS_UPDATED_EVENT, refresh);
    return () => {
      active = false;
      window.clearInterval(timer);
      window.removeEventListener(NOTIFICATIONS_UPDATED_EVENT, refresh);
    };
  }, [user?.id, user?.role]);

  if (user?.role !== "user") return null;

  const label = unreadCount > 0
    ? `${t("nav.viewNotifications")} · ${t("notifications.unreadCount", { count: unreadCount })}`
    : t("nav.viewNotifications");
  const badge = unreadCount > 0 ? (
    <span
      aria-hidden="true"
      className="absolute -right-1 -top-1 flex min-w-4 items-center justify-center rounded-full bg-book-primary px-1 text-[9px] leading-3 text-white"
    >
      {unreadCount > 99 ? "99+" : unreadCount}
    </span>
  ) : null;

  return (
    <BookIconButton
      label={label}
      size="sm"
      className="relative shrink-0"
      onClick={() => navigate("/notifications")}
    >
      <Bell size={14} />
      {badge}
    </BookIconButton>
  );
}
