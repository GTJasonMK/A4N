use a4n_core::{error::AppError, models::project::novel_project};
use async_trait::async_trait;

#[derive(Debug)]
pub struct ExportedProjectPackage {
    pub project: novel_project::Model,
    pub package_bytes: Vec<u8>,
}

#[derive(Debug)]
pub struct ImportProjectPackageApplicationInput {
    pub user_id: i64,
    pub package_bytes: Vec<u8>,
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectPackageTransferError {
    #[error("{0}")]
    Application(#[from] AppError),
    #[error("{0}")]
    Message(String),
}

impl From<String> for ProjectPackageTransferError {
    fn from(message: String) -> Self {
        Self::Message(message)
    }
}

#[async_trait]
pub trait ProjectPackageFileHostPort: Send + Sync {
    fn write_imported_media_file(
        &mut self,
        relative_path: &str,
        bytes: &[u8],
        label: &str,
    ) -> Result<(), String>;

    async fn rollback_imported_media_files(&mut self) -> Result<(), String>;
}

pub(crate) async fn import_result_with_file_rollback<T>(
    result: Result<T, AppError>,
    host_port: &mut impl ProjectPackageFileHostPort,
) -> Result<T, ProjectPackageTransferError> {
    match result {
        Ok(value) => Ok(value),
        Err(primary) => match host_port.rollback_imported_media_files().await {
            Ok(()) => Err(ProjectPackageTransferError::Application(primary)),
            Err(cleanup_error) => Err(ProjectPackageTransferError::Message(format!(
                "{primary}；{cleanup_error}"
            ))),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct TestFileHost {
        rollback_calls: usize,
    }

    #[async_trait]
    impl ProjectPackageFileHostPort for TestFileHost {
        fn write_imported_media_file(
            &mut self,
            _relative_path: &str,
            _bytes: &[u8],
            _label: &str,
        ) -> Result<(), String> {
            Ok(())
        }

        async fn rollback_imported_media_files(&mut self) -> Result<(), String> {
            self.rollback_calls += 1;
            Ok(())
        }
    }

    #[tokio::test]
    async fn failed_import_rolls_back_written_files_once() {
        let mut host = TestFileHost::default();
        let error = import_result_with_file_rollback::<()>(
            Err(AppError::InvalidArgument("导入失败".to_string())),
            &mut host,
        )
        .await
        .expect_err("失败导入必须触发文件回滚");

        assert_eq!(host.rollback_calls, 1);
        assert!(error.to_string().contains("导入失败"));
    }
}
