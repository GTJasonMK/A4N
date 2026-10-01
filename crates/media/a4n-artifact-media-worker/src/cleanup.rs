use super::*;
use a4n_storage::remove_data_file_in_data_dir;

impl WorkerState {
    pub(super) async fn process_cleanup(
        &self,
        claim: a4n_db::cleanup_outbox::ClaimedFileArtifactCleanup,
    ) -> Result<(), String> {
        let id = claim.record.id;
        let lease_token = claim
            .record
            .lease_token
            .as_deref()
            .ok_or_else(|| format!("cleanup outbox claim 缺少 lease token: id={id}"))?;
        a4n_db::cleanup_outbox::renew_claimed_cleanup(self.db.connection(), id, lease_token)
            .await
            .map_err(|error| error.to_string())?;
        let candidates = a4n_db::media::filter_unreferenced_media_paths(
            self.db.connection(),
            &claim.payload.paths,
        )
        .await
        .map_err(|error| format!("复核文件 artifact 引用失败: {error}"))?;
        let candidates =
            a4n_db::media::filter_unreferenced_media_paths(self.db.connection(), &candidates)
                .await
                .map_err(|error| format!("二次复核文件 artifact 引用失败: {error}"))?;
        let protected = a4n_db::media::protected_image_publication_cleanup_paths(
            self.db.connection(),
            claim.payload.project_id,
            &candidates,
        )
        .await
        .map_err(|error| format!("复核图片发布保护路径失败: {error}"))?;
        let paths = candidates
            .into_iter()
            .filter(|path| !protected.contains(path))
            .collect::<Vec<_>>();
        let data_dir = self.data_dir.clone();
        let remove_result = tokio::task::spawn_blocking(move || {
            let mut errors = Vec::new();
            for path in paths {
                if let Err(error) = remove_data_file_in_data_dir(
                    data_dir.as_path(),
                    &path,
                    "文件 artifact cleanup 媒体文件",
                ) {
                    errors.push(error);
                }
            }
            if errors.is_empty() {
                Ok(())
            } else {
                Err(errors.join("；"))
            }
        })
        .await
        .map_err(|error| format!("文件 artifact cleanup 任务异常退出: {error}"))?;
        match remove_result {
            Ok(()) => a4n_db::cleanup_outbox::complete_claimed_cleanup(
                self.db.connection(),
                id,
                lease_token,
            )
            .await
            .map_err(|error| error.to_string()),
            Err(error) => {
                let retry_after =
                    a4n_cleanup_contract::cleanup_retry_after_seconds(claim.record.attempt_count);
                a4n_db::cleanup_outbox::retry_claimed_cleanup(
                    self.db.connection(),
                    id,
                    lease_token,
                    error,
                    chrono::Duration::seconds(retry_after),
                )
                .await
                .map_err(|error| error.to_string())
            }
        }
    }
}
