pub use a4n_domain::notifications::{
    BroadcastNotificationInput, BroadcastNotificationReceipt, CreatorNotificationInbox,
    CreatorNotificationItem,
};

#[async_trait::async_trait]
pub trait NotificationApplicationPort: Send + Sync {
    async fn broadcast_to_creators(
        &self,
        caller_user_id: i64,
        input: BroadcastNotificationInput,
    ) -> Result<BroadcastNotificationReceipt, a4n_error::AppError>;

    async fn creator_inbox(
        &self,
        caller_user_id: i64,
    ) -> Result<CreatorNotificationInbox, a4n_error::AppError>;

    async fn mark_creator_notification_read(
        &self,
        caller_user_id: i64,
        notification_id: i64,
    ) -> Result<(), a4n_error::AppError>;
}
