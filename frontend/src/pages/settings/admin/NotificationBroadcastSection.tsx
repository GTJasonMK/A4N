import { useState } from "react";
import { Send } from "lucide-react";
import { broadcastCreatorNotification } from "../../../api/admin";
import { extractErrorMessage } from "../../../api";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { useToast } from "../../../components/feedback/Toast";
import { BookButton } from "../../../components/ui/BookButton";
import { useI18n } from "../../../i18n";

export function NotificationBroadcastSection() {
  const { t } = useI18n();
  const addToast = useToast((state) => state.addToast);
  const [title, setTitle] = useState("");
  const [body, setBody] = useState("");
  const [sending, setSending] = useState(false);

  const send = async () => {
    const normalizedTitle = title.trim();
    const normalizedBody = body.trim();
    if (!normalizedTitle || !normalizedBody || sending) return;
    const confirmed = await confirmDialog({
      title: t("settingsNotification.confirmTitle"),
      message: t("settingsNotification.confirmMessage"),
      confirmText: t("settingsNotification.send"),
      dialogType: "warning",
    });
    if (!confirmed) return;

    setSending(true);
    try {
      const receipt = await broadcastCreatorNotification({ title: normalizedTitle, body: normalizedBody });
      addToast(t("settingsNotification.sent", { count: receipt.recipient_count }), "success");
      setTitle("");
      setBody("");
    } catch (error) {
      addToast(extractErrorMessage(error, t("settingsNotification.sendFailed")), "error");
    } finally {
      setSending(false);
    }
  };

  return (
    <section className="max-w-3xl space-y-4">
      <h2 className="text-sm font-semibold text-book-text-main">
        {t("settingsNotification.section")}
      </h2>
      <label className="block space-y-1.5">
        <span className="text-xs font-medium text-book-text-sub">{t("settingsNotification.title")}</span>
        <input
          value={title}
          onChange={(event) => setTitle(event.target.value)}
          maxLength={120}
          autoComplete="off"
          className="a4n-input w-full"
          placeholder={t("settingsNotification.titlePlaceholder")}
        />
        <span className="block text-right text-caption text-book-text-muted">{title.length}/120</span>
      </label>
      <label className="block space-y-1.5">
        <span className="text-xs font-medium text-book-text-sub">{t("settingsNotification.body")}</span>
        <textarea
          value={body}
          onChange={(event) => setBody(event.target.value)}
          maxLength={5000}
          rows={7}
          className="a4n-input min-h-36 w-full resize-y py-2"
          placeholder={t("settingsNotification.bodyPlaceholder")}
        />
        <span className="block text-right text-caption text-book-text-muted">{body.length}/5000</span>
      </label>
      <div className="flex justify-end">
        <BookButton
          variant="primary"
          disabled={!title.trim() || !body.trim() || sending}
          aria-busy={sending}
          onClick={() => void send()}
        >
          <Send size={14} />
          {t(sending ? "settingsNotification.sending" : "settingsNotification.send")}
        </BookButton>
      </div>
    </section>
  );
}
