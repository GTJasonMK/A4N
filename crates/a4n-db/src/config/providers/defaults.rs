use a4n_error::AppError;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, Statement, TransactionTrait};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderConfigKind {
    Llm,
    Embedding,
    Image,
    Video,
}

impl ProviderConfigKind {
    fn table_name(self) -> &'static str {
        match self {
            Self::Llm => "llm_config",
            Self::Embedding => "embedding_config",
            Self::Image => "image_gen_config",
            Self::Video => "video_gen_config",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Llm => "LLM 配置",
            Self::Embedding => "Embedding 配置",
            Self::Image => "图片配置",
            Self::Video => "视频配置",
        }
    }
}

pub(crate) async fn clear_provider_config_defaults<C>(
    db: &C,
    kind: ProviderConfigKind,
    except_id: Option<i64>,
) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    let table = kind.table_name();
    let statement = match except_id {
        Some(config_id) => Statement::from_sql_and_values(
            db.get_database_backend(),
            format!(
                "UPDATE {table} SET is_default = FALSE, updated_at = CURRENT_TIMESTAMP \
                 WHERE is_default AND id <> $1"
            ),
            [config_id.into()],
        ),
        None => Statement::from_string(
            db.get_database_backend(),
            format!(
                "UPDATE {table} SET is_default = FALSE, updated_at = CURRENT_TIMESTAMP \
                 WHERE is_default"
            ),
        ),
    };
    db.execute(statement).await?;
    Ok(())
}

pub async fn set_default_provider_config(
    db: &DatabaseConnection,
    kind: ProviderConfigKind,
    config_id: i64,
) -> Result<(), AppError> {
    let txn = db.begin().await.map_err(AppError::Database)?;
    clear_provider_config_defaults(&txn, kind, Some(config_id))
        .await
        .map_err(AppError::Database)?;

    let table = kind.table_name();
    let updated = txn
        .execute(Statement::from_sql_and_values(
            txn.get_database_backend(),
            format!(
                "UPDATE {table} SET is_default = TRUE, updated_at = CURRENT_TIMESTAMP WHERE id = $1"
            ),
            [config_id.into()],
        ))
        .await
        .map_err(AppError::Database)?;
    if updated.rows_affected() != 1 {
        return Err(AppError::NotFound(format!(
            "{}不存在: {config_id}",
            kind.label()
        )));
    }

    txn.commit().await.map_err(AppError::Database)
}
