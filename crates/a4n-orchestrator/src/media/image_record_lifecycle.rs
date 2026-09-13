use std::collections::{HashMap, HashSet};

use a4n_api_contract::image::{
    DeleteImageInput, GeneratedImageIdentity, GeneratedImageSummary, ImageMediaReferencePort,
    ImagePublicationRecoveryReadPort, ImageRecordMutationApplicationPort,
    ImageRecordReadApplicationPort, ListImagesByChapterInput, ListImagesByProjectInput,
    ListImagesByProjectsInput, ListVideoReferenceImagesInput, UpdateGeneratedImageInput,
    UploadImageInput,
};
use a4n_db::video::{
    CreateGeneratedImageRecordDraftInput, CreateImagePublicationIntentInput,
    ImagePublicationFileFacts, UpdateGeneratedImageRecordInput,
};
use a4n_db_models::models::{ai::generated_image, media::image_publication};
use a4n_domain::media::ImagePublicationRecoveryRecord;
use a4n_domain::media::{IMAGE_PUBLICATION_STATUS_PENDING, IMAGE_PUBLICATION_STATUS_PUBLISHED};
use a4n_domain::project::required_text;
use a4n_domain::{
    EXTERNAL_IMPORT_MODEL,
    image::{IMAGE_TYPE_OBJECT_MATERIAL, IMAGE_TYPE_SCENE_MATERIAL},
};
use a4n_error::AppError;
use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{ColumnTrait, Condition, EntityTrait, QueryFilter, QuerySelect};
use serde_json::{Map, Value, json};
use tokio::sync::OwnedMutexGuard;

use crate::{
    OrchestratorContext,
    collaboration::ProjectAccessRole,
    video::{
        image_type_rank, is_video_reference_image_candidate,
        resolve_video_reference_image_project_ids, video_reference_scope_rank,
    },
};
use a4n_media::{DecodedUploadedImage, decode_uploaded_image_data};

const MAX_IMPORT_IMAGE_BYTES: usize = 50 * 1024 * 1024;

fn image_publication_recovery_record(
    publication: image_publication::Model,
) -> ImagePublicationRecoveryRecord {
    ImagePublicationRecoveryRecord {
        publication_id: publication.publication_id,
        project_id: publication.project_id,
        requested_by_user_id: publication.requested_by_user_id,
        staging_path: publication.staging_path,
        final_path: publication.final_path,
        image_type: publication.image_type,
        expected_file_size: publication.expected_file_size,
        expected_sha256: publication.expected_sha256,
        file_size: publication.file_size,
        status: publication.status,
        version: publication.version,
        image_id: publication.image_id,
    }
}

/// Host-facing image import boundary. File decoding, publication intent
/// creation and final publication stay in the media owner.
#[async_trait]
pub trait ImageImportApplicationPort: Send + Sync {
    async fn prepare_imported_upload(
        &self,
        user_id: i64,
        input: UploadImageInput,
    ) -> Result<PreparedImportedImageUpload, AppError>;

    async fn create_imported_publication_intent(
        &self,
        user_id: i64,
        prepared: &PreparedImportedImageUpload,
        publication_id: String,
        staging_path: String,
        final_path: String,
        expected_sha256: String,
    ) -> Result<(ImagePublicationRecoveryRecord, OwnedMutexGuard<()>), AppError>;

    async fn publish_imported_image(
        &self,
        user_id: i64,
        project_id: i64,
        publication_id: &str,
        file_size: i64,
        sha256: String,
    ) -> Result<GeneratedImageSummary, AppError>;
}

/// Host-facing publication recovery boundary. Runtime supplies file facts and
/// physical cleanup; the media owner keeps publication state transitions.
#[async_trait]
pub trait ImagePublicationRecoveryPort: Send + Sync {
    async fn lock_image_publications(&self) -> OwnedMutexGuard<()>;

    async fn publish_recovered_image(
        &self,
        user_id: i64,
        project_id: i64,
        publication_id: &str,
        file_size: i64,
        sha256: String,
    ) -> Result<(), AppError>;

    async fn ensure_image_publication_identity(
        &self,
        user_id: i64,
        publication_id: &str,
    ) -> Result<(), AppError>;

    async fn mark_image_publication_reclaiming(
        &self,
        publication_id: &str,
        version: i64,
        reason: String,
    ) -> Result<ImagePublicationRecoveryRecord, AppError>;

    async fn mark_image_publication_reclaimed(
        &self,
        publication_id: &str,
        version: i64,
        reason: Option<String>,
    ) -> Result<(), AppError>;
}

pub struct PreparedImportedImageUpload {
    pub project_id: i64,
    pub image_type: String,
    pub prompt: String,
    pub model: String,
    pub chapter_id: Option<i64>,
    pub panel_id: Option<String>,
    pub metadata_json: Option<Map<String, Value>>,
    pub decoded: DecodedUploadedImage,
}

impl OrchestratorContext {
    pub async fn protected_image_publication_cleanup_paths(
        &self,
        project_id: i64,
        paths: &[String],
    ) -> Result<HashSet<String>, AppError> {
        if project_id <= 0 {
            return Err(AppError::InvalidArgument(
                "图片 cleanup project_id 必须大于 0".to_string(),
            ));
        }
        if paths.is_empty() {
            return Ok(HashSet::new());
        }
        let publications = image_publication::Entity::find()
            .filter(image_publication::Column::ProjectId.eq(project_id))
            .filter(image_publication::Column::Status.is_in([
                IMAGE_PUBLICATION_STATUS_PENDING,
                IMAGE_PUBLICATION_STATUS_PUBLISHED,
            ]))
            .filter(
                Condition::any()
                    .add(image_publication::Column::StagingPath.is_in(paths.to_vec()))
                    .add(image_publication::Column::FinalPath.is_in(paths.to_vec())),
            )
            .all(self.database.connection())
            .await
            .map_err(AppError::Database)?;
        let mut protected = HashSet::new();
        for publication in publications {
            if publication.status == IMAGE_PUBLICATION_STATUS_PENDING {
                if paths.contains(&publication.staging_path) {
                    protected.insert(publication.staging_path);
                }
                if paths.contains(&publication.final_path) {
                    protected.insert(publication.final_path);
                }
            } else if paths.contains(&publication.final_path) {
                protected.insert(publication.final_path);
            }
        }
        Ok(protected)
    }

    pub async fn project_has_active_image_publication(
        &self,
        project_id: i64,
    ) -> Result<bool, AppError> {
        if project_id <= 0 {
            return Err(AppError::InvalidArgument(
                "图片 cleanup project_id 必须大于 0".to_string(),
            ));
        }
        Ok(image_publication::Entity::find()
            .select_only()
            .column(image_publication::Column::PublicationId)
            .filter(image_publication::Column::ProjectId.eq(project_id))
            .filter(image_publication::Column::Status.is_in([
                IMAGE_PUBLICATION_STATUS_PENDING,
                IMAGE_PUBLICATION_STATUS_PUBLISHED,
            ]))
            .into_tuple::<String>()
            .one(self.database.connection())
            .await
            .map_err(AppError::Database)?
            .is_some())
    }

    pub async fn create_image_publication_intent_for_user_validated(
        &self,
        user_id: i64,
        input: CreateImagePublicationIntentInput,
    ) -> Result<(image_publication::Model, OwnedMutexGuard<()>), AppError> {
        self.verify_project_permission(input.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        self.create_image_publication_intent(input).await
    }

    pub async fn publish_image_publication_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        publication_id: &str,
        facts: ImagePublicationFileFacts,
        generation_run: Option<a4n_db::generation::RecordCompletedGenerationRunWithCheckpointInput>,
    ) -> Result<generated_image::Model, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        match generation_run {
            Some(generation_run) => {
                self.publish_image_publication_with_generation_run_checkpoint(
                    publication_id,
                    facts,
                    generation_run,
                )
                .await
            }
            None => self.publish_image_publication(publication_id, facts).await,
        }
    }

    pub async fn list_generated_images_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<generated_image::Model>, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Viewer)
            .await?;
        self.generated_images_for_project(project_id).await
    }

    pub async fn list_generated_images_for_projects_for_user_validated(
        &self,
        user_id: i64,
        project_ids: Vec<i64>,
    ) -> Result<Vec<generated_image::Model>, AppError> {
        let mut normalized_project_ids = Vec::with_capacity(project_ids.len());
        let mut seen = HashSet::with_capacity(project_ids.len());
        for project_id in project_ids {
            if seen.insert(project_id) {
                normalized_project_ids.push(project_id);
            }
        }
        if normalized_project_ids.is_empty() {
            return Err(AppError::InvalidArgument(
                "project_ids 不能为空".to_string(),
            ));
        }
        self.verify_project_permissions(
            &normalized_project_ids,
            user_id,
            ProjectAccessRole::Viewer,
        )
        .await?;
        let mut rows = self
            .generated_images_for_projects(&normalized_project_ids)
            .await?;
        sort_generated_images_by_project_order(&mut rows, &normalized_project_ids);
        Ok(rows)
    }

    pub async fn list_chapter_images_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<generated_image::Model>, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Viewer)
            .await?;
        self.generated_images_for_valid_chapter(project_id, chapter_id)
            .await
    }

    pub async fn list_video_reference_images_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
        requested_project_ids: Vec<i64>,
    ) -> Result<Vec<generated_image::Model>, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let allowed_project_ids = self.video_asset_project_ids(project_id).await?;
        let project_ids =
            resolve_video_reference_image_project_ids(requested_project_ids, &allowed_project_ids)
                .map_err(AppError::InvalidArgument)?;
        self.verify_project_permissions(&project_ids, user_id, ProjectAccessRole::Editor)
            .await?;
        let (primary_image_ids_result, candidate_images_result) = futures::join!(
            self.primary_material_image_ids_for_projects(&project_ids),
            self.video_reference_candidate_images(&project_ids, chapter_id),
        );
        let primary_image_ids = primary_image_ids_result?
            .into_iter()
            .collect::<HashSet<_>>();
        let mut by_id = HashMap::new();
        for image in candidate_images_result? {
            if is_video_reference_image_candidate(&image, chapter_id) {
                by_id.insert(image.id, image);
            }
        }
        let mut images = by_id.into_values().collect::<Vec<_>>();
        sort_video_reference_images(&mut images, chapter_id, &primary_image_ids);
        Ok(images)
    }

    pub async fn prepare_imported_image_upload_for_user_validated(
        &self,
        user_id: i64,
        input: UploadImageInput,
    ) -> Result<PreparedImportedImageUpload, AppError> {
        let project_id = input.project_id;
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let image_type = required_text(&input.image_type, "图片类型")
            .map(str::to_string)
            .map_err(AppError::InvalidArgument)?;
        let prompt = required_text(&input.prompt, "图片提示词")
            .map(str::to_string)
            .map_err(AppError::InvalidArgument)?;
        let decoded = decode_uploaded_image_data(
            &input.file_name,
            &input.mime_type,
            &input.data_base64,
            MAX_IMPORT_IMAGE_BYTES,
        )
        .map_err(AppError::InvalidArgument)?;
        Ok(PreparedImportedImageUpload {
            project_id,
            image_type,
            prompt,
            model: EXTERNAL_IMPORT_MODEL.to_string(),
            chapter_id: input.chapter_id,
            panel_id: input.panel_id,
            metadata_json: input.metadata_json,
            decoded,
        })
    }

    pub async fn create_imported_image_record_for_user_validated(
        &self,
        user_id: i64,
        prepared: PreparedImportedImageUpload,
        file_path: String,
        file_size: i64,
    ) -> Result<generated_image::Model, AppError> {
        self.verify_project_permission(prepared.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let PreparedImportedImageUpload {
            project_id,
            image_type,
            prompt,
            model,
            chapter_id,
            panel_id,
            metadata_json,
            decoded,
        } = prepared;
        let metadata_json = imported_image_metadata(
            metadata_json,
            decoded.source_file_name,
            decoded.extension,
            decoded.mime_type,
        );
        self.create_generated_image_record(CreateGeneratedImageRecordDraftInput {
            project_id,
            image_type,
            file_path,
            prompt,
            model,
            width: decoded.width,
            height: decoded.height,
            file_size: Some(file_size),
            chapter_id,
            panel_id,
            metadata_json,
            created_at: None,
        })
        .await
    }

    pub async fn create_imported_image_publication_intent_for_user_validated(
        &self,
        user_id: i64,
        prepared: &PreparedImportedImageUpload,
        publication_id: String,
        staging_path: String,
        final_path: String,
        expected_sha256: String,
    ) -> Result<(image_publication::Model, OwnedMutexGuard<()>), AppError> {
        self.verify_project_permission(prepared.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let metadata_json = imported_image_metadata(
            prepared.metadata_json.clone(),
            prepared.decoded.source_file_name.clone(),
            prepared.decoded.extension,
            prepared.decoded.mime_type,
        );
        let expected_file_size = i64::try_from(prepared.decoded.bytes.len())
            .map_err(|_| AppError::InvalidArgument("导入图片过大，无法记录文件大小".to_string()))?;
        self.create_image_publication_intent_for_user_validated(
            user_id,
            CreateImagePublicationIntentInput {
                publication_id,
                project_id: prepared.project_id,
                requested_by_user_id: user_id,
                staging_path,
                final_path: final_path.clone(),
                image: CreateGeneratedImageRecordDraftInput {
                    project_id: prepared.project_id,
                    image_type: prepared.image_type.clone(),
                    file_path: final_path,
                    prompt: prepared.prompt.clone(),
                    model: prepared.model.clone(),
                    width: prepared.decoded.width,
                    height: prepared.decoded.height,
                    file_size: Some(expected_file_size),
                    chapter_id: prepared.chapter_id,
                    panel_id: prepared.panel_id.clone(),
                    metadata_json,
                    created_at: None,
                },
                expected_file_size,
                expected_sha256,
            },
        )
        .await
    }

    pub async fn publish_imported_image_publication_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        publication_id: &str,
        facts: ImagePublicationFileFacts,
    ) -> Result<generated_image::Model, AppError> {
        self.publish_image_publication_for_user_validated(
            user_id,
            project_id,
            publication_id,
            facts,
            None,
        )
        .await
    }

    pub async fn update_generated_image_for_user_validated(
        &self,
        user_id: i64,
        image_id: i64,
        input: UpdateGeneratedImageRecordInput,
    ) -> Result<generated_image::Model, AppError> {
        self.generated_image_for_user_validated(user_id, image_id)
            .await?;
        self.update_generated_image_record(image_id, input).await
    }

    pub async fn delete_generated_image_for_user_validated(
        &self,
        user_id: i64,
        image_id: i64,
    ) -> Result<(), AppError> {
        let image = self
            .generated_image_for_user_validated(user_id, image_id)
            .await?;
        self.delete_generated_image_validated(image).await
    }

    pub async fn delete_generated_image_validated(
        &self,
        image: generated_image::Model,
    ) -> Result<(), AppError> {
        if image.id <= 0 {
            return Err(AppError::InvalidArgument(
                "图片记录 ID 必须是正整数".to_string(),
            ));
        }
        self.delete_generated_image_record_with_cleanup(image.id)
            .await
    }

    async fn generated_image_for_user_validated(
        &self,
        user_id: i64,
        image_id: i64,
    ) -> Result<generated_image::Model, AppError> {
        let image = self
            .generated_image_by_id(image_id)
            .await?
            .ok_or_else(|| AppError::NotFound("图片不存在".to_string()))?;
        self.verify_project_permission(image.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        Ok(image)
    }
}

#[async_trait]
impl ImageRecordReadApplicationPort for OrchestratorContext {
    async fn by_id(
        &self,
        user_id: i64,
        image_id: i64,
    ) -> Result<Option<GeneratedImageSummary>, AppError> {
        let Some(image) = self.generated_image_by_id(image_id).await? else {
            return Ok(None);
        };
        self.verify_project_permission(image.project_id, user_id, ProjectAccessRole::Viewer)
            .await?;
        Ok(Some(super::image_generation::generated_image_summary(
            &image,
        )))
    }

    async fn reference_by_ids(
        &self,
        user_id: i64,
        project_id: i64,
        image_ids: &[i64],
    ) -> Result<Vec<GeneratedImageSummary>, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Viewer)
            .await?;
        Ok(self
            .portrait_reference_images_by_ids(project_id, image_ids)
            .await?
            .iter()
            .map(super::image_generation::generated_image_summary)
            .collect())
    }

    async fn list_by_project(
        &self,
        user_id: i64,
        input: ListImagesByProjectInput,
    ) -> Result<Vec<GeneratedImageSummary>, AppError> {
        Ok(self
            .list_generated_images_for_user_validated(user_id, input.project_id)
            .await?
            .iter()
            .map(super::image_generation::generated_image_summary)
            .collect())
    }

    async fn list_by_projects(
        &self,
        user_id: i64,
        input: ListImagesByProjectsInput,
    ) -> Result<Vec<GeneratedImageSummary>, AppError> {
        Ok(self
            .list_generated_images_for_projects_for_user_validated(user_id, input.project_ids)
            .await?
            .iter()
            .map(super::image_generation::generated_image_summary)
            .collect())
    }

    async fn list_by_chapter(
        &self,
        user_id: i64,
        input: ListImagesByChapterInput,
    ) -> Result<Vec<GeneratedImageSummary>, AppError> {
        Ok(self
            .list_chapter_images_for_user_validated(user_id, input.project_id, input.chapter_id)
            .await?
            .iter()
            .map(super::image_generation::generated_image_summary)
            .collect())
    }

    async fn list_video_reference_images(
        &self,
        user_id: i64,
        input: ListVideoReferenceImagesInput,
    ) -> Result<Vec<GeneratedImageSummary>, AppError> {
        Ok(self
            .list_video_reference_images_for_user_validated(
                user_id,
                input.project_id,
                input.chapter_id,
                input.image_project_ids,
            )
            .await?
            .iter()
            .map(super::image_generation::generated_image_summary)
            .collect())
    }
}

#[async_trait]
impl ImageRecordMutationApplicationPort for OrchestratorContext {
    async fn update(
        &self,
        user_id: i64,
        input: UpdateGeneratedImageInput,
    ) -> Result<GeneratedImageSummary, AppError> {
        if input.is_empty() {
            return Err(AppError::InvalidArgument(
                "图片更新至少需要一个字段".to_string(),
            ));
        }
        let image_id = input.image_id;
        let clear_chapter_id = input.chapter_id.as_ref().is_some_and(Option::is_none);
        let clear_panel_id = input.panel_id.as_ref().is_some_and(Option::is_none);
        let clear_ai_score = input.ai_score.as_ref().is_some_and(Option::is_none);
        let image = self
            .update_generated_image_for_user_validated(
                user_id,
                image_id,
                UpdateGeneratedImageRecordInput {
                    prompt: input.prompt,
                    image_type: input.image_type,
                    chapter_id: input.chapter_id.flatten(),
                    clear_chapter_id,
                    panel_id: input.panel_id.flatten(),
                    clear_panel_id,
                    ai_score: input.ai_score.flatten(),
                    clear_ai_score,
                    is_favorite: input.is_favorite,
                },
            )
            .await?;
        Ok(super::image_generation::generated_image_summary(&image))
    }

    async fn delete(&self, user_id: i64, input: DeleteImageInput) -> Result<(), AppError> {
        self.delete_generated_image_for_user_validated(user_id, input.image_id)
            .await
    }
}

#[async_trait]
impl ImageImportApplicationPort for OrchestratorContext {
    async fn prepare_imported_upload(
        &self,
        user_id: i64,
        input: UploadImageInput,
    ) -> Result<PreparedImportedImageUpload, AppError> {
        self.prepare_imported_image_upload_for_user_validated(user_id, input)
            .await
    }

    async fn create_imported_publication_intent(
        &self,
        user_id: i64,
        prepared: &PreparedImportedImageUpload,
        publication_id: String,
        staging_path: String,
        final_path: String,
        expected_sha256: String,
    ) -> Result<(ImagePublicationRecoveryRecord, OwnedMutexGuard<()>), AppError> {
        let (publication, guard) = self
            .create_imported_image_publication_intent_for_user_validated(
                user_id,
                prepared,
                publication_id,
                staging_path,
                final_path,
                expected_sha256,
            )
            .await?;
        Ok((image_publication_recovery_record(publication), guard))
    }

    async fn publish_imported_image(
        &self,
        user_id: i64,
        project_id: i64,
        publication_id: &str,
        file_size: i64,
        sha256: String,
    ) -> Result<GeneratedImageSummary, AppError> {
        let image = self
            .publish_imported_image_publication_for_user_validated(
                user_id,
                project_id,
                publication_id,
                ImagePublicationFileFacts { file_size, sha256 },
            )
            .await?;
        Ok(super::image_generation::generated_image_summary(&image))
    }
}

#[async_trait]
impl ImagePublicationRecoveryReadPort for OrchestratorContext {
    async fn image_publications_for_recovery(
        &self,
        limit: usize,
        published_after: Option<&str>,
    ) -> Result<Vec<ImagePublicationRecoveryRecord>, AppError> {
        Ok(
            OrchestratorContext::image_publications_for_recovery(self, limit, published_after)
                .await?
                .into_iter()
                .map(image_publication_recovery_record)
                .collect(),
        )
    }

    async fn image_publication_by_id(
        &self,
        publication_id: &str,
    ) -> Result<Option<ImagePublicationRecoveryRecord>, AppError> {
        Ok(
            OrchestratorContext::image_publication_by_id(self, publication_id)
                .await?
                .map(image_publication_recovery_record),
        )
    }

    async fn generated_image_identity(
        &self,
        image_id: i64,
    ) -> Result<Option<GeneratedImageIdentity>, AppError> {
        Ok(self
            .generated_image_by_id(image_id)
            .await?
            .map(|image| GeneratedImageIdentity {
                project_id: image.project_id,
                file_path: image.file_path,
                image_type: image.image_type,
                file_size: image.file_size,
            }))
    }
}

#[async_trait]
impl ImagePublicationRecoveryPort for OrchestratorContext {
    async fn lock_image_publications(&self) -> OwnedMutexGuard<()> {
        OrchestratorContext::lock_image_publications(self).await
    }

    async fn publish_recovered_image(
        &self,
        user_id: i64,
        project_id: i64,
        publication_id: &str,
        file_size: i64,
        sha256: String,
    ) -> Result<(), AppError> {
        self.publish_imported_image_publication_for_user_validated(
            user_id,
            project_id,
            publication_id,
            ImagePublicationFileFacts { file_size, sha256 },
        )
        .await
        .map(|_| ())
    }

    async fn ensure_image_publication_identity(
        &self,
        user_id: i64,
        publication_id: &str,
    ) -> Result<(), AppError> {
        let publication = OrchestratorContext::image_publication_by_id(self, publication_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("图片发布意图不存在: {publication_id}")))?;
        match publication.image_type.as_str() {
            "portrait" => self
                .ensure_portrait_publication_identity_for_user_validated(user_id, &publication)
                .await
                .map(|_| ()),
            IMAGE_TYPE_SCENE_MATERIAL | IMAGE_TYPE_OBJECT_MATERIAL => {
                self.ensure_profile_material_publication_identity_for_user_validated(
                    user_id,
                    &publication,
                )
                .await
            }
            _ => Ok(()),
        }
    }

    async fn mark_image_publication_reclaiming(
        &self,
        publication_id: &str,
        version: i64,
        reason: String,
    ) -> Result<ImagePublicationRecoveryRecord, AppError> {
        OrchestratorContext::mark_image_publication_reclaiming(
            self,
            publication_id,
            version,
            reason,
        )
        .await
        .map(image_publication_recovery_record)
    }

    async fn mark_image_publication_reclaimed(
        &self,
        publication_id: &str,
        version: i64,
        reason: Option<String>,
    ) -> Result<(), AppError> {
        OrchestratorContext::mark_image_publication_reclaimed(self, publication_id, version, reason)
            .await
            .map(|_| ())
    }
}

#[async_trait]
impl ImageMediaReferencePort for OrchestratorContext {
    async fn unreferenced_media_paths(&self, paths: &[String]) -> Result<Vec<String>, AppError> {
        OrchestratorContext::unreferenced_media_paths(self, paths).await
    }

    async fn media_path_prefix_has_persisted_reference(
        &self,
        prefix: &str,
    ) -> Result<bool, AppError> {
        OrchestratorContext::media_path_prefix_has_persisted_reference(self, prefix).await
    }

    async fn project_has_active_image_publication(
        &self,
        project_id: i64,
    ) -> Result<bool, AppError> {
        OrchestratorContext::project_has_active_image_publication(self, project_id).await
    }

    async fn protected_image_publication_cleanup_paths(
        &self,
        project_id: i64,
        paths: &[String],
    ) -> Result<HashSet<String>, AppError> {
        OrchestratorContext::protected_image_publication_cleanup_paths(self, project_id, paths)
            .await
    }
}

fn sort_generated_images_by_project_order(
    rows: &mut [generated_image::Model],
    project_ids: &[i64],
) {
    let project_order = project_ids
        .iter()
        .enumerate()
        .map(|(index, project_id)| (*project_id, index))
        .collect::<HashMap<_, _>>();
    rows.sort_by(|left, right| {
        project_order[&left.project_id]
            .cmp(&project_order[&right.project_id])
            .then_with(|| right.created_at.cmp(&left.created_at))
    });
}

#[cfg(test)]
mod project_image_sort_tests {
    use chrono::{Duration, Utc};

    use super::sort_generated_images_by_project_order;
    use a4n_db_models::models::ai::generated_image;

    fn image(
        id: i64,
        project_id: i64,
        created_at: chrono::NaiveDateTime,
    ) -> generated_image::Model {
        generated_image::Model {
            id,
            project_id,
            image_type: String::new(),
            file_path: String::new(),
            prompt: String::new(),
            model: String::new(),
            width: 1,
            height: 1,
            file_size: None,
            ai_score: None,
            is_favorite: false,
            chapter_id: None,
            panel_id: None,
            metadata_json: None,
            created_at,
        }
    }

    #[test]
    fn project_image_sort_keeps_source_priority_then_newest() {
        let now = Utc::now().naive_utc();
        let mut rows = vec![
            image(3, 2, now),
            image(1, 1, now - Duration::seconds(2)),
            image(2, 1, now),
        ];

        sort_generated_images_by_project_order(&mut rows, &[1, 2]);

        assert_eq!(
            rows.iter().map(|row| row.id).collect::<Vec<_>>(),
            vec![2, 1, 3]
        );
    }
}

fn sort_video_reference_images(
    images: &mut [generated_image::Model],
    chapter_id: i64,
    primary_image_ids: &HashSet<i64>,
) {
    images.sort_by(|a, b| {
        video_reference_scope_rank(a, chapter_id)
            .cmp(&video_reference_scope_rank(b, chapter_id))
            .then_with(|| {
                primary_image_ids
                    .contains(&b.id)
                    .cmp(&primary_image_ids.contains(&a.id))
            })
            .then_with(|| b.is_favorite.cmp(&a.is_favorite))
            .then_with(|| image_type_rank(&a.image_type).cmp(&image_type_rank(&b.image_type)))
            .then_with(|| b.created_at.cmp(&a.created_at))
            .then_with(|| b.id.cmp(&a.id))
    });
}

fn imported_image_metadata(
    metadata_json: Option<Map<String, Value>>,
    source_file_name: String,
    source_extension: &'static str,
    mime_type: &'static str,
) -> Option<Value> {
    let mut metadata = metadata_json.unwrap_or_default();
    metadata.insert(
        "import".to_string(),
        json!({
            "external": true,
            "source_file_name": source_file_name,
            "source_extension": source_extension,
            "mime_type": mime_type,
            "imported_at": Utc::now().to_rfc3339(),
        }),
    );
    super::image_generation::normalize_image_metadata(Some(Value::Object(metadata)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_domain::image::IMAGE_TYPE_SCENE_MATERIAL;

    #[test]
    fn video_reference_sort_prefers_primary_before_favorite() {
        let created_at = chrono::NaiveDate::from_ymd_opt(2026, 7, 24)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        let image = |id, favorite| generated_image::Model {
            id,
            project_id: 1,
            image_type: IMAGE_TYPE_SCENE_MATERIAL.to_string(),
            file_path: format!("images/{id}.png"),
            prompt: "scene".to_string(),
            model: "test".to_string(),
            width: 1,
            height: 1,
            file_size: Some(1),
            ai_score: None,
            is_favorite: favorite,
            chapter_id: Some(2),
            panel_id: None,
            metadata_json: None,
            created_at,
        };
        let mut images = vec![image(1, true), image(2, false)];
        sort_video_reference_images(&mut images, 2, &HashSet::from([2]));
        assert_eq!(
            images.iter().map(|image| image.id).collect::<Vec<_>>(),
            vec![2, 1]
        );
    }
}
