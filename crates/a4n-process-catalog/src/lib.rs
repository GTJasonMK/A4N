mod release;

use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

pub use release::{
    RELEASE_MANIFEST_SCHEMA_VERSION, ReleaseBinaryManifest, ReleaseManifest,
    ReleaseRuntimeFileManifest, ReleaseToolCatalogEntry, ReleaseToolManifest, release_tool_catalog,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessKind {
    #[default]
    CapabilitySidecar,
    StatefulWorker,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MigrationCompatibility {
    #[serde(default)]
    pub current: Option<String>,
    #[serde(default)]
    pub min_compatible: Option<String>,
    #[serde(default)]
    pub max_compatible: Option<String>,
}

/// 统一的进程元数据视图。
///
/// `ServiceCatalogEntry` 保留 sidecar 的历史字段形状；这个类型把 sidecar
/// 和 stateful worker 归一化到同一个查询结果，供生命周期管理器和后续发布
/// 编排使用。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessMetadata {
    pub name: String,
    pub kind: ProcessKind,
    pub binary: String,
    #[serde(default)]
    pub capability_version: Option<String>,
    #[serde(default)]
    pub contract_version: Option<String>,
    pub addr_env: String,
    #[serde(default)]
    pub url_env: Option<String>,
    pub default_addr: String,
    #[serde(default)]
    pub local_only: bool,
    #[serde(default)]
    pub data_scope: Vec<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub migration: MigrationCompatibility,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ServiceCatalogWire {
    name: String,
    binary: String,
    capability_version: String,
    addr_env: String,
    url_env: String,
    default_addr: String,
    #[serde(default)]
    local_only: bool,
    #[serde(default)]
    kind: ProcessKind,
    #[serde(default)]
    contract_version: Option<String>,
    #[serde(default)]
    data_scope: Vec<String>,
    #[serde(default)]
    dependencies: Vec<String>,
    #[serde(default)]
    migration: MigrationCompatibility,
}

/// 进程管理、开发脚本和发布包共享的 sidecar 清单。
///
/// 清单文件是唯一的机器可读来源；运行时和检查脚本都应从这里派生，
/// 不再各自维护服务名、端口和环境变量的平行列表。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ServiceCatalogEntry {
    pub name: String,
    pub binary: String,
    pub capability_version: String,
    pub addr_env: String,
    pub url_env: String,
    pub default_addr: String,
    #[serde(default)]
    pub local_only: bool,
}

impl<'de> Deserialize<'de> for ServiceCatalogEntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        ServiceCatalogWire::deserialize(deserializer).map(Into::into)
    }
}

impl From<ServiceCatalogWire> for ServiceCatalogEntry {
    fn from(entry: ServiceCatalogWire) -> Self {
        Self {
            name: entry.name,
            binary: entry.binary,
            capability_version: entry.capability_version,
            addr_env: entry.addr_env,
            url_env: entry.url_env,
            default_addr: entry.default_addr,
            local_only: entry.local_only,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatefulWorkerCatalogEntry {
    pub name: String,
    pub kind: ProcessKind,
    pub binary: String,
    pub contract_version: String,
    pub capability_version: String,
    pub addr_env: String,
    #[serde(default)]
    pub url_env: Option<String>,
    pub default_addr: String,
    #[serde(default)]
    pub local_only: bool,
    #[serde(default)]
    pub data_scope: Vec<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub migration: MigrationCompatibility,
}

pub fn service_catalog() -> Result<Vec<ServiceCatalogEntry>, serde_json::Error> {
    serde_json::from_str::<Vec<ServiceCatalogWire>>(include_str!("../service-catalog.json"))
        .map(|entries| entries.into_iter().map(Into::into).collect())
}

pub fn stateful_worker_catalog() -> Result<Vec<StatefulWorkerCatalogEntry>, serde_json::Error> {
    serde_json::from_str(include_str!("../worker-catalog.json"))
}

fn embedded_service_catalog() -> &'static [ServiceCatalogEntry] {
    static CATALOG: OnceLock<Vec<ServiceCatalogEntry>> = OnceLock::new();
    CATALOG
        .get_or_init(|| service_catalog().expect("embedded service catalog must be valid"))
        .as_slice()
}

pub fn service_catalog_entry(name: &str) -> Option<ServiceCatalogEntry> {
    service_catalog_entry_ref(name).cloned()
}

pub fn service_catalog_entry_ref(name: &str) -> Option<&'static ServiceCatalogEntry> {
    embedded_service_catalog()
        .iter()
        .find(|entry| entry.name == name)
}

pub fn service_catalog_entry_for_capability_version(
    capability_version: &str,
) -> Option<ServiceCatalogEntry> {
    service_catalog_entry_for_capability_version_ref(capability_version).cloned()
}

pub fn service_catalog_entry_for_capability_version_ref(
    capability_version: &str,
) -> Option<&'static ServiceCatalogEntry> {
    embedded_service_catalog()
        .iter()
        .find(|entry| entry.capability_version == capability_version)
}

fn embedded_stateful_worker_catalog() -> &'static [StatefulWorkerCatalogEntry] {
    static CATALOG: OnceLock<Vec<StatefulWorkerCatalogEntry>> = OnceLock::new();
    CATALOG
        .get_or_init(|| {
            stateful_worker_catalog().expect("embedded stateful worker catalog must be valid")
        })
        .as_slice()
}

impl From<&ServiceCatalogWire> for ProcessMetadata {
    fn from(entry: &ServiceCatalogWire) -> Self {
        Self {
            name: entry.name.clone(),
            kind: entry.kind,
            binary: entry.binary.clone(),
            capability_version: Some(entry.capability_version.clone()),
            contract_version: entry.contract_version.clone(),
            addr_env: entry.addr_env.clone(),
            url_env: Some(entry.url_env.clone()),
            default_addr: entry.default_addr.clone(),
            local_only: entry.local_only,
            data_scope: entry.data_scope.clone(),
            capabilities: Vec::new(),
            dependencies: entry.dependencies.clone(),
            migration: entry.migration.clone(),
        }
    }
}

impl From<&StatefulWorkerCatalogEntry> for ProcessMetadata {
    fn from(entry: &StatefulWorkerCatalogEntry) -> Self {
        Self {
            name: entry.name.clone(),
            kind: entry.kind,
            binary: entry.binary.clone(),
            capability_version: Some(entry.capability_version.clone()),
            contract_version: Some(entry.contract_version.clone()),
            addr_env: entry.addr_env.clone(),
            url_env: entry.url_env.clone(),
            default_addr: entry.default_addr.clone(),
            local_only: entry.local_only,
            data_scope: entry.data_scope.clone(),
            capabilities: entry.capabilities.clone(),
            dependencies: entry.dependencies.clone(),
            migration: entry.migration.clone(),
        }
    }
}

pub fn process_catalog() -> Result<Vec<ProcessMetadata>, serde_json::Error> {
    let sidecar_records =
        serde_json::from_str::<Vec<ServiceCatalogWire>>(include_str!("../service-catalog.json"))?;
    let sidecars = sidecar_records
        .iter()
        .map(ProcessMetadata::from)
        .collect::<Vec<_>>();
    let workers = stateful_worker_catalog()?
        .iter()
        .map(ProcessMetadata::from)
        .collect::<Vec<_>>();
    Ok(sidecars.into_iter().chain(workers).collect())
}

fn embedded_process_catalog() -> &'static [ProcessMetadata] {
    static CATALOG: OnceLock<Vec<ProcessMetadata>> = OnceLock::new();
    CATALOG
        .get_or_init(|| process_catalog().expect("embedded process catalog must be valid"))
        .as_slice()
}

pub fn process_catalog_entry(name: &str) -> Option<ProcessMetadata> {
    process_catalog_entry_ref(name).cloned()
}

fn process_catalog_entry_ref(name: &str) -> Option<&'static ProcessMetadata> {
    embedded_process_catalog()
        .iter()
        .find(|entry| entry.name == name)
}

pub fn stateful_worker_catalog_entry_ref(
    name: &str,
) -> Option<&'static StatefulWorkerCatalogEntry> {
    embedded_stateful_worker_catalog()
        .iter()
        .find(|entry| entry.name == name)
}

#[cfg(test)]
mod tests {
    use super::{
        ProcessKind, ServiceCatalogEntry, process_catalog, process_catalog_entry, service_catalog,
        service_catalog_entry_for_capability_version, stateful_worker_catalog,
    };

    #[test]
    fn catalog_has_unique_process_identity_and_environment_keys() {
        let entries = service_catalog().expect("embedded service catalog must be valid");
        assert_eq!(entries.len(), 9);
        for field in [
            entries
                .iter()
                .map(|entry| entry.name.as_str())
                .collect::<Vec<_>>(),
            entries
                .iter()
                .map(|entry| entry.binary.as_str())
                .collect::<Vec<_>>(),
            entries
                .iter()
                .map(|entry| entry.addr_env.as_str())
                .collect::<Vec<_>>(),
            entries
                .iter()
                .map(|entry| entry.url_env.as_str())
                .collect::<Vec<_>>(),
            entries
                .iter()
                .map(|entry| entry.capability_version.as_str())
                .collect::<Vec<_>>(),
        ] {
            let unique = field.iter().collect::<std::collections::HashSet<_>>();
            assert_eq!(unique.len(), field.len());
        }
        assert!(entries.iter().all(|entry| {
            !entry.capability_version.trim().is_empty()
                && entry.default_addr.parse::<std::net::SocketAddr>().is_ok()
        }));
    }

    #[test]
    fn catalog_resolves_capability_version_to_one_service() {
        assert_eq!(
            service_catalog_entry_for_capability_version("export-render-v3")
                .map(|entry| entry.name),
            Some("export".to_string())
        );
        assert!(service_catalog_entry_for_capability_version("unknown-v1").is_none());
        assert_eq!(
            service_catalog_entry_for_capability_version("media-ffmpeg-v1").map(|entry| entry.name),
            Some("media".to_string())
        );
        assert!(
            super::service_catalog_entry("media")
                .expect("media service catalog entry")
                .local_only
        );
    }

    #[test]
    fn stateful_worker_catalog_has_contract_scope_and_migration_metadata() {
        let entries = stateful_worker_catalog().expect("embedded worker catalog must be valid");
        assert_eq!(entries.len(), 27);
        let entry = entries
            .iter()
            .find(|entry| entry.name == "agent-worker")
            .expect("agent worker entry");
        assert_eq!(entry.name, "agent-worker");
        assert_eq!(entry.kind, ProcessKind::StatefulWorker);
        assert_eq!(entry.binary, "a4n-agent-worker");
        assert_eq!(entry.contract_version, "worker-schema-v6");
        assert_eq!(entry.addr_env, "A4N_AGENT_WORKER_ADDR");
        assert_eq!(entry.url_env.as_deref(), Some("A4N_AGENT_WORKER_URL"));
        assert_eq!(entry.default_addr, "127.0.0.1:47650");
        assert!(!entry.data_scope.is_empty());
        assert_eq!(
            entry.capabilities,
            vec![
                "agent.run".to_string(),
                "agent.host_command".to_string(),
                "agent.thread.ensure_main".to_string(),
                "agent.thread.create".to_string(),
                "agent.thread.fork".to_string(),
                "agent.thread.update".to_string(),
                "agent.thread.archive".to_string(),
                "agent.thread.list".to_string(),
                "agent.thread.show".to_string(),
                "agent.thread.messages".to_string(),
                "agent.run.show".to_string(),
                "agent.event.list".to_string(),
                "agent.memory.list".to_string(),
                "agent.memory.create".to_string(),
                "agent.memory.retract".to_string(),
                "agent.approval.resolve".to_string(),
                "agent.delegation.spawn".to_string(),
                "agent.delegation.review".to_string(),
                "agent.skill.settings.list".to_string(),
                "agent.skill.settings.update".to_string(),
                "agent.skill.search".to_string(),
                "agent.skill.load".to_string(),
                "agent.run.send".to_string(),
                "agent.run.control".to_string(),
                "agent.run.send_child".to_string(),
                "agent.run.control_child".to_string(),
            ]
        );
        assert_eq!(entry.migration.current.as_deref(), Some("agent_runtime"));
        assert_eq!(
            entry.migration.min_compatible.as_deref(),
            Some("agent_runtime")
        );
        let novel = entries
            .iter()
            .find(|entry| entry.name == "novel-planning-worker")
            .expect("novel planning worker entry");
        assert_eq!(novel.kind, ProcessKind::StatefulWorker);
        assert_eq!(novel.binary, "a4n-novel-planning-worker");
        assert_eq!(novel.contract_version, "novel-planning-shooting-script-v1");
        assert_eq!(novel.addr_env, "A4N_NOVEL_PLANNING_WORKER_ADDR");
        assert_eq!(
            novel.url_env.as_deref(),
            Some("A4N_NOVEL_PLANNING_WORKER_URL")
        );
        assert_eq!(novel.default_addr, "127.0.0.1:47651");
        assert_eq!(
            novel.capabilities,
            vec![
                "shooting_script.current".to_string(),
                "shooting_script.unit_progress".to_string(),
                "shooting_script.edit".to_string(),
                "shooting_script.delete".to_string(),
                "shooting_script.publish".to_string(),
                "visual_plan.persistence".to_string()
            ]
        );
        assert_eq!(novel.migration.current.as_deref(), Some("shooting_script"));
        let artifact = entries
            .iter()
            .find(|entry| entry.name == "artifact-media-worker")
            .expect("artifact media worker entry");
        assert_eq!(artifact.kind, ProcessKind::StatefulWorker);
        assert_eq!(artifact.binary, "a4n-artifact-media-worker");
        assert_eq!(artifact.contract_version, "artifact-media-v1");
        assert_eq!(artifact.addr_env, "A4N_ARTIFACT_MEDIA_WORKER_ADDR");
        assert_eq!(artifact.default_addr, "127.0.0.1:47652");
        assert_eq!(artifact.capabilities, vec!["artifact-media".to_string()]);
        let outline = entries
            .iter()
            .find(|entry| entry.name == "outline-worker")
            .expect("outline worker entry");
        assert_eq!(outline.kind, ProcessKind::StatefulWorker);
        assert_eq!(outline.binary, "a4n-outline-worker");
        assert_eq!(outline.contract_version, "outline-records-v1");
        assert_eq!(outline.addr_env, "A4N_OUTLINE_WORKER_ADDR");
        assert_eq!(outline.default_addr, "127.0.0.1:47653");
        assert_eq!(
            outline.capabilities,
            vec![
                "outline.records.read".to_string(),
                "outline.records.mutation".to_string()
            ]
        );
        let blueprint = entries
            .iter()
            .find(|entry| entry.name == "blueprint-worker")
            .expect("blueprint worker entry");
        assert_eq!(blueprint.kind, ProcessKind::StatefulWorker);
        assert_eq!(blueprint.binary, "a4n-blueprint-worker");
        assert_eq!(blueprint.contract_version, "blueprint-read-v1");
        assert_eq!(blueprint.addr_env, "A4N_BLUEPRINT_WORKER_ADDR");
        assert_eq!(blueprint.default_addr, "127.0.0.1:47654");
        assert_eq!(
            blueprint.capabilities,
            vec!["blueprint.records.read".to_string()]
        );
        let asset_graph = entries
            .iter()
            .find(|entry| entry.name == "asset-graph-worker")
            .expect("asset graph worker entry");
        assert_eq!(asset_graph.kind, ProcessKind::StatefulWorker);
        assert_eq!(asset_graph.binary, "a4n-asset-graph-worker");
        assert_eq!(asset_graph.contract_version, "project-asset-graph-v1");
        assert_eq!(asset_graph.addr_env, "A4N_ASSET_GRAPH_WORKER_ADDR");
        assert_eq!(asset_graph.default_addr, "127.0.0.1:47655");
        assert_eq!(
            asset_graph.capabilities,
            vec!["project.asset_graph.read".to_string()]
        );
        let foreshadowing = entries
            .iter()
            .find(|entry| entry.name == "foreshadowing-worker")
            .expect("foreshadowing worker entry");
        assert_eq!(foreshadowing.kind, ProcessKind::StatefulWorker);
        assert_eq!(foreshadowing.binary, "a4n-foreshadowing-worker");
        assert_eq!(foreshadowing.contract_version, "foreshadowing-read-v1");
        assert_eq!(foreshadowing.addr_env, "A4N_FORESHADOWING_WORKER_ADDR");
        assert_eq!(foreshadowing.default_addr, "127.0.0.1:47656");
        assert_eq!(
            foreshadowing.capabilities,
            vec!["profile.foreshadowing.read".to_string()]
        );
        let object = entries
            .iter()
            .find(|entry| entry.name == "object-profile-worker")
            .expect("object profile worker entry");
        assert_eq!(object.kind, ProcessKind::StatefulWorker);
        assert_eq!(object.binary, "a4n-object-profile-worker");
        assert_eq!(object.contract_version, "object-profile-mutation-v1");
        assert_eq!(object.addr_env, "A4N_OBJECT_PROFILE_WORKER_ADDR");
        assert_eq!(object.default_addr, "127.0.0.1:47657");
        assert_eq!(
            object.capabilities,
            vec![
                "profile.object.read".to_string(),
                "profile.object.mutation".to_string()
            ]
        );
        let scene = entries
            .iter()
            .find(|entry| entry.name == "scene-profile-worker")
            .expect("scene profile worker entry");
        assert_eq!(scene.kind, ProcessKind::StatefulWorker);
        assert_eq!(scene.binary, "a4n-scene-profile-worker");
        assert_eq!(scene.contract_version, "scene-profile-records-v1");
        assert_eq!(scene.addr_env, "A4N_SCENE_PROFILE_WORKER_ADDR");
        assert_eq!(scene.default_addr, "127.0.0.1:47658");
        assert_eq!(
            scene.capabilities,
            vec![
                "profile.scene.read".to_string(),
                "profile.scene.mutation".to_string()
            ]
        );
        assert_eq!(
            scene.migration.current.as_deref(),
            Some("scene_profile_mutation")
        );
        let profile_material = entries
            .iter()
            .find(|entry| entry.name == "profile-material-worker")
            .expect("profile material worker entry");
        assert_eq!(profile_material.kind, ProcessKind::StatefulWorker);
        assert_eq!(profile_material.binary, "a4n-profile-material-worker");
        assert_eq!(profile_material.contract_version, "profile-material-v1");
        assert_eq!(
            profile_material.addr_env,
            "A4N_PROFILE_MATERIAL_WORKER_ADDR"
        );
        assert_eq!(profile_material.default_addr, "127.0.0.1:47661");
        assert_eq!(
            profile_material.capabilities,
            vec![
                "profile.material.read".to_string(),
                "profile.material.mutation".to_string()
            ]
        );
        let project_status = entries
            .iter()
            .find(|entry| entry.name == "project-status-worker")
            .expect("project status worker entry");
        assert_eq!(project_status.kind, ProcessKind::StatefulWorker);
        assert_eq!(project_status.binary, "a4n-project-status-worker");
        assert_eq!(project_status.contract_version, "project-status-v2");
        assert_eq!(project_status.addr_env, "A4N_PROJECT_STATUS_WORKER_ADDR");
        assert_eq!(project_status.default_addr, "127.0.0.1:47662");
        assert_eq!(
            project_status.capabilities,
            vec![
                "project.status.read".to_string(),
                "project.status.write".to_string()
            ]
        );
        let theme_read = entries
            .iter()
            .find(|entry| entry.name == "theme-read-worker")
            .expect("theme read worker entry");
        assert_eq!(theme_read.kind, ProcessKind::StatefulWorker);
        assert_eq!(theme_read.binary, "a4n-theme-read-worker");
        assert_eq!(theme_read.contract_version, "theme-read-v1");
        assert_eq!(theme_read.addr_env, "A4N_THEME_READ_WORKER_ADDR");
        assert_eq!(theme_read.default_addr, "127.0.0.1:47663");
        assert_eq!(
            theme_read.capabilities,
            vec!["theme.read".to_string(), "theme.mutation".to_string()]
        );
        let project_read = entries
            .iter()
            .find(|entry| entry.name == "project-read-worker")
            .expect("project read worker entry");
        assert_eq!(project_read.kind, ProcessKind::StatefulWorker);
        assert_eq!(project_read.binary, "a4n-project-read-worker");
        assert_eq!(project_read.contract_version, "project-read-v1");
        assert_eq!(project_read.addr_env, "A4N_PROJECT_READ_WORKER_ADDR");
        assert_eq!(project_read.default_addr, "127.0.0.1:47664");
        assert_eq!(project_read.capabilities, vec!["project.read".to_string()]);
        let import_review_read = entries
            .iter()
            .find(|entry| entry.name == "import-review-read-worker")
            .expect("import review read worker entry");
        assert_eq!(import_review_read.kind, ProcessKind::StatefulWorker);
        assert_eq!(import_review_read.binary, "a4n-import-review-worker");
        assert_eq!(import_review_read.contract_version, "import-review-read-v1");
        assert_eq!(
            import_review_read.addr_env,
            "A4N_IMPORT_REVIEW_READ_WORKER_ADDR"
        );
        assert_eq!(import_review_read.default_addr, "127.0.0.1:47665");
        assert_eq!(
            import_review_read.capabilities,
            vec![
                "import_review.read".to_string(),
                "import_review.mutation".to_string()
            ]
        );
        let inspiration_history = entries
            .iter()
            .find(|entry| entry.name == "inspiration-history-worker")
            .expect("inspiration history worker entry");
        assert_eq!(inspiration_history.kind, ProcessKind::StatefulWorker);
        assert_eq!(inspiration_history.binary, "a4n-inspiration-worker");
        assert_eq!(
            inspiration_history.contract_version,
            "inspiration-history-v1"
        );
        assert_eq!(
            inspiration_history.addr_env,
            "A4N_INSPIRATION_HISTORY_WORKER_ADDR"
        );
        assert_eq!(inspiration_history.default_addr, "127.0.0.1:47666");
        assert_eq!(
            inspiration_history.capabilities,
            vec!["inspiration.history.read".to_string()]
        );
        let project_access = entries
            .iter()
            .find(|entry| entry.name == "project-access-worker")
            .expect("project access worker entry");
        assert_eq!(project_access.kind, ProcessKind::StatefulWorker);
        assert_eq!(project_access.binary, "a4n-project-access-worker");
        assert_eq!(project_access.contract_version, "project-access-v1");
        assert_eq!(project_access.addr_env, "A4N_PROJECT_ACCESS_WORKER_ADDR");
        assert_eq!(project_access.default_addr, "127.0.0.1:47667");
        assert_eq!(
            project_access.capabilities,
            vec!["project.access.read".to_string()]
        );
        let project_collaboration = entries
            .iter()
            .find(|entry| entry.name == "project-collaboration-worker")
            .expect("project collaboration worker entry");
        assert_eq!(project_collaboration.kind, ProcessKind::StatefulWorker);
        assert_eq!(
            project_collaboration.binary,
            "a4n-project-collaboration-worker"
        );
        assert_eq!(
            project_collaboration.contract_version,
            "project-collaboration-v1"
        );
        assert_eq!(
            project_collaboration.addr_env,
            "A4N_PROJECT_COLLABORATION_WORKER_ADDR"
        );
        assert_eq!(project_collaboration.default_addr, "127.0.0.1:47668");
        assert_eq!(
            project_collaboration.capabilities,
            vec!["project.collaboration".to_string()]
        );
        let project_mutation = entries
            .iter()
            .find(|entry| entry.name == "project-mutation-worker")
            .expect("project mutation worker entry");
        assert_eq!(project_mutation.kind, ProcessKind::StatefulWorker);
        assert_eq!(project_mutation.binary, "a4n-project-mutation-worker");
        assert_eq!(project_mutation.contract_version, "project-mutation-v1");
        assert_eq!(
            project_mutation.addr_env,
            "A4N_PROJECT_MUTATION_WORKER_ADDR"
        );
        assert_eq!(project_mutation.default_addr, "127.0.0.1:47669");
        assert_eq!(
            project_mutation.capabilities,
            vec!["project.mutation".to_string()]
        );
        let chapter_scene = entries
            .iter()
            .find(|entry| entry.name == "chapter-scene-worker")
            .expect("chapter scene worker entry");
        assert_eq!(chapter_scene.kind, ProcessKind::StatefulWorker);
        assert_eq!(chapter_scene.binary, "a4n-chapter-scene-worker");
        assert_eq!(chapter_scene.contract_version, "chapter-scene-v1");
        assert_eq!(chapter_scene.addr_env, "A4N_CHAPTER_SCENE_WORKER_ADDR");
        assert_eq!(chapter_scene.default_addr, "127.0.0.1:47670");
        assert_eq!(
            chapter_scene.capabilities,
            vec!["chapter.scene".to_string()]
        );
        let chapter_content = entries
            .iter()
            .find(|entry| entry.name == "chapter-content-worker")
            .expect("chapter content worker entry");
        assert_eq!(chapter_content.kind, ProcessKind::StatefulWorker);
        assert_eq!(chapter_content.binary, "a4n-chapter-content-worker");
        assert_eq!(chapter_content.contract_version, "chapter-content-v1");
        assert_eq!(chapter_content.addr_env, "A4N_CHAPTER_CONTENT_WORKER_ADDR");
        assert_eq!(chapter_content.default_addr, "127.0.0.1:47671");
        assert_eq!(
            chapter_content.capabilities,
            vec!["chapter.content".to_string()]
        );
        let workflow_read = entries
            .iter()
            .find(|entry| entry.name == "workflow-read-worker")
            .expect("workflow read worker entry");
        assert_eq!(workflow_read.kind, ProcessKind::StatefulWorker);
        assert_eq!(workflow_read.binary, "a4n-workflow-read-worker");
        assert_eq!(workflow_read.contract_version, "workflow-read-v1");
        assert_eq!(workflow_read.addr_env, "A4N_WORKFLOW_READ_WORKER_ADDR");
        assert_eq!(workflow_read.default_addr, "127.0.0.1:47672");
        assert_eq!(
            workflow_read.capabilities,
            vec!["workflow.read".to_string()]
        );
        let video_record = entries
            .iter()
            .find(|entry| entry.name == "video-record-worker")
            .expect("video record worker entry");
        assert_eq!(video_record.kind, ProcessKind::StatefulWorker);
        assert_eq!(video_record.binary, "a4n-video-record-worker");
        assert_eq!(video_record.contract_version, "video-record-v3");
        assert_eq!(video_record.addr_env, "A4N_VIDEO_RECORD_WORKER_ADDR");
        assert_eq!(video_record.default_addr, "127.0.0.1:47675");
        assert_eq!(
            video_record.capabilities,
            vec![
                "video.record.read".to_string(),
                "video.record.mutation".to_string(),
                "video.prompt.read".to_string(),
                "video.prompt.draft".to_string()
            ]
        );
        let manga_record = entries
            .iter()
            .find(|entry| entry.name == "manga-record-worker")
            .expect("manga record worker entry");
        assert_eq!(manga_record.kind, ProcessKind::StatefulWorker);
        assert_eq!(manga_record.binary, "a4n-manga-record-worker");
        assert_eq!(manga_record.contract_version, "manga-record-v2");
        assert_eq!(manga_record.addr_env, "A4N_MANGA_RECORD_WORKER_ADDR");
        assert_eq!(manga_record.default_addr, "127.0.0.1:47676");
        assert_eq!(
            manga_record.capabilities,
            vec![
                "manga.record.read".to_string(),
                "manga.pipeline.read".to_string()
            ]
        );
        let config_read = entries
            .iter()
            .find(|entry| entry.name == "config-read-worker")
            .expect("config read worker entry");
        assert_eq!(config_read.kind, ProcessKind::StatefulWorker);
        assert_eq!(config_read.binary, "a4n-config-read-worker");
        assert_eq!(config_read.contract_version, "config-read-v2");
        assert_eq!(config_read.addr_env, "A4N_CONFIG_READ_WORKER_ADDR");
        assert_eq!(config_read.default_addr, "127.0.0.1:47673");
        assert_eq!(
            config_read.capabilities,
            vec![
                "config.read".to_string(),
                "config.mutation".to_string(),
                "config.settings".to_string()
            ]
        );
        let character_mutation = entries
            .iter()
            .find(|entry| entry.name == "character-mutation-worker")
            .expect("character mutation worker entry");
        assert_eq!(character_mutation.kind, ProcessKind::StatefulWorker);
        assert_eq!(character_mutation.binary, "a4n-character-mutation-worker");
        assert_eq!(character_mutation.contract_version, "character-mutation-v7");
        assert_eq!(
            character_mutation.addr_env,
            "A4N_CHARACTER_MUTATION_WORKER_ADDR"
        );
        assert_eq!(character_mutation.default_addr, "127.0.0.1:47674");
        assert_eq!(
            character_mutation.capabilities,
            vec![
                "character.mutation".to_string(),
                "character.overview".to_string(),
                "character.timeline.read".to_string()
            ]
        );
        let portrait = entries
            .iter()
            .find(|entry| entry.name == "portrait-worker")
            .expect("portrait worker entry");
        assert_eq!(portrait.kind, ProcessKind::StatefulWorker);
        assert_eq!(portrait.binary, "a4n-portrait-worker");
        assert_eq!(portrait.contract_version, "portrait-records-v3");
        assert_eq!(portrait.addr_env, "A4N_PORTRAIT_WORKER_ADDR");
        assert_eq!(portrait.default_addr, "127.0.0.1:47659");
        assert_eq!(
            portrait.capabilities,
            vec![
                "portrait.records.read".to_string(),
                "portrait.records.mutation".to_string()
            ]
        );
        let source_snapshot = entries
            .iter()
            .find(|entry| entry.name == "source-snapshot-worker")
            .expect("source snapshot worker entry");
        assert_eq!(source_snapshot.kind, ProcessKind::StatefulWorker);
        assert_eq!(source_snapshot.binary, "a4n-source-snapshot-worker");
        assert_eq!(source_snapshot.contract_version, "source-snapshot-read-v1");
        assert_eq!(source_snapshot.addr_env, "A4N_SOURCE_SNAPSHOT_WORKER_ADDR");
        assert_eq!(source_snapshot.default_addr, "127.0.0.1:47660");
        assert_eq!(
            source_snapshot.capabilities,
            vec!["project.source_snapshot.read".to_string()]
        );
    }

    #[test]
    fn normalized_process_catalog_keeps_sidecars_and_workers_separate() {
        let entries = process_catalog().expect("embedded process catalog must be valid");
        assert_eq!(entries.len(), 36);
        assert_eq!(
            entries
                .iter()
                .filter(|entry| entry.kind == ProcessKind::CapabilitySidecar)
                .count(),
            9
        );
        let sidecar = process_catalog_entry("prompt").expect("prompt metadata");
        assert_eq!(sidecar.kind, ProcessKind::CapabilitySidecar);
        assert_eq!(
            sidecar.capability_version.as_deref(),
            Some("prompt-render-v4")
        );
        assert_eq!(
            sidecar.contract_version.as_deref(),
            Some("service-protocol-v2")
        );
        assert_eq!(sidecar.default_addr, "127.0.0.1:47641");
        assert_eq!(sidecar.data_scope, vec!["prompt.render"]);
        let worker = process_catalog_entry("agent-worker").expect("agent worker metadata");
        assert_eq!(worker.kind, ProcessKind::StatefulWorker);
        assert_eq!(worker.contract_version.as_deref(), Some("worker-schema-v6"));
        assert_eq!(
            worker.capability_version.as_deref(),
            Some("agent-worker-v1")
        );
    }

    #[test]
    fn legacy_sidecar_json_uses_compatible_metadata_defaults() {
        let legacy = r#"
            {
              "name":"legacy",
              "binary":"a4n-legacy-service",
              "capability_version":"legacy-v1",
              "addr_env":"A4N_LEGACY_ADDR",
              "url_env":"A4N_LEGACY_URL",
              "default_addr":"127.0.0.1:47650"
            }
        "#;
        let entry: super::ServiceCatalogEntry =
            serde_json::from_str(legacy).expect("old sidecar catalog entry should still parse");
        assert_eq!(entry.capability_version, "legacy-v1");
        assert_eq!(entry.default_addr, "127.0.0.1:47650");
    }

    #[test]
    fn legacy_sidecar_struct_accepts_new_metadata_without_changing_its_shape() {
        let catalog_entry = r#"
            {
              "name":"prompt",
              "binary":"a4n-prompt-service",
              "capability_version":"prompt-render-v4",
              "addr_env":"A4N_PROMPT_SERVICE_ADDR",
              "url_env":"A4N_PROMPT_SERVICE_URL",
              "default_addr":"127.0.0.1:47641",
              "kind":"capability_sidecar",
              "contract_version":"service-protocol-v2",
              "data_scope":["prompt.render"],
              "dependencies":[],
              "migration":{"current":null,"min_compatible":null,"max_compatible":null}
            }
        "#;
        let entry: ServiceCatalogEntry =
            serde_json::from_str(catalog_entry).expect("new catalog metadata should be readable");
        assert_eq!(entry.name, "prompt");
        let serialized = serde_json::to_value(entry).expect("legacy entry should serialize");
        assert!(serialized.get("kind").is_none());
        assert!(serialized.get("contract_version").is_none());
    }
}
