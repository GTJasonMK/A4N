use super::contracts::{WebgalExportAsset, WebgalExportAssets, WebgalFigureKey};
use super::rules::{
    media_file_extension, media_file_extension_for_webgal_role, webgal_figure_position,
};
use a4n_core::models::{character::character_portrait, galgame::galgame_line};
use a4n_orchestrator::galgame::{GALGAME_ENTITY_LINE, GALGAME_ENTITY_SCENE, GalgameScriptDetail};
use a4n_orchestrator::media::{
    MEDIA_BINDING_ROLE_BACKGROUND, MEDIA_BINDING_ROLE_BGM, MEDIA_BINDING_ROLE_CG,
    MEDIA_BINDING_ROLE_SFX, galgame_media_binding_required_media_kind,
};

pub(super) fn webgal_line_figure<'a>(
    line: &galgame_line::Model,
    assets: &'a WebgalExportAssets,
) -> Result<Option<(&'static str, &'a WebgalExportAsset)>, String> {
    let Some(profile_id) = line.speaker_profile_id else {
        return Ok(None);
    };
    let Some(position) = webgal_figure_position(line.position.as_deref(), line.id)? else {
        return Ok(None);
    };
    let key = WebgalFigureKey {
        profile_id,
        expression: line.expression.clone(),
        pose: line.pose.clone(),
    };
    if key.expression.is_some() || key.pose.is_some() {
        return Ok(assets.figures.get(&key).map(|asset| (position, asset)));
    }
    Ok(assets
        .figures
        .get(&WebgalFigureKey {
            profile_id,
            expression: None,
            pose: None,
        })
        .map(|asset| (position, asset)))
}

pub(super) fn webgal_export_assets(
    detail: &GalgameScriptDetail,
    portraits: &[character_portrait::Model],
) -> Result<WebgalExportAssets, String> {
    let mut assets = WebgalExportAssets::default();
    for item in &detail.media_bindings {
        let binding = &item.binding;
        let asset = &item.asset;
        let expected_media_kind =
            galgame_media_binding_required_media_kind(&binding.target_kind, &binding.role)
                .ok_or_else(|| {
                    format!(
                        "Galgame 媒体绑定目标/角色无效，无法导出 WebGAL: binding_id={}, target_kind={}, role={}",
                        binding.id, binding.target_kind, binding.role
                    )
                })?;
        if asset.media_kind != expected_media_kind {
            return Err(format!(
                "Galgame 媒体绑定资产类型不匹配，无法导出 WebGAL: binding_id={}, media_asset_id={}, role={}, expected_media_kind={}, actual_media_kind={}",
                binding.id, asset.id, binding.role, expected_media_kind, asset.media_kind
            ));
        }
        let file_path = asset
            .file_path
            .as_deref()
            .ok_or_else(|| format!("媒体资产 {} 缺少文件路径", asset.id))?;
        let extension = media_file_extension_for_webgal_role(file_path, &binding.role)?;
        match (binding.target_kind.as_str(), binding.role.as_str()) {
            (GALGAME_ENTITY_SCENE, MEDIA_BINDING_ROLE_BACKGROUND) => {
                assets.scene_backgrounds.insert(
                    binding.target_id,
                    WebgalExportAsset {
                        script_name: format!(
                            "background_{}_{}.{}",
                            binding.target_id, asset.id, extension
                        ),
                        zip_path: format!(
                            "game/background/background_{}_{}.{}",
                            binding.target_id, asset.id, extension
                        ),
                        data_path: file_path.to_string(),
                    },
                );
            }
            (GALGAME_ENTITY_LINE, MEDIA_BINDING_ROLE_CG) => {
                assets.line_cgs.insert(
                    binding.target_id,
                    WebgalExportAsset {
                        script_name: format!("cg_{}_{}.{}", binding.target_id, asset.id, extension),
                        zip_path: format!(
                            "game/background/cg_{}_{}.{}",
                            binding.target_id, asset.id, extension
                        ),
                        data_path: file_path.to_string(),
                    },
                );
            }
            (GALGAME_ENTITY_LINE, MEDIA_BINDING_ROLE_BGM) => {
                assets.line_bgms.insert(
                    binding.target_id,
                    WebgalExportAsset {
                        script_name: format!(
                            "bgm_{}_{}.{}",
                            binding.target_id, asset.id, extension
                        ),
                        zip_path: format!(
                            "game/bgm/bgm_{}_{}.{}",
                            binding.target_id, asset.id, extension
                        ),
                        data_path: file_path.to_string(),
                    },
                );
            }
            (GALGAME_ENTITY_LINE, MEDIA_BINDING_ROLE_SFX) => {
                assets.line_sfx.insert(
                    binding.target_id,
                    WebgalExportAsset {
                        script_name: format!(
                            "sfx_{}_{}.{}",
                            binding.target_id, asset.id, extension
                        ),
                        zip_path: format!(
                            "game/vocal/sfx_{}_{}.{}",
                            binding.target_id, asset.id, extension
                        ),
                        data_path: file_path.to_string(),
                    },
                );
            }
            _ => {
                return Err(format!(
                    "Galgame 媒体绑定目标/角色无效，无法导出 WebGAL: binding_id={}, target_kind={}, role={}",
                    binding.id, binding.target_kind, binding.role
                ));
            }
        }
    }
    for portrait in portraits {
        let extension = media_file_extension(&portrait.image_path)?;
        let key = if portrait.is_primary && portrait.expression.is_none() && portrait.pose.is_none()
        {
            WebgalFigureKey {
                profile_id: portrait.profile_id,
                expression: None,
                pose: None,
            }
        } else {
            WebgalFigureKey {
                profile_id: portrait.profile_id,
                expression: portrait.expression.clone(),
                pose: portrait.pose.clone(),
            }
        };
        assets.figures.insert(
            key,
            WebgalExportAsset {
                script_name: format!(
                    "figure_{}_{}.{}",
                    portrait.profile_id, portrait.id, extension
                ),
                zip_path: format!(
                    "game/figure/figure_{}_{}.{}",
                    portrait.profile_id, portrait.id, extension
                ),
                data_path: portrait.image_path.clone(),
            },
        );
    }
    Ok(assets)
}
