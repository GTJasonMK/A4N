use a4n_core::models::project::novel_project;
use chrono::Utc;
use sea_orm::{DatabaseConnection, TransactionTrait};

use super::{
    branch::branch_text_project_data_record, duplicate::duplicate_text_project_data_record,
};
use crate::project::duplicate::{
    ProjectDuplicateError, text::rules::ensure_text_project_record_for_copy,
};

pub async fn duplicate_text_project_record(
    db: &DatabaseConnection,
    original: &novel_project::Model,
    user_id: i64,
    new_title: &str,
) -> Result<novel_project::Model, ProjectDuplicateError> {
    ensure_text_project_record_for_copy(original, "复制文本项目")?;
    let txn = db.begin().await?;
    let now = Utc::now().naive_utc();
    let project = duplicate_text_project_data_record(&txn, original, user_id, new_title, now).await;
    match project {
        Ok(project) => {
            txn.commit().await?;
            Ok(project)
        }
        Err(err) => Err(err),
    }
}

pub async fn branch_text_project_record(
    db: &DatabaseConnection,
    original: &novel_project::Model,
    branch_title: &str,
    branch_status: &str,
) -> Result<novel_project::Model, ProjectDuplicateError> {
    ensure_text_project_record_for_copy(original, "创建项目分支")?;
    let txn = db.begin().await?;
    let now = Utc::now().naive_utc();
    let project =
        branch_text_project_data_record(&txn, original, branch_title, branch_status, now).await;
    match project {
        Ok(project) => {
            txn.commit().await?;
            Ok(project)
        }
        Err(err) => Err(err),
    }
}
