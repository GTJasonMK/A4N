use std::collections::BTreeMap;

use a4n_domain::{
    galgame::GalgamePipelineBeatRefV4,
    media_authoring::{
        AdaptationPlanV1, AdaptationPlanVersion, AdaptationTarget,
        legacy::{
            HistoricalGalgameLine, HistoricalGalgameScene, historical_galgame_beat_id,
            historical_galgame_unit, historical_galgame_unit_id, upgrade_galgame_presentation,
            upgrade_galgame_spec_presentations,
        },
    },
};

use super::{ProjectFamilyPackage, error, insert_document, receipt};

pub(super) fn upgrade(package: &mut ProjectFamilyPackage, next: &mut i64) -> Result<(), String> {
    let mut groups = BTreeMap::<
        (i64, i64, i64, Option<i64>),
        Vec<a4n_domain::project::ProjectFamilyGalgameSceneRecord>,
    >::new();
    let mut sources = BTreeMap::new();
    for scene in &package.galgame_scenes {
        let script = package
            .galgame_scripts
            .iter()
            .find(|row| row.id == scene.script_id)
            .ok_or("旧游戏脚本不存在")?;
        let project = package
            .projects
            .iter()
            .find(|row| row.id == script.project_id)
            .ok_or("旧游戏项目不存在")?;
        if project.project_kind != "galgame"
            || project.root_project_id != Some(package.root_project_id)
        {
            return Err("旧游戏项目归属无效".into());
        }
        let source = package
            .source_snapshot_chapters
            .iter()
            .find(|row| row.id == scene.source_snapshot_chapter_id)
            .ok_or("旧游戏原文快照不存在")?;
        if source.root_project_id != package.root_project_id
            || a4n_domain::source_text_hash(&source.content) != source.content_hash
        {
            return Err("旧游戏原文哈希不一致".into());
        }
        if !matches!(scene.entry_kind.as_str(), "main" | "branch") {
            return Err("旧游戏场景类型无效".into());
        }
        groups
            .entry((
                project.id,
                source.snapshot_id,
                source.source_chapter_id,
                (scene.entry_kind == "branch").then_some(scene.id),
            ))
            .or_default()
            .push(scene.clone());
        sources.insert(scene.id, source.clone());
    }
    let mut bindings = BTreeMap::new();
    for ((project_id, snapshot_id, chapter_id, branch), mut scenes) in groups {
        scenes.sort_by_key(|scene| scene.scene_index);
        let source = &sources[&scenes[0].id];
        let mut units = Vec::new();
        for scene in &scenes {
            let mut lines = package
                .galgame_lines
                .iter()
                .filter(|line| line.scene_id == scene.id)
                .collect::<Vec<_>>();
            lines.sort_by_key(|line| line.line_index);
            let lines = lines
                .into_iter()
                .map(|line| {
                    Ok(HistoricalGalgameLine {
                        id: line.id,
                        index: line.line_index,
                        line_type: line.line_type.clone(),
                        text: line.text.clone(),
                        source_span_ids: line
                            .text_anchor_id
                            .map(|id| anchor_spans(package, id, source))
                            .transpose()?
                            .unwrap_or_default(),
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            let participants = scene
                .participants_json
                .as_array()
                .ok_or("旧游戏参与者必须是数组")?
                .iter()
                .map(|value| {
                    value["name"]
                        .as_str()
                        .filter(|name| !name.trim().is_empty())
                        .map(str::to_owned)
                        .ok_or_else(|| "旧游戏参与者名称缺失".to_string())
                })
                .collect::<Result<Vec<_>, _>>()?;
            units.push(historical_galgame_unit(HistoricalGalgameScene {
                id: scene.id,
                title: scene.title.clone(),
                location: scene.location.clone(),
                participants,
                lines,
            })?);
        }
        let plan = AdaptationPlanV1 {
            schema_version: AdaptationPlanVersion::V1,
            target: AdaptationTarget::Galgame,
            intention: "保留已经保存的游戏场景、对白、选择和人工编辑；历史导演意图不作推测。"
                .into(),
            decisions: Vec::new(),
            units,
        };
        plan.validate(
            &a4n_domain::source_spans::source_spans_for_content(
                chapter_id,
                source.chapter_number,
                &source.content,
            )
            .into_iter()
            .map(|span| span.id)
            .collect(),
        )?;
        let complete = branch.is_none()
            && package.projects.iter().any(|project| {
                project.id == project_id && project.source_snapshot_id == Some(snapshot_id)
            })
            && package
                .galgame_scenes
                .iter()
                .filter(|scene| {
                    scene.entry_kind == "main"
                        && sources
                            .get(&scene.id)
                            .is_some_and(|source| source.source_chapter_id == chapter_id)
                        && package.galgame_scripts.iter().any(|script| {
                            script.id == scene.script_id && script.project_id == project_id
                        })
                })
                .count()
                == scenes.len();
        let doc = insert_document(
            package,
            project_id,
            source,
            "adaptation_plan",
            serde_json::to_value(plan).map_err(error)?,
            Vec::new(),
            complete,
            next,
        )?;
        for scene in scenes {
            bindings.insert(scene.id, (doc.id, project_id));
        }
    }
    for index in 0..package.galgame_scenes.len() {
        let original = package.galgame_scenes[index].clone();
        let (plan_id, project_id) = bindings[&original.id];
        let scene = &mut package.galgame_scenes[index];
        scene.adaptation_plan_id = Some(plan_id);
        scene.adaptation_unit_id = Some(historical_galgame_unit_id(scene.id));
        let after = serde_json::to_value(scene).map_err(error)?;
        receipt(
            package,
            "galgame_scene_source",
            original.id,
            original.id,
            project_id,
            serde_json::to_value(original).map_err(error)?,
            &after,
        )?;
    }
    let mut line_refs = BTreeMap::new();
    for index in 0..package.galgame_lines.len() {
        let original = package.galgame_lines[index].clone();
        let source = sources
            .get(&original.scene_id)
            .ok_or("旧游戏演出行引用了不存在的场景")?;
        let reference = GalgamePipelineBeatRefV4 {
            source_chapter_id: source.source_chapter_id,
            unit_id: historical_galgame_unit_id(original.scene_id),
            beat_id: historical_galgame_beat_id(original.id),
        };
        let converted =
            upgrade_galgame_presentation(&original.presentation_json, reference.clone())?;
        package.galgame_lines[index].presentation_json = converted.clone();
        receipt(
            package,
            "galgame_presentation",
            original.id,
            original.id,
            bindings[&original.scene_id].1,
            serde_json::to_value(&original).map_err(error)?,
            &converted,
        )?;
        line_refs.insert(original.id, reference);
    }
    for spec in &mut package.generation_specs {
        if spec.target_kind.starts_with("galgame_") {
            upgrade_galgame_spec_presentations(&mut spec.params_json, &line_refs)?;
        }
    }
    Ok(())
}

fn anchor_spans(
    package: &ProjectFamilyPackage,
    id: i64,
    source: &a4n_domain::project::ProjectFamilySourceSnapshotChapterRecord,
) -> Result<Vec<String>, String> {
    let anchor = package
        .text_anchors
        .iter()
        .find(|anchor| anchor.id == id)
        .ok_or("旧游戏文本锚点不存在")?;
    if anchor.project_id != source.root_project_id
        || anchor.source_snapshot_chapter_id != Some(source.id)
        || anchor.start_offset < 0
        || anchor.end_offset <= anchor.start_offset
        || a4n_domain::source_text_hash(&anchor.quote) != anchor.quote_hash
    {
        return Err("旧游戏文本锚点身份无效".into());
    }
    let quote = source
        .content
        .chars()
        .skip(anchor.start_offset as usize)
        .take((anchor.end_offset - anchor.start_offset) as usize)
        .collect::<String>();
    if quote != anchor.quote {
        return Err("旧游戏文本锚点与原文不一致".into());
    }
    let spans = a4n_domain::source_spans::source_spans_for_content(
        source.source_chapter_id,
        source.chapter_number,
        &source.content,
    )
    .into_iter()
    .filter(|span| {
        span.char_start < anchor.end_offset as usize && span.char_end > anchor.start_offset as usize
    })
    .map(|span| span.id)
    .collect::<Vec<_>>();
    if spans.is_empty() {
        return Err("旧游戏文本锚点不对应原文片段".into());
    }
    Ok(spans)
}
