use super::{command_input, empty_command_args, i64_arg, reject_unknown_command_args, to_value};
use crate::routes::error::ApiError;
use crate::state::ServerState;
use a4n_runtime::host_api::notifications as notification_actions;
use serde_json::Value;

pub(super) async fn try_dispatch(
    state: &ServerState,
    user_id: i64,
    args: &Value,
    cmd: &str,
) -> Result<Option<Value>, ApiError> {
    let result = match cmd {
        "broadcast_creator_notification" => {
            reject_unknown_command_args(args, cmd, &["input"])?;
            let input: notification_actions::BroadcastNotificationInput = command_input(args, cmd)?;
            to_value(
                notification_actions::broadcast_creator_notification(
                    state.runtime(),
                    user_id,
                    input,
                )
                .await?,
            )?
        }
        "list_creator_notifications" => {
            empty_command_args(args, cmd)?;
            to_value(
                notification_actions::list_creator_notifications(state.runtime(), user_id).await?,
            )?
        }
        "mark_creator_notification_read" => {
            reject_unknown_command_args(args, cmd, &["notificationId"])?;
            let notification_id = i64_arg(args, "notificationId")?;
            notification_actions::mark_creator_notification_read(
                state.runtime(),
                user_id,
                notification_id,
            )
            .await?;
            Value::Null
        }
        _ => return Ok(None),
    };
    Ok(Some(result))
}
