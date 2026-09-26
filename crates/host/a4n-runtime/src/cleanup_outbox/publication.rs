use crate::{
    AppState,
    cleanup_outbox::enqueue_file_artifact_cleanup_by_project,
    file_utils::{inspect_data_file_facts, promote_data_file},
};
use a4n_domain::image::{IMAGE_TYPE_OBJECT_MATERIAL, IMAGE_TYPE_SCENE_MATERIAL};
use a4n_domain::media::{
    IMAGE_PUBLICATION_STATUS_PENDING, IMAGE_PUBLICATION_STATUS_PUBLISHED,
    IMAGE_PUBLICATION_STATUS_RECLAIMING, ImagePublicationRecoveryRecord,
};
use a4n_error::AppError;

const RECOVERY_LIMIT: usize = 64;

pub(super) async fn recover_image_publications(
    state: &AppState,
    published_after: &mut Option<String>,
    skip_imported: bool,
) {
    let publications = match state
        .image_publication_recovery_read_application()
        .image_publications_for_recovery(RECOVERY_LIMIT, published_after.as_deref())
        .await
    {
        Ok(publications) => publications,
        Err(error) => {
            tracing::warn!(error = %error, "图片发布意图恢复查询失败");
            return;
        }
    };
    for publication in publications {
        if publication.status == IMAGE_PUBLICATION_STATUS_PUBLISHED {
            *published_after = Some(publication.publication_id.clone());
        }
        if should_skip_imported_model(&publication.model, skip_imported) {
            continue;
        }
        if let Err(error) = recover_image_publication(state, publication.clone()).await {
            tracing::warn!(
                error = %error,
                publication_id = %publication.publication_id,
                "图片发布意图恢复失败，保留意图等待下一轮"
            );
        }
    }
}

async fn recover_image_publication(
    state: &AppState,
    publication: ImagePublicationRecoveryRecord,
) -> Result<(), String> {
    let _guard = state
        .image_publication_recovery_application()
        .lock_image_publications()
        .await;
    // A writer may have committed while recovery waited for the same guard.
    let Some(publication) = state
        .image_publication_recovery_read_application()
        .image_publication_by_id(&publication.publication_id)
        .await
        .map_err(|error| error.to_string())?
    else {
        return Ok(());
    };
    match publication.status.as_str() {
        IMAGE_PUBLICATION_STATUS_PENDING => recover_pending_publication(state, publication).await,
        IMAGE_PUBLICATION_STATUS_PUBLISHED => {
            recover_published_publication(state, publication).await
        }
        IMAGE_PUBLICATION_STATUS_RECLAIMING => {
            recover_reclaiming_publication(state, publication).await
        }
        _ => Ok(()),
    }
}

async fn recover_pending_publication(
    state: &AppState,
    publication: ImagePublicationRecoveryRecord,
) -> Result<(), String> {
    let staging = inspect_data_file_facts(
        state.data_dir(),
        &publication.staging_path,
        "图片发布暂存文件",
    )?;
    let final_file = inspect_data_file_facts(
        state.data_dir(),
        &publication.final_path,
        "图片发布最终文件",
    )?;

    if let Some(final_facts) = final_file.as_ref() {
        if !facts_match(final_facts, &publication) {
            return reclaim_publication(state, publication, "最终文件事实不匹配").await;
        }
        publish_recovered_file(state, &publication, final_facts).await?;
        if staging.is_some() {
            enqueue_staging_cleanup(state, &publication).await?;
        }
        return Ok(());
    }

    let Some(staging_facts) = staging.as_ref() else {
        return reclaim_publication(state, publication, "发布意图没有可用文件").await;
    };
    if !facts_match(staging_facts, &publication) {
        return reclaim_publication(state, publication, "暂存文件事实不匹配").await;
    }
    promote_data_file(
        state.data_dir(),
        &publication.staging_path,
        &publication.final_path,
        "图片发布恢复",
    )?;
    let final_facts = inspect_data_file_facts(
        state.data_dir(),
        &publication.final_path,
        "图片发布最终文件",
    )?
    .ok_or_else(|| "图片发布恢复提升后找不到最终文件".to_string())?;
    if !facts_match(&final_facts, &publication) {
        return reclaim_publication(state, publication, "提升后的最终文件事实不匹配").await;
    }
    publish_recovered_file(state, &publication, &final_facts).await
}

async fn recover_published_publication(
    state: &AppState,
    publication: ImagePublicationRecoveryRecord,
) -> Result<(), String> {
    let final_facts = inspect_data_file_facts(
        state.data_dir(),
        &publication.final_path,
        "已发布图片最终文件",
    )?;
    let final_facts = match final_facts {
        Some(final_facts) => final_facts,
        None => {
            let staging_facts = inspect_data_file_facts(
                state.data_dir(),
                &publication.staging_path,
                "已发布图片暂存文件",
            )?;
            if let Some(staging_facts) = staging_facts {
                if !facts_match(&staging_facts, &publication) {
                    return Err(format!(
                        "已发布图片最终文件缺失且暂存文件事实不匹配，保留文件等待人工恢复: publication_id={}, image_id={:?}",
                        publication.publication_id, publication.image_id
                    ));
                }
                promote_data_file(
                    state.data_dir(),
                    &publication.staging_path,
                    &publication.final_path,
                    "已发布图片恢复",
                )?;
                inspect_data_file_facts(
                    state.data_dir(),
                    &publication.final_path,
                    "已发布图片恢复后的最终文件",
                )?
                .ok_or_else(|| "已发布图片恢复提升后找不到最终文件".to_string())?
            } else {
                let image_is_missing = match publication.image_id {
                    Some(image_id) => state
                        .image_publication_recovery_read_application()
                        .generated_image_identity(image_id)
                        .await
                        .map_err(|error| error.to_string())?
                        .is_none(),
                    None => true,
                };
                if image_is_missing {
                    return reclaim_publication(
                        state,
                        publication,
                        "已发布图片记录和最终文件均缺失，按历史孤儿意图回收",
                    )
                    .await;
                }
                return Err(format!(
                    "已发布图片最终文件缺失，需人工恢复或从备份还原: publication_id={}, image_id={:?}",
                    publication.publication_id, publication.image_id
                ));
            }
        }
    };
    if !facts_match(&final_facts, &publication) {
        return Err(format!(
            "已发布图片最终文件事实不匹配，保留数据库记录并显式告警: publication_id={}, image_id={:?}",
            publication.publication_id, publication.image_id
        ));
    }
    state
        .image_publication_recovery_application()
        .publish_recovered_image(
            publication.requested_by_user_id,
            publication.project_id,
            &publication.publication_id,
            final_facts.1,
            final_facts.2.clone(),
        )
        .await
        .map_err(|error| error.to_string())?;
    let publication = state
        .image_publication_recovery_read_application()
        .image_publication_by_id(&publication.publication_id)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "图片发布恢复后找不到 publication".to_string())?;
    ensure_published_generated_image_identity(state, &publication).await?;
    state
        .image_publication_recovery_application()
        .ensure_image_publication_identity(
            publication.requested_by_user_id,
            &publication.publication_id,
        )
        .await
        .map_err(|error| error.to_string())?;
    if inspect_data_file_facts(
        state.data_dir(),
        &publication.staging_path,
        "已发布图片暂存文件",
    )?
    .is_some()
    {
        enqueue_staging_cleanup(state, &publication).await?;
    }
    Ok(())
}

async fn ensure_published_generated_image_identity(
    state: &AppState,
    publication: &ImagePublicationRecoveryRecord,
) -> Result<(), String> {
    let image_id = publication.image_id.ok_or_else(|| {
        format!(
            "已发布图片意图缺少 image_id，保留文件等待人工恢复: publication_id={}",
            publication.publication_id
        )
    })?;
    let image = state
        .image_publication_recovery_read_application()
        .generated_image_identity(image_id)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| {
            format!(
                "已发布图片记录不存在，保留文件等待人工恢复: publication_id={}, image_id={image_id}",
                publication.publication_id
            )
        })?;
    if image.project_id != publication.project_id
        || image.file_path != publication.final_path
        || image.image_type != publication.image_type
        || image.file_size != publication.file_size
    {
        return Err(format!(
            "已发布图片记录身份与发布意图不一致，保留文件: publication_id={}, image_id={image_id}",
            publication.publication_id
        ));
    }
    Ok(())
}

async fn publish_recovered_file(
    state: &AppState,
    publication: &ImagePublicationRecoveryRecord,
    facts: &(std::path::PathBuf, i64, String),
) -> Result<(), String> {
    let restore_profile_identity = matches!(
        publication.image_type.as_str(),
        IMAGE_TYPE_SCENE_MATERIAL | IMAGE_TYPE_OBJECT_MATERIAL
    );
    state
        .image_publication_recovery_application()
        .ensure_image_publication_identity(
            publication.requested_by_user_id,
            &publication.publication_id,
        )
        .await
        .map_err(|error| error.to_string())?;
    match state
        .image_publication_recovery_application()
        .publish_recovered_image(
            publication.requested_by_user_id,
            publication.project_id,
            &publication.publication_id,
            facts.1,
            facts.2.clone(),
        )
        .await
    {
        Ok(_) if restore_profile_identity => state
            .image_publication_recovery_application()
            .ensure_image_publication_identity(
                publication.requested_by_user_id,
                &publication.publication_id,
            )
            .await
            .map_err(|error| error.to_string()),
        Ok(_) => Ok(()),
        Err(error) if should_reclaim_after_publish_error(&error) => {
            reclaim_publication(state, publication.clone(), error.to_string()).await
        }
        Err(error) => Err(error.to_string()),
    }
}

async fn recover_reclaiming_publication(
    state: &AppState,
    publication: ImagePublicationRecoveryRecord,
) -> Result<(), String> {
    enqueue_reclaim_cleanup(state, &publication).await?;
    let mut existing_paths = Vec::new();
    for path in [&publication.staging_path, &publication.final_path] {
        if inspect_data_file_facts(state.data_dir(), path, "图片发布回收文件")?.is_some() {
            existing_paths.push(path.clone());
        }
    }
    if existing_paths.is_empty() {
        state
            .image_publication_recovery_application()
            .mark_image_publication_reclaimed(
                &publication.publication_id,
                publication.version,
                None,
            )
            .await
            .map_err(|error| error.to_string())?;
        return Ok(());
    }
    let unreferenced = state
        .image_media_reference_application()
        .unreferenced_media_paths(&existing_paths)
        .await
        .map_err(|error| error.to_string())?;
    if unreferenced.len() < existing_paths.len() {
        state
            .image_publication_recovery_application()
            .mark_image_publication_reclaimed(
                &publication.publication_id,
                publication.version,
                Some("回收复核发现已有媒体引用，保留被引用文件".to_string()),
            )
            .await
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

async fn reclaim_publication(
    state: &AppState,
    publication: ImagePublicationRecoveryRecord,
    reason: impl Into<String>,
) -> Result<(), String> {
    let publication = state
        .image_publication_recovery_application()
        .mark_image_publication_reclaiming(
            &publication.publication_id,
            publication.version,
            reason.into(),
        )
        .await
        .map_err(|error| error.to_string())?;
    enqueue_reclaim_cleanup(state, &publication).await?;
    recover_reclaiming_publication(state, publication).await
}

async fn enqueue_reclaim_cleanup(
    state: &AppState,
    publication: &ImagePublicationRecoveryRecord,
) -> Result<(), String> {
    let paths = [
        publication.staging_path.clone(),
        publication.final_path.clone(),
    ];
    enqueue_file_artifact_cleanup_by_project(
        state,
        &format!(
            "image-publication-reclaim:{}:{}",
            publication.publication_id, publication.version
        ),
        &paths,
    )
    .await
}

async fn enqueue_staging_cleanup(
    state: &AppState,
    publication: &ImagePublicationRecoveryRecord,
) -> Result<(), String> {
    enqueue_file_artifact_cleanup_by_project(
        state,
        &format!("image-publication-staging:{}", publication.publication_id),
        std::slice::from_ref(&publication.staging_path),
    )
    .await
}

fn facts_match(
    facts: &(std::path::PathBuf, i64, String),
    publication: &ImagePublicationRecoveryRecord,
) -> bool {
    facts.1 == publication.expected_file_size && facts.2 == publication.expected_sha256
}

fn should_reclaim_after_publish_error(error: &AppError) -> bool {
    matches!(
        error,
        AppError::Forbidden(_)
            | AppError::NotFound(_)
            | AppError::InvalidArgument(_)
            | AppError::InvalidState { .. }
    )
}

fn should_skip_imported_model(model: &str, skip_imported: bool) -> bool {
    skip_imported && model == a4n_domain::EXTERNAL_IMPORT_MODEL
}

#[cfg(test)]
mod tests {
    use super::should_skip_imported_model;

    #[test]
    fn artifact_worker_only_removes_imported_recovery_from_host() {
        assert!(should_skip_imported_model(
            a4n_domain::EXTERNAL_IMPORT_MODEL,
            true
        ));
        assert!(!should_skip_imported_model("generated-image-model", true));
        assert!(!should_skip_imported_model(
            a4n_domain::EXTERNAL_IMPORT_MODEL,
            false
        ));
    }
}
