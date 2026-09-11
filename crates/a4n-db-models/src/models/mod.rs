//! 数据模型模块
//!
//! 按业务领域分类：
//! - `user/`    用户与账户
//! - `project/` 项目与蓝图
//! - `chapter/` 章节与大纲
//! - `character/` 角色与属性
//! - `asset/`   素材与物体档案
//! - `task/`    任务队列
//! - `ai/`      AI 配置与生成

pub mod user {
    pub mod conversation;
    pub mod invite_code;
    pub mod llm_usage_log;
    pub mod message;
    pub mod role;
    pub mod theme_config;
    pub mod theme_preference;
    pub mod theme_resource;
    pub mod usage_stats;
    #[path = "user.rs"]
    pub mod user_entity;
    pub use user_entity as user;
    pub mod user_session;
}

pub mod project {
    pub mod blueprint_character;
    pub mod blueprint_relationship;
    pub mod blueprint_version;
    pub mod import_review_queue;
    pub mod novel_blueprint;
    pub mod novel_project;
    pub mod profile_evidence;
    pub mod project_invite;
    pub mod project_member;
    pub mod project_source_snapshot;
    pub mod project_status_history;
    pub mod source_snapshot_chapter;
    pub mod source_snapshot_outline;
    pub mod source_snapshot_profile;
    pub mod text_anchor;
}

pub mod canvas {
    pub mod canvas_edge;
    pub mod canvas_graph;
    pub mod canvas_node;
    pub mod canvas_text_asset;
}

pub mod generation {
    pub mod generation_run;
    pub mod generation_run_target;
    pub mod generation_spec;
    pub mod generation_spec_entity_ref;
    pub mod generation_spec_media_ref;
    pub mod generation_spec_text_anchor;
    pub mod generation_start_request;
    pub mod generation_task_checkpoint;
    pub mod llm_call_trace;
    pub mod llm_context_pack;
    pub mod prompt_artifact;
    pub mod workflow_checkpoint;
    pub mod workflow_progress_event;
    pub mod workflow_run;
}

pub mod media {
    pub mod image_publication;
    pub mod media_asset;
    pub mod media_binding;
    pub mod media_provenance;
}

pub mod package;

pub mod video_project {
    pub mod assembly_clip;
    pub mod assembly_timeline;
    pub mod video_segment;
}

pub mod galgame {
    pub mod galgame_choice;
    pub mod galgame_line;
    pub mod galgame_scene;
    pub mod galgame_script;
}

pub mod source_sync {
    pub mod source_sync_conflict;
    pub mod source_sync_decision;
    pub mod source_sync_run;
}

pub mod chapter {
    #[path = "chapter.rs"]
    pub mod chapter_entity;
    pub use chapter_entity as chapter;
    pub mod chapter_manga_prompt;
    pub mod chapter_outline;
    pub mod chapter_outline_provenance;
    pub mod chapter_scene;
    pub mod chapter_shooting_script;
    pub mod chapter_video_prompt;
    pub mod chapter_video_timeline;
    pub mod chapter_video_timeline_item;
    pub mod chapter_visual_plan;
    pub mod part_outline;
}

pub mod character {
    pub mod attribute_event;
    pub mod character_portrait;
    pub mod character_state_index;
    pub mod foreshadowing_index;
    pub mod protagonist_behavior_record;
    pub mod protagonist_deletion_mark;
    pub mod protagonist_profile;
    pub mod snapshot_reference;
}

pub mod asset {
    pub mod object_profile;
    pub mod profile_primary_material;
}

pub mod scene {
    pub mod scene_entity_state;
    pub mod scene_occurrence;
    pub mod scene_profile;
    pub mod scene_transition;
}

pub mod task {
    pub mod cleanup_outbox;
    pub mod task_dependency;
    pub mod task_queue;
}

pub mod ai {
    pub mod embedding_config;
    pub mod generated_image;
    pub mod generated_video;
    pub mod image_gen_config;
    pub mod llm_config;
    pub mod optimization_patch;
    pub mod optimization_patch_set;
    pub mod optimization_session;
    pub mod optimization_session_message;
    pub mod optimization_turn;
    pub mod optimization_turn_event;
    pub mod system_settings;
    pub mod video_gen_config;
}

pub mod agent;
