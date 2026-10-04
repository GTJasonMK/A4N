use super::*;
use std::path::PathBuf;

use a4n_artifact_media_contract::ArtifactMediaMaintenanceReport;
use a4n_db::cleanup_outbox::FileArtifactCleanupPayload;
use a4n_domain::EXTERNAL_IMPORT_MODEL;
use a4n_domain::media::{
    IMAGE_PUBLICATION_STATUS_PENDING, IMAGE_PUBLICATION_STATUS_PUBLISHED,
    IMAGE_PUBLICATION_STATUS_RECLAIMING, ImagePublicationFileFacts, ImagePublicationRecord,
    ImagePublicationRecoveryRecord,
};
use a4n_storage::{inspect_data_file_facts, promote_data_file};

impl WorkerState {
    pub(super) async fn maintain(
        &self,
        request: ArtifactMediaMaintenanceRequest,
    ) -> Result<ArtifactMediaMaintenanceReport, String> {
        request.validate().map_err(|error| error.to_string())?;
        let mut failures = Vec::new();
        let mut publications_examined = 0;
        let publications = a4n_db::video::image_publication_recovery_records_for_model(
            self.db.connection(),
            usize::from(request.publication_limit),
            None,
            Some(EXTERNAL_IMPORT_MODEL),
        )
        .await
        .map_err(|error| error.to_string())?;
        for publication in publications {
            publications_examined += 1;
            if let Err(error) = self.recover_publication(publication).await {
                failures.push(error);
            }
        }

        let mut cleanups_processed = 0;
        while cleanups_processed < request.cleanup_limit {
            let Some(claim) =
                a4n_db::cleanup_outbox::claim_file_artifact_cleanup(self.db.connection(), None)
                    .await
                    .map_err(|error| error.to_string())?
            else {
                break;
            };
            cleanups_processed += 1;
            if let Err(error) = self.process_cleanup(claim).await {
                failures.push(error);
            }
        }

        Ok(ArtifactMediaMaintenanceReport {
            publications_examined,
            cleanups_processed,
            failures,
        })
    }

    async fn recover_publication(&self, requested: ImagePublicationRecord) -> Result<(), String> {
        if requested.model != EXTERNAL_IMPORT_MODEL {
            return Ok(());
        }
        let publication = recovery_record(requested);
        match publication.status.as_str() {
            IMAGE_PUBLICATION_STATUS_PENDING => self.recover_pending(publication).await,
            IMAGE_PUBLICATION_STATUS_PUBLISHED => self.recover_published(publication).await,
            IMAGE_PUBLICATION_STATUS_RECLAIMING => self.recover_reclaiming(publication).await,
            _ => Ok(()),
        }
    }

    async fn recover_pending(
        &self,
        publication: ImagePublicationRecoveryRecord,
    ) -> Result<(), String> {
        let staging = inspect_data_file_facts(
            self.data_dir.as_path(),
            &publication.staging_path,
            "图片发布暂存文件",
        )?;
        let final_file = inspect_data_file_facts(
            self.data_dir.as_path(),
            &publication.final_path,
            "图片发布最终文件",
        )?;
        if let Some(final_facts) = final_file.as_ref() {
            if !facts_match(final_facts, &publication) {
                return self.reclaim(publication, "最终文件事实不匹配").await;
            }
            self.publish_recovered(&publication, final_facts).await?;
            if staging.is_some() {
                self.enqueue_staging_cleanup(&publication).await?;
            }
            return Ok(());
        }
        let Some(staging_facts) = staging else {
            return self.reclaim(publication, "发布意图没有可用文件").await;
        };
        if !facts_match(&staging_facts, &publication) {
            return self.reclaim(publication, "暂存文件事实不匹配").await;
        }
        promote_data_file(
            self.data_dir.as_path(),
            &publication.staging_path,
            &publication.final_path,
            "图片发布恢复",
        )?;
        let final_facts = inspect_data_file_facts(
            self.data_dir.as_path(),
            &publication.final_path,
            "图片发布最终文件",
        )?
        .ok_or_else(|| "图片发布恢复提升后找不到最终文件".to_string())?;
        if !facts_match(&final_facts, &publication) {
            return self
                .reclaim(publication, "提升后的最终文件事实不匹配")
                .await;
        }
        self.publish_recovered(&publication, &final_facts).await
    }

    async fn recover_published(
        &self,
        publication: ImagePublicationRecoveryRecord,
    ) -> Result<(), String> {
        let final_facts = match inspect_data_file_facts(
            &self.data_dir,
            &publication.final_path,
            "已发布图片最终文件",
        )? {
            Some(facts) => facts,
            None => {
                if let Some(staging_facts) = inspect_data_file_facts(
                    self.data_dir.as_path(),
                    &publication.staging_path,
                    "已发布图片暂存文件",
                )? {
                    if !facts_match(&staging_facts, &publication) {
                        return Err(format!(
                            "已发布图片最终文件缺失且暂存文件事实不匹配: publication_id={}",
                            publication.publication_id
                        ));
                    }
                    promote_data_file(
                        self.data_dir.as_path(),
                        &publication.staging_path,
                        &publication.final_path,
                        "已发布图片恢复",
                    )?;
                    inspect_data_file_facts(
                        self.data_dir.as_path(),
                        &publication.final_path,
                        "已发布图片恢复后的最终文件",
                    )?
                    .ok_or_else(|| "已发布图片恢复提升后找不到最终文件".to_string())?
                } else if publication.image_id.is_none() || {
                    let Some(image_id) = publication.image_id else {
                        return self
                            .reclaim(
                                publication,
                                "已发布图片记录和最终文件均缺失，按历史孤儿意图回收",
                            )
                            .await;
                    };
                    a4n_db::video::read_models::generated_image_by_id(
                        self.db.connection(),
                        image_id,
                    )
                    .await
                    .map_err(|error| error.to_string())?
                    .is_none()
                } {
                    return self
                        .reclaim(
                            publication,
                            "已发布图片记录和最终文件均缺失，按历史孤儿意图回收",
                        )
                        .await;
                } else {
                    return Err(format!(
                        "已发布图片最终文件缺失，需人工恢复或从备份还原: publication_id={}, image_id={:?}",
                        publication.publication_id, publication.image_id
                    ));
                }
            }
        };
        if !facts_match(&final_facts, &publication) {
            return Err(format!(
                "已发布图片最终文件事实不匹配，保留数据库记录并显式告警: publication_id={}",
                publication.publication_id
            ));
        }
        self.publish_recovered(&publication, &final_facts).await?;
        if inspect_data_file_facts(
            self.data_dir.as_path(),
            &publication.staging_path,
            "已发布图片暂存文件",
        )?
        .is_some()
        {
            self.enqueue_staging_cleanup(&publication).await?;
        }
        Ok(())
    }

    async fn publish_recovered(
        &self,
        publication: &ImagePublicationRecoveryRecord,
        facts: &(PathBuf, i64, String),
    ) -> Result<(), String> {
        a4n_db::video::read_models::publish_image_publication(
            self.db.connection(),
            &publication.publication_id,
            ImagePublicationFileFacts {
                file_size: facts.1,
                sha256: facts.2.clone(),
            },
        )
        .await
        .map(|_| ())
        .map_err(|error| error.to_string())
    }

    async fn reclaim(
        &self,
        publication: ImagePublicationRecoveryRecord,
        reason: &str,
    ) -> Result<(), String> {
        let publication = a4n_db::video::mark_image_publication_reclaiming_record(
            self.db.connection(),
            &publication.publication_id,
            publication.version,
            reason.to_string(),
        )
        .await
        .map_err(|error| error.to_string())?;
        let publication = recovery_record(publication);
        self.enqueue_reclaim_cleanup(&publication).await?;
        self.recover_reclaiming(publication).await
    }

    async fn recover_reclaiming(
        &self,
        publication: ImagePublicationRecoveryRecord,
    ) -> Result<(), String> {
        self.enqueue_reclaim_cleanup(&publication).await?;
        let mut existing_paths = Vec::new();
        for path in [&publication.staging_path, &publication.final_path] {
            if inspect_data_file_facts(&self.data_dir, path, "图片发布回收文件")?.is_some()
            {
                existing_paths.push(path.clone());
            }
        }
        if existing_paths.is_empty() {
            a4n_db::video::mark_image_publication_reclaimed_record(
                self.db.connection(),
                &publication.publication_id,
                publication.version,
                None,
            )
            .await
            .map(|_| ())
            .map_err(|error| error.to_string())?;
            return Ok(());
        }
        let unreferenced =
            a4n_db::media::filter_unreferenced_media_paths(self.db.connection(), &existing_paths)
                .await
                .map_err(|error| error.to_string())?;
        if unreferenced.len() < existing_paths.len() {
            a4n_db::video::mark_image_publication_reclaimed_record(
                self.db.connection(),
                &publication.publication_id,
                publication.version,
                Some("回收复核发现已有媒体引用，保留被引用文件".to_string()),
            )
            .await
            .map(|_| ())
            .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    async fn enqueue_reclaim_cleanup(
        &self,
        publication: &ImagePublicationRecoveryRecord,
    ) -> Result<(), String> {
        self.enqueue_cleanup(
            format!(
                "image-publication-reclaim:{}:{}",
                publication.publication_id, publication.version
            ),
            publication.project_id,
            vec![
                publication.staging_path.clone(),
                publication.final_path.clone(),
            ],
        )
        .await
    }

    async fn enqueue_staging_cleanup(
        &self,
        publication: &ImagePublicationRecoveryRecord,
    ) -> Result<(), String> {
        self.enqueue_cleanup(
            format!("image-publication-staging:{}", publication.publication_id),
            publication.project_id,
            vec![publication.staging_path.clone()],
        )
        .await
    }

    async fn enqueue_cleanup(
        &self,
        dedupe_key: String,
        project_id: i64,
        paths: Vec<String>,
    ) -> Result<(), String> {
        a4n_db::cleanup_outbox::insert_file_artifact_cleanup_intent(
            self.db.connection(),
            dedupe_key,
            FileArtifactCleanupPayload::new(project_id, paths),
        )
        .await
        .map(|_| ())
        .map_err(|error| error.to_string())
    }
}
