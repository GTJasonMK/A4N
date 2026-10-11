import { apiInvoke } from "./index";
import type { CreatorNotificationInbox } from "./generated/host";
export type { CreatorNotificationInbox } from "./generated/host";

export const NOTIFICATIONS_UPDATED_EVENT = "a4n-notifications-updated";

export async function listCreatorNotifications(): Promise<CreatorNotificationInbox> {
  return apiInvoke<CreatorNotificationInbox>("list_creator_notifications");
}

export async function markCreatorNotificationRead(notificationId: number): Promise<void> {
  return apiInvoke<void>("mark_creator_notification_read", { notificationId });
}
