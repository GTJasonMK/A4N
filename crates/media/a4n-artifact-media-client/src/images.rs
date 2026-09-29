use super::{ArtifactMediaClientError, ArtifactMediaWorkerClient, app_error};
use a4n_api_contract::image::{
    DeleteImageInput, GeneratedImageSummary, ImageLibraryReadApplicationPort,
    ImageRecordMutationApplicationPort, ListImagesByChapterInput, ListImagesByProjectInput,
    ListImagesByProjectsInput, UpdateGeneratedImageInput,
};
use a4n_artifact_media_contract::{
    ARTIFACT_MEDIA_IMAGE_BY_ID_COMMAND, ARTIFACT_MEDIA_IMAGE_DELETE_COMMAND,
    ARTIFACT_MEDIA_IMAGE_LIST_CHAPTER_COMMAND, ARTIFACT_MEDIA_IMAGE_LIST_PROJECT_COMMAND,
    ARTIFACT_MEDIA_IMAGE_LIST_PROJECTS_COMMAND, ARTIFACT_MEDIA_IMAGE_UPDATE_COMMAND,
};

impl ArtifactMediaWorkerClient {
    pub async fn image_by_id(
        &self,
        actor_user_id: i64,
        image_id: i64,
    ) -> Result<Option<GeneratedImageSummary>, ArtifactMediaClientError> {
        self.completed_payload(
            ARTIFACT_MEDIA_IMAGE_BY_ID_COMMAND,
            self.user_envelope(
                "artifact-media.image-by-id",
                actor_user_id,
                serde_json::json!({ "image_id": image_id }),
            ),
        )
        .await
    }

    pub async fn image_list_by_project(
        &self,
        actor_user_id: i64,
        input: ListImagesByProjectInput,
    ) -> Result<Vec<GeneratedImageSummary>, ArtifactMediaClientError> {
        self.completed_payload(
            ARTIFACT_MEDIA_IMAGE_LIST_PROJECT_COMMAND,
            self.user_envelope(
                "artifact-media.image-list-project",
                actor_user_id,
                serde_json::to_value(input).expect("image list input serializes"),
            ),
        )
        .await
    }

    pub async fn image_list_by_projects(
        &self,
        actor_user_id: i64,
        input: ListImagesByProjectsInput,
    ) -> Result<Vec<GeneratedImageSummary>, ArtifactMediaClientError> {
        self.completed_payload(
            ARTIFACT_MEDIA_IMAGE_LIST_PROJECTS_COMMAND,
            self.user_envelope(
                "artifact-media.image-list-projects",
                actor_user_id,
                serde_json::to_value(input).expect("image list input serializes"),
            ),
        )
        .await
    }

    pub async fn image_list_by_chapter(
        &self,
        actor_user_id: i64,
        input: ListImagesByChapterInput,
    ) -> Result<Vec<GeneratedImageSummary>, ArtifactMediaClientError> {
        self.completed_payload(
            ARTIFACT_MEDIA_IMAGE_LIST_CHAPTER_COMMAND,
            self.user_envelope(
                "artifact-media.image-list-chapter",
                actor_user_id,
                serde_json::to_value(input).expect("image list input serializes"),
            ),
        )
        .await
    }

    pub async fn update_image(
        &self,
        actor_user_id: i64,
        input: UpdateGeneratedImageInput,
    ) -> Result<GeneratedImageSummary, ArtifactMediaClientError> {
        if actor_user_id <= 0 {
            return Err(ArtifactMediaClientError::InvalidRequest(
                "actor_user_id must be positive".to_string(),
            ));
        }
        self.completed_payload(
            ARTIFACT_MEDIA_IMAGE_UPDATE_COMMAND,
            self.user_envelope(
                "artifact-media.image-update",
                actor_user_id,
                serde_json::to_value(input).expect("image update input serializes"),
            ),
        )
        .await
    }

    pub async fn delete_image(
        &self,
        actor_user_id: i64,
        input: DeleteImageInput,
    ) -> Result<(), ArtifactMediaClientError> {
        if actor_user_id <= 0 {
            return Err(ArtifactMediaClientError::InvalidRequest(
                "actor_user_id must be positive".to_string(),
            ));
        }
        let _: serde_json::Value = self
            .completed_payload(
                ARTIFACT_MEDIA_IMAGE_DELETE_COMMAND,
                self.user_envelope(
                    "artifact-media.image-delete",
                    actor_user_id,
                    serde_json::to_value(input).expect("image delete input serializes"),
                ),
            )
            .await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl ImageLibraryReadApplicationPort for ArtifactMediaWorkerClient {
    async fn by_id(
        &self,
        user_id: i64,
        image_id: i64,
    ) -> Result<Option<GeneratedImageSummary>, a4n_error::AppError> {
        if user_id <= 0 || image_id <= 0 {
            return Err(a4n_error::AppError::InvalidArgument(
                "user_id 和 image_id 必须大于 0".into(),
            ));
        }
        self.image_by_id(user_id, image_id).await.map_err(app_error)
    }

    async fn list_by_project(
        &self,
        user_id: i64,
        input: ListImagesByProjectInput,
    ) -> Result<Vec<GeneratedImageSummary>, a4n_error::AppError> {
        if user_id <= 0 {
            return Err(a4n_error::AppError::InvalidArgument(
                "user_id 必须大于 0".into(),
            ));
        }
        self.image_list_by_project(user_id, input)
            .await
            .map_err(app_error)
    }

    async fn list_by_projects(
        &self,
        user_id: i64,
        input: ListImagesByProjectsInput,
    ) -> Result<Vec<GeneratedImageSummary>, a4n_error::AppError> {
        if user_id <= 0 {
            return Err(a4n_error::AppError::InvalidArgument(
                "user_id 必须大于 0".into(),
            ));
        }
        self.image_list_by_projects(user_id, input)
            .await
            .map_err(app_error)
    }

    async fn list_by_chapter(
        &self,
        user_id: i64,
        input: ListImagesByChapterInput,
    ) -> Result<Vec<GeneratedImageSummary>, a4n_error::AppError> {
        if user_id <= 0 {
            return Err(a4n_error::AppError::InvalidArgument(
                "user_id 必须大于 0".into(),
            ));
        }
        self.image_list_by_chapter(user_id, input)
            .await
            .map_err(app_error)
    }
}

#[async_trait::async_trait]
impl ImageRecordMutationApplicationPort for ArtifactMediaWorkerClient {
    async fn update(
        &self,
        user_id: i64,
        input: UpdateGeneratedImageInput,
    ) -> Result<GeneratedImageSummary, a4n_error::AppError> {
        self.update_image(user_id, input).await.map_err(app_error)
    }

    async fn delete(
        &self,
        user_id: i64,
        input: DeleteImageInput,
    ) -> Result<(), a4n_error::AppError> {
        self.delete_image(user_id, input).await.map_err(app_error)
    }
}
