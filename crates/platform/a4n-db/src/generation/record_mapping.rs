use a4n_db_models::models::generation::{generation_spec, prompt_artifact};
use a4n_domain::generation::{GenerationSpecRecord, PromptArtifactRecord};

pub fn generation_spec_record_from_model(model: generation_spec::Model) -> GenerationSpecRecord {
    GenerationSpecRecord {
        id: model.id,
        project_id: model.project_id,
        root_project_id: model.root_project_id,
        source_context_kind: model.source_context_kind,
        source_snapshot_id: model.source_snapshot_id,
        source_chapter_id: model.source_chapter_id,
        source_snapshot_chapter_id: model.source_snapshot_chapter_id,
        canvas_text_asset_id: model.canvas_text_asset_id,
        target_kind: model.target_kind,
        target_entity_kind: model.target_entity_kind,
        target_entity_id: model.target_entity_id,
        status: model.status,
        source_version_hash: model.source_version_hash,
        params_json: model.params_json,
        created_at: model.created_at,
        updated_at: model.updated_at,
    }
}

pub fn prompt_artifact_record_from_model(model: prompt_artifact::Model) -> PromptArtifactRecord {
    PromptArtifactRecord {
        id: model.id,
        generation_spec_id: model.generation_spec_id,
        prompt_kind: model.prompt_kind,
        prompt_text: model.prompt_text,
        renderer_version: model.renderer_version,
        edited_by_user: model.edited_by_user,
        created_at: model.created_at,
        updated_at: model.updated_at,
    }
}
