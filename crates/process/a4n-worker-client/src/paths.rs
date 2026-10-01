use super::{WORKER_COMMANDS_PATH, WORKER_OPERATIONS_PATH, WorkerClientError};

pub(super) fn command_path(command: &str) -> Result<String, WorkerClientError> {
    validate_path_segment("command", command)?;
    Ok(format!("{WORKER_COMMANDS_PATH}/{command}"))
}

pub(super) fn operation_path(
    operation_id: &str,
    action: &str,
) -> Result<String, WorkerClientError> {
    validate_path_segment("operation_id", operation_id)?;
    Ok(format!("{WORKER_OPERATIONS_PATH}/{operation_id}/{action}"))
}

fn validate_path_segment(field: &'static str, value: &str) -> Result<(), WorkerClientError> {
    if value.is_empty()
        || matches!(value, "." | "..")
        || value.chars().any(|character| {
            !character.is_ascii()
                || character.is_ascii_control()
                || character.is_whitespace()
                || matches!(character, '/' | '\\' | '?' | '#' | '%')
        })
    {
        return Err(WorkerClientError::InvalidPathSegment { field });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::WorkerClientError;

    use super::{command_path, operation_path};

    #[test]
    fn paths_validate_untrusted_segments_before_building_urls() {
        assert_eq!(command_path("demo.run").unwrap(), "/v1/commands/demo.run");
        assert_eq!(
            operation_path("operation-1", "cancel").unwrap(),
            "/v1/operations/operation-1/cancel"
        );
        for invalid_segment in [
            "",
            ".",
            "..",
            "a/b",
            "a\\\\b",
            "a?b",
            "a#b",
            "a%2fb",
            "two words",
            "章节",
        ] {
            assert!(matches!(
                command_path(invalid_segment),
                Err(WorkerClientError::InvalidPathSegment { field: "command" })
            ));
        }
        assert!(matches!(
            operation_path("../escape", "recover"),
            Err(WorkerClientError::InvalidPathSegment {
                field: "operation_id"
            })
        ));
    }
}
