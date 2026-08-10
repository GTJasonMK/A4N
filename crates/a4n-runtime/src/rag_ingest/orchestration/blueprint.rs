use super::super::characters::{ingest_character_records_inner, ingest_relationships_inner};
use super::super::story_records::{
    ingest_blueprint_metadata_inner, ingest_blueprints_inner, ingest_world_setting_inner,
};
use super::super::sync::ExpectedSyncStats as IngestStats;
use super::errors::rag_ingest_error_message;
use crate::state::AppState;
use a4n_core::models::project::{blueprint_character, blueprint_relationship, novel_blueprint};

pub async fn sync_blueprint_rag_records(
    state: &AppState,
    project_id: i64,
) -> Result<serde_json::Value, String> {
    let trace_id = crate::sidecar_clients::rag_ingest_operation_trace_id(project_id, None)?;
    crate::sidecar_clients::scope_rag_operation_trace(
        trace_id,
        sync_blueprint_rag_records_inner(state, project_id),
    )
    .await
}

pub async fn sync_restored_blueprint_rag_records(
    state: &AppState,
    project_id: i64,
) -> Result<serde_json::Value, String> {
    let trace_id = crate::sidecar_clients::rag_ingest_operation_trace_id(project_id, None)?;
    crate::sidecar_clients::scope_rag_operation_trace(
        trace_id,
        sync_restored_blueprint_rag_records_inner(state, project_id),
    )
    .await
}

async fn sync_restored_blueprint_rag_records_inner(
    state: &AppState,
    project_id: i64,
) -> Result<serde_json::Value, String> {
    let expected_sources = restored_blueprint_rag_sources(state, project_id).await?;
    let blueprint_result = sync_blueprint_rag_records_inner(state, project_id).await;
    let cast_result = sync_blueprint_cast_rag_records_inner(state, project_id).await;
    match (blueprint_result, cast_result) {
        (Ok(blueprint), Ok(cast)) => {
            let current_sources = restored_blueprint_rag_sources(state, project_id).await?;
            ensure_restored_blueprint_rag_sources_unchanged(&expected_sources, &current_sources)?;
            Ok(serde_json::json!({
                "blueprint": blueprint,
                "cast": cast,
            }))
        }
        (blueprint, cast) => {
            let mut errors = Vec::new();
            if let Err(error) = blueprint {
                errors.push(format!("蓝图正文/元数据/世界观：{error}"));
            }
            if let Err(error) = cast {
                errors.push(format!("蓝图角色/关系：{error}"));
            }
            Err(errors.join("；"))
        }
    }
}

async fn sync_blueprint_rag_records_inner(
    state: &AppState,
    project_id: i64,
) -> Result<serde_json::Value, String> {
    let expected_sources = state
        .rag()
        .novel_blueprints_for_rag_ingest(project_id)
        .await
        .map_err(|error| error.to_string())?;
    let blueprint_result = ingest_blueprints_inner(state, project_id, None).await;
    let metadata_result = ingest_blueprint_metadata_inner(state, project_id, None).await;
    let world_setting_result = ingest_world_setting_inner(state, project_id, None).await;
    let mut total_stats = IngestStats::default();
    let mut errors = Vec::new();
    let blueprint_count = match blueprint_result {
        Ok(stats) => {
            let count = stats.item_count;
            total_stats.merge(stats)?;
            count
        }
        Err(error) => {
            errors.push(format!("蓝图正文同步失败：{error}"));
            0
        }
    };
    let blueprint_metadata_count = match metadata_result {
        Ok(stats) => {
            let count = stats.item_count;
            total_stats.merge(stats)?;
            count
        }
        Err(error) => {
            errors.push(format!("蓝图元数据同步失败：{error}"));
            0
        }
    };
    let world_setting_count = match world_setting_result {
        Ok(stats) => {
            let count = stats.item_count;
            total_stats.merge(stats)?;
            count
        }
        Err(error) => {
            errors.push(format!("世界观同步失败：{error}"));
            0
        }
    };
    errors.append(&mut total_stats.errors);
    if !errors.is_empty() {
        return Err(rag_ingest_error_message("蓝图 RAG 同步", &errors));
    }
    let current_sources = state
        .rag()
        .novel_blueprints_for_rag_ingest(project_id)
        .await
        .map_err(|error| error.to_string())?;
    ensure_blueprint_rag_sources_unchanged(&expected_sources, &current_sources)?;

    Ok(serde_json::json!({
        "blueprint_count": blueprint_count,
        "blueprint_metadata_count": blueprint_metadata_count,
        "world_setting_count": world_setting_count,
        "chunk_count": total_stats.chunk_count,
        "added_count": total_stats.added_count,
        "updated_count": total_stats.updated_count,
        "skipped_count": total_stats.skipped_count,
        "deleted_count": total_stats.deleted_count,
    }))
}

async fn sync_blueprint_cast_rag_records_inner(
    state: &AppState,
    project_id: i64,
) -> Result<serde_json::Value, String> {
    let character_result = ingest_character_records_inner(state, project_id, None).await;
    let relationship_result = ingest_relationships_inner(state, project_id, None).await;
    let mut total_stats = IngestStats::default();
    let mut errors = Vec::new();
    let character_count = match character_result {
        Ok(stats) => {
            let count = stats.item_count;
            total_stats.merge(stats)?;
            count
        }
        Err(error) => {
            errors.push(format!("角色设定同步失败：{error}"));
            0
        }
    };
    let relationship_count = match relationship_result {
        Ok(stats) => {
            let count = stats.item_count;
            total_stats.merge(stats)?;
            count
        }
        Err(error) => {
            errors.push(format!("角色关系同步失败：{error}"));
            0
        }
    };
    errors.append(&mut total_stats.errors);
    if !errors.is_empty() {
        return Err(rag_ingest_error_message("蓝图角色与关系 RAG 同步", &errors));
    }

    Ok(serde_json::json!({
        "character_count": character_count,
        "relationship_count": relationship_count,
        "chunk_count": total_stats.chunk_count,
        "added_count": total_stats.added_count,
        "updated_count": total_stats.updated_count,
        "skipped_count": total_stats.skipped_count,
        "deleted_count": total_stats.deleted_count,
    }))
}

async fn restored_blueprint_rag_sources(
    state: &AppState,
    project_id: i64,
) -> Result<RestoredBlueprintRagSources, String> {
    let mut blueprints = state
        .rag()
        .novel_blueprints_for_rag_ingest(project_id)
        .await
        .map_err(|error| error.to_string())?;
    blueprints.sort_by_key(|blueprint| blueprint.id);
    Ok(RestoredBlueprintRagSources {
        blueprints,
        cast: blueprint_cast_rag_sources(
            state
                .rag()
                .blueprint_characters_for_rag_ingest(project_id)
                .await
                .map_err(|error| error.to_string())?,
            state
                .rag()
                .blueprint_relationships_for_rag_ingest(project_id)
                .await
                .map_err(|error| error.to_string())?,
        ),
    })
}

fn ensure_blueprint_rag_sources_unchanged(
    expected: &[novel_blueprint::Model],
    current: &[novel_blueprint::Model],
) -> Result<(), String> {
    if expected == current {
        return Ok(());
    }
    Err("蓝图正式资料在 RAG 同步期间发生变化，请重新同步资料索引".to_string())
}

#[derive(PartialEq)]
struct BlueprintCastRagSources {
    characters: Vec<blueprint_character::Model>,
    relationships: Vec<blueprint_relationship::Model>,
}

#[derive(PartialEq)]
struct RestoredBlueprintRagSources {
    blueprints: Vec<novel_blueprint::Model>,
    cast: BlueprintCastRagSources,
}

fn blueprint_cast_rag_sources(
    mut characters: Vec<blueprint_character::Model>,
    mut relationships: Vec<blueprint_relationship::Model>,
) -> BlueprintCastRagSources {
    characters.sort_by_key(|character| character.id);
    relationships.sort_by_key(|relationship| relationship.id);
    BlueprintCastRagSources {
        characters,
        relationships,
    }
}

fn ensure_restored_blueprint_rag_sources_unchanged(
    expected: &RestoredBlueprintRagSources,
    current: &RestoredBlueprintRagSources,
) -> Result<(), String> {
    if expected == current {
        return Ok(());
    }
    Err("蓝图正式资料在 RAG 同步期间发生变化，请重新同步资料索引".to_string())
}

#[cfg(test)]
mod tests {
    use a4n_core::models::project::{blueprint_relationship, novel_blueprint};
    use chrono::NaiveDateTime;
    use serde_json::json;

    use super::{
        RestoredBlueprintRagSources, blueprint_cast_rag_sources,
        ensure_blueprint_rag_sources_unchanged, ensure_restored_blueprint_rag_sources_unchanged,
    };

    fn blueprint(version: &str) -> novel_blueprint::Model {
        novel_blueprint::Model {
            id: 1,
            project_id: 2,
            title: "标题".to_string(),
            genre: "悬疑".to_string(),
            style: None,
            tone: None,
            target_audience: None,
            one_sentence_summary: None,
            full_synopsis: None,
            world_setting: Some(json!({"city": "旧城"})),
            current_version: version.to_string(),
            needs_part_outlines: false,
            total_chapters: Some(10),
            chapters_per_part: None,
            avatar_svg: None,
            avatar_animal: None,
            created_at: NaiveDateTime::MIN,
            updated_at: NaiveDateTime::MIN,
        }
    }

    fn relationship(id: i64, description: &str) -> blueprint_relationship::Model {
        blueprint_relationship::Model {
            id,
            project_id: 2,
            relationship_type: "rival".to_string(),
            character_a: "林岚".to_string(),
            character_b: "沈澜".to_string(),
            description: Some(description.to_string()),
            strength: Some(7),
            evolution: None,
            created_at: NaiveDateTime::MIN,
        }
    }

    #[test]
    fn blueprint_rag_sync_rejects_changed_formal_source() {
        let error =
            ensure_blueprint_rag_sources_unchanged(&[blueprint("1.0")], &[blueprint("1.1")])
                .expect_err("changed formal blueprint source must reject stale sync");

        assert!(error.contains("正式资料"));
    }

    #[test]
    fn blueprint_cast_rag_sync_uses_stable_sources_and_rejects_changes() {
        let expected = RestoredBlueprintRagSources {
            blueprints: vec![blueprint("1.0")],
            cast: blueprint_cast_rag_sources(
                Vec::new(),
                vec![relationship(2, "互相试探"), relationship(1, "旧识")],
            ),
        };
        let reordered = RestoredBlueprintRagSources {
            blueprints: vec![blueprint("1.0")],
            cast: blueprint_cast_rag_sources(
                Vec::new(),
                vec![relationship(1, "旧识"), relationship(2, "互相试探")],
            ),
        };
        ensure_restored_blueprint_rag_sources_unchanged(&expected, &reordered)
            .expect("row order must not change source identity");

        let changed = RestoredBlueprintRagSources {
            blueprints: vec![blueprint("1.0")],
            cast: blueprint_cast_rag_sources(
                Vec::new(),
                vec![relationship(1, "旧识"), relationship(2, "已经决裂")],
            ),
        };
        let error = ensure_restored_blueprint_rag_sources_unchanged(&expected, &changed)
            .expect_err("changed relationship must reject stale sync");
        assert!(error.contains("正式资料"));
    }
}
