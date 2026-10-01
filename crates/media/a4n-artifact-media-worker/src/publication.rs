use super::*;
use a4n_domain::EXTERNAL_IMPORT_MODEL;
use a4n_storage::inspect_data_file_facts;

impl WorkerState {
    pub(super) async fn publish_image(
        &self,
        actor_user_id: i64,
        request: PublishImageRequest,
    ) -> Result<PublishedImageResponse, String> {
        request.validate().map_err(|error| error.to_string())?;
        a4n_db::collaboration::verify_project_permission(
            self.db.connection(),
            request.project_id,
            actor_user_id,
            a4n_db::collaboration::ProjectAccessRole::Editor,
        )
        .await
        .map_err(|error| error.to_string())?;
        let publication =
            a4n_db::video::image_publication_by_id(self.db.connection(), &request.publication_id)
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
            self.db.connection(),
            &request.publication_id,
            a4n_domain::media::ImagePublicationFileFacts {
                file_size: facts.1,
                sha256: facts.2,
            },
        )
        .await
        .map_err(|error| error.to_string())?;
        let publication =
            a4n_db::video::image_publication_by_id(self.db.connection(), &request.publication_id)
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
}
