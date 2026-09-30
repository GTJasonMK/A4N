use super::*;

impl WorkerState {
    pub(super) async fn unreferenced_paths(
        &self,
        request: MediaPathsRequest,
    ) -> Result<Vec<String>, String> {
        request.validate().map_err(|error| error.to_string())?;
        a4n_db::media::filter_unreferenced_media_paths(self.db.connection(), &request.paths)
            .await
            .map_err(|error| error.to_string())
    }

    pub(super) async fn path_prefix_referenced(
        &self,
        request: MediaPathPrefixRequest,
    ) -> Result<bool, String> {
        request.validate().map_err(|error| error.to_string())?;
        a4n_db::media::media_path_prefix_has_persisted_reference(
            self.db.connection(),
            &request.prefix,
        )
        .await
        .map_err(|error| error.to_string())
    }

    pub(super) async fn project_has_publication(
        &self,
        request: ProjectPublicationRequest,
    ) -> Result<bool, String> {
        request.validate().map_err(|error| error.to_string())?;
        a4n_db::media::project_has_active_image_publication(
            self.db.connection(),
            request.project_id,
        )
        .await
        .map_err(|error| error.to_string())
    }

    pub(super) async fn protected_paths(
        &self,
        request: ProjectPathsRequest,
    ) -> Result<Vec<String>, String> {
        request.validate().map_err(|error| error.to_string())?;
        let paths = a4n_db::media::protected_image_publication_cleanup_paths(
            self.db.connection(),
            request.project_id,
            &request.paths,
        )
        .await
        .map_err(|error| error.to_string())?;
        Ok(paths.into_iter().collect())
    }
}
