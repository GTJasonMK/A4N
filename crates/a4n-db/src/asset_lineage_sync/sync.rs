use a4n_domain::image::IMAGE_TYPE_VIDEO_TAIL_FRAME;

use super::*;

pub async fn sync_project_asset_lineage_record(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<AssetLineageBackfillSummary, AssetLineageSyncError> {
    validate_positive_id(project_id, "project_id")?;
    let tx = db.begin().await.map_err(AssetLineageSyncError::Database)?;
    let mut summary = AssetLineageBackfillSummary::default();
    let family = load_family_lineage_context(&tx, project_id).await?;
    summary.scanned_projects = family.project_ids.len() as u64;
    sync_family_asset_lineage(&tx, &family, &mut summary).await?;
    tx.commit().await.map_err(AssetLineageSyncError::Database)?;
    Ok(summary)
}

async fn sync_family_asset_lineage<C>(
    db: &C,
    family: &FamilyLineageContext,
    summary: &mut AssetLineageBackfillSummary,
) -> Result<(), AssetLineageSyncError>
where
    C: ConnectionTrait,
{
    let manga_prompts = chapter_manga_prompt::Entity::find()
        .filter(chapter_manga_prompt::Column::ProjectId.is_in(family.project_ids.iter().copied()))
        .filter(chapter_manga_prompt::Column::PromptType.eq(MANGA_PAGE_PROMPT_TYPE))
        .order_by_asc(chapter_manga_prompt::Column::Id)
        .all(db)
        .await
        .map_err(AssetLineageSyncError::Database)?;
    summary.scanned_manga_prompts = manga_prompts.len() as u64;

    let mut manga_prompt_refs = BTreeMap::<i64, SyncedPromptRef>::new();
    let mut manga_prompt_by_image = BTreeMap::<i64, chapter_manga_prompt::Model>::new();
    for prompt in &manga_prompts {
        if let Some(prompt_ref) = sync_manga_prompt(db, family, prompt, summary).await? {
            manga_prompt_refs.insert(prompt.id, prompt_ref);
        }
        if let Some(image_id) = prompt.generated_image_id {
            manga_prompt_by_image.insert(image_id, prompt.clone());
        }
    }

    let video_prompts = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.is_in(family.project_ids.iter().copied()))
        .order_by_asc(chapter_video_prompt::Column::Id)
        .all(db)
        .await
        .map_err(AssetLineageSyncError::Database)?;
    summary.scanned_video_prompts = video_prompts.len() as u64;

    let mut video_prompt_refs = BTreeMap::<i64, SyncedPromptRef>::new();
    let mut video_prompt_by_video = BTreeMap::<i64, chapter_video_prompt::Model>::new();
    for prompt in &video_prompts {
        if let Some(prompt_ref) = sync_video_prompt(db, family, prompt, summary).await? {
            video_prompt_refs.insert(prompt.id, prompt_ref);
        }
        if let Some(video_id) = prompt.generated_video_id {
            video_prompt_by_video.insert(video_id, prompt.clone());
        }
    }

    let generated_images = generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.is_in(family.project_ids.iter().copied()))
        .order_by_asc(generated_image::Column::Id)
        .all(db)
        .await
        .map_err(AssetLineageSyncError::Database)?;
    summary.scanned_generated_images = generated_images.len() as u64;

    let mut image_media_by_legacy_id = BTreeMap::<i64, i64>::new();
    for image in &generated_images {
        if !should_sync_generated_image(image, &manga_prompt_by_image) {
            continue;
        }
        let prompt = manga_prompt_by_image.get(&image.id);
        let prompt_ref = prompt.and_then(|prompt| manga_prompt_refs.get(&prompt.id));
        if let Some(media) =
            sync_generated_image_media(db, family, image, prompt, prompt_ref, None, summary).await?
        {
            image_media_by_legacy_id.insert(image.id, media.id);
        }
    }

    let generated_videos = generated_video::Entity::find()
        .filter(generated_video::Column::ProjectId.is_in(family.project_ids.iter().copied()))
        .order_by_asc(generated_video::Column::Id)
        .all(db)
        .await
        .map_err(AssetLineageSyncError::Database)?;
    summary.scanned_generated_videos = generated_videos.len() as u64;

    let mut video_media_by_legacy_id = BTreeMap::<i64, i64>::new();
    for video in &generated_videos {
        let prompt = video_prompt_by_video
            .get(&video.id)
            .or_else(|| prompt_from_video_metadata(video, &video_prompts));
        let prompt_ref = prompt.and_then(|prompt| video_prompt_refs.get(&prompt.id));
        let source_media_asset_id = video
            .source_image_id
            .and_then(|id| image_media_by_legacy_id.get(&id).copied())
            .or_else(|| {
                video
                    .source_video_id
                    .and_then(|id| video_media_by_legacy_id.get(&id).copied())
            });
        let media = sync_generated_video_media(
            db,
            family,
            video,
            prompt,
            prompt_ref,
            source_media_asset_id,
            summary,
        )
        .await?;
        if let Some(media) = media {
            video_media_by_legacy_id.insert(video.id, media.id);
        }
    }

    for image in &generated_images {
        if image.image_type != IMAGE_TYPE_VIDEO_TAIL_FRAME {
            continue;
        }
        let Some(source_video_id) = image
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("video_id"))
            .and_then(Value::as_i64)
        else {
            continue;
        };
        let Some(source_media_asset_id) = video_media_by_legacy_id.get(&source_video_id).copied()
        else {
            continue;
        };
        let prompt = manga_prompt_by_image.get(&image.id);
        let prompt_ref = prompt.and_then(|prompt| manga_prompt_refs.get(&prompt.id));
        let media = sync_generated_image_media(
            db,
            family,
            image,
            prompt,
            prompt_ref,
            Some(source_media_asset_id),
            summary,
        )
        .await?;
        if let Some(media) = media {
            image_media_by_legacy_id.insert(image.id, media.id);
        }
    }

    for prompt in &video_prompts {
        if let Some(prompt_ref) = video_prompt_refs.get(&prompt.id) {
            sync_video_prompt_media_refs(
                db,
                prompt_ref.generation_spec_id,
                prompt.reference_image_ids_json.as_ref(),
                &image_media_by_legacy_id,
                summary,
            )
            .await?;
        }
    }

    Ok(())
}
