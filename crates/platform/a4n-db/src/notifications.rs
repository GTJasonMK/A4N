use a4n_db_models::models::notification::announcement;
use a4n_domain::auth::ROLE_USER;
use a4n_domain::notifications::{
    BroadcastNotificationInput, BroadcastNotificationReceipt, CreatorNotificationInbox,
    CreatorNotificationItem,
};
use a4n_error::AppError;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, DbBackend, FromQueryResult, Set, Statement};

#[derive(Debug, FromQueryResult)]
struct NotificationRow {
    id: i64,
    title: String,
    body: String,
    created_at: chrono::NaiveDateTime,
    read_at: Option<chrono::NaiveDateTime>,
}

#[derive(Debug, FromQueryResult)]
struct UnreadCountRow {
    unread_count: i64,
}

pub async fn broadcast_to_creators(
    db: &impl crate::DbExecutor,
    caller_user_id: i64,
    input: BroadcastNotificationInput,
) -> Result<BroadcastNotificationReceipt, AppError> {
    if caller_user_id <= 0 {
        return Err(AppError::InvalidArgument(
            "caller_user_id 必须大于 0".into(),
        ));
    }
    let input = input
        .normalize_and_validate()
        .map_err(AppError::InvalidArgument)?;
    let now = Utc::now().naive_utc();
    let transaction = db.begin().await?;
    let notification = announcement::ActiveModel {
        title: Set(input.title),
        body: Set(input.body),
        created_by_user_id: Set(Some(caller_user_id)),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(&transaction)
    .await?;

    let recipients = transaction
        .execute(Statement::from_sql_and_values(
            DbBackend::Postgres,
            r#"INSERT INTO "creator_notification_recipient"
               ("notification_id", "recipient_user_id")
               SELECT $1, id FROM "user"
               WHERE role = $2 AND is_active = TRUE"#,
            [notification.id.into(), ROLE_USER.into()],
        ))
        .await?
        .rows_affected();
    if recipients == 0 {
        transaction.rollback().await?;
        return Err(AppError::InvalidArgument(
            "当前没有启用的创作者可以接收通知".into(),
        ));
    }
    transaction.commit().await?;

    Ok(BroadcastNotificationReceipt {
        notification_id: notification.id,
        recipient_count: recipients,
        created_at: notification.created_at.and_utc().to_rfc3339(),
    })
}

pub async fn creator_inbox(
    db: &impl crate::DbExecutor,
    user_id: i64,
) -> Result<CreatorNotificationInbox, AppError> {
    validate_user_id(user_id)?;
    let items = NotificationRow::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        r#"SELECT n.id, n.title, n.body, n.created_at, r.read_at
           FROM "creator_notification" n
           INNER JOIN "creator_notification_recipient" r
             ON r.notification_id = n.id
           WHERE r.recipient_user_id = $1
           ORDER BY n.created_at DESC, n.id DESC"#,
        [user_id.into()],
    ))
    .all(db)
    .await?;
    let unread_count = UnreadCountRow::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        r#"SELECT COUNT(*) AS unread_count
           FROM "creator_notification_recipient"
           WHERE recipient_user_id = $1 AND read_at IS NULL"#,
        [user_id.into()],
    ))
    .one(db)
    .await?
    .ok_or_else(|| AppError::Internal(anyhow::anyhow!("读取通知未读数时 COUNT 查询未返回结果")))?
    .unread_count
    .max(0) as u64;

    Ok(CreatorNotificationInbox {
        items: items
            .into_iter()
            .map(|row| CreatorNotificationItem {
                id: row.id,
                title: row.title,
                body: row.body,
                created_at: row.created_at.and_utc().to_rfc3339(),
                is_read: row.read_at.is_some(),
            })
            .collect(),
        unread_count,
    })
}

pub async fn mark_creator_notification_read(
    db: &impl crate::DbExecutor,
    user_id: i64,
    notification_id: i64,
) -> Result<(), AppError> {
    validate_user_id(user_id)?;
    if notification_id <= 0 {
        return Err(AppError::InvalidArgument(
            "notification_id 必须大于 0".into(),
        ));
    }
    let result = db
        .execute(Statement::from_sql_and_values(
            DbBackend::Postgres,
            r#"UPDATE "creator_notification_recipient"
               SET read_at = COALESCE(read_at, $1)
               WHERE notification_id = $2 AND recipient_user_id = $3"#,
            [
                Utc::now().naive_utc().into(),
                notification_id.into(),
                user_id.into(),
            ],
        ))
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("通知不存在或无权访问".into()));
    }
    Ok(())
}

fn validate_user_id(user_id: i64) -> Result<(), AppError> {
    if user_id <= 0 {
        return Err(AppError::InvalidArgument("user_id 必须大于 0".into()));
    }
    Ok(())
}
