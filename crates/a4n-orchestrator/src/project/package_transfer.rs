use std::collections::{HashMap, HashSet};

use crate::project::ImportMediaPayload;
use a4n_api_contract::project_transfer::{
    MediaFileEntry, ProjectPackageExportInput, ProjectPackageTransferError,
    ProjectPackageTransferHostPort,
};
use a4n_error::AppError;
use serde::Serialize;

pub(crate) fn project_package_export_input<T: Serialize>(
    package: &T,
    media_files: &[MediaFileEntry],
    manifest_path: &str,
    manifest_label: &str,
    package_label: &str,
) -> Result<ProjectPackageExportInput, ProjectPackageTransferError> {
    Ok(ProjectPackageExportInput {
        manifest: serde_json::to_value(package)
            .map_err(|error| ProjectPackageTransferError::Message(error.to_string()))?,
        manifest_path: manifest_path.to_string(),
        manifest_label: manifest_label.to_string(),
        package_label: package_label.to_string(),
        media_files: media_files.to_vec(),
    })
}

pub(crate) fn index_import_media_payload(
    media_files: &[MediaFileEntry],
    bytes: HashMap<String, Vec<u8>>,
) -> Result<ImportMediaPayload, String> {
    let mut entries = std::collections::HashMap::with_capacity(media_files.len());
    let mut package_paths = HashSet::with_capacity(media_files.len());
    for entry in media_files {
        let key = crate::project::media_file_key(&entry.owner_kind, entry.owner_id, &entry.field);
        if entries.insert(key, entry.clone()).is_some() {
            return Err(format!(
                "媒体文件清单重复: owner_kind={}, owner_id={}, field={}",
                entry.owner_kind, entry.owner_id, entry.field
            ));
        }
        if !package_paths.insert(entry.package_path.clone()) {
            return Err(format!(
                "媒体文件清单 package_path 重复: {}",
                entry.package_path
            ));
        }
        let Some(file_bytes) = bytes.get(&entry.package_path) else {
            return Err(format!("项目包缺少媒体文件 {}", entry.package_path));
        };
        if file_bytes.is_empty() {
            return Err(format!("项目包媒体文件内容为空: {}", entry.package_path));
        }
    }
    if bytes.len() != package_paths.len() || bytes.keys().any(|path| !package_paths.contains(path))
    {
        return Err("媒体文件 payload 包含清单外字节".to_string());
    }
    Ok(ImportMediaPayload { entries, bytes })
}

pub(crate) async fn import_result_with_file_rollback<
    T,
    Host: ?Sized + ProjectPackageTransferHostPort,
>(
    result: Result<T, AppError>,
    host_port: &mut Host,
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
    use a4n_api_contract::project_transfer::{
        ProjectPackageExportInput, ProjectPackageImportPayload, ProjectPackageTransferHostPort,
    };
    use async_trait::async_trait;

    #[derive(Default)]
    struct TestFileHost {
        rollback_calls: usize,
    }

    #[async_trait]
    impl ProjectPackageTransferHostPort for TestFileHost {
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

        async fn render_export_package(
            &self,
            _input: ProjectPackageExportInput,
        ) -> Result<Vec<u8>, String> {
            Err("测试 Host 不支持导出".to_string())
        }

        async fn read_import_package(
            &self,
            _package_bytes: Vec<u8>,
            _manifest_path: &str,
            _package_label: &str,
            _manifest_label: &str,
        ) -> Result<ProjectPackageImportPayload, String> {
            Err("测试 Host 不支持导入".to_string())
        }
    }

    #[tokio::test]
    async fn failed_import_rolls_back_written_files_once() {
        let mut host = TestFileHost::default();
        let error = import_result_with_file_rollback::<(), TestFileHost>(
            Err(AppError::InvalidArgument("导入失败".to_string())),
            &mut host,
        )
        .await
        .expect_err("失败导入必须触发文件回滚");

        assert_eq!(host.rollback_calls, 1);
        assert!(error.to_string().contains("导入失败"));
    }

    #[test]
    fn index_import_media_payload_rejects_empty_and_extra_bytes() {
        let entry = MediaFileEntry {
            owner_kind: "media_asset".to_string(),
            owner_id: 1,
            field: "file_path".to_string(),
            original_path: "projects/1/media/file.png".to_string(),
            package_path: "media/media_asset/1/file.png".to_string(),
        };

        let error = index_import_media_payload(
            std::slice::from_ref(&entry),
            HashMap::from([(entry.package_path.clone(), Vec::new())]),
        )
        .expect_err("空媒体字节必须失败");
        assert!(error.contains("内容为空"));

        let error = index_import_media_payload(
            &[entry],
            HashMap::from([
                ("media/media_asset/1/file.png".to_string(), vec![1]),
                ("media/extra.png".to_string(), vec![2]),
            ]),
        )
        .expect_err("清单外媒体字节必须失败");
        assert!(error.contains("清单外字节"));
    }
}
