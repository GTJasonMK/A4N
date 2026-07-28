use a4n_core::error::AppError;
use a4n_llm::context::{LlmContextManifest, LlmContextManifestPack};
use serde_json::{Value, json};

use super::VIDEO_PREVIOUS_OUTPUT_SHOOTING_SCRIPT;

#[derive(Clone)]
pub(in crate::video) struct VideoSegmentPlanningContextTemplate {
    manifest: LlmContextManifest,
}

impl VideoSegmentPlanningContextTemplate {
    pub(super) fn new(manifest: LlmContextManifest) -> Self {
        Self { manifest }
    }

    pub(in crate::video) fn manifest(&self, dynamic_user_content: String) -> LlmContextManifest {
        let mut manifest = self.manifest.clone();
        manifest.dynamic_user_content = dynamic_user_content;
        manifest
    }

    pub(in crate::video) fn packs(&self) -> &[LlmContextManifestPack] {
        &self.manifest.packs
    }

    pub(in crate::video) fn checkpoint_identity_json(&self) -> Result<Value, AppError> {
        let chapter = self.required_pack_content("video_segment_planning_chapter")?;
        let prompt_rule_hashes = self.required_pack_content("video_prompt_rule_hashes")?;
        let project_context = self
            .pack_content("video_project_context")
            .cloned()
            .unwrap_or(Value::Null);
        Ok(json!({
            "chapter": chapter,
            "continuity_map_sha256": self.continuity_sha256()?,
            "project_context": project_context,
            "prompt_rule_hashes": prompt_rule_hashes
        }))
    }

    pub(in crate::video) fn continuity_sha256(&self) -> Result<&str, AppError> {
        self.manifest
            .packs
            .iter()
            .find(|pack| pack.pack_kind == VIDEO_PREVIOUS_OUTPUT_SHOOTING_SCRIPT)
            .and_then(|pack| pack.source_refs_json.as_ref())
            .and_then(|refs| refs.get("continuity_sha256"))
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::InvalidState {
                current: "missing_video_segment_planning_continuity_identity".to_string(),
                operation: "读取视频片段规划上下文身份".to_string(),
            })
    }

    fn required_pack_content(&self, pack_kind: &str) -> Result<&Value, AppError> {
        self.pack_content(pack_kind)
            .ok_or_else(|| AppError::InvalidState {
                current: format!("missing_video_segment_planning_pack:{pack_kind}"),
                operation: "读取视频片段规划上下文身份".to_string(),
            })
    }

    fn pack_content(&self, pack_kind: &str) -> Option<&Value> {
        self.manifest
            .packs
            .iter()
            .find(|pack| pack.pack_kind == pack_kind)
            .and_then(|pack| pack.variants.first())
            .map(|variant| &variant.canonical_content_json)
    }
}
