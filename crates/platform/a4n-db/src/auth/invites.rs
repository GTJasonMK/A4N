use super::*;

use super::rules::{normalize_invite_code, validate_positive_user_id};
use super::users::create_user_on;
use a4n_domain::admin::InviteCodeSummary;
use a4n_domain::auth::ROLE_USER;
use sea_orm::QuerySelect;

pub async fn register_user_with_invite(
    db: &impl crate::DbExecutor,
    username: &str,
    email: &str,
    password: &str,
    raw_invite_code: &str,
) -> Result<user::Model, AppError> {
    let code = normalize_invite_code(raw_invite_code)?;
    let txn = db.begin().await.map_err(AppError::from)?;

    let result = async {
        let invite = invite_code::Entity::find()
            .filter(invite_code::Column::Code.eq(code))
            .one(&txn)
            .await
            .map_err(AppError::from)?
            .ok_or_else(|| AppError::InvalidArgument("邀请码无效".into()))?;
        if invite.used_at.is_some() || invite.used_by_user_id.is_some() {
            return Err(AppError::InvalidArgument("邀请码已被使用".into()));
        }

        let user = create_user_on(&txn, username, email, password, ROLE_USER).await?;
        let now = Utc::now().naive_utc();
        let updated = invite_code::Entity::update_many()
            .col_expr(
                invite_code::Column::UsedByUserId,
                Expr::value(Some(user.id)),
            )
            .col_expr(invite_code::Column::UsedAt, Expr::value(Some(now)))
            .filter(invite_code::Column::Id.eq(invite.id))
            .filter(invite_code::Column::UsedAt.is_null())
            .filter(invite_code::Column::UsedByUserId.is_null())
            .exec(&txn)
            .await
            .map_err(AppError::from)?;
        if updated.rows_affected != 1 {
            return Err(AppError::InvalidArgument("邀请码已被使用".into()));
        }
        Ok(user)
    }
    .await;

    match result {
        Ok(user) => {
            txn.commit().await.map_err(AppError::from)?;
            Ok(user)
        }
        Err(err) => {
            txn.rollback().await.map_err(AppError::from)?;
            Err(err)
        }
    }
}

pub async fn create_invite_code(
    db: &impl crate::DbExecutor,
    code: &str,
    created_by_user_id: Option<i64>,
) -> Result<InviteCodeSummary, AppError> {
    let code = normalize_invite_code(code)?;
    if let Some(user_id) = created_by_user_id {
        validate_positive_user_id(user_id)?;
    }
    if invite_code::Entity::find()
        .select_only()
        .column(invite_code::Column::Id)
        .filter(invite_code::Column::Code.eq(code.clone()))
        .into_tuple::<i64>()
        .one(db)
        .await
        .map_err(AppError::from)?
        .is_some()
    {
        return Err(AppError::InvalidArgument("邀请码已存在".into()));
    }
    let now = Utc::now().naive_utc();
    invite_code::ActiveModel {
        code: Set(code),
        created_by_user_id: Set(created_by_user_id),
        used_by_user_id: Set(None),
        created_at: Set(now),
        used_at: Set(None),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AppError::from)
    .map(invite_code_summary)
}

pub async fn list_invite_codes(
    db: &impl crate::DbExecutor,
) -> Result<Vec<InviteCodeSummary>, AppError> {
    invite_code::Entity::find()
        .order_by_desc(invite_code::Column::CreatedAt)
        .order_by_desc(invite_code::Column::Id)
        .all(db)
        .await
        .map_err(AppError::from)
        .map(|codes| codes.into_iter().map(invite_code_summary).collect())
}

fn invite_code_summary(code: invite_code::Model) -> InviteCodeSummary {
    InviteCodeSummary {
        id: code.id,
        code: code.code,
        created_by_user_id: code.created_by_user_id,
        used_by_user_id: code.used_by_user_id,
        created_at: Some(code.created_at.and_utc().to_rfc3339()),
        used_at: code.used_at.map(|value| value.and_utc().to_rfc3339()),
    }
}

#[cfg(test)]
mod tests {
    use super::invite_code_summary;
    use a4n_db_models::models::user::invite_code;
    use chrono::NaiveDate;

    #[test]
    fn invite_code_summary_formats_timestamps_as_utc_instants() {
        let timestamp = NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let summary = invite_code_summary(invite_code::Model {
            id: 3,
            code: "invite".into(),
            created_by_user_id: Some(7),
            used_by_user_id: None,
            created_at: timestamp,
            used_at: Some(timestamp),
        });

        assert!(summary.created_at.unwrap().ends_with("+00:00"));
        assert!(summary.used_at.unwrap().ends_with("+00:00"));
    }
}
