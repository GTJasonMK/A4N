use super::archive::{webgal_config_text, write_zip_data_file, write_zip_text};
use super::assets::webgal_export_assets;
use super::render::render_webgal_scene;
use super::validation::{validate_webgal_choice_contract, validate_webgal_scene_entry_kinds};
use a4n_core::models::character::character_portrait;
use a4n_orchestrator::galgame::GalgameScriptDetail;
use std::collections::{BTreeMap, BTreeSet};
use std::io::Cursor;
use std::path::Path;

pub(super) fn build_webgal_package(
    data_dir: &Path,
    detail: &GalgameScriptDetail,
    portraits: &[character_portrait::Model],
) -> Result<Vec<u8>, String> {
    if detail.scenes.is_empty() {
        return Err("Galgame 剧本没有可导出的场".to_string());
    }
    validate_webgal_scene_entry_kinds(detail)?;
    let scene_file_by_id = detail
        .scenes
        .iter()
        .enumerate()
        .map(|(index, scene)| {
            (
                scene.scene.id,
                format!("scene_{:03}_{}.txt", index + 1, scene.scene.id),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let scene_entry_by_id = detail
        .scenes
        .iter()
        .map(|scene| (scene.scene.id, scene.scene.entry_kind.clone()))
        .collect::<BTreeMap<_, _>>();
    let speaker_name_by_id = detail
        .speaker_profiles
        .iter()
        .map(|profile| (profile.id, profile.name.clone()))
        .collect::<BTreeMap<_, _>>();
    let choices_by_line = detail
        .scenes
        .iter()
        .flat_map(|scene| scene.choices.iter())
        .fold(BTreeMap::<i64, Vec<_>>::new(), |mut acc, choice| {
            acc.entry(choice.line_id).or_default().push(choice);
            acc
        });
    validate_webgal_choice_contract(detail, &scene_file_by_id)?;
    let assets = webgal_export_assets(detail, portraits)?;

    let mut buffer = Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut buffer);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        write_zip_text(
            &mut zip,
            "game/config.txt",
            &webgal_config_text(detail),
            options,
        )?;

        let first_scene = scene_file_by_id
            .get(&detail.scenes[0].scene.id)
            .ok_or_else(|| "Galgame 首场导出文件名缺失".to_string())?;
        write_zip_text(
            &mut zip,
            "game/scene/start.txt",
            &format!("changeScene:{};\n", first_scene),
            options,
        )?;

        for scene in &detail.scenes {
            let file_name = scene_file_by_id
                .get(&scene.scene.id)
                .ok_or_else(|| format!("Galgame 场导出文件名缺失: {}", scene.scene.id))?;
            let script = render_webgal_scene(
                scene,
                &scene_file_by_id,
                &scene_entry_by_id,
                &speaker_name_by_id,
                &choices_by_line,
                &assets,
            )?;
            write_zip_text(
                &mut zip,
                &format!("game/scene/{file_name}"),
                &script,
                options,
            )?;
        }

        let mut written_assets = BTreeSet::new();
        for asset in assets
            .scene_backgrounds
            .values()
            .chain(assets.line_cgs.values())
            .chain(assets.line_bgms.values())
            .chain(assets.line_sfx.values())
            .chain(assets.figures.values())
        {
            if written_assets.insert(asset.zip_path.clone()) {
                write_zip_data_file(data_dir, &mut zip, asset, options)?;
            }
        }

        zip.finish()
            .map_err(|err| format!("WebGAL zip 打包失败: {err}"))?;
    }
    Ok(buffer.into_inner())
}
