use super::*;

pub(super) fn package_with_regenerate_scene_decision(
    decision_payload_json: Value,
) -> ProjectFamilyPackage {
    let mut package = package_with_galgame_project();
    let snapshot_chapter = package.source_snapshot_chapters[0].clone();
    package.text_core.chapters[0] = chapter_record(snapshot_chapter.source_chapter_id, 1, None);
    package.text_core.character_profiles = vec![character_profile_record(601)];
    package.text_core.scene_profiles = vec![scene_profile_record(701)];
    package.text_anchors = vec![snapshot_text_anchor_record(&snapshot_chapter)];
    package.source_sync_runs = vec![source_sync_run_record()];
    let mut conflict = handled_source_sync_conflict_for_decision(
        SourceSyncConflictStatus::Resolved.as_str(),
        SourceSyncResolutionStrategy::RegenerateScene.as_str(),
        decision_payload_json,
    );
    conflict.conflict_scope = "text_anchor".to_string();
    conflict.target_kind = "galgame_scene".to_string();
    conflict.target_id = 2201;
    package.source_sync_conflicts = vec![conflict];
    package
}

pub(super) fn regenerate_scene_decision_payload() -> Value {
    let adaptation_projection = json!({
        "scenes": [{
            "index": 1,
            "origin": "adapted",
            "title": "雨后街道",
            "location": "街道",
            "time_of_day": "清晨",
            "participants": ["林晓雨"],
            "summary": "雨停后重新确认去向",
            "source_span_ids": ["ch1:ta702"],
            "adaptation_plan_id": 2401,
            "adaptation_plan_revision": 1,
            "source_chapter_id": 2001,
            "source_snapshot_chapter_id": 1001,
            "chapter_number": 1,
            "unit_id": "opening",
            "beats": [{
                "id": "narration", "kind": "action", "is_climax": false, "origin": "adapted", "importance": "required",
                "action": "角色抬头观察雨后的街道",
                "reaction": "同伴停下等待",
                "entry_state": "站在街道入口",
                "exit_state": "确认街道出口",
                "source_span_ids": ["ch1:ta702"]
            }]
        }]
    });
    let script_adaptation = json!({
        "scene_lines": [{
            "scene_index": 1,
            "lines": [{
                "index": 1,
                "origin": "adapted", "beat_ref": {"source_chapter_id":2001, "unit_id":"opening", "beat_id":"narration"},
                "line_type": "narration",
                "speaker_name": null,
                "text": "雨停了。",
                "source_span_ids": ["ch1:ta702"]
            }]
        }],
        "branch_scene_lines": []
    });
    let character_staging = json!({
        "scene_lines": [{
            "scene_index": 1,
            "lines": [{
                "index": 1,
                "origin": "adapted", "beat_ref": {"source_chapter_id":2001, "unit_id":"opening", "beat_id":"narration"},
                "line_type": "dialogue",
                "speaker_profile_id": 601,
                "speaker_name": "林晓雨",
                "text": "雨停了。",
                "presentation": {
                    "schema_version": "galgame_line_presentation_v2",
                    "visible_cast": [{
                        "character_profile_id": 601,
                        "expression": "neutral",
                        "pose": "standing",
                        "position": "center",
                        "facing": "front",
                        "layer": "front"
                    }],
                    "enter": [],
                    "exit": [],
                    "focus_character_profile_id": 601,
                    "hold": false,
                    "player_knowledge": []
                },
                "source_span_ids": ["ch1:ta702"]
            }]
        }],
        "branch_scene_lines": []
    });
    let visual_planning = json!({
        "backgrounds": [{
            "scene_index": 1,
            "background_note": "雨后的街道。",
            "scene_profile_id": 701,
            "source_span_ids": ["ch1:ta702"]
        }],
        "event_cgs": []
    });
    json!({
        "user_action": "regenerate_scene",
        "galgame_scene_id": 2201,
        "reason": "source_changed",
        "result": {
            "galgame_scene_id": 2201,
            "old_source_snapshot_chapter_id": 1001,
            "new_source_snapshot_chapter_id": 1001,
            "old_line_count": 1,
            "new_line_count": 1,
            "deleted_generation_spec_count": 0,
            "deleted_portrait_generation_spec_count": 0,
            "resolved_related_conflict_count": 0,
            "mode": "llm_partial_regeneration",
            "stage_outputs": {
                "adaptation_projection": adaptation_projection,
                "script_adaptation": script_adaptation,
                "character_staging": character_staging,
                "visual_planning": visual_planning,
                "branch_design": {
                    "choices": []
                }
            }
        }
    })
}
