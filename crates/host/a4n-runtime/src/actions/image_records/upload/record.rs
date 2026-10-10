use super::file::create_imported_image_output;
use crate::{
    AppState,
    cleanup_outbox::enqueue_file_artifact_cleanup_by_project,
    file_utils::{inspect_data_file_facts, promote_data_file},
};
use a4n_api_contract::image::GeneratedImageSummary;
use a4n_artifact_media_contract::PublishImageRequest;
use a4n_domain::media::{IMAGE_PUBLICATION_STATUS_PENDING, IMAGE_PUBLICATION_STATUS_PUBLISHED};
use a4n_image_host_contract::PreparedImportedImageUpload;
use a4n_storage::{paths::data_dir_relative_path, write_new_synced_data_file};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use uuid::Uuid;

pub(super) async fn persist_imported_image(
    state: &AppState,
    user_id: i64,
    prepared: PreparedImportedImageUpload,
) -> Result<GeneratedImageSummary, String> {
    let project_id = prepared.project_id;
    let chapter_id = prepared.chapter_id;
    let extension = prepared.decoded.extension;
    let bytes = &prepared.decoded.bytes;
    let file_size =
        i64::try_from(bytes.len()).map_err(|_| "导入图片过大，无法记录文件大小".to_string())?;
    let sha256 = format!("{:x}", Sha256::digest(bytes));
    let publication_id = format!("image-publication-{}", Uuid::new_v4().simple());
    let stem = format!("imported_image_{}", Uuid::new_v4().simple());
    let (staging_path, final_path) =
        create_imported_image_output(state.data_dir(), project_id, chapter_id, extension, &stem)?;
    let staging_relative_path =
        data_dir_relative_path(state.data_dir(), &staging_path, "导入图片")?;
    let final_relative_path = data_dir_relative_path(state.data_dir(), &final_path, "导入图片")?;
    let (publication, _publication_guard) = state
        .image_import_publication_application()
        .create_imported_publication_intent(
            user_id,
            &prepared,
            publication_id,
            staging_relative_path,
            final_relative_path,
            sha256.clone(),
        )
        .await
        .map_err(|error| error.to_string())?;

    if publication.status == IMAGE_PUBLICATION_STATUS_PUBLISHED {
        let image_id = publication
            .image_id
            .ok_or_else(|| "已发布图片意图缺少 image_id".to_string())?;
        return state
            .image_record_read_application()
            .by_id(user_id, image_id)
            .await
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("已发布图片不存在: {image_id}"));
    }
    if publication.status != IMAGE_PUBLICATION_STATUS_PENDING {
        return Err(format!(
            "图片发布意图当前状态不允许写入: {}",
            publication.status
        ));
    }

    let staging_facts = inspect_data_file_facts(
        state.data_dir(),
        &publication.staging_path,
        "导入图片暂存文件",
    )?;
    match staging_facts {
        Some(facts) if !facts_match(&facts, file_size, &sha256) => {
            return Err("导入图片暂存文件事实与发布意图不一致".to_string());
        }
        None => write_new_synced_data_file(
            state.data_dir(),
            &publication.staging_path,
            bytes,
            "导入图片暂存文件",
        )?,
        Some(_) => {}
    }

    let final_facts = inspect_data_file_facts(
        state.data_dir(),
        &publication.final_path,
        "导入图片最终文件",
    )?;
    match final_facts {
        Some(facts) if !facts_match(&facts, file_size, &sha256) => {
            return Err("导入图片最终文件事实与发布意图不一致".to_string());
        }
        None => promote_data_file(
            state.data_dir(),
            &publication.staging_path,
            &publication.final_path,
            "导入图片文件",
        )?,
        Some(_) => {}
    }

    let final_facts = inspect_data_file_facts(
        state.data_dir(),
        &publication.final_path,
        "导入图片最终文件",
    )?
    .ok_or_else(|| "导入图片最终文件在发布前消失".to_string())?;
    if !facts_match(&final_facts, file_size, &sha256) {
        return Err("导入图片最终文件在发布前被修改".to_string());
    }
    let record = if let Some(worker) = state.artifact_media_publication_application() {
        let published = worker
            .publish_image(
                user_id,
                PublishImageRequest {
                    project_id,
                    publication_id: publication.publication_id.clone(),
                    file_size: final_facts.1,
                    sha256: final_facts.2,
                },
            )
            .await?;
        state
            .image_record_read_application()
            .by_id(user_id, published.image_id)
            .await
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("已发布图片不存在: {}", published.image_id))?
    } else {
        state
            .image_import_publication_application()
            .publish_imported_image(
                user_id,
                project_id,
                &publication.publication_id,
                final_facts.1,
                final_facts.2,
            )
            .await
            .map_err(|error| error.to_string())?
    };
    if inspect_data_file_facts(
        state.data_dir(),
        &publication.staging_path,
        "导入图片暂存文件",
    )?
    .is_some()
        && let Err(error) = enqueue_file_artifact_cleanup_by_project(
            state,
            &format!("image-publication-staging:{}", publication.publication_id),
            std::slice::from_ref(&publication.staging_path),
        )
        .await
    {
        tracing::warn!(
            error = %error,
            publication_id = %publication.publication_id,
            "图片发布成功但暂存文件清理意图登记失败"
        );
    }
    Ok(record)
}

fn facts_match(facts: &(PathBuf, i64, String), file_size: i64, sha256: &str) -> bool {
    facts.1 == file_size && facts.2 == sha256
}
