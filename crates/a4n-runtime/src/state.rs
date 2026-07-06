//! 应用全局运行时状态。
//!
//! 这个状态不依赖 Tauri 或 Axum，用于让桌面端和 Web server 共享同一套
//! services、engine、队列和存储初始化逻辑。

use crate::prompt_overrides::{
    MissingProjectMessage, ProjectOverrideIdentity, ensure_project_exists,
    parse_project_override_key, validate_project_override_content,
};
use a4n_core::config::app_config::AppConfig;
use a4n_core::event::bridge::BroadcastEventBus as CoreEventBus;
use a4n_core::models::ai::system_settings;
use a4n_engine::embedding::provider::EmbeddingProvider;
use a4n_engine::llm::provider::LlmProvider;
use a4n_engine::prompt::engine::PromptEngine;
use a4n_engine::prompt::tera_engine::TeraPromptEngine;
use a4n_services::admin::AdminService;
use a4n_services::asset::ObjectProfileService;
use a4n_services::asset_lineage_sync::AssetLineageSyncService;
use a4n_services::auth::AuthService;
use a4n_services::blueprint::BlueprintService;
use a4n_services::canvas::CanvasService;
use a4n_services::chapter::ChapterService;
use a4n_services::character::CharacterService;
use a4n_services::character::snapshot_store::Sha256SnapshotStore;
use a4n_services::collaboration::ProjectCollaborationService;
use a4n_services::config::ConfigService;
use a4n_services::export::ExportService;
use a4n_services::galgame::GalgameService;
use a4n_services::generation::GenerationRunService;
use a4n_services::image::ImageService;
use a4n_services::import::ImportService;
use a4n_services::inspiration::InspirationService;
use a4n_services::manga::MangaService;
use a4n_services::media::MediaAssetService;
use a4n_services::optimize::OptimizeService;
use a4n_services::outline::OutlineService;
use a4n_services::portrait::PortraitService;
use a4n_services::project::ProjectService;
use a4n_services::queue::PersistentTaskQueue;
use a4n_services::rag::RagService;
use a4n_services::scene::SceneProfileService;
use a4n_services::source_sync::SourceSyncService;
use a4n_services::text_anchor::TextAnchorService;
use a4n_services::theme::ThemeService;
use a4n_services::video::VideoService;
use a4n_services::workflow::WorkflowRunService;
use sea_orm::{ConnectionTrait, DatabaseConnection, EntityTrait, Statement};
use std::collections::hash_map::Entry;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const CONTRACT_BOUND_PROMPT_PATHS: &[&str] = &[
    "novel/04_writing/writing.md",
    "novel/05_analysis/content_optimization.md",
    "novel/05_analysis/content_optimization_chat.md",
    "novel/05_analysis/content_optimization_patch.md",
    "novel/05_analysis/chapter_fact_extraction.md",
    "novel/05_analysis/chapter_summary_synthesis.md",
    "novel/05_analysis/chapter_character_state.md",
    "novel/05_analysis/chapter_foreshadowing_events.md",
    "novel/05_analysis/object_discovery.md",
    "novel/05_analysis/scene_profile_candidates.md",
    "novel/05_analysis/scene_occurrence_extraction.md",
    "novel/05_analysis/scene_transition_extraction.md",
    "novel/05_analysis/scene_entity_state_extraction.md",
    "novel/06_manga/manga_event_beats.md",
    "novel/06_manga/manga_character_binding.md",
    "novel/06_manga/manga_speech_selection.md",
    "novel/06_manga/manga_visual_anchor_extraction.md",
    "novel/06_manga/manga_item_extraction.md",
    "novel/06_manga/manga_chapter_comic_synthesis.md",
    "novel/06_manga/manga_page_planning.md",
    "novel/06_manga/manga_page_drawing_prompt.md",
    "novel/07_protagonist/character_portrait_generation.md",
    "novel/08_video/chapter_video_builder_rules.md",
    "novel/08_video/chapter_video_provider_prompt.md",
    "novel/08_video/chapter_video_first_frame_provider_prompt.md",
    "novel/08_video/chapter_video_first_frame_prompts.md",
    "novel/08_video/chapter_video_first_frame_video_prompt.md",
    "novel/08_video/chapter_video_first_frame_video_prompt_review.md",
    "novel/08_video/chapter_video_frame_prompts.md",
    "novel/08_video/chapter_video_prompt.md",
    "novel/08_video/chapter_video_prompt_review.md",
    "novel/08_video/chapter_video_chat_prompt.md",
    "novel/08_video/chapter_video_chat_prompt_review.md",
    "novel/09_galgame/scene_segmentation.md",
    "novel/09_galgame/script_adaptation.md",
    "novel/09_galgame/character_staging.md",
    "novel/09_galgame/visual_planning.md",
    "novel/09_galgame/branch_design.md",
];

const RETIRED_BACKEND_PROMPT_NAMES: &[&str] = &[
    "content_optimization_agent_decision",
    "content_optimization_rewrite",
];

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<sea_orm::DatabaseConnection>,
    pub config: AppConfig,
    pub event_bus: Arc<CoreEventBus>,
    pub task_manager: Arc<a4n_core::event::channel::TaskManager>,

    pub project: ProjectService,
    pub collaboration: ProjectCollaborationService,
    pub object_profile: ObjectProfileService,
    pub scene_profile: SceneProfileService,
    pub blueprint: BlueprintService,
    pub canvas: CanvasService,
    pub outline: OutlineService,
    pub chapter: ChapterService,
    pub config_svc: ConfigService,
    pub inspiration: InspirationService,
    pub character: CharacterService,
    pub portrait: PortraitService,
    pub manga: MangaService,
    pub galgame: GalgameService,
    pub image: ImageService,
    pub video: VideoService,
    pub optimize: OptimizeService,
    pub export: ExportService,
    pub import: ImportService,
    pub admin: AdminService,
    pub rag: RagService,
    pub text_anchor: TextAnchorService,
    pub theme: ThemeService,
    pub auth: AuthService,
    pub generation_run: GenerationRunService,
    pub workflow_run: WorkflowRunService,
    pub media_asset: MediaAssetService,
    pub asset_lineage_sync: AssetLineageSyncService,
    pub source_sync: SourceSyncService,
    pub task_queue: Arc<PersistentTaskQueue>,

    pub llm: Arc<dyn LlmProvider>,
    pub engine: Arc<dyn PromptEngine>,
    pub embedding: Arc<dyn EmbeddingProvider>,
    pub snapshot_store: Arc<Sha256SnapshotStore>,
}

impl AppState {
    pub async fn new(config: AppConfig) -> Result<Self, a4n_core::error::AppError> {
        validate_runtime_config(&config)?;

        let db = sea_orm::Database::connect(&config.database_url)
            .await
            .map_err(a4n_core::error::AppError::Database)?;
        let db = Arc::new(db);

        a4n_migration::migrator::run_migrations(&db)
            .await
            .map_err(a4n_core::error::AppError::Database)?;
        ensure_compatible_schema(&db)
            .await
            .map_err(a4n_core::error::AppError::Database)?;
        crate::baseline_configs::ensure_baseline_model_configs(&db, &config).await?;

        let event_bus = Arc::new(CoreEventBus::new(config.system.event_bus_capacity));
        let task_manager = Arc::new(a4n_core::event::channel::TaskManager::new());
        let task_queue = Arc::new(PersistentTaskQueue::new(db.clone()).await?);
        let prompt_engine = create_prompt_engine(db.as_ref(), &config).await?;

        let lance_vec = Arc::new(
            a4n_engine::rag::lance_store::LanceVectorStore::open(config.data_dir.join("lance"))
                .await?,
        );
        let tantivy_idx = Arc::new(a4n_engine::rag::tantivy_index::TantivyTextIndex::new(
            config.data_dir.join("tantivy"),
        )?);
        let vector_store: Arc<dyn a4n_engine::rag::vector_store::VectorStore> = Arc::new(
            a4n_engine::rag::hybrid_store::HybridVectorStore::new(lance_vec, tantivy_idx),
        );

        Ok(Self {
            project: ProjectService::new(db.clone(), event_bus.clone()),
            collaboration: ProjectCollaborationService::new(db.clone()),
            object_profile: ObjectProfileService::new(db.clone()),
            scene_profile: SceneProfileService::new(db.clone()),
            blueprint: BlueprintService::new(db.clone(), event_bus.clone()),
            canvas: CanvasService::new(db.clone()),
            outline: OutlineService::new(db.clone()),
            chapter: ChapterService::new(db.clone(), event_bus.clone()),
            config_svc: ConfigService::new(db.clone()),
            inspiration: InspirationService::new(db.clone()),
            character: CharacterService::new(db.clone()),
            portrait: PortraitService::new(db.clone()),
            manga: MangaService::new(db.clone()),
            galgame: GalgameService::new(db.clone()),
            image: ImageService::new(db.clone()),
            video: VideoService::new(db.clone()),
            optimize: OptimizeService::new(db.clone()),
            export: ExportService::new(db.clone()),
            import: ImportService::new(db.clone()),
            admin: AdminService::new(db.clone()),
            rag: RagService::new(vector_store),
            text_anchor: TextAnchorService::new(db.clone()),
            theme: ThemeService::new(db.clone()),
            auth: AuthService::new(db.clone()),
            generation_run: GenerationRunService::new(db.clone()),
            workflow_run: WorkflowRunService::new(db.clone()),
            media_asset: MediaAssetService::new(db.clone(), config.data_dir.clone()),
            asset_lineage_sync: AssetLineageSyncService::new(db.clone()),
            source_sync: SourceSyncService::new(db.clone()),
            task_queue: task_queue.clone(),
            llm: Arc::new(crate::dynamic_llm::DynamicLlmProvider::new(
                db.clone(),
                task_queue.clone(),
            )),
            engine: Arc::new(prompt_engine),
            embedding: Arc::new(crate::dynamic_embedding::DynamicEmbeddingProvider::new(
                db.clone(),
                task_queue,
                config.embedding.clone(),
                crate::dynamic_embedding::default_fastembed_cache_dir(&config),
            )),
            snapshot_store: Arc::new(Sha256SnapshotStore::new(db.clone())),
            db,
            config,
            event_bus,
            task_manager,
        })
    }
}

async fn ensure_compatible_schema(db: &DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    let backend = db.get_database_backend();
    for sql in [
        "ALTER TABLE IF EXISTS workflow_checkpoint ADD COLUMN IF NOT EXISTS input_fingerprint_sha256 TEXT",
        "ALTER TABLE IF EXISTS generation_run ADD COLUMN IF NOT EXISTS workflow_run_id BIGINT",
        "ALTER TABLE IF EXISTS generation_run ADD COLUMN IF NOT EXISTS call_key TEXT",
        "ALTER TABLE IF EXISTS generation_run ADD COLUMN IF NOT EXISTS input_fingerprint_sha256 TEXT",
        "ALTER TABLE IF EXISTS generation_run ADD COLUMN IF NOT EXISTS input_fingerprint_json JSONB",
        "ALTER TABLE IF EXISTS generation_run ADD COLUMN IF NOT EXISTS raw_response_json JSONB",
        "ALTER TABLE IF EXISTS generation_run ADD COLUMN IF NOT EXISTS parsed_output_json JSONB",
        "ALTER TABLE IF EXISTS generation_run ADD COLUMN IF NOT EXISTS validation_json JSONB",
        "ALTER TABLE IF EXISTS generation_run ADD COLUMN IF NOT EXISTS artifact_refs_json JSONB",
        "ALTER TABLE IF EXISTS generation_run ADD COLUMN IF NOT EXISTS attempt INTEGER NOT NULL DEFAULT 1",
        "ALTER TABLE IF EXISTS task_queue ADD COLUMN IF NOT EXISTS workflow_run_id BIGINT",
        "CREATE INDEX IF NOT EXISTS idx_workflow_checkpoint_run_step_fingerprint ON workflow_checkpoint (workflow_run_id, step_key, input_fingerprint_sha256)",
        "CREATE INDEX IF NOT EXISTS idx_generation_run_workflow_run ON generation_run (workflow_run_id)",
        "CREATE INDEX IF NOT EXISTS idx_generation_run_workflow_step_call ON generation_run (workflow_run_id, step_key, call_key)",
        "CREATE INDEX IF NOT EXISTS idx_generation_run_workflow_call_fingerprint ON generation_run (workflow_run_id, step_key, call_key, input_fingerprint_sha256)",
        "CREATE INDEX IF NOT EXISTS idx_generation_run_workflow_status ON generation_run (workflow_run_id, status)",
        "CREATE INDEX IF NOT EXISTS idx_task_queue_workflow_run ON task_queue (workflow_run_id)",
    ] {
        db.execute(Statement::from_string(backend, sql.to_string()))
            .await?;
    }
    db.execute(Statement::from_string(
        backend,
        r#"
        CREATE TABLE IF NOT EXISTS profile_primary_material (
            id BIGSERIAL PRIMARY KEY,
            project_id BIGINT NOT NULL,
            profile_kind TEXT NOT NULL,
            profile_id BIGINT NOT NULL,
            primary_image_id BIGINT NOT NULL,
            created_at TIMESTAMP NOT NULL,
            updated_at TIMESTAMP NOT NULL
        )
        "#,
    ))
    .await?;
    db.execute(Statement::from_string(
        backend,
        r#"
        CREATE UNIQUE INDEX IF NOT EXISTS idx_profile_primary_material_unique
        ON profile_primary_material (project_id, profile_kind, profile_id)
        "#,
    ))
    .await?;
    Ok(())
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("config", &self.config)
            .finish()
    }
}

fn validate_runtime_config(config: &AppConfig) -> Result<(), a4n_core::error::AppError> {
    if !config.database_url.starts_with("postgres://")
        && !config.database_url.starts_with("postgresql://")
    {
        return Err(a4n_core::error::AppError::ConfigError(
            "A4N_DATABASE_URL 必须是 postgres:// 或 postgresql:// 连接串".to_string(),
        ));
    }
    if config
        .security
        .jwt_secret
        .as_deref()
        .map(str::trim)
        .filter(|secret| !secret.is_empty())
        .is_none()
    {
        return Err(a4n_core::error::AppError::ConfigError(
            "A4N_JWT_SECRET 不能为空".to_string(),
        ));
    }
    Ok(())
}

async fn create_prompt_engine(
    db: &DatabaseConnection,
    config: &AppConfig,
) -> Result<TeraPromptEngine, a4n_core::error::AppError> {
    let prompts_dir = ensure_portable_prompts_dir(&config.data_dir)
        .map_err(a4n_core::error::AppError::ConfigError)?;
    let base = TeraPromptEngine::from_directory(&prompts_dir)?;
    let engine = base.with_git_store(Arc::new(
        a4n_engine::prompt::git_store::GitPromptStore::open_or_init(
            config.data_dir.join("prompts.git"),
        )?,
    ))?;
    hydrate_project_overrides_from_settings(db, &engine)
        .await
        .map_err(a4n_core::error::AppError::ConfigError)?;
    Ok(engine)
}

async fn hydrate_project_overrides_from_settings(
    db: &DatabaseConnection,
    engine: &TeraPromptEngine,
) -> Result<usize, String> {
    let plan = load_project_override_hydration_plan(db).await?;
    let overrides = plan
        .into_iter()
        .map(|item| (item.identity.project_id, item.identity.name, item.content))
        .collect();
    engine
        .hydrate_project_overrides(overrides)
        .map_err(|e| e.to_string())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProjectOverrideHydration {
    identity: ProjectOverrideIdentity,
    content: String,
}

async fn load_project_override_hydration_plan(
    db: &DatabaseConnection,
) -> Result<Vec<ProjectOverrideHydration>, String> {
    let records = system_settings::Entity::find()
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    let plan = project_override_hydration_candidates_from_records(&records)?;

    for item in &plan {
        ensure_project_exists(db, item.identity.project_id, MissingProjectMessage::Hydrate).await?;
    }

    Ok(plan)
}

fn project_override_hydration_candidates_from_records(
    records: &[system_settings::Model],
) -> Result<Vec<ProjectOverrideHydration>, String> {
    let mut plan = Vec::new();

    for record in records {
        let Some(identity) = parse_project_override_key(&record.key)? else {
            continue;
        };
        let content = project_override_record_content(record)?;
        plan.push(ProjectOverrideHydration { identity, content });
    }

    Ok(plan)
}

fn project_override_record_content(record: &system_settings::Model) -> Result<String, String> {
    let content = record
        .value
        .as_str()
        .ok_or_else(|| format!("提示词覆盖记录 value 必须是字符串: {}", record.key))?;

    validate_project_override_content(content)?;

    Ok(content.to_string())
}

fn ensure_portable_prompts_dir(data_dir: &Path) -> Result<PathBuf, String> {
    let portable_dir = data_dir.join("prompts");
    if portable_dir.join("_registry.yaml").is_file() {
        sync_missing_bundled_prompts(data_dir, &portable_dir)
            .map_err(|err| format!("同步随包新增 prompts 失败：{err}"))?;
        tracing::info!("已定位用户 prompts 目录：{portable_dir:?}");
        return Ok(portable_dir);
    }

    for source in bundled_prompts_candidates(data_dir) {
        if source.join("_registry.yaml").is_file() {
            if let Err(err) = copy_dir_missing(&source, &portable_dir) {
                tracing::warn!(
                    source = ?source,
                    dest = ?portable_dir,
                    error = %err,
                    "初始化用户 prompts 目录失败"
                );
                continue;
            }
            tracing::info!(
                source = ?source,
                dest = ?portable_dir,
                "已初始化用户 prompts 目录"
            );
            return Ok(portable_dir);
        }
    }

    Err("未找到 prompts 目录（已在 DATA_DIR/CWD/EXE_DIR/share/a4n 下查找）".to_string())
}

fn sync_missing_bundled_prompts(data_dir: &Path, portable_dir: &Path) -> Result<(), String> {
    for source in bundled_prompts_candidates(data_dir) {
        if source == portable_dir || !source.join("_registry.yaml").is_file() {
            continue;
        }
        copy_dir_missing(&source, portable_dir).map_err(|err| {
            format!(
                "复制缺失提示词文件失败：source={}, dest={}：{err}",
                source.display(),
                portable_dir.display()
            )
        })?;
        merge_missing_prompt_registry_entries(&source, portable_dir)?;
        remove_retired_backend_prompt_registry_entries(portable_dir)?;
        sync_contract_bound_prompts(&source, portable_dir)?;
        tracing::info!(
            source = ?source,
            dest = ?portable_dir,
            "已同步随包新增 prompts"
        );
        return Ok(());
    }
    Ok(())
}

fn remove_retired_backend_prompt_registry_entries(dest_dir: &Path) -> Result<(), String> {
    let dest_path = dest_dir.join("_registry.yaml");
    let mut dest = read_registry_file(&dest_path)?;
    let mut removed = Vec::new();

    for key in RETIRED_BACKEND_PROMPT_NAMES {
        if dest.prompts.remove(*key).is_some() {
            removed.push(*key);
        }
    }

    if removed.is_empty() {
        return Ok(());
    }

    write_registry_file(&dest_path, &dest)?;
    tracing::info!(
        prompts = %removed.join("、"),
        dest = ?dest_dir,
        "已移除停用的后端绑定 prompts 注册项"
    );
    Ok(())
}

fn merge_missing_prompt_registry_entries(source_dir: &Path, dest_dir: &Path) -> Result<(), String> {
    let source_path = source_dir.join("_registry.yaml");
    let dest_path = dest_dir.join("_registry.yaml");
    let source = read_registry_file(&source_path)?;
    let mut dest = read_registry_file(&dest_path)?;
    let mut changed = false;

    if dest.version.is_none() {
        dest.version = source.version.clone();
        changed = true;
    }
    if dest.last_updated.is_none() {
        dest.last_updated = source.last_updated.clone();
        changed = true;
    }
    for (key, value) in source.categories {
        if let Entry::Vacant(entry) = dest.categories.entry(key) {
            entry.insert(value);
            changed = true;
        }
    }
    for (key, value) in source.prompts {
        if let Entry::Vacant(entry) = dest.prompts.entry(key) {
            entry.insert(value);
            changed = true;
        }
    }

    if changed {
        write_registry_file(&dest_path, &dest)?;
    }
    Ok(())
}

fn sync_contract_bound_prompts(source_dir: &Path, dest_dir: &Path) -> Result<(), String> {
    let mut changed = Vec::new();
    for rel_path in CONTRACT_BOUND_PROMPT_PATHS {
        let source_path = source_dir.join(rel_path);
        if !source_path.is_file() {
            continue;
        }
        let dest_path = dest_dir.join(rel_path);
        let source_content = std::fs::read(&source_path)
            .map_err(|err| format!("读取契约提示词失败：{}：{err}", source_path.display()))?;
        let needs_update = match std::fs::read(&dest_path) {
            Ok(dest_content) => dest_content != source_content,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => true,
            Err(err) => {
                return Err(format!(
                    "读取用户契约提示词失败：{}：{err}",
                    dest_path.display()
                ));
            }
        };
        if !needs_update {
            continue;
        }
        if let Some(parent) = dest_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|err| format!("创建契约提示词目录失败：{}：{err}", parent.display()))?;
        }
        write_file_atomically(&dest_path, &source_content)?;
        changed.push(*rel_path);
    }
    if !changed.is_empty() {
        tracing::info!(
            prompts = %changed.join("、"),
            source = ?source_dir,
            dest = ?dest_dir,
            "已同步后端契约绑定 prompts"
        );
    }
    Ok(())
}

fn read_registry_file(path: &Path) -> Result<a4n_engine::prompt::registry::RegistryFile, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|err| format!("读取提示词注册表失败：{}：{err}", path.display()))?;
    serde_yaml::from_str(&text)
        .map_err(|err| format!("解析提示词注册表失败：{}：{err}", path.display()))
}

fn write_registry_file(
    path: &Path,
    registry: &a4n_engine::prompt::registry::RegistryFile,
) -> Result<(), String> {
    let text =
        serde_yaml::to_string(registry).map_err(|err| format!("序列化提示词注册表失败：{err}"))?;
    let temp_path = path.with_extension("yaml.tmp");
    std::fs::write(&temp_path, text)
        .map_err(|err| format!("写入临时提示词注册表失败：{}：{err}", temp_path.display()))?;
    std::fs::rename(&temp_path, path).map_err(|err| {
        let _ = std::fs::remove_file(&temp_path);
        format!(
            "替换提示词注册表失败：{} -> {}：{err}",
            temp_path.display(),
            path.display()
        )
    })
}

fn write_file_atomically(path: &Path, content: &[u8]) -> Result<(), String> {
    let temp_path = path.with_extension("tmp");
    std::fs::write(&temp_path, content)
        .map_err(|err| format!("写入临时文件失败：{}：{err}", temp_path.display()))?;
    std::fs::rename(&temp_path, path).map_err(|err| {
        let _ = std::fs::remove_file(&temp_path);
        format!(
            "替换文件失败：{} -> {}：{err}",
            temp_path.display(),
            path.display()
        )
    })
}

fn bundled_prompts_candidates(data_dir: &Path) -> Vec<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Some(portable_home) = data_dir.parent() {
        candidates.push(portable_home.join("prompts"));
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("prompts"));
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent()
    {
        candidates.push(parent.join("prompts"));
        if let Some(grand) = parent.parent() {
            candidates.push(grand.join("prompts"));
            candidates.push(grand.join("share").join("a4n").join("prompts"));
        }
    }
    candidates
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(label: &str) -> PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        std::env::temp_dir().join(format!("a4n_{label}_{}_{}", std::process::id(), now))
    }

    fn write_valid_prompt_dir(dir: &Path) {
        fs::create_dir_all(dir).expect("create prompt dir");
        fs::write(
            dir.join("existing.md"),
            "---\nname: existing\n---\nExisting",
        )
        .expect("write prompt");
        fs::write(
            dir.join("_registry.yaml"),
            r#"version: "1.0"
prompts:
  existing:
    path: "existing.md"
    title: "Existing"
    category: "writing"
    description: "Existing prompt"
    tags: ["test"]
    version: "1.0"
    used_by: []
    dependencies: []
categories:
  writing:
    label: "写作"
    order: 1
"#,
        )
        .expect("write registry");
    }

    fn system_setting(id: i64, key: &str, value: serde_json::Value) -> system_settings::Model {
        system_settings::Model {
            id,
            key: key.to_string(),
            value,
            category: "test".to_string(),
            description: None,
            updated_at: chrono::Utc::now().naive_utc(),
        }
    }

    #[test]
    fn hydrate_project_overrides_plan_accepts_valid_overrides_and_unrelated_settings() {
        let records = vec![
            system_setting(1, "llm_config", serde_json::json!({"provider": "local"})),
            system_setting(
                2,
                "prompt_override_42_writing_prompt",
                serde_json::json!("Override"),
            ),
            system_setting(
                3,
                "prompt_override_7_prompt_with_underscores",
                serde_json::json!("Another override"),
            ),
        ];

        let plan = project_override_hydration_candidates_from_records(&records).unwrap();

        assert_eq!(
            plan,
            vec![
                ProjectOverrideHydration {
                    identity: ProjectOverrideIdentity {
                        project_id: 42,
                        name: "writing_prompt".to_string(),
                    },
                    content: "Override".to_string(),
                },
                ProjectOverrideHydration {
                    identity: ProjectOverrideIdentity {
                        project_id: 7,
                        name: "prompt_with_underscores".to_string(),
                    },
                    content: "Another override".to_string(),
                },
            ]
        );
    }

    #[test]
    fn hydrate_project_overrides_plan_rejects_invalid_override_key() {
        let records = vec![
            system_setting(1, "prompt_override_42_valid", serde_json::json!("Override")),
            system_setting(2, "prompt_override_42_", serde_json::json!("Bad key")),
        ];

        let err = project_override_hydration_candidates_from_records(&records)
            .expect_err("invalid persisted override key should fail before engine apply");

        assert!(err.contains("提示词覆盖 key name 不能为空"));
    }

    #[test]
    fn hydrate_project_overrides_plan_rejects_bad_override_value() {
        let records = vec![
            system_setting(
                1,
                "prompt_override_42_non_string",
                serde_json::json!({"content": "not a string"}),
            ),
            system_setting(2, "prompt_override_42_blank", serde_json::json!("   ")),
        ];

        let err = project_override_hydration_candidates_from_records(&records)
            .expect_err("non-string persisted override value should fail before engine apply");

        assert!(err.contains("提示词覆盖记录 value 必须是字符串"));

        let err = project_override_hydration_candidates_from_records(&records[1..])
            .expect_err("blank persisted override value should fail before engine apply");

        assert!(err.contains("提示词覆盖内容不能为空"));
    }

    #[test]
    fn existing_user_prompts_sync_failure_is_not_silently_ignored() {
        let root = unique_temp_dir("prompt_sync_failure");
        let data_dir = root.join("data");
        let portable_dir = data_dir.join("prompts");
        let bundled_dir = root.join("prompts");
        write_valid_prompt_dir(&portable_dir);
        fs::create_dir_all(&bundled_dir).expect("create bundled dir");
        fs::write(bundled_dir.join("_registry.yaml"), "prompts: [").expect("write bad registry");

        let result = ensure_portable_prompts_dir(&data_dir);

        fs::remove_dir_all(&root).ok();
        let err = result.expect_err("sync failure should stop prompt engine startup");
        assert!(err.contains("同步随包新增 prompts 失败"));
        assert!(err.contains("解析提示词注册表失败"));
    }

    #[test]
    fn contract_bound_sync_overwrites_stale_writing_prompt() {
        let root = unique_temp_dir("contract_prompt_sync");
        let source_dir = root.join("bundled");
        let dest_dir = root.join("data").join("prompts");
        let rel_path = Path::new("novel/04_writing/writing.md");
        let source_path = source_dir.join(rel_path);
        let dest_path = dest_dir.join(rel_path);

        fs::create_dir_all(source_path.parent().expect("source parent"))
            .expect("create source parent");
        fs::create_dir_all(dest_path.parent().expect("dest parent")).expect("create dest parent");
        fs::write(&source_path, "只返回章节正文纯文本").expect("write bundled writing prompt");
        fs::write(&dest_path, "只返回 title summary full_content JSON")
            .expect("write stale writing prompt");

        sync_contract_bound_prompts(&source_dir, &dest_dir).expect("sync contract prompts");

        assert_eq!(
            fs::read_to_string(&dest_path).expect("read synced writing prompt"),
            "只返回章节正文纯文本"
        );
        fs::remove_dir_all(&root).ok();
    }
}

fn copy_dir_missing(src: &Path, dest: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_missing(&src_path, &dest_path)?;
        } else if file_type.is_file() && !dest_path.exists() {
            if let Some(parent) = dest_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&src_path, &dest_path)?;
        }
    }
    Ok(())
}
