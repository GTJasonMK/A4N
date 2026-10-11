use std::collections::BTreeMap;

use a4n_db_models::models::{
    galgame::{galgame_line, galgame_scene, galgame_script},
    generation::generation_spec,
    project::{
        media_authoring_document as document, novel_project, source_snapshot_chapter, text_anchor,
    },
};
use a4n_domain::{
    galgame::GalgamePipelineBeatRefV4,
    media_authoring::{
        AdaptationPlanV1, AdaptationPlanVersion, AdaptationTarget, AdaptationUnit,
        legacy::{
            HistoricalGalgameLine, HistoricalGalgameScene, historical_galgame_beat_id as beat_id,
            historical_galgame_unit, historical_galgame_unit_id as unit_id,
            upgrade_galgame_presentation,
        },
    },
    source_spans::{SourceSpan, source_spans_for_content},
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbBackend, DbErr, EntityTrait, QueryFilter,
    QueryOrder, Set, Statement,
};
use serde_json::Value;

use super::{artifacts::insert_document, hash_value, record_receipt};

pub(super) async fn migrate_galgame<C: ConnectionTrait>(
    db: &C,
    now: sea_orm::prelude::DateTime,
) -> Result<(), DbErr> {
    for script in galgame_script::Entity::find().all(db).await? {
        let project = novel_project::Entity::find_by_id(script.project_id)
            .one(db)
            .await?
            .ok_or_else(|| failure("历史游戏项目不存在"))?;
        if project.project_kind != "galgame"
            || project.root_project_id != Some(script.root_project_id)
        {
            return Err(failure("历史游戏项目归属无效"));
        }
        let scenes = galgame_scene::Entity::find()
            .filter(galgame_scene::Column::ScriptId.eq(script.id))
            .order_by_asc(galgame_scene::Column::SceneIndex)
            .all(db)
            .await?;
        let mut main_groups = BTreeMap::<(i64, i64), Vec<galgame_scene::Model>>::new();
        let mut total_mains = BTreeMap::<i64, usize>::new();
        let mut sources = BTreeMap::new();
        let mut lines = BTreeMap::new();
        for scene in &scenes {
            let source =
                source_snapshot_chapter::Entity::find_by_id(scene.source_snapshot_chapter_id)
                    .one(db)
                    .await?
                    .ok_or_else(|| failure("历史游戏来源章节不存在"))?;
            if source.root_project_id != script.root_project_id
                || a4n_domain::source_text_hash(&source.content) != source.content_hash
            {
                return Err(failure("历史游戏原文身份或内容哈希无效"));
            }
            let scene_lines = galgame_line::Entity::find()
                .filter(galgame_line::Column::SceneId.eq(scene.id))
                .order_by_asc(galgame_line::Column::LineIndex)
                .all(db)
                .await?;
            lines.insert(scene.id, scene_lines);
            if scene.entry_kind == "main" {
                *total_mains.entry(source.source_chapter_id).or_default() += 1;
                if scene.adaptation_plan_id.is_none() {
                    main_groups
                        .entry((source.snapshot_id, source.source_chapter_id))
                        .or_default()
                        .push(scene.clone());
                }
            }
            sources.insert(scene.id, source);
        }
        let mut bindings = BTreeMap::<i64, (i64, String)>::new();
        for ((snapshot_id, chapter_id), group) in main_groups {
            let source = &sources[&group[0].id];
            let mut units = Vec::new();
            for scene in &group {
                units.push(
                    historical_unit(db, scene, &lines[&scene.id], &sources[&scene.id]).await?,
                );
            }
            let plan = historical_plan(units);
            plan.validate(
                &source_spans_for_content(chapter_id, source.chapter_number, &source.content)
                    .into_iter()
                    .map(|span| span.id)
                    .collect(),
            )
            .map_err(failure)?;
            let doc = insert_document(
                db,
                project.id,
                source,
                "adaptation_plan",
                serde_json::to_value(&plan).map_err(json_error)?,
                Vec::new(),
                now,
            )
            .await?;
            let complete = project.source_snapshot_id == Some(snapshot_id)
                && total_mains.get(&chapter_id) == Some(&group.len());
            if complete {
                let current = document::Entity::find()
                    .filter(document::Column::ProjectId.eq(project.id))
                    .filter(document::Column::SourceSnapshotId.eq(snapshot_id))
                    .filter(document::Column::SourceChapterId.eq(chapter_id))
                    .filter(document::Column::Kind.eq("adaptation_plan"))
                    .filter(document::Column::Status.eq("adopted"))
                    .one(db)
                    .await?;
                if current.is_none() {
                    document::ActiveModel {
                        id: Set(doc.id),
                        status: Set("adopted".into()),
                        ..Default::default()
                    }
                    .update(db)
                    .await?;
                }
            }
            for scene in group {
                bindings.insert(scene.id, (doc.id, unit_id(scene.id)));
            }
        }
        for scene in &scenes {
            if scene.adaptation_plan_id.is_some() {
                continue;
            }
            if let std::collections::btree_map::Entry::Vacant(binding) = bindings.entry(scene.id) {
                // Branches keep their own frozen source, including mixed-source
                // games whose main scene has already moved to a newer snapshot.
                let source = &sources[&scene.id];
                let plan = historical_plan(vec![
                    historical_unit(db, scene, &lines[&scene.id], source).await?,
                ]);
                plan.validate(
                    &source_spans_for_content(
                        source.source_chapter_id,
                        source.chapter_number,
                        &source.content,
                    )
                    .into_iter()
                    .map(|span| span.id)
                    .collect(),
                )
                .map_err(failure)?;
                let doc = insert_document(
                    db,
                    project.id,
                    source,
                    "adaptation_plan",
                    serde_json::to_value(plan).map_err(json_error)?,
                    Vec::new(),
                    now,
                )
                .await?;
                binding.insert((doc.id, unit_id(scene.id)));
            }
            let (plan_id, unit) = &bindings[&scene.id];
            let before: Value = db
                .query_one(Statement::from_sql_and_values(
                    DbBackend::Postgres,
                    "SELECT to_jsonb(s) AS payload FROM galgame_scene s WHERE id=$1",
                    [scene.id.into()],
                ))
                .await?
                .ok_or_else(|| failure("迁移期间游戏场景丢失"))?
                .try_get("", "payload")?;
            let mut active: galgame_scene::ActiveModel = scene.clone().into();
            active.adaptation_plan_id = Set(Some(*plan_id));
            active.adaptation_unit_id = Set(Some(unit.clone()));
            active.update(db).await?;
            record_receipt(
                db,
                "galgame_scene_source",
                scene.id,
                scene.id,
                project.id,
                &before,
                &hash_value(&serde_json::json!({"plan_id":plan_id,"unit_id":unit}))?,
            )
            .await?;
        }
        let mut line_refs = BTreeMap::new();
        for scene in &scenes {
            for line in &lines[&scene.id] {
                let version = line
                    .presentation_json
                    .get("schema_version")
                    .and_then(Value::as_str);
                if version == Some("galgame_line_presentation_v3") {
                    if let Some(value) = line
                        .presentation_json
                        .get("beat_ref")
                        .filter(|value| !value.is_null())
                    {
                        line_refs.insert(
                            line.id,
                            serde_json::from_value(value.clone()).map_err(json_error)?,
                        );
                    }
                    continue;
                }
                let presentation = upgrade_galgame_presentation(
                    &line.presentation_json,
                    GalgamePipelineBeatRefV4 {
                        source_chapter_id: sources[&scene.id].source_chapter_id,
                        unit_id: bindings
                            .get(&scene.id)
                            .map(|(_, unit)| unit.clone())
                            .or_else(|| scene.adaptation_unit_id.clone())
                            .ok_or_else(|| failure("游戏演出来源缺失"))?,
                        beat_id: beat_id(line.id),
                    },
                )
                .map_err(failure)?;
                line_refs.insert(
                    line.id,
                    serde_json::from_value(presentation["beat_ref"].clone()).map_err(json_error)?,
                );
                let before = serde_json::to_value(line).map_err(json_error)?;
                galgame_line::ActiveModel {
                    id: Set(line.id),
                    presentation_json: Set(presentation.clone()),
                    ..Default::default()
                }
                .update(db)
                .await?;
                record_receipt(
                    db,
                    "galgame_presentation",
                    line.id,
                    line.id,
                    project.id,
                    &before,
                    &hash_value(&presentation)?,
                )
                .await?;
            }
        }
        for spec in generation_spec::Entity::find()
            .filter(generation_spec::Column::ProjectId.eq(project.id))
            .all(db)
            .await?
        {
            let mut params = spec.params_json.clone();
            a4n_domain::media_authoring::legacy::upgrade_galgame_spec_presentations(
                &mut params,
                &line_refs,
            )
            .map_err(failure)?;
            if params == spec.params_json {
                continue;
            }
            generation_spec::ActiveModel {
                id: Set(spec.id),
                params_json: Set(params.clone()),
                ..Default::default()
            }
            .update(db)
            .await?;
            record_receipt(
                db,
                "galgame_generation_spec",
                spec.id,
                spec.id,
                project.id,
                &serde_json::to_value(spec).map_err(json_error)?,
                &hash_value(&params)?,
            )
            .await?;
        }
    }
    Ok(())
}

async fn historical_unit<C: ConnectionTrait>(
    db: &C,
    scene: &galgame_scene::Model,
    lines: &[galgame_line::Model],
    source: &source_snapshot_chapter::Model,
) -> Result<AdaptationUnit, DbErr> {
    let spans = source_spans_for_content(
        source.source_chapter_id,
        source.chapter_number,
        &source.content,
    );
    let mut saved = Vec::new();
    for line in lines {
        let source_span_ids = match line.text_anchor_id {
            Some(id) => anchor_spans(db, id, source, &spans).await?,
            None => Vec::new(),
        };
        saved.push(HistoricalGalgameLine {
            id: line.id,
            index: line.line_index,
            line_type: line.line_type.clone(),
            text: line.text.clone(),
            source_span_ids,
        });
    }
    let participants = scene
        .participants_json
        .as_array()
        .ok_or_else(|| failure("历史游戏人物集合无效"))?
        .iter()
        .map(|participant| {
            participant
                .get("name")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .map(str::to_owned)
                .ok_or_else(|| failure("历史游戏参与者缺少名称"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    historical_galgame_unit(HistoricalGalgameScene {
        id: scene.id,
        title: scene.title.clone(),
        location: scene.location.clone(),
        participants,
        lines: saved,
    })
    .map_err(failure)
}

async fn anchor_spans<C: ConnectionTrait>(
    db: &C,
    id: i64,
    source: &source_snapshot_chapter::Model,
    spans: &[SourceSpan],
) -> Result<Vec<String>, DbErr> {
    let anchor = text_anchor::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| failure("历史游戏文本锚点不存在"))?;
    if anchor.project_id != source.root_project_id
        || anchor.source_snapshot_chapter_id != Some(source.id)
        || anchor.start_offset < 0
        || anchor.end_offset <= anchor.start_offset
        || a4n_domain::source_text_hash(&anchor.quote) != anchor.quote_hash
    {
        return Err(failure("历史游戏文本锚点来源或哈希不一致"));
    }
    let quote = source
        .content
        .chars()
        .skip(anchor.start_offset as usize)
        .take((anchor.end_offset - anchor.start_offset) as usize)
        .collect::<String>();
    if quote != anchor.quote {
        return Err(failure("历史游戏文本锚点与冻结正文不一致"));
    }
    let ids = spans
        .iter()
        .filter(|span| {
            span.char_start < anchor.end_offset as usize
                && span.char_end > anchor.start_offset as usize
        })
        .map(|span| span.id.clone())
        .collect::<Vec<_>>();
    if ids.is_empty() {
        return Err(failure("历史游戏文本锚点无法对应原文章节片段"));
    }
    Ok(ids)
}

fn historical_plan(units: Vec<AdaptationUnit>) -> AdaptationPlanV1 {
    AdaptationPlanV1 {
        schema_version: AdaptationPlanVersion::V1,
        target: AdaptationTarget::Galgame,
        page_count: None,
        intention: "迁移已有游戏脚本；保留实际场景、对白和人工编辑，未记录的导演意图不作推测"
            .into(),
        decisions: Vec::new(),
        units,
    }
}
fn failure(message: impl Into<String>) -> DbErr {
    DbErr::Custom(message.into())
}
fn json_error(error: serde_json::Error) -> DbErr {
    failure(error.to_string())
}
