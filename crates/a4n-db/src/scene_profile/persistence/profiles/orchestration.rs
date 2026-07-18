use std::collections::BTreeMap;

use sea_orm::ConnectionTrait;

use super::resolution::upsert_scene_profile_record;
use super::rules::SceneProfileDraftTracker;
use crate::scene_profile::{SceneProfileDbResult, SceneProfileRecordDraft};

pub(in crate::scene_profile) async fn upsert_scene_profile_records<C>(
    db: &C,
    project_id: i64,
    drafts: &[SceneProfileRecordDraft],
) -> SceneProfileDbResult<BTreeMap<String, i64>>
where
    C: ConnectionTrait,
{
    let mut tracker = SceneProfileDraftTracker::default();
    let mut map = BTreeMap::new();
    for draft in drafts {
        tracker.accept(draft)?;
        let profile = upsert_scene_profile_record(db, project_id, draft).await?;
        map.insert(draft.scene_ref.clone(), profile.id);
    }
    Ok(map)
}
