use a4n_core::error::AppError;
use a4n_db::inspiration::{InspirationExchangeInsertInput, insert_inspiration_exchange};

use super::validation::required_message_content;
use super::{OrchestratorContext, inspiration_db_error_to_app};

impl OrchestratorContext {
    pub(super) async fn save_inspiration_exchange(
        &self,
        project_id: i64,
        user_input: &str,
        assistant_output: &str,
    ) -> Result<(), AppError> {
        let user_input = required_message_content(user_input)?;
        let assistant_output = required_message_content(assistant_output)?;
        insert_inspiration_exchange(
            self.db.connection(),
            InspirationExchangeInsertInput {
                project_id,
                user_input: user_input.to_string(),
                assistant_output: assistant_output.to_string(),
            },
        )
        .await
        .map_err(inspiration_db_error_to_app)?;
        Ok(())
    }
}
