use crate::project::MediaFileEntry;
use a4n_domain::project::*;
use a4n_domain::video_prompt::ChapterVideoPromptRecord;
use serde::{Deserialize, Serialize};

pub const PROJECT_FAMILY_PACKAGE_VERSION: &str = "project_family_v8";
pub const MANIFEST_PATH: &str = "a4n-project-family.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectFamilyPackage {
    pub version: String,
    pub exported_at: String,
    pub mode: String,
    pub root_project_id: i64,
    pub projects: Vec<ProjectRecord>,
    pub text_core: TextCorePackage,
    pub source_snapshots: Vec<ProjectFamilySourceSnapshotRecord>,
    pub source_snapshot_chapters: Vec<ProjectFamilySourceSnapshotChapterRecord>,
    pub source_snapshot_outlines: Vec<ProjectFamilySourceSnapshotOutlineRecord>,
    pub source_snapshot_profiles: Vec<ProjectFamilySourceSnapshotProfileRecord>,
    pub text_anchors: Vec<ProjectFamilyTextAnchorRecord>,
    pub profile_evidence: Vec<ProjectFamilyProfileEvidenceRecord>,
    pub chapter_visual_plans: Vec<ProjectFamilyVisualPlanRecord>,
    pub chapter_shooting_scripts: Vec<ProjectFamilyShootingScriptRecord>,
    pub authoring_documents: Vec<ProjectFamilyAuthoringDocumentRecord>,
    pub authoring_receipts: Vec<ProjectFamilyAuthoringMigrationReceiptRecord>,
    pub character_portraits: Vec<ProjectFamilyCharacterPortraitRecord>,
    pub generated_images: Vec<ProjectFamilyGeneratedImageRecord>,
    pub chapter_manga_prompts: Vec<ProjectFamilyMangaPromptRecord>,
    #[serde(default)]
    pub manga_pipeline_artifacts: Vec<ProjectFamilyMangaPipelineArtifactRecord>,
    pub chapter_video_prompts: Vec<ChapterVideoPromptRecord>,
    pub generated_videos: Vec<ProjectFamilyGeneratedVideoRecord>,
    pub generation_specs: Vec<ProjectFamilyGenerationSpecRecord>,
    pub generation_spec_text_anchors: Vec<ProjectFamilyGenerationSpecTextAnchorRecord>,
    pub generation_spec_entity_refs: Vec<ProjectFamilyGenerationSpecEntityRefRecord>,
    pub generation_spec_media_refs: Vec<ProjectFamilyGenerationSpecMediaRefRecord>,
    pub prompt_artifacts: Vec<ProjectFamilyPromptArtifactRecord>,
    pub media_assets: Vec<ProjectFamilyMediaAssetRecord>,
    pub media_provenance: Vec<ProjectFamilyMediaProvenanceRecord>,
    pub media_bindings: Vec<ProjectFamilyMediaBindingRecord>,
    pub galgame_scripts: Vec<ProjectFamilyGalgameScriptRecord>,
    pub galgame_scenes: Vec<ProjectFamilyGalgameSceneRecord>,
    pub galgame_lines: Vec<ProjectFamilyGalgameLineRecord>,
    pub galgame_choices: Vec<ProjectFamilyGalgameChoiceRecord>,
    pub video_segments: Vec<ProjectFamilyVideoSegmentRecord>,
    pub assembly_timelines: Vec<ProjectFamilyAssemblyTimelineRecord>,
    pub assembly_clips: Vec<ProjectFamilyAssemblyClipRecord>,
    pub source_sync_runs: Vec<ProjectFamilySourceSyncRunRecord>,
    pub source_sync_conflicts: Vec<ProjectFamilySourceSyncConflictRecord>,
    pub source_sync_decisions: Vec<ProjectFamilySourceSyncDecisionRecord>,
    pub media_files: Vec<MediaFileEntry>,
}

pub type TextCorePackage = ProjectFamilyTextCoreRows;
