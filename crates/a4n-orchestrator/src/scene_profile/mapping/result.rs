use a4n_db::scene_profile::SceneSyncRecordResult;

use super::super::contracts::SceneSyncResult;

/// 将 DB 场景同步结果映射为 service 侧 `SceneSyncResult`。
pub fn scene_sync_result_from_record(result: SceneSyncRecordResult) -> SceneSyncResult {
    SceneSyncResult {
        profile_ids: result.profile_ids,
        profile_ref_ids: result.profile_ref_ids,
        occurrence_count: result.occurrence_count,
        transition_count: result.transition_count,
        entity_state_count: result.entity_state_count,
    }
}
