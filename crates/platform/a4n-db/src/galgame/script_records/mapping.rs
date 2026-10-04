use a4n_db_models::models::galgame::{galgame_choice, galgame_line, galgame_scene};
use a4n_domain::galgame::{GalgameChoiceRecord, GalgameLineRecord, GalgameSceneRecord};

pub(crate) fn line_record(row: galgame_line::Model) -> GalgameLineRecord {
    GalgameLineRecord {
        id: row.id,
        scene_id: row.scene_id,
        line_index: row.line_index,
        line_type: row.line_type,
        speaker_profile_id: row.speaker_profile_id,
        text: row.text,
        expression: row.expression,
        pose: row.pose,
        position: row.position,
        presentation_json: row.presentation_json,
        text_anchor_id: row.text_anchor_id,
    }
}

pub(crate) fn scene_record(row: galgame_scene::Model) -> GalgameSceneRecord {
    GalgameSceneRecord {
        id: row.id,
        script_id: row.script_id,
        source_snapshot_chapter_id: row.source_snapshot_chapter_id,
        adaptation_unit_id: row.adaptation_unit_id,
        adaptation_plan_id: row.adaptation_plan_id,
        scene_index: row.scene_index,
        title: row.title,
        location: row.location,
        time_of_day: row.time_of_day,
        participants_json: row.participants_json,
        background_note: row.background_note,
        entry_kind: row.entry_kind,
        next_scene_id: row.next_scene_id,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

pub(crate) fn choice_record(row: galgame_choice::Model) -> GalgameChoiceRecord {
    GalgameChoiceRecord {
        id: row.id,
        line_id: row.line_id,
        option_index: row.option_index,
        option_text: row.option_text,
        target_scene_id: row.target_scene_id,
        created_at: row.created_at,
    }
}
