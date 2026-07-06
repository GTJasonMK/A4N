use chrono::{NaiveDateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
    TransactionTrait,
};

use a4n_core::error::AppError;
use a4n_core::models::ai::{generated_video, video_gen_config};

use crate::video::{
    UpdateVideoConfigParams, VideoConfigParams, validate_video_config_model,
    validate_video_config_params,
};

use super::ConfigService;
use super::set_single_default_with;

impl ConfigService {
    config_record_read_methods!(
        entity = video_gen_config::Entity,
        model = video_gen_config::Model,
        list = list_video => <video_gen_config::Entity as sea_orm::EntityTrait>::find()
            .order_by_asc(video_gen_config::Column::Name),
        get = get_video,
        not_found = |_| "视频配置不存在".into(),
    );

    pub async fn create_video(
        &self,
        params: VideoConfigParams,
    ) -> Result<video_gen_config::Model, AppError> {
        let params = validate_video_config_params(params).map_err(AppError::InvalidArgument)?;
        self.ensure_config_name_available::<video_gen_config::Entity>(
            "视频配置",
            &params.name,
            video_gen_config::Column::Name,
            video_gen_config::Column::Id,
            None,
        )
        .await?;
        let now = Utc::now().naive_utc();
        video_active_model(params, now, now, false)
            .insert(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    pub async fn update_video(
        &self,
        config_id: i64,
        params: UpdateVideoConfigParams,
    ) -> Result<video_gen_config::Model, AppError> {
        let existing = self.get_video(config_id).await?;
        let api_key_encrypted = params
            .api_key_encrypted
            .unwrap_or_else(|| existing.api_key_encrypted.clone());
        let params = validate_video_config_params(VideoConfigParams {
            name: params.name,
            backend: params.backend,
            base_url: params.base_url,
            api_key_encrypted,
            model: params.model,
            max_concurrent: params.max_concurrent,
            default_width: params.default_width,
            default_height: params.default_height,
            default_duration_secs: params.default_duration_secs,
            min_duration_secs: params.min_duration_secs,
            max_duration_secs: params.max_duration_secs,
            duration_mode: params.duration_mode,
            supported_durations_json: params.supported_durations_json,
            default_fps: params.default_fps,
            provider_options_json: params.provider_options_json,
        })
        .map_err(AppError::InvalidArgument)?;
        self.ensure_config_name_available::<video_gen_config::Entity>(
            "视频配置",
            &params.name,
            video_gen_config::Column::Name,
            video_gen_config::Column::Id,
            Some(config_id),
        )
        .await?;
        let mut active = video_active_model(
            params,
            existing.created_at,
            Utc::now().naive_utc(),
            existing.is_default,
        );
        active.id = Set(existing.id);
        active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    pub async fn delete_video(&self, config_id: i64) -> Result<(), AppError> {
        let config = self.get_video(config_id).await?;
        super::ensure_default_config_delete_allowed("视频配置", config.is_default)?;
        let active_video_ids =
            active_generated_video_ids_for_config(self.db.as_ref(), config_id).await?;
        if !active_video_ids.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "视频配置 #{config_id} 仍有 {} 个进行中的视频任务，不能删除",
                active_video_ids.len()
            )));
        }
        video_gen_config::Entity::delete_by_id(config_id)
            .exec(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        Ok(())
    }

    pub async fn set_default_video(
        &self,
        config_id: i64,
    ) -> Result<video_gen_config::Model, AppError> {
        let txn = self.db.begin().await.map_err(AppError::Database)?;
        let target = video_gen_config::Entity::find_by_id(config_id)
            .one(&txn)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound("视频配置不存在".into()))?;
        validate_video_config_model(&target).map_err(AppError::InvalidArgument)?;

        let now = Utc::now().naive_utc();
        let updated = set_single_default_with::<video_gen_config::Entity, _, _>(
            &txn,
            target,
            video_gen_config::Column::IsDefault,
            |active| active.updated_at = Set(now),
        )
        .await?;
        txn.commit().await.map_err(AppError::Database)?;
        Ok(updated)
    }
}

pub async fn active_generated_video_ids_for_config<C>(
    db: &C,
    config_id: i64,
) -> Result<Vec<i64>, AppError>
where
    C: ConnectionTrait,
{
    let mut active: Vec<_> = generated_video::Entity::find()
        .filter(generated_video::Column::Status.is_in(["pending", "running"]))
        .all(db)
        .await
        .map_err(AppError::Database)?
        .into_iter()
        .filter(|video| {
            video
                .metadata_json
                .as_ref()
                .and_then(|metadata| metadata.get("config_id"))
                .and_then(|value| value.as_i64())
                == Some(config_id)
        })
        .collect();
    active.sort_by_key(|video| (video.created_at, video.id));
    Ok(active.into_iter().map(|video| video.id).collect())
}

fn video_active_model(
    params: VideoConfigParams,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
    is_default: bool,
) -> video_gen_config::ActiveModel {
    video_gen_config::ActiveModel {
        name: Set(params.name),
        backend: Set(params.backend),
        base_url: Set(Some(params.base_url)),
        api_key_encrypted: Set(params.api_key_encrypted),
        model: Set(params.model),
        max_concurrent: Set(params.max_concurrent),
        default_width: Set(params.default_width),
        default_height: Set(params.default_height),
        default_duration_secs: Set(params.default_duration_secs),
        min_duration_secs: Set(params.min_duration_secs),
        max_duration_secs: Set(params.max_duration_secs),
        duration_mode: Set(params.duration_mode),
        supported_durations_json: Set(params.supported_durations_json),
        default_fps: Set(params.default_fps),
        provider_options_json: Set(params.provider_options_json),
        is_default: Set(is_default),
        created_at: Set(created_at),
        updated_at: Set(updated_at),
        ..Default::default()
    }
}
