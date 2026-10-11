import type { BroadcastNotificationInput, BroadcastNotificationReceipt } from "../generated/host";
import { apiInvoke } from "../index";

export async function broadcastCreatorNotification(
  input: BroadcastNotificationInput,
): Promise<BroadcastNotificationReceipt> {
  return apiInvoke<BroadcastNotificationReceipt>("broadcast_creator_notification", { input });
}
