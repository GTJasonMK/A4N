use a4n_core::error::AppError;
use a4n_db::import_reverse::ImportReverseDbError;
use apply::{
    reverse_blueprint_apply_draft, reverse_blueprint_apply_result, reverse_outline_apply_result,
    reverse_outline_rows, reverse_part_outline_apply_result, reverse_part_outline_rows,
};
use serde_json::Value;

use crate::OrchestratorContext;

mod apply;
mod reverse_json;

fn import_reverse_app_error(error: ImportReverseDbError) -> AppError {
    match error {
        ImportReverseDbError::Database(error) => AppError::Database(error),
        ImportReverseDbError::NotFound(message) => AppError::NotFound(message),
        ImportReverseDbError::InvalidArgument(message) => AppError::InvalidArgument(message),
    }
}

pub(in crate::import_reverse) fn validate_reverse_engineer_output(
    prompt_name: &str,
    data: &Value,
) -> Result<(), AppError> {
    match prompt_name {
        "reverse_blueprint" => reverse_blueprint_apply_draft(data)
            .map(|_| ())
            .map_err(AppError::InvalidArgument),
        "reverse_outline" => reverse_outline_rows(data)
            .map(|_| ())
            .map_err(AppError::InvalidArgument),
        "reverse_part_outline" => reverse_part_outline_rows(data)
            .map(|_| ())
            .map_err(AppError::InvalidArgument),
        other => Err(AppError::InvalidArgument(format!(
            "未注册的导入反推结构化输出合同: {other}"
        ))),
    }
}

impl OrchestratorContext {
    pub async fn apply_reverse_blueprint_data(
        &self,
        project_id: i64,
        data: &Value,
    ) -> Result<Value, AppError> {
        let draft = reverse_blueprint_apply_draft(data).map_err(AppError::InvalidArgument)?;
        let result = a4n_db::import_reverse::apply_reverse_blueprint_records_transaction(
            self.db.connection(),
            project_id,
            draft,
        )
        .await
        .map_err(import_reverse_app_error)?;
        Ok(reverse_blueprint_apply_result(result))
    }

    pub async fn apply_reverse_outline_data(
        &self,
        project_id: i64,
        data: &Value,
    ) -> Result<Value, AppError> {
        let rows = reverse_outline_rows(data).map_err(AppError::InvalidArgument)?;
        let count = a4n_db::import_reverse::apply_reverse_outline_records_transaction(
            self.db.connection(),
            project_id,
            &rows,
        )
        .await
        .map_err(AppError::Database)?;
        Ok(reverse_outline_apply_result(count))
    }

    pub async fn apply_reverse_part_outline_data(
        &self,
        project_id: i64,
        data: &Value,
    ) -> Result<Value, AppError> {
        let rows = reverse_part_outline_rows(data).map_err(AppError::InvalidArgument)?;
        let count = a4n_db::import_reverse::apply_reverse_part_outline_records_transaction(
            self.db.connection(),
            project_id,
            &rows,
        )
        .await
        .map_err(AppError::Database)?;
        Ok(reverse_part_outline_apply_result(count))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_reverse_not_found_remains_not_found() {
        let error = import_reverse_app_error(ImportReverseDbError::NotFound(
            "项目不存在: project_id=7".to_string(),
        ));

        assert!(
            matches!(error, AppError::NotFound(message) if message == "项目不存在: project_id=7")
        );
    }
}
