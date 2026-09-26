use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use a4n_api_contract::image::{
    DeleteImageInput, GeneratedImageSummary, ListImagesByChapterInput, ListImagesByProjectInput,
    ListImagesByProjectsInput, UpdateGeneratedImageInput,
};
use a4n_artifact_media_contract::{
    ARTIFACT_MEDIA_CAPABILITY_VERSION, ARTIFACT_MEDIA_IMAGE_BY_ID_COMMAND,
    ARTIFACT_MEDIA_IMAGE_DELETE_COMMAND, ARTIFACT_MEDIA_IMAGE_LIST_CHAPTER_COMMAND,
    ARTIFACT_MEDIA_IMAGE_LIST_PROJECT_COMMAND, ARTIFACT_MEDIA_IMAGE_LIST_PROJECTS_COMMAND,
    ARTIFACT_MEDIA_IMAGE_REFERENCE_IDS_COMMAND, ARTIFACT_MEDIA_IMAGE_UPDATE_COMMAND,
    ARTIFACT_MEDIA_MAINTENANCE_COMMAND, ARTIFACT_MEDIA_PATH_PREFIX_REFERENCED_COMMAND,
    ARTIFACT_MEDIA_PROJECT_PUBLICATION_COMMAND, ARTIFACT_MEDIA_PROTECTED_PATHS_COMMAND,
    ARTIFACT_MEDIA_PUBLISH_IMAGE_COMMAND, ARTIFACT_MEDIA_UNREFERENCED_PATHS_COMMAND,
    ARTIFACT_MEDIA_WORKER_ID, ArtifactMediaMaintenanceReport, ArtifactMediaMaintenanceRequest,
    ImageReferenceIdsRequest, MediaPathPrefixRequest, MediaPathsRequest, ProjectPathsRequest,
    ProjectPublicationRequest, PublishImageRequest, PublishedImageResponse,
};
use a4n_db::cleanup_outbox::FileArtifactCleanupPayload;
use a4n_domain::EXTERNAL_IMPORT_MODEL;
use a4n_domain::media::{
    IMAGE_PUBLICATION_STATUS_PENDING, IMAGE_PUBLICATION_STATUS_PUBLISHED,
    IMAGE_PUBLICATION_STATUS_RECLAIMING, ImagePublicationFileFacts, ImagePublicationRecoveryRecord,
};
use a4n_storage::{inspect_data_file_facts, promote_data_file, remove_data_file_in_data_dir};
use a4n_worker_contract::{
    WORKER_CAPABILITIES_PATH, WORKER_HEALTH_PATH, WORKER_SCHEMA_VERSION, WorkerCapabilities,
    WorkerCapability, WorkerEnvelope, WorkerError, WorkerErrorCode, WorkerHealth,
    WorkerHealthStatus, WorkerOperationStatus, WorkerResponse,
};
use axum::{
    Json, Router,
    extract::{Path as AxumPath, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde_json::{Value, json};
use tokio::time::MissedTickBehavior;

const DEFAULT_ADDR: &str = "127.0.0.1:47652";
const RECOVERY_LIMIT: usize = 64;
const CLEANUP_LIMIT: usize = 64;
const MAINTENANCE_INTERVAL: Duration = Duration::from_secs(30);
const AUTH_HEADER: &str = "x-a4n-worker-token";

#[derive(Clone)]
struct WorkerState {
    db: a4n_db::DbHandle,
    data_dir: Arc<PathBuf>,
    token: Arc<str>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ImageIdRequest {
    image_id: i64,
}

impl WorkerState {
    async fn maintain(
        &self,
        request: ArtifactMediaMaintenanceRequest,
    ) -> Result<ArtifactMediaMaintenanceReport, String> {
        request.validate().map_err(|error| error.to_string())?;
        let mut failures = Vec::new();
        let mut publications_examined = 0;
        let publications = a4n_db::video::image_publications_for_recovery_for_model(
            &self.db,
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
            let Some(claim) = a4n_db::cleanup_outbox::claim_file_artifact_cleanup(&self.db, None)
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

    async fn recover_publication(
        &self,
        requested: a4n_db_models::models::media::image_publication::Model,
    ) -> Result<(), String> {
        let Some(publication) =
            a4n_db::video::image_publication_by_id(&self.db, &requested.publication_id)
                .await
                .map_err(|error| error.to_string())?
        else {
            return Ok(());
        };
        if publication.model != EXTERNAL_IMPORT_MODEL {
            return Ok(());
        }
        let publication = recovery_record(publication);
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
                    a4n_db::video::generated_image_by_id(&self.db, image_id)
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
        a4n_db::video::publish_image_publication(
            &self.db,
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
        let publication = a4n_db::video::mark_image_publication_reclaiming(
            &self.db,
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
            a4n_db::video::mark_image_publication_reclaimed(
                &self.db,
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
            a4n_db::media::filter_unreferenced_media_paths(&self.db, &existing_paths)
                .await
                .map_err(|error| error.to_string())?;
        if unreferenced.len() < existing_paths.len() {
            a4n_db::video::mark_image_publication_reclaimed(
                &self.db,
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
            &self.db,
            dedupe_key,
            FileArtifactCleanupPayload::new(project_id, paths),
        )
        .await
        .map(|_| ())
        .map_err(|error| error.to_string())
    }

    async fn process_cleanup(
        &self,
        claim: a4n_db::cleanup_outbox::ClaimedFileArtifactCleanup,
    ) -> Result<(), String> {
        let id = claim.record.id;
        let lease_token = claim
            .record
            .lease_token
            .as_deref()
            .ok_or_else(|| format!("cleanup outbox claim 缺少 lease token: id={id}"))?;
        a4n_db::cleanup_outbox::renew_claimed_cleanup(&self.db, id, lease_token)
            .await
            .map_err(|error| error.to_string())?;
        let candidates =
            a4n_db::media::filter_unreferenced_media_paths(&self.db, &claim.payload.paths)
                .await
                .map_err(|error| format!("复核文件 artifact 引用失败: {error}"))?;
        let candidates = a4n_db::media::filter_unreferenced_media_paths(&self.db, &candidates)
            .await
            .map_err(|error| format!("二次复核文件 artifact 引用失败: {error}"))?;
        let protected = a4n_db::media::protected_image_publication_cleanup_paths(
            &self.db,
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
            Ok(()) => a4n_db::cleanup_outbox::complete_claimed_cleanup(&self.db, id, lease_token)
                .await
                .map_err(|error| error.to_string()),
            Err(error) => {
                let retry_after =
                    a4n_cleanup_contract::cleanup_retry_after_seconds(claim.record.attempt_count);
                a4n_db::cleanup_outbox::retry_claimed_cleanup(
                    &self.db,
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

    async fn unreferenced_paths(&self, request: MediaPathsRequest) -> Result<Vec<String>, String> {
        request.validate().map_err(|error| error.to_string())?;
        a4n_db::media::filter_unreferenced_media_paths(&self.db, &request.paths)
            .await
            .map_err(|error| error.to_string())
    }

    async fn path_prefix_referenced(
        &self,
        request: MediaPathPrefixRequest,
    ) -> Result<bool, String> {
        request.validate().map_err(|error| error.to_string())?;
        a4n_db::media::media_path_prefix_has_persisted_reference(&self.db, &request.prefix)
            .await
            .map_err(|error| error.to_string())
    }

    async fn project_has_publication(
        &self,
        request: ProjectPublicationRequest,
    ) -> Result<bool, String> {
        request.validate().map_err(|error| error.to_string())?;
        a4n_db::media::project_has_active_image_publication(&self.db, request.project_id)
            .await
            .map_err(|error| error.to_string())
    }

    async fn protected_paths(&self, request: ProjectPathsRequest) -> Result<Vec<String>, String> {
        request.validate().map_err(|error| error.to_string())?;
        let paths = a4n_db::media::protected_image_publication_cleanup_paths(
            &self.db,
            request.project_id,
            &request.paths,
        )
        .await
        .map_err(|error| error.to_string())?;
        Ok(paths.into_iter().collect())
    }

    async fn publish_image(
        &self,
        actor_user_id: i64,
        request: PublishImageRequest,
    ) -> Result<PublishedImageResponse, String> {
        request.validate().map_err(|error| error.to_string())?;
        a4n_db::collaboration::verify_project_permission(
            &self.db,
            request.project_id,
            actor_user_id,
            a4n_db::collaboration::ProjectAccessRole::Editor,
        )
        .await
        .map_err(|error| error.to_string())?;
        let publication = a4n_db::video::image_publication_by_id(&self.db, &request.publication_id)
            .await
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("图片发布意图不存在: {}", request.publication_id))?;
        if publication.project_id != request.project_id
            || publication.model != EXTERNAL_IMPORT_MODEL
        {
            return Err("Artifact/Media Worker 只允许发布当前项目的外部导入图片".to_string());
        }
        let facts = inspect_data_file_facts(
            self.data_dir.as_path(),
            &publication.final_path,
            "图片发布最终文件",
        )?
        .ok_or_else(|| "图片发布最终文件不存在".to_string())?;
        if facts.1 != request.file_size || facts.2 != request.sha256 {
            return Err("图片文件事实与发布请求不一致".to_string());
        }
        let image = a4n_db::video::publish_image_publication(
            &self.db,
            &request.publication_id,
            a4n_domain::media::ImagePublicationFileFacts {
                file_size: facts.1,
                sha256: facts.2,
            },
        )
        .await
        .map_err(|error| error.to_string())?;
        let publication = a4n_db::video::image_publication_by_id(&self.db, &request.publication_id)
            .await
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "图片发布完成后找不到 publication".to_string())?;
        Ok(PublishedImageResponse {
            publication_id: publication.publication_id,
            image_id: image.id,
            project_id: image.project_id,
            image_type: image.image_type,
            file_path: image.file_path,
            file_size: image.file_size.unwrap_or_default(),
            revision: publication.version,
        })
    }

    async fn image_by_id(
        &self,
        user_id: i64,
        request: ImageIdRequest,
    ) -> Result<Option<GeneratedImageSummary>, String> {
        if request.image_id <= 0 {
            return Err("image_id must be positive".to_string());
        }
        let Some(image) = a4n_db::video::generated_image_by_id(&self.db, request.image_id)
            .await
            .map_err(|error| error.to_string())?
        else {
            return Ok(None);
        };
        verify_image_permission(&self.db, user_id, image.project_id, "viewer").await?;
        Ok(Some(generated_image_summary(&image)))
    }

    async fn image_list_by_project(
        &self,
        user_id: i64,
        request: ListImagesByProjectInput,
    ) -> Result<Vec<GeneratedImageSummary>, String> {
        if request.project_id <= 0 {
            return Err("project_id must be positive".to_string());
        }
        verify_image_permission(&self.db, user_id, request.project_id, "viewer").await?;
        a4n_db::video::generated_images_for_project(&self.db, request.project_id)
            .await
            .map(|images| images.iter().map(generated_image_summary).collect())
            .map_err(|error| error.to_string())
    }

    async fn image_list_by_projects(
        &self,
        user_id: i64,
        request: ListImagesByProjectsInput,
    ) -> Result<Vec<GeneratedImageSummary>, String> {
        let project_ids = normalized_project_ids(request.project_ids)?;
        for project_id in &project_ids {
            verify_image_permission(&self.db, user_id, *project_id, "viewer").await?;
        }
        let mut images = a4n_db::video::generated_images_for_projects(&self.db, &project_ids)
            .await
            .map_err(|error| error.to_string())?;
        sort_images_by_project_order(&mut images, &project_ids);
        Ok(images.iter().map(generated_image_summary).collect())
    }

    async fn image_list_by_chapter(
        &self,
        user_id: i64,
        request: ListImagesByChapterInput,
    ) -> Result<Vec<GeneratedImageSummary>, String> {
        if request.project_id <= 0 || request.chapter_id <= 0 {
            return Err("project_id 和 chapter_id must be positive".to_string());
        }
        verify_image_permission(&self.db, user_id, request.project_id, "viewer").await?;
        a4n_db::video::ensure_image_chapter_belongs_to_project_record(
            &self.db,
            request.project_id,
            request.chapter_id,
        )
        .await
        .map_err(|error| error.to_string())?;
        a4n_db::video::generated_images_for_chapter(
            &self.db,
            request.project_id,
            request.chapter_id,
        )
        .await
        .map(|images| images.iter().map(generated_image_summary).collect())
        .map_err(|error| error.to_string())
    }

    async fn image_reference_by_ids(
        &self,
        user_id: i64,
        request: ImageReferenceIdsRequest,
    ) -> Result<Vec<GeneratedImageSummary>, String> {
        if request.project_id <= 0 || request.image_ids.iter().any(|id| *id <= 0) {
            return Err("project_id 和 image_ids 必须为正整数".to_string());
        }
        verify_image_permission(&self.db, user_id, request.project_id, "viewer").await?;
        a4n_db::character::portrait_reference_images_by_ids(
            &self.db,
            request.project_id,
            &request.image_ids,
        )
        .await
        .map(|images| images.iter().map(generated_image_summary).collect())
        .map_err(|error| error.to_string())
    }

    async fn update_image(
        &self,
        user_id: i64,
        input: UpdateGeneratedImageInput,
    ) -> Result<GeneratedImageSummary, String> {
        if input.is_empty() {
            return Err("图片更新至少需要一个字段".to_string());
        }
        let image = a4n_db::video::generated_image_by_id(&self.db, input.image_id)
            .await
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "图片不存在".to_string())?;
        verify_image_permission(&self.db, user_id, image.project_id, "editor").await?;
        let updated = a4n_db::video::update_generated_image_record(
            &self.db,
            input.image_id,
            a4n_domain::media::UpdateGeneratedImageRecordInput {
                prompt: input.prompt,
                image_type: input.image_type,
                chapter_id: input.chapter_id.flatten(),
                clear_chapter_id: input.chapter_id.as_ref().is_some_and(Option::is_none),
                panel_id: input.panel_id.clone().flatten(),
                clear_panel_id: input.panel_id.as_ref().is_some_and(Option::is_none),
                ai_score: input.ai_score.flatten(),
                clear_ai_score: input.ai_score.as_ref().is_some_and(Option::is_none),
                is_favorite: input.is_favorite,
            },
        )
        .await
        .map_err(|error| error.to_string())?;
        Ok(generated_image_summary(&updated))
    }

    async fn delete_image(&self, user_id: i64, input: DeleteImageInput) -> Result<(), String> {
        if input.image_id <= 0 {
            return Err("image_id must be positive".to_string());
        }
        let image = a4n_db::video::generated_image_by_id(&self.db, input.image_id)
            .await
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "图片不存在".to_string())?;
        verify_image_permission(&self.db, user_id, image.project_id, "editor").await?;
        a4n_db::video::delete_generated_image_record_with_cleanup(&self.db, input.image_id)
            .await
            .map_err(|error| error.to_string())
    }
}

pub fn run() -> anyhow::Result<()> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async_main())
}

async fn async_main() -> anyhow::Result<()> {
    init_tracing();
    let database_url = a4n_config::required_non_empty_env_string("A4N_DATABASE_URL")?;
    a4n_config::validate_postgres_database_url("A4N_DATABASE_URL", &database_url)?;
    let data_dir = a4n_config::app_config::AppConfig::default().data_dir;
    std::fs::create_dir_all(&data_dir)?;
    let token = a4n_config::required_non_empty_env_string(
        a4n_artifact_media_contract::ARTIFACT_MEDIA_WORKER_TOKEN_ENV,
    )?;
    let addr = worker_addr()?;
    let db = a4n_db::DbHandle::connect_with_default_options(database_url).await?;
    let state = WorkerState {
        db,
        data_dir: Arc::new(data_dir),
        token: Arc::from(token),
    };
    let maintenance_state = state.clone();
    let maintenance = tokio::spawn(async move {
        let mut interval = tokio::time::interval(MAINTENANCE_INTERVAL);
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        interval.tick().await;
        loop {
            interval.tick().await;
            if let Err(error) = maintenance_state
                .maintain(ArtifactMediaMaintenanceRequest {
                    publication_limit: RECOVERY_LIMIT as u16,
                    cleanup_limit: CLEANUP_LIMIT as u16,
                })
                .await
            {
                tracing::warn!(error = %error, "artifact media worker maintenance failed");
            }
        }
    });
    let app = Router::new()
        .route(WORKER_HEALTH_PATH, get(health))
        .route(WORKER_CAPABILITIES_PATH, get(capabilities))
        .route("/v1/commands/{command}", post(command))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(worker_id = ARTIFACT_MEDIA_WORKER_ID, %addr, "artifact media worker listening");
    let result = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await;
    maintenance.abort();
    result?;
    Ok(())
}

async fn health(State(state): State<WorkerState>, headers: HeaderMap) -> Response {
    if !authorized(&state, &headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let status = match state.db.ping().await {
        Ok(()) => WorkerHealthStatus::Healthy,
        Err(_) => WorkerHealthStatus::Degraded,
    };
    Json(WorkerHealth {
        schema_version: WORKER_SCHEMA_VERSION,
        worker_id: ARTIFACT_MEDIA_WORKER_ID.to_string(),
        status,
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
    .into_response()
}

async fn capabilities(State(state): State<WorkerState>, headers: HeaderMap) -> Response {
    if !authorized(&state, &headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    Json(WorkerCapabilities {
        schema_version: WORKER_SCHEMA_VERSION,
        worker_id: ARTIFACT_MEDIA_WORKER_ID.to_string(),
        capabilities: vec![WorkerCapability {
            name: "artifact-media".to_string(),
            version: ARTIFACT_MEDIA_CAPABILITY_VERSION.to_string(),
            enabled: true,
        }],
    })
    .into_response()
}

async fn command(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    AxumPath(command): AxumPath<String>,
    Json(request): Json<WorkerEnvelope<Value>>,
) -> Response {
    if !authorized(&state, &headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    if let Err(error) = request.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": error.to_string() })),
        )
            .into_response();
    }
    if request.lease.worker_id != ARTIFACT_MEDIA_WORKER_ID {
        return Json(failed_response(
            &request,
            WorkerErrorCode::LeaseLost,
            "worker lease identity does not match artifact media worker",
        ))
        .into_response();
    }
    let user_command = command == ARTIFACT_MEDIA_PUBLISH_IMAGE_COMMAND
        || matches!(
            command.as_str(),
            ARTIFACT_MEDIA_IMAGE_BY_ID_COMMAND
                | ARTIFACT_MEDIA_IMAGE_LIST_PROJECT_COMMAND
                | ARTIFACT_MEDIA_IMAGE_LIST_PROJECTS_COMMAND
                | ARTIFACT_MEDIA_IMAGE_LIST_CHAPTER_COMMAND
                | ARTIFACT_MEDIA_IMAGE_UPDATE_COMMAND
                | ARTIFACT_MEDIA_IMAGE_DELETE_COMMAND
        );
    if (!user_command
        && request.principal.kind != a4n_worker_contract::WorkerPrincipalKind::Service)
        || (user_command
            && request.principal.kind != a4n_worker_contract::WorkerPrincipalKind::User)
    {
        return Json(failed_response(
            &request,
            WorkerErrorCode::Unauthorized,
            "artifact media command principal kind is invalid",
        ))
        .into_response();
    }
    let result = match command.as_str() {
        ARTIFACT_MEDIA_MAINTENANCE_COMMAND => {
            match parse_payload::<ArtifactMediaMaintenanceRequest>(request.payload.clone()) {
                Ok(payload) => state.maintain(payload).await.map(|payload| json!(payload)),
                Err(error) => Err(error),
            }
        }
        ARTIFACT_MEDIA_UNREFERENCED_PATHS_COMMAND => {
            match parse_payload::<MediaPathsRequest>(request.payload.clone()) {
                Ok(payload) => state
                    .unreferenced_paths(payload)
                    .await
                    .map(|payload| json!(payload)),
                Err(error) => Err(error),
            }
        }
        ARTIFACT_MEDIA_PATH_PREFIX_REFERENCED_COMMAND => {
            match parse_payload::<MediaPathPrefixRequest>(request.payload.clone()) {
                Ok(payload) => state
                    .path_prefix_referenced(payload)
                    .await
                    .map(|payload| json!(payload)),
                Err(error) => Err(error),
            }
        }
        ARTIFACT_MEDIA_PROJECT_PUBLICATION_COMMAND => {
            match parse_payload::<ProjectPublicationRequest>(request.payload.clone()) {
                Ok(payload) => state
                    .project_has_publication(payload)
                    .await
                    .map(|payload| json!(payload)),
                Err(error) => Err(error),
            }
        }
        ARTIFACT_MEDIA_PROTECTED_PATHS_COMMAND => {
            match parse_payload::<ProjectPathsRequest>(request.payload.clone()) {
                Ok(payload) => state
                    .protected_paths(payload)
                    .await
                    .map(|payload| json!(payload)),
                Err(error) => Err(error),
            }
        }
        ARTIFACT_MEDIA_PUBLISH_IMAGE_COMMAND => {
            match parse_payload::<PublishImageRequest>(request.payload.clone()) {
                Ok(payload) => match request.principal.subject.parse::<i64>() {
                    Ok(actor_user_id) if actor_user_id > 0 => state
                        .publish_image(actor_user_id, payload)
                        .await
                        .map(|payload| json!(payload)),
                    _ => Err("user principal subject must be a positive integer".to_string()),
                },
                Err(error) => Err(error),
            }
        }
        ARTIFACT_MEDIA_IMAGE_BY_ID_COMMAND => {
            match parse_payload::<ImageIdRequest>(request.payload.clone()) {
                Ok(payload) => match actor_user_id(&request) {
                    Ok(user_id) => state
                        .image_by_id(user_id, payload)
                        .await
                        .map(|payload| json!(payload)),
                    Err(error) => Err(error),
                },
                Err(error) => Err(error),
            }
        }
        ARTIFACT_MEDIA_IMAGE_LIST_PROJECT_COMMAND => {
            match parse_payload::<ListImagesByProjectInput>(request.payload.clone()) {
                Ok(payload) => match actor_user_id(&request) {
                    Ok(user_id) => state
                        .image_list_by_project(user_id, payload)
                        .await
                        .map(|payload| json!(payload)),
                    Err(error) => Err(error),
                },
                Err(error) => Err(error),
            }
        }
        ARTIFACT_MEDIA_IMAGE_LIST_PROJECTS_COMMAND => {
            match parse_payload::<ListImagesByProjectsInput>(request.payload.clone()) {
                Ok(payload) => match actor_user_id(&request) {
                    Ok(user_id) => state
                        .image_list_by_projects(user_id, payload)
                        .await
                        .map(|payload| json!(payload)),
                    Err(error) => Err(error),
                },
                Err(error) => Err(error),
            }
        }
        ARTIFACT_MEDIA_IMAGE_LIST_CHAPTER_COMMAND => {
            match parse_payload::<ListImagesByChapterInput>(request.payload.clone()) {
                Ok(payload) => match actor_user_id(&request) {
                    Ok(user_id) => state
                        .image_list_by_chapter(user_id, payload)
                        .await
                        .map(|payload| json!(payload)),
                    Err(error) => Err(error),
                },
                Err(error) => Err(error),
            }
        }
        ARTIFACT_MEDIA_IMAGE_REFERENCE_IDS_COMMAND => {
            match parse_payload::<ImageReferenceIdsRequest>(request.payload.clone()) {
                Ok(payload) => match actor_user_id(&request) {
                    Ok(user_id) => state
                        .image_reference_by_ids(user_id, payload)
                        .await
                        .map(|payload| json!(payload)),
                    Err(error) => Err(error),
                },
                Err(error) => Err(error),
            }
        }
        ARTIFACT_MEDIA_IMAGE_UPDATE_COMMAND => {
            match parse_payload::<UpdateGeneratedImageInput>(request.payload.clone()) {
                Ok(payload) => match actor_user_id(&request) {
                    Ok(user_id) => state
                        .update_image(user_id, payload)
                        .await
                        .map(|payload| json!(payload)),
                    Err(error) => Err(error),
                },
                Err(error) => Err(error),
            }
        }
        ARTIFACT_MEDIA_IMAGE_DELETE_COMMAND => {
            match parse_payload::<DeleteImageInput>(request.payload.clone()) {
                Ok(payload) => match actor_user_id(&request) {
                    Ok(user_id) => state
                        .delete_image(user_id, payload)
                        .await
                        .map(|payload| json!(payload)),
                    Err(error) => Err(error),
                },
                Err(error) => Err(error),
            }
        }
        _ => Err(format!("unsupported artifact media command: {command}")),
    };
    match result {
        Ok(payload) => Json(completed_response(&request, payload)).into_response(),
        Err(error) => {
            Json(failed_response(&request, classify_error(&error), error)).into_response()
        }
    }
}

mod support;
use support::*;

fn worker_addr() -> anyhow::Result<SocketAddr> {
    let value = std::env::var("A4N_ARTIFACT_MEDIA_WORKER_ADDR")
        .unwrap_or_else(|_| DEFAULT_ADDR.to_string());
    value
        .parse()
        .map_err(|error| anyhow::anyhow!("A4N_ARTIFACT_MEDIA_WORKER_ADDR 无效: {error}"))
}

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("install SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = terminate.recv() => {},
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

#[cfg(test)]
mod tests {
    use super::facts_match;
    use a4n_domain::media::ImagePublicationRecoveryRecord;
    use std::path::PathBuf;

    #[test]
    fn recovery_only_accepts_exact_file_facts() {
        let publication = ImagePublicationRecoveryRecord {
            publication_id: "publication".to_string(),
            project_id: 1,
            requested_by_user_id: 2,
            staging_path: "projects/1/a.part".to_string(),
            final_path: "projects/1/a.png".to_string(),
            image_type: "project_reference".to_string(),
            model: a4n_domain::EXTERNAL_IMPORT_MODEL.to_string(),
            expected_file_size: 3,
            expected_sha256: "abc".to_string(),
            file_size: None,
            status: "pending".to_string(),
            version: 1,
            image_id: None,
        };
        assert!(facts_match(
            &(PathBuf::new(), 3, "abc".to_string()),
            &publication
        ));
        assert!(!facts_match(
            &(PathBuf::new(), 4, "abc".to_string()),
            &publication
        ));
    }
}
