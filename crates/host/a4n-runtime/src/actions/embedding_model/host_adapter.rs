//! Embedding action input adapter.

use a4n_domain::project::required_text as required_project_text;
use a4n_error::AppError;

pub(super) fn required_text(value: &str, label: &str) -> Result<String, AppError> {
    required_project_text(value, label)
        .map(str::to_string)
        .map_err(AppError::InvalidArgument)
}
