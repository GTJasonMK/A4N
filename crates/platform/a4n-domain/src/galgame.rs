use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};

pub const SCENE_ENTRY_MAIN: &str = "main";
pub const SCENE_ENTRY_BRANCH: &str = "branch";
pub const LINE_TYPE_NARRATION: &str = "narration";
pub const LINE_TYPE_DIALOGUE: &str = "dialogue";
pub const LINE_TYPE_MONOLOGUE: &str = "monologue";
pub const LINE_TYPE_STAGE_DIRECTION: &str = "stage_direction";
pub const LINE_TYPE_CHOICE: &str = "choice";
pub const GALGAME_TARGET_BACKGROUND: &str = "galgame_background";
pub const GALGAME_TARGET_CG: &str = "galgame_cg";
pub const GALGAME_TARGET_PORTRAIT_EXPRESSION: &str = "portrait_expression";
pub const GALGAME_ENTITY_SCENE: &str = "galgame_scene";
pub const GALGAME_ENTITY_LINE: &str = "galgame_line";
pub const GALGAME_ENTITY_CHARACTER_PROFILE: &str = "character_profile";
pub const GALGAME_EXPRESSIONS: &[&str] = &[
    "neutral",
    "smile",
    "sad",
    "angry",
    "surprised",
    "shy",
    "serious",
];
pub const GALGAME_PIPELINE_TOTAL_STEPS: u32 = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GalgameLineType {
    Dialogue,
    Narration,
    Monologue,
    StageDirection,
    Choice,
}

impl GalgameLineType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Dialogue => LINE_TYPE_DIALOGUE,
            Self::Narration => LINE_TYPE_NARRATION,
            Self::Monologue => LINE_TYPE_MONOLOGUE,
            Self::StageDirection => LINE_TYPE_STAGE_DIRECTION,
            Self::Choice => LINE_TYPE_CHOICE,
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            LINE_TYPE_DIALOGUE => Some(Self::Dialogue),
            LINE_TYPE_NARRATION => Some(Self::Narration),
            LINE_TYPE_MONOLOGUE => Some(Self::Monologue),
            LINE_TYPE_STAGE_DIRECTION => Some(Self::StageDirection),
            LINE_TYPE_CHOICE => Some(Self::Choice),
            _ => None,
        }
    }
}

impl fmt::Display for GalgameLineType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GalgameSceneEntryKind {
    Main,
    Branch,
}

impl GalgameSceneEntryKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Main => SCENE_ENTRY_MAIN,
            Self::Branch => SCENE_ENTRY_BRANCH,
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            SCENE_ENTRY_MAIN => Some(Self::Main),
            SCENE_ENTRY_BRANCH => Some(Self::Branch),
            _ => None,
        }
    }
}

impl fmt::Display for GalgameSceneEntryKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GalgameMediaTargetKind {
    GalgameScene,
    GalgameLine,
}

impl GalgameMediaTargetKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GalgameScene => GALGAME_ENTITY_SCENE,
            Self::GalgameLine => GALGAME_ENTITY_LINE,
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            GALGAME_ENTITY_SCENE => Some(Self::GalgameScene),
            GALGAME_ENTITY_LINE => Some(Self::GalgameLine),
            _ => None,
        }
    }
}

impl fmt::Display for GalgameMediaTargetKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GalgameMediaRole {
    Background,
    Cg,
    Bgm,
    Sfx,
}

impl GalgameMediaRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Background => "background",
            Self::Cg => "cg",
            Self::Bgm => "bgm",
            Self::Sfx => "sfx",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "background" => Some(Self::Background),
            "cg" => Some(Self::Cg),
            "bgm" => Some(Self::Bgm),
            "sfx" => Some(Self::Sfx),
            _ => None,
        }
    }
}

impl fmt::Display for GalgameMediaRole {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GalgameAudioRole {
    Bgm,
    Sfx,
}

impl GalgameAudioRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Bgm => "bgm",
            Self::Sfx => "sfx",
        }
    }
}

impl fmt::Display for GalgameAudioRole {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GalgameGenerationTargetKind {
    GalgameBackground,
    GalgameCg,
    PortraitExpression,
}

impl GalgameGenerationTargetKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GalgameBackground => GALGAME_TARGET_BACKGROUND,
            Self::GalgameCg => GALGAME_TARGET_CG,
            Self::PortraitExpression => GALGAME_TARGET_PORTRAIT_EXPRESSION,
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            GALGAME_TARGET_BACKGROUND => Some(Self::GalgameBackground),
            GALGAME_TARGET_CG => Some(Self::GalgameCg),
            GALGAME_TARGET_PORTRAIT_EXPRESSION => Some(Self::PortraitExpression),
            _ => None,
        }
    }
}

impl fmt::Display for GalgameGenerationTargetKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GalgameGenerationSpecUpsertInput {
    pub canonical_key: String,
    pub project_id: i64,
    pub root_project_id: i64,
    pub source_snapshot_id: i64,
    pub source_chapter_id: Option<i64>,
    pub source_snapshot_chapter_id: Option<i64>,
    pub target_kind: String,
    pub target_entity_kind: String,
    pub target_entity_id: Option<i64>,
    pub params_json: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GalgameAssetPlanSpecRecordInput {
    pub spec: GalgameGenerationSpecUpsertInput,
    pub text_anchor_ids: Vec<i64>,
    pub replace_entity_refs: bool,
    pub entity_profile_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GalgamePromptArtifactUpsertInput {
    pub generation_spec_id: i64,
    pub prompt_kind: String,
    pub prompt_text: String,
    pub renderer_version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GalgameTargetEntityKind {
    GalgameScene,
    GalgameLine,
    CharacterProfile,
}

impl GalgameTargetEntityKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GalgameScene => GALGAME_ENTITY_SCENE,
            Self::GalgameLine => GALGAME_ENTITY_LINE,
            Self::CharacterProfile => GALGAME_ENTITY_CHARACTER_PROFILE,
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            GALGAME_ENTITY_SCENE => Some(Self::GalgameScene),
            GALGAME_ENTITY_LINE => Some(Self::GalgameLine),
            GALGAME_ENTITY_CHARACTER_PROFILE => Some(Self::CharacterProfile),
            _ => None,
        }
    }
}

impl fmt::Display for GalgameTargetEntityKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GalgameCgImportance {
    Normal,
    High,
    Critical,
}

impl GalgameCgImportance {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "normal" => Some(Self::Normal),
            "high" => Some(Self::High),
            "critical" => Some(Self::Critical),
            _ => None,
        }
    }
}

impl fmt::Display for GalgameCgImportance {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GalgameTransition {
    Cut,
    Fade,
    Dissolve,
}

impl GalgameTransition {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Cut => "cut",
            Self::Fade => "fade",
            Self::Dissolve => "dissolve",
        }
    }
}

impl fmt::Display for GalgameTransition {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum GalgameLinePresentationSchemaVersion {
    #[serde(rename = "galgame_line_presentation_v2")]
    #[schemars(rename = "galgame_line_presentation_v2")]
    V2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GalgameStagePositionV2 {
    Left,
    Center,
    Right,
}

impl GalgameStagePositionV2 {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Center => "center",
            Self::Right => "right",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "left" => Some(Self::Left),
            "center" => Some(Self::Center),
            "right" => Some(Self::Right),
            _ => None,
        }
    }
}

impl fmt::Display for GalgameStagePositionV2 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GalgameFacingV2 {
    Left,
    Right,
    Front,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GalgameStageLayerV2 {
    Back,
    Middle,
    Front,
}

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct GalgamePipelineBeatRefV4 {
    pub source_chapter_id: i64,
    pub unit_id: String,
    pub beat_id: String,
}

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct GalgameAdaptationUnitRef {
    pub source_chapter_id: i64,
    pub unit_id: String,
}

impl GalgamePipelineBeatRefV4 {
    pub fn validate(&self) -> Result<(), String> {
        if self.source_chapter_id <= 0 {
            return Err("source_chapter_id 必须大于 0".into());
        }
        crate::media_authoring::AdaptationBeatRef {
            unit_id: self.unit_id.clone(),
            beat_id: self.beat_id.clone(),
        }
        .validate()
    }

    pub fn unit_ref(&self) -> GalgameAdaptationUnitRef {
        GalgameAdaptationUnitRef {
            source_chapter_id: self.source_chapter_id,
            unit_id: self.unit_id.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum GalgameLineDocumentSchemaVersion {
    #[serde(rename = "galgame_line_presentation_v3")]
    V3,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GalgameVisibleCastMemberV2 {
    pub character_profile_id: i64,
    pub expression: String,
    pub pose: String,
    pub position: GalgameStagePositionV2,
    pub facing: GalgameFacingV2,
    pub layer: GalgameStageLayerV2,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GalgameLinePresentationV2 {
    pub schema_version: GalgameLinePresentationSchemaVersion,
    pub visible_cast: Vec<GalgameVisibleCastMemberV2>,
    pub enter: Vec<i64>,
    pub exit: Vec<i64>,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub focus_character_profile_id: Option<i64>,
    pub hold: bool,
    pub player_knowledge: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GalgameLinePresentationDocumentV3 {
    pub schema_version: GalgameLineDocumentSchemaVersion,
    pub origin: crate::media_authoring::AdaptationOrigin,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub beat_ref: Option<GalgamePipelineBeatRefV4>,
    pub visible_cast: Vec<GalgameVisibleCastMemberV2>,
    pub enter: Vec<i64>,
    pub exit: Vec<i64>,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub focus_character_profile_id: Option<i64>,
    pub hold: bool,
    pub player_knowledge: Vec<String>,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub cg: Option<bool>,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub cg_note: Option<String>,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub cg_importance: Option<GalgameCgImportance>,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub transition: Option<GalgameTransition>,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub branch: Option<bool>,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub branch_id: Option<String>,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub returns_to_scene_index: Option<i32>,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub after_beat_ref: Option<GalgamePipelineBeatRefV4>,
}

fn deserialize_required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

impl GalgameLinePresentationDocumentV3 {
    pub fn validate_source_identity(&self, text_anchor_id: Option<i64>) -> Result<(), String> {
        use crate::media_authoring::AdaptationOrigin;
        match self.origin {
            AdaptationOrigin::Source | AdaptationOrigin::Adapted if self.beat_ref.is_none() => {
                return Err("原文或改编演出必须引用本地节拍".into());
            }
            AdaptationOrigin::Added if text_anchor_id.is_some() => {
                return Err("新增演出不能冒充原文依据".into());
            }
            _ => {}
        }
        for reference in self.beat_ref.iter().chain(self.after_beat_ref.iter()) {
            reference.validate()?;
        }
        Ok(())
    }

    pub fn validate_adaptation(
        &self,
        plan: &crate::media_authoring::AdaptationPlanV1,
        chapter_id: i64,
        unit_id: &str,
        text_anchor_id: Option<i64>,
    ) -> Result<(), String> {
        self.validate_source_identity(text_anchor_id)?;
        for reference in self.beat_ref.iter().chain(self.after_beat_ref.iter()) {
            if reference.source_chapter_id != chapter_id
                || reference.unit_id != unit_id
                || plan
                    .beat(&crate::media_authoring::AdaptationBeatRef {
                        unit_id: reference.unit_id.clone(),
                        beat_id: reference.beat_id.clone(),
                    })
                    .is_none()
            {
                return Err("演出引用的节拍不属于该场景的本地改编版本".into());
            }
        }
        if matches!(
            self.origin,
            crate::media_authoring::AdaptationOrigin::Source
                | crate::media_authoring::AdaptationOrigin::Adapted
        ) && text_anchor_id.is_none()
            && let Some(reference) = &self.beat_ref
            && plan
                .beat(&crate::media_authoring::AdaptationBeatRef {
                    unit_id: reference.unit_id.clone(),
                    beat_id: reference.beat_id.clone(),
                })
                .is_some_and(|beat| beat.source_span_ids.is_empty())
        {
            return Err("原文或改编演出的本地节拍缺少原文依据".into());
        }
        Ok(())
    }

    pub fn directing_presentation(&self) -> GalgameLinePresentationV2 {
        GalgameLinePresentationV2 {
            schema_version: GalgameLinePresentationSchemaVersion::V2,
            visible_cast: self.visible_cast.clone(),
            enter: self.enter.clone(),
            exit: self.exit.clone(),
            focus_character_profile_id: self.focus_character_profile_id,
            hold: self.hold,
            player_knowledge: self.player_knowledge.clone(),
        }
    }
}

impl From<GalgameLinePresentationV2> for GalgameLinePresentationDocumentV3 {
    fn from(presentation: GalgameLinePresentationV2) -> Self {
        Self {
            schema_version: GalgameLineDocumentSchemaVersion::V3,
            origin: crate::media_authoring::AdaptationOrigin::Added,
            beat_ref: None,
            visible_cast: presentation.visible_cast,
            enter: presentation.enter,
            exit: presentation.exit,
            focus_character_profile_id: presentation.focus_character_profile_id,
            hold: presentation.hold,
            player_knowledge: presentation.player_knowledge,
            cg: None,
            cg_note: None,
            cg_importance: None,
            transition: None,
            branch: None,
            branch_id: None,
            returns_to_scene_index: None,
            after_beat_ref: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GalgamePipelineStartStage {
    AdaptationPlan,
    BranchDesign,
    ScriptAdaptation,
    CharacterStaging,
    VisualPlanning,
    PersistScript,
    AssetGeneration,
}

impl GalgamePipelineStartStage {
    pub const ALL: [Self; 7] = [
        Self::AdaptationPlan,
        Self::BranchDesign,
        Self::ScriptAdaptation,
        Self::CharacterStaging,
        Self::VisualPlanning,
        Self::PersistScript,
        Self::AssetGeneration,
    ];

    pub const fn key(self) -> &'static str {
        self.status().0
    }

    pub const fn order(self) -> i32 {
        self.status().1
    }

    pub const fn message(self) -> &'static str {
        self.status().2
    }

    pub const fn checkpoint_keys_to_clear(self) -> &'static [&'static str] {
        match self {
            Self::AdaptationPlan => &[
                "scenes",
                "branch_plan",
                "lines_draft",
                "lines_staged",
                "visual_plan",
                "persist_script",
                "asset_generation",
            ],
            Self::BranchDesign => &[
                "branch_plan",
                "lines_draft",
                "lines_staged",
                "visual_plan",
                "persist_script",
                "asset_generation",
            ],
            Self::ScriptAdaptation => &[
                "lines_draft",
                "lines_staged",
                "visual_plan",
                "persist_script",
                "asset_generation",
            ],
            Self::CharacterStaging => &[
                "lines_staged",
                "visual_plan",
                "persist_script",
                "asset_generation",
            ],
            Self::VisualPlanning => &["visual_plan", "persist_script", "asset_generation"],
            Self::PersistScript => &["persist_script", "asset_generation"],
            Self::AssetGeneration => &["asset_generation"],
        }
    }

    const fn status(self) -> (&'static str, i32, &'static str) {
        match self {
            Self::AdaptationPlan => ("adaptation_plan", 1, "正在准备本项目的改编计划"),
            Self::BranchDesign => ("branch_design", 2, "正在基于导演节拍设计 Galgame 分支"),
            Self::ScriptAdaptation => ("script_adaptation", 3, "正在改编 Galgame 主线与支线剧本行"),
            Self::CharacterStaging => ("character_staging", 4, "正在绑定角色并标注演出状态"),
            Self::VisualPlanning => ("visual_planning", 5, "正在规划背景与事件 CG"),
            Self::PersistScript => ("persist_script", 6, "正在写入 Galgame 剧本"),
            Self::AssetGeneration => ("asset_generation", 7, "正在创建 Galgame 资产生成意图"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::GalgameLinePresentationDocumentV3;
    use serde_json::json;

    #[test]
    fn presentation_document_requires_nullable_wire_fields() {
        let canonical = json!({
            "schema_version": "galgame_line_presentation_v3",
            "origin": "added", "beat_ref": null,
            "visible_cast": [],
            "enter": [],
            "exit": [],
            "focus_character_profile_id": null,
            "hold": false,
            "player_knowledge": [],
            "cg": null,
            "cg_note": null,
            "cg_importance": null,
            "transition": null,
            "branch": null,
            "branch_id": null,
            "returns_to_scene_index": null,
            "after_beat_ref": null
        });

        serde_json::from_value::<GalgameLinePresentationDocumentV3>(canonical.clone())
            .expect("canonical presentation");
        for field in [
            "origin",
            "beat_ref",
            "focus_character_profile_id",
            "cg",
            "cg_note",
            "cg_importance",
            "transition",
            "branch",
            "branch_id",
            "returns_to_scene_index",
            "after_beat_ref",
        ] {
            let mut missing = canonical.clone();
            missing
                .as_object_mut()
                .expect("presentation object")
                .remove(field);
            assert!(
                serde_json::from_value::<GalgameLinePresentationDocumentV3>(missing).is_err(),
                "missing {field} must fail"
            );
        }
    }
}
