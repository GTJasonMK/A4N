use super::{ArtifactMediaWorkerClient, app_error};
use a4n_api_contract::image::{
    DeleteImageInput, GeneratedImageSummary, ImageLibraryReadApplicationPort,
    ImageRecordMutationApplicationPort, ListImagesByChapterInput, ListImagesByProjectInput,
    ListImagesByProjectsInput, UpdateGeneratedImageInput,
};

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
