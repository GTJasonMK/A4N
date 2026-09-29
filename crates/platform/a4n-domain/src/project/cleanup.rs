use serde::{Deserialize, Serialize};

/// 章节资料变更后需要由 Host 处理的派生数据清理结果。
///
/// 该投影只描述事实记录 ID 和变更标记；数据库摘要到此结构的
/// 转换仍由 Orchestrator 负责。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ChapterDerivedCleanupArtifacts {
    pub blueprint_facts_changed: bool,
    pub character_facts_changed: bool,
    pub stale_character_state_ids: Vec<i64>,
    pub stale_attribute_event_ids: Vec<i64>,
    pub changed_foreshadowing_ids: Vec<i64>,
    pub scene_records_changed: bool,
}

impl ChapterDerivedCleanupArtifacts {
    pub fn merge(&mut self, other: Self) {
        self.blueprint_facts_changed |= other.blueprint_facts_changed;
        self.character_facts_changed |= other.character_facts_changed;
        self.stale_character_state_ids
            .extend(other.stale_character_state_ids);
        self.stale_attribute_event_ids
            .extend(other.stale_attribute_event_ids);
        self.changed_foreshadowing_ids
            .extend(other.changed_foreshadowing_ids);
        self.scene_records_changed |= other.scene_records_changed;

        self.stale_character_state_ids.sort_unstable();
        self.stale_character_state_ids.dedup();
        self.stale_attribute_event_ids.sort_unstable();
        self.stale_attribute_event_ids.dedup();
        self.changed_foreshadowing_ids.sort_unstable();
        self.changed_foreshadowing_ids.dedup();
    }
}
