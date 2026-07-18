use super::super::super::types::{SettingsAllConfigImportError, settings_all_config_database};
use a4n_core::models::ai::{embedding_config, image_gen_config, llm_config, video_gen_config};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

macro_rules! clear_config_defaults {
    ($fn_name:ident, $entity:ident, $operation:literal) => {
        pub(super) async fn $fn_name<C>(db: &C) -> Result<(), SettingsAllConfigImportError>
        where
            C: ConnectionTrait,
        {
            $entity::Entity::update_many()
                .col_expr(
                    $entity::Column::IsDefault,
                    sea_orm::sea_query::Expr::value(false),
                )
                .filter($entity::Column::IsDefault.eq(true))
                .exec(db)
                .await
                .map_err(|source| settings_all_config_database($operation, source))?;
            Ok(())
        }
    };
}

clear_config_defaults!(clear_llm_defaults, llm_config, "清除既有默认 LLM 配置失败");
clear_config_defaults!(
    clear_embedding_defaults,
    embedding_config,
    "清除既有默认 Embedding 配置失败"
);
clear_config_defaults!(
    clear_image_defaults,
    image_gen_config,
    "清除既有默认图片配置失败"
);
clear_config_defaults!(
    clear_video_defaults,
    video_gen_config,
    "清除既有默认视频配置失败"
);
