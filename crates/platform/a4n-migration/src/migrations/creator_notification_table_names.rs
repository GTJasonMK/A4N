use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let legacy = (
            manager.has_table("notification").await?,
            manager.has_table("recipient").await?,
        );
        let current = (
            manager.has_table("creator_notification").await?,
            manager.has_table("creator_notification_recipient").await?,
        );
        match (legacy, current) {
            ((false, false), (true, true)) => Ok(()),
            ((true, true), (false, false)) => {
                // PostgreSQL 重命名保留通知、已读状态、索引和外键关系。
                manager
                    .get_connection()
                    .execute_unprepared(
                        "ALTER TABLE notification RENAME TO creator_notification; \
                     ALTER TABLE recipient RENAME TO creator_notification_recipient",
                    )
                    .await?;
                Ok(())
            }
            _ => Err(DbErr::Custom(format!(
                "通知表状态不一致，拒绝自动合并或丢弃数据：legacy={legacy:?}, current={current:?}"
            ))),
        }
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom("通知表名修复不可单独回退".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_db::{auth::create_user_summary, notifications::*};
    use a4n_domain::notifications::BroadcastNotificationInput;
    use sea_orm::{DatabaseConnection, TransactionTrait};

    async fn users(db: &DatabaseConnection) -> (i64, i64, i64) {
        let mut ids = Vec::new();
        for (name, role) in [("sender", "admin"), ("alice", "user"), ("bob", "user")] {
            ids.push(
                create_user_summary(
                    db,
                    name,
                    &format!("{name}@example.test"),
                    "Notification-test-42",
                    role,
                )
                .await
                .unwrap()
                .id,
            );
        }
        (ids[0], ids[1], ids[2])
    }

    #[tokio::test]
    async fn fresh_notification_schema_supports_broadcast_and_per_user_reads() {
        let db = crate::test_support::connect_test_database().await;
        let (sender, alice, bob) = users(&db).await;
        assert!(creator_inbox(&db, alice).await.unwrap().items.is_empty());
        let receipt = broadcast_to_creators(
            &db,
            sender,
            BroadcastNotificationInput {
                title: "通知标题".into(),
                body: "通知正文".into(),
            },
        )
        .await
        .unwrap();
        assert_eq!(receipt.recipient_count, 2);
        let inbox = creator_inbox(&db, alice).await.unwrap();
        assert_eq!(inbox.unread_count, 1);
        assert_eq!(inbox.items[0].title, "通知标题");
        assert_eq!(inbox.items[0].body, "通知正文");
        assert!(!inbox.items[0].is_read);
        mark_creator_notification_read(&db, alice, receipt.notification_id)
            .await
            .unwrap();
        mark_creator_notification_read(&db, alice, receipt.notification_id)
            .await
            .unwrap();
        let read = creator_inbox(&db, alice).await.unwrap();
        assert_eq!(read.unread_count, 0);
        assert!(read.items[0].is_read);
        assert_eq!(creator_inbox(&db, bob).await.unwrap().unread_count, 1);
        assert!(creator_inbox(&db, sender).await.unwrap().items.is_empty());
        assert!(
            mark_creator_notification_read(&db, sender, receipt.notification_id)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn legacy_notification_tables_keep_data_and_reject_ambiguous_states() {
        let db = crate::test_support::connect_test_database().await;
        let (sender, alice, _) = users(&db).await;
        let receipt = broadcast_to_creators(
            &db,
            sender,
            BroadcastNotificationInput {
                title: "升级前通知".into(),
                body: "保留正文和已读状态".into(),
            },
        )
        .await
        .unwrap();
        mark_creator_notification_read(&db, alice, receipt.notification_id)
            .await
            .unwrap();
        let before = creator_inbox(&db, alice).await.unwrap();
        let tx = db.begin().await.unwrap();
        tx.execute_unprepared(
            "ALTER TABLE creator_notification RENAME TO notification; \
             ALTER TABLE creator_notification_recipient RENAME TO recipient",
        )
        .await
        .unwrap();
        let manager = SchemaManager::new(&tx);
        Migration.up(&manager).await.unwrap();
        Migration.up(&manager).await.unwrap();
        let after = creator_inbox(&tx, alice).await.unwrap();
        assert_eq!(
            serde_json::to_value(after).unwrap(),
            serde_json::to_value(before).unwrap()
        );
        tx.execute_unprepared("CREATE TABLE notification (id bigint PRIMARY KEY)")
            .await
            .unwrap();
        assert!(
            Migration
                .up(&manager)
                .await
                .unwrap_err()
                .to_string()
                .contains("拒绝自动合并")
        );
        assert_eq!(creator_inbox(&tx, alice).await.unwrap().items.len(), 1);
        tx.rollback().await.unwrap();
    }
}
