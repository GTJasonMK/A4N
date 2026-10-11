use crate::AppState;
pub use a4n_api_contract::notifications::{
    BroadcastNotificationInput, BroadcastNotificationReceipt, CreatorNotificationInbox,
};

pub async fn broadcast_creator_notification(
    state: &AppState,
    caller_user_id: i64,
    input: BroadcastNotificationInput,
) -> Result<BroadcastNotificationReceipt, a4n_error::AppError> {
    state
        .notification_application()
        .broadcast_to_creators(caller_user_id, input)
        .await
}

pub async fn list_creator_notifications(
    state: &AppState,
    caller_user_id: i64,
) -> Result<CreatorNotificationInbox, a4n_error::AppError> {
    state
        .notification_application()
        .creator_inbox(caller_user_id)
        .await
}

pub async fn mark_creator_notification_read(
    state: &AppState,
    caller_user_id: i64,
    notification_id: i64,
) -> Result<(), a4n_error::AppError> {
    state
        .notification_application()
        .mark_creator_notification_read(caller_user_id, notification_id)
        .await
}
