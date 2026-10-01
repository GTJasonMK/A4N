use a4n_db_models::models::{
    ai::generated_image,
    asset::{object_profile, profile_primary_material},
    scene::scene_profile,
};
use a4n_domain::profile_material::{
    ProfileMaterialImageRecord, ProfileMaterialObjectRecord, ProfileMaterialSceneRecord,
    ProfilePrimaryMaterialRecord,
};

pub fn scene_profile_source_record(profile: &scene_profile::Model) -> ProfileMaterialSceneRecord {
    ProfileMaterialSceneRecord {
        id: profile.id,
        project_id: profile.project_id,
        name: profile.name.clone(),
        scene_type: profile.scene_type.clone(),
        importance: profile.importance.clone(),
        base_description: profile.base_description.clone(),
        visual_description: profile.visual_description.clone(),
        spatial_layout: profile.spatial_layout.clone(),
        atmosphere: profile.atmosphere.clone(),
        entry_points_json: profile.entry_points_json.clone(),
        rules_json: profile.rules_json.clone(),
        first_seen_chapter: profile.first_seen_chapter,
        last_seen_chapter: profile.last_seen_chapter,
        image_prompt_seed: profile.image_prompt_seed.clone(),
        updated_at: profile.updated_at,
    }
}

pub fn object_profile_source_record(
    profile: &object_profile::Model,
) -> ProfileMaterialObjectRecord {
    ProfileMaterialObjectRecord {
        id: profile.id,
        project_id: profile.project_id,
        name: profile.name.clone(),
        object_type: profile.object_type.clone(),
        importance: profile.importance.clone(),
        visual_description: profile.visual_description.clone(),
        material_and_state: profile.material_and_state.clone(),
        story_usage: profile.story_usage.clone(),
        visual_usage: profile.visual_usage.clone(),
        generation_usage: profile.generation_usage.clone(),
        location_context: profile.location_context.clone(),
        first_seen_chapter: profile.first_seen_chapter,
        last_seen_chapter: profile.last_seen_chapter,
        image_prompt_seed: profile.image_prompt_seed.clone(),
        updated_at: profile.updated_at,
    }
}

pub(super) fn image_record(image: generated_image::Model) -> ProfileMaterialImageRecord {
    ProfileMaterialImageRecord {
        id: image.id,
        project_id: image.project_id,
        image_type: image.image_type,
        file_path: image.file_path,
        prompt: image.prompt,
        model: image.model,
        width: image.width,
        height: image.height,
        file_size: image.file_size,
        ai_score: image.ai_score,
        is_favorite: image.is_favorite,
        chapter_id: image.chapter_id,
        panel_id: image.panel_id,
        metadata_json: image.metadata_json,
        created_at: image.created_at,
    }
}

pub(super) fn primary_material_record(
    material: profile_primary_material::Model,
) -> ProfilePrimaryMaterialRecord {
    ProfilePrimaryMaterialRecord {
        profile_kind: material.profile_kind,
        profile_id: material.profile_id,
        primary_image_id: material.primary_image_id,
    }
}
