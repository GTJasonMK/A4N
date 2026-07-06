use super::{
    delete_character_attribute_event_rag_by_ids, delete_character_state_rag_by_ids,
    delete_foreshadowing_rag_by_ids, delete_protagonist_rag_by_ids,
};
use crate::file_utils::{deletable_data_file_path, remove_data_file_in_data_dir};
use crate::state::AppState;
use a4n_core::models::{
    chapter::{chapter, chapter_outline},
    character::{protagonist_profile, snapshot_reference},
};
use a4n_services::character::{
    CharacterProfileDeletionSummary, CharacterTimelineDeletionSummary, apply_profile_current_state,
    delete_profile_in, profile_first_seen_chapter, snapshot_attributes_from_data,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
    TransactionTrait,
};
use std::collections::BTreeSet;
use std::path::Path;

use self::foreshadowing::cleanup_foreshadowing_for_chapter_in;
use self::media::cleanup_chapter_media_data_in;
use self::optimization::cleanup_chapter_optimization_data_in;
use super::rag_records::reingest_foreshadowing_rag_by_ids;

mod foreshadowing;
mod media;
mod optimization;

#[derive(Debug, Default)]
pub struct ChapterDerivedCleanupArtifacts {
    stale_profile_ids: Vec<i64>,
    stale_character_state_ids: Vec<i64>,
    stale_attribute_event_ids: Vec<i64>,
    stale_foreshadowing_ids: Vec<i64>,
    changed_foreshadowing_ids: Vec<i64>,
    media_file_paths: Vec<String>,
    scene_records_changed: bool,
}

impl ChapterDerivedCleanupArtifacts {
    fn extend_deleted_profile(&mut self, summary: CharacterProfileDeletionSummary) {
        self.stale_profile_ids.push(summary.profile_id);
        self.stale_character_state_ids
            .extend(summary.state_index_ids);
        self.stale_attribute_event_ids
            .extend(summary.attribute_event_ids);
        self.media_file_paths.extend(summary.portrait_paths);
    }

    fn extend_character_timeline(&mut self, artifacts: CharacterTimelineDeletionSummary) {
        self.stale_character_state_ids
            .extend(artifacts.state_index_ids);
        self.stale_attribute_event_ids
            .extend(artifacts.attribute_event_ids);
    }

    fn extend_media_file_paths(&mut self, paths: Vec<String>) {
        self.media_file_paths.extend(paths);
    }
}

async fn cleanup_character_profile_timeline_for_chapter_in<C>(
    state: &AppState,
    db: &C,
    project_id: i64,
    chapter_number: i32,
) -> Result<ChapterDerivedCleanupArtifacts, String>
where
    C: ConnectionTrait,
{
    let profiles = protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .order_by_asc(protagonist_profile::Column::Id)
        .all(db)
        .await
        .map_err(|err| format!("查询角色档案失败: {err}"))?;
    if profiles.is_empty() {
        return Ok(ChapterDerivedCleanupArtifacts::default());
    }

    let mut artifacts = ChapterDerivedCleanupArtifacts::default();

    for profile in profiles {
        if profile_first_seen_chapter(&profile)
            .is_some_and(|first_seen| first_seen >= chapter_number)
        {
            let deleted = delete_profile_in(db, profile.id)
                .await
                .map_err(|err| err.to_string())?;
            artifacts.extend_deleted_profile(deleted);
            continue;
        }

        let should_rewind = profile.last_synced_chapter >= chapter_number;
        let cleanup_to_chapter = if should_rewind {
            profile.last_synced_chapter
        } else {
            chapter_number
        };

        let deleted = state
            .character
            .delete_timeline_artifacts_from_chapter(
                db,
                profile.id,
                chapter_number,
                Some(cleanup_to_chapter),
            )
            .await
            .map_err(|err| err.to_string())?;
        artifacts.extend_character_timeline(deleted);

        if should_rewind {
            let restored_chapter = chapter_number - 1;
            let restored_state =
                profile_state_at_or_before_chapter(db, &profile, restored_chapter).await?;
            state
                .character
                .rewind_deletion_marks_after(db, profile.id, restored_chapter)
                .await
                .map_err(|err| err.to_string())?;

            let mut active: protagonist_profile::ActiveModel = profile.into();
            apply_profile_current_state(&mut active, restored_state);
            active.last_synced_chapter = Set(restored_chapter.max(0));
            active.updated_at = Set(chrono::Utc::now().naive_utc());
            active.update(db).await.map_err(|err| err.to_string())?;
        }
    }

    Ok(artifacts)
}

async fn profile_state_at_or_before_chapter<C>(
    db: &C,
    profile: &protagonist_profile::Model,
    chapter_number: i32,
) -> Result<serde_json::Value, String>
where
    C: ConnectionTrait,
{
    if profile_state_should_restore_base(profile, chapter_number) {
        return Ok(profile.base_attributes.clone());
    }

    let snapshot = snapshot_reference::Entity::find()
        .filter(snapshot_reference::Column::ProfileId.eq(profile.id))
        .filter(snapshot_reference::Column::ChapterNumber.eq(chapter_number))
        .one(db)
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| {
            format!(
                "角色 {} 缺少第 {} 章快照，无法回滚档案状态",
                profile.name, chapter_number
            )
        })?;

    snapshot_attributes_from_data(&snapshot.data, chapter_number).map_err(|err| err.to_string())
}

fn profile_state_should_restore_base(
    profile: &protagonist_profile::Model,
    chapter_number: i32,
) -> bool {
    chapter_number <= 0
        || profile_first_seen_chapter(profile).is_some_and(|first_seen| chapter_number < first_seen)
}

pub(super) async fn preflight_chapter_derived_cleanup(
    state: &AppState,
    project_id: i64,
    chapter_number: i32,
) -> Result<(), String> {
    let profiles = protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .order_by_asc(protagonist_profile::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(|err| format!("章节派生数据清理预检失败：查询角色档案失败: {err}"))?;
    let snapshot_chapter = chapter_number - 1;
    let snapshot_profile_ids = profiles
        .iter()
        .filter(|profile| profile_cleanup_requires_snapshot(profile, chapter_number))
        .map(|profile| profile.id)
        .collect::<Vec<_>>();
    if snapshot_profile_ids.is_empty() {
        return Ok(());
    }

    let existing_snapshot_profile_ids = snapshot_reference::Entity::find()
        .filter(snapshot_reference::Column::ProfileId.is_in(snapshot_profile_ids))
        .filter(snapshot_reference::Column::ChapterNumber.eq(snapshot_chapter))
        .all(state.db.as_ref())
        .await
        .map_err(|err| format!("章节派生数据清理预检失败：查询角色快照失败: {err}"))?
        .into_iter()
        .map(|snapshot| snapshot.profile_id)
        .collect::<BTreeSet<_>>();
    let missing_names = profiles_missing_cleanup_snapshots(
        &profiles,
        &existing_snapshot_profile_ids,
        chapter_number,
    );
    if missing_names.is_empty() {
        return Ok(());
    }

    Err(format!(
        "章节派生数据清理预检失败：以下角色缺少第 {snapshot_chapter} 章快照，无法在重写第 {chapter_number} 章前回滚档案状态：{}",
        missing_names.join("、")
    ))
}

fn profile_cleanup_requires_snapshot(
    profile: &protagonist_profile::Model,
    chapter_number: i32,
) -> bool {
    profile.last_synced_chapter >= chapter_number
        && !profile_state_should_restore_base(profile, chapter_number - 1)
}

fn profiles_missing_cleanup_snapshots(
    profiles: &[protagonist_profile::Model],
    existing_snapshot_profile_ids: &BTreeSet<i64>,
    chapter_number: i32,
) -> Vec<String> {
    profiles
        .iter()
        .filter(|profile| profile_cleanup_requires_snapshot(profile, chapter_number))
        .filter(|profile| !existing_snapshot_profile_ids.contains(&profile.id))
        .map(|profile| profile.name.clone())
        .collect()
}

pub async fn cleanup_chapter_derived_data(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
    chapter_number: i32,
    include_generated_outputs: bool,
) -> Result<(), String> {
    let txn = state.db.begin().await.map_err(|err| err.to_string())?;
    let artifacts = cleanup_chapter_derived_data_in(
        state,
        &txn,
        project_id,
        chapter_id,
        chapter_number,
        include_generated_outputs,
    )
    .await?;
    validate_chapter_media_file_paths(&state.config.data_dir, &artifacts.media_file_paths)?;
    txn.commit().await.map_err(|err| err.to_string())?;
    finish_chapter_derived_cleanup(state, project_id, chapter_id, chapter_number, artifacts)
        .await?;
    Ok(())
}

pub async fn cleanup_chapter_derived_data_in<C>(
    state: &AppState,
    db: &C,
    project_id: i64,
    chapter_id: i64,
    chapter_number: i32,
    include_generated_outputs: bool,
) -> Result<ChapterDerivedCleanupArtifacts, String>
where
    C: ConnectionTrait,
{
    let mut artifacts = ChapterDerivedCleanupArtifacts::default();
    let character_artifacts =
        cleanup_character_profile_timeline_for_chapter_in(state, db, project_id, chapter_number)
            .await?;
    artifacts
        .stale_profile_ids
        .extend(character_artifacts.stale_profile_ids);
    artifacts
        .stale_character_state_ids
        .extend(character_artifacts.stale_character_state_ids);
    artifacts
        .stale_attribute_event_ids
        .extend(character_artifacts.stale_attribute_event_ids);
    artifacts
        .media_file_paths
        .extend(character_artifacts.media_file_paths);

    artifacts.scene_records_changed = state
        .scene_profile
        .delete_chapter_scene_sync_in(db, project_id, chapter_id)
        .await
        .map_err(|err| err.to_string())?;

    if include_generated_outputs {
        let foreshadowing_artifacts =
            cleanup_foreshadowing_for_chapter_in(db, project_id, chapter_number).await?;
        artifacts
            .stale_foreshadowing_ids
            .extend(foreshadowing_artifacts.stale_ids);
        artifacts
            .changed_foreshadowing_ids
            .extend(foreshadowing_artifacts.changed_ids);
        artifacts.extend_media_file_paths(
            cleanup_chapter_media_data_in(db, project_id, chapter_id).await?,
        );
    }

    Ok(artifacts)
}

pub async fn delete_chapter_content_record_in<C>(
    state: &AppState,
    db: &C,
    project_id: i64,
    chapter_id: i64,
    chapter_number: i32,
) -> Result<ChapterDerivedCleanupArtifacts, String>
where
    C: ConnectionTrait,
{
    let artifacts =
        cleanup_chapter_derived_data_in(state, db, project_id, chapter_id, chapter_number, true)
            .await?;
    cleanup_chapter_optimization_data_in(db, chapter_id).await?;
    chapter::Entity::delete_by_id(chapter_id)
        .exec(db)
        .await
        .map_err(|err| format!("删除章节正文记录失败: {err}"))?;
    Ok(artifacts)
}

pub async fn delete_latest_chapter_outlines_cascade(
    state: &AppState,
    project_id: i64,
    count: u64,
) -> Result<(), String> {
    let outlines = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .order_by_asc(chapter_outline::Column::ChapterNumber)
        .all(state.db.as_ref())
        .await
        .map_err(|err| format!("查询章节大纲失败: {err}"))?;

    let to_delete = latest_chapter_outlines_to_delete(&outlines, count)?;
    let outline_ids = to_delete
        .iter()
        .map(|outline| outline.id)
        .collect::<Vec<_>>();
    let chapter_numbers = to_delete
        .iter()
        .map(|outline| outline.chapter_number)
        .collect::<Vec<_>>();

    if let Some(chapter_number) = state
        .chapter
        .active_generation_chapter(project_id, &chapter_numbers)
    {
        return Err(generating_chapter_delete_error(chapter_number));
    }

    let mut chapters = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::ChapterNumber.is_in(chapter_numbers))
        .order_by_asc(chapter::Column::ChapterNumber)
        .all(state.db.as_ref())
        .await
        .map_err(|err| format!("查询待级联删除章节失败: {err}"))?;
    if let Some(chapter) = chapters
        .iter()
        .find(|chapter| chapter.status == "generating")
    {
        return Err(generating_chapter_delete_error(chapter.chapter_number));
    }

    let txn = state.db.begin().await.map_err(|err| err.to_string())?;
    let mut cleanup_results = Vec::new();
    for chapter in chapters.drain(..) {
        let artifacts = delete_chapter_content_record_in(
            state,
            &txn,
            project_id,
            chapter.id,
            chapter.chapter_number,
        )
        .await?;
        cleanup_results.push((chapter.id, chapter.chapter_number, artifacts));
    }
    for (_, chapter_number, artifacts) in &cleanup_results {
        validate_chapter_media_file_paths(&state.config.data_dir, &artifacts.media_file_paths)
            .map_err(|err| {
                format!("章节派生媒体文件清理预检失败：chapter_number={chapter_number}：{err}")
            })?;
    }

    chapter_outline::Entity::delete_many()
        .filter(chapter_outline::Column::Id.is_in(outline_ids))
        .exec(&txn)
        .await
        .map_err(|err| format!("删除章节大纲失败: {err}"))?;

    txn.commit().await.map_err(|err| err.to_string())?;

    for (chapter_id, chapter_number, artifacts) in cleanup_results {
        finish_chapter_derived_cleanup(state, project_id, chapter_id, chapter_number, artifacts)
            .await?;
    }
    super::refresh_project_current_chapter(state, project_id)
        .await
        .map_err(|err| err.to_string())?;
    super::sync_project_completion_status(state, project_id)
        .await
        .map_err(|err| err.to_string())?;
    Ok(())
}

fn latest_chapter_outlines_to_delete(
    outlines: &[chapter_outline::Model],
    count: u64,
) -> Result<Vec<chapter_outline::Model>, String> {
    if count == 0 {
        return Err("删除章纲数量必须大于 0".to_string());
    }
    if count as usize > outlines.len() {
        return Err(format!(
            "删除章纲数量超过现有章纲数量：count={count}, total={}",
            outlines.len()
        ));
    }

    let mut seen = BTreeSet::new();
    for outline in outlines {
        if outline.chapter_number <= 0 {
            return Err(format!(
                "已有章节大纲章节号无效：chapter_number={}，请先清理章纲状态后再删除",
                outline.chapter_number
            ));
        }
        if !seen.insert(outline.chapter_number) {
            return Err(format!(
                "第 {} 章已有重复章节大纲，请先清理章纲状态后再删除",
                outline.chapter_number
            ));
        }
    }

    let mut sorted = outlines.to_vec();
    sorted.sort_by_key(|outline| (outline.chapter_number, outline.id));
    Ok(sorted.into_iter().rev().take(count as usize).collect())
}

fn generating_chapter_delete_error(chapter_number: i32) -> String {
    format!("第 {chapter_number} 章正在生成中，请等待当前生成完成后再删除章纲")
}

fn validate_chapter_media_file_paths(data_dir: &Path, paths: &[String]) -> Result<(), String> {
    for path in paths {
        deletable_data_file_path(data_dir, path, "章节媒体文件")?;
    }
    Ok(())
}

async fn finish_chapter_derived_cleanup(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
    chapter_number: i32,
    artifacts: ChapterDerivedCleanupArtifacts,
) -> Result<(), String> {
    state
        .rag
        .delete_chapter_by_id(project_id, chapter_id, chapter_number)
        .await
        .map_err(|err| format!("清理章节 RAG 数据失败：chapter_id={chapter_id}：{err}"))?;

    delete_protagonist_rag_by_ids(state, project_id, &artifacts.stale_profile_ids)
        .await
        .map_err(|err| {
            format!(
                "清理角色档案 RAG 数据失败：chapter_number={chapter_number}, profile_ids={:?}：{err}",
                artifacts.stale_profile_ids
            )
        })?;
    delete_character_state_rag_by_ids(state, project_id, &artifacts.stale_character_state_ids)
        .await
        .map_err(|err| {
            format!(
                "清理角色状态 RAG 数据失败：chapter_number={chapter_number}, state_ids={:?}：{err}",
                artifacts.stale_character_state_ids
            )
        })?;
    delete_character_attribute_event_rag_by_ids(
        state,
        project_id,
        &artifacts.stale_attribute_event_ids,
    )
    .await
    .map_err(|err| {
        format!(
            "清理角色属性变更 RAG 数据失败：chapter_number={chapter_number}, attribute_event_ids={:?}：{err}",
            artifacts.stale_attribute_event_ids
        )
    })?;
    delete_foreshadowing_rag_by_ids(state, project_id, &artifacts.stale_foreshadowing_ids)
        .await
        .map_err(|err| {
            format!(
                "清理伏笔 RAG 数据失败：chapter_number={chapter_number}, foreshadowing_ids={:?}：{err}",
                artifacts.stale_foreshadowing_ids
            )
        })?;
    reingest_foreshadowing_rag_by_ids(state, project_id, &artifacts.changed_foreshadowing_ids)
        .await
        .map_err(|err| {
            format!(
                "更新伏笔 RAG 数据失败：chapter_number={chapter_number}, foreshadowing_ids={:?}：{err}",
                artifacts.changed_foreshadowing_ids
            )
        })?;
    if artifacts.scene_records_changed {
        crate::rag_ingest::sync_scene_rag_records(state, project_id)
            .await
            .map_err(|err| {
                format!("更新场景档案 RAG 数据失败：chapter_number={chapter_number}：{err}")
            })?;
    }

    for path in artifacts.media_file_paths {
        remove_data_file_in_data_dir(&state.config.data_dir, &path, "章节媒体文件")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .expect("system clock must be after UNIX_EPOCH");
        std::env::temp_dir().join(format!("{prefix}_{}_{}", std::process::id(), stamp))
    }

    fn profile_for_context(last_synced_chapter: i32) -> protagonist_profile::Model {
        let now = chrono::Utc::now().naive_utc();
        protagonist_profile::Model {
            id: 1,
            project_id: 1,
            name: "林岚".to_string(),
            gender: None,
            age: None,
            base_attributes: serde_json::json!({}),
            current_state: serde_json::json!({
                "explicit_attributes": {
                    "appearance": "穿青色劲装，左腕有旧伤",
                    "weapon": "短刀"
                },
                "implicit_attributes": {
                    "personality": "谨慎但会为弱者冒险"
                },
                "social_attributes": {
                    "身份": "边城医馆学徒"
                }
            }),
            character_arc: None,
            voice_fingerprint: None,
            appearance: None,
            background: None,
            personality: None,
            last_synced_chapter,
            created_at: now,
            updated_at: now,
        }
    }

    fn outline_for_delete(id: i64, chapter_number: i32) -> chapter_outline::Model {
        let now = chrono::Utc::now().naive_utc();
        chapter_outline::Model {
            id,
            project_id: 1,
            part_id: None,
            chapter_number,
            title: Some(format!("第 {chapter_number} 章")),
            summary: Some("章节摘要".to_string()),
            beats: None,
            key_scenes: None,
            pacing_score: None,
            foreshadowing_setup: None,
            foreshadowing_payoff: None,
            characters_involved: None,
            word_count_target: None,
            status: "draft".to_string(),
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn latest_chapter_outlines_to_delete_selects_highest_chapter_numbers() {
        let outlines = [
            outline_for_delete(11, 1),
            outline_for_delete(12, 3),
            outline_for_delete(13, 2),
        ];

        let selected = latest_chapter_outlines_to_delete(&outlines, 2)
            .expect("valid outlines should select latest chapter numbers");

        let selected_numbers = selected
            .iter()
            .map(|outline| outline.chapter_number)
            .collect::<Vec<_>>();
        assert_eq!(selected_numbers, vec![3, 2]);
    }

    #[test]
    fn latest_chapter_outlines_to_delete_rejects_duplicate_chapter_number() {
        let outlines = [
            outline_for_delete(11, 1),
            outline_for_delete(12, 2),
            outline_for_delete(13, 2),
        ];

        let err = latest_chapter_outlines_to_delete(&outlines, 1)
            .expect_err("duplicate chapter outlines must fail before cascade cleanup");

        assert!(err.contains("第 2 章已有重复章节大纲"));
    }

    #[test]
    fn latest_chapter_outlines_to_delete_rejects_invalid_chapter_number() {
        let outlines = [outline_for_delete(11, 0)];

        let err = latest_chapter_outlines_to_delete(&outlines, 1)
            .expect_err("invalid chapter number must fail before cascade cleanup");

        assert!(err.contains("章节号无效"));
    }

    #[test]
    fn profile_state_restore_base_before_first_seen_chapter() {
        let mut profile = profile_for_context(2);
        profile.base_attributes = serde_json::json!({
            "explicit_attributes": {
                "first_seen_chapter": 2,
                "role": "西市胡商"
            },
            "implicit_attributes": {},
            "social_attributes": {}
        });

        assert!(profile_state_should_restore_base(&profile, 0));
        assert!(profile_state_should_restore_base(&profile, 1));
        assert!(!profile_state_should_restore_base(&profile, 2));
    }

    #[test]
    fn profile_first_seen_chapter_extracts_text_derived_origin() {
        let mut profile = profile_for_context(0);
        profile.base_attributes = serde_json::json!({
            "explicit_attributes": {
                "first_seen_chapter": 5
            },
            "implicit_attributes": {},
            "social_attributes": {}
        });

        assert_eq!(profile_first_seen_chapter(&profile), Some(5));
    }

    #[test]
    fn profile_state_restore_base_without_first_seen_only_for_initial_state() {
        let profile = profile_for_context(2);

        assert!(profile_state_should_restore_base(&profile, 0));
        assert!(!profile_state_should_restore_base(&profile, 1));
    }

    #[test]
    fn cleanup_preflight_requires_snapshot_when_rewriting_synced_chapter() {
        let profile = profile_for_context(5);

        assert!(profile_cleanup_requires_snapshot(&profile, 3));
        assert!(!profile_cleanup_requires_snapshot(&profile, 6));
    }

    #[test]
    fn cleanup_preflight_reports_profiles_missing_required_snapshot() {
        let profile = profile_for_context(5);
        let missing = profiles_missing_cleanup_snapshots(&[profile], &BTreeSet::new(), 3);

        assert_eq!(missing, vec!["林岚".to_string()]);
    }

    #[test]
    fn cleanup_preflight_accepts_existing_required_snapshot() {
        let profile = profile_for_context(5);
        let mut existing = BTreeSet::new();
        existing.insert(profile.id);

        let missing = profiles_missing_cleanup_snapshots(&[profile], &existing, 3);

        assert!(missing.is_empty());
    }

    #[test]
    fn media_file_preflight_accepts_existing_and_missing_data_files() {
        let data_dir = unique_temp_dir("a4n_chapter_cleanup_media_ok");
        let existing_path = data_dir.join("projects/1/images/ref.png");
        std::fs::create_dir_all(existing_path.parent().expect("nested parent"))
            .expect("create nested dir");
        std::fs::write(&existing_path, b"demo").expect("write existing file");

        validate_chapter_media_file_paths(
            &data_dir,
            &[
                "projects/1/images/ref.png".to_string(),
                "projects/1/images/missing.png".to_string(),
            ],
        )
        .expect("existing and missing in-data-dir files should pass preflight");

        assert!(existing_path.exists(), "preflight must not delete files");
        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn media_file_preflight_rejects_paths_outside_data_dir() {
        let root = unique_temp_dir("a4n_chapter_cleanup_media_bad");
        let data_dir = root.join("data");
        std::fs::create_dir_all(&data_dir).expect("create data dir");

        let err = validate_chapter_media_file_paths(&data_dir, &["../outside.png".to_string()])
            .expect_err("traversal media path must fail before DB commit");

        assert!(err.contains("不在应用数据目录内"));
        let _ = std::fs::remove_dir_all(&root);
    }
}
