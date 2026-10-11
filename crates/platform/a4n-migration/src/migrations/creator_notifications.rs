use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Notification {
    Table,
    Id,
    Title,
    Body,
    CreatedByUserId,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Recipient {
    Table,
    Id,
    NotificationId,
    RecipientUserId,
    ReadAt,
}

#[derive(DeriveIden)]
enum User {
    Table,
    Id,
}

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Notification::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Notification::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Notification::Title).text().not_null())
                    .col(ColumnDef::new(Notification::Body).text().not_null())
                    .col(ColumnDef::new(Notification::CreatedByUserId).big_integer())
                    .col(
                        ColumnDef::new(Notification::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_creator_notification_sender")
                            .from(Notification::Table, Notification::CreatedByUserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Recipient::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Recipient::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Recipient::NotificationId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Recipient::RecipientUserId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Recipient::ReadAt).timestamp())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_creator_notification_recipient_notification")
                            .from(Recipient::Table, Recipient::NotificationId)
                            .to(Notification::Table, Notification::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_creator_notification_recipient_user")
                            .from(Recipient::Table, Recipient::RecipientUserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_creator_notification_recipient_unique")
                    .table(Recipient::Table)
                    .col(Recipient::NotificationId)
                    .col(Recipient::RecipientUserId)
                    .unique()
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_creator_notification_recipient_user")
                    .table(Recipient::Table)
                    .col(Recipient::RecipientUserId)
                    .col(Recipient::NotificationId)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Recipient::Table).if_exists().to_owned())
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(Notification::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
    }
}
