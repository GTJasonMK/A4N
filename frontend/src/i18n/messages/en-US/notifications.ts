import type { Messages } from "../zh-CN";

export const notifications = {
  title: "Notifications",
  inbox: "Notification inbox",
  unreadCount: "{count} unread",
  loadFailed: "Failed to load notifications",
  markReadFailed: "Failed to mark as read",
  empty: "No notifications",
  unread: "Unread",
  read: "Read",
  markRead: "Mark as read",
  accessDenied: "The notification inbox is available to creators only",
} satisfies Messages["notifications"];

export const settingsNotification = {
  section: "Broadcast notifications",
  title: "Title",
  titlePlaceholder: "Enter a notification title",
  body: "Message",
  bodyPlaceholder: "Enter a message for all creators",
  send: "Send notification",
  sending: "Sending",
  confirmTitle: "Send a site-wide notification?",
  confirmMessage: "This will send the message to all active creators and cannot be recalled.",
  sent: "Sent to {count} creators",
  sendFailed: "Failed to send notification",
} satisfies Messages["settingsNotification"];
