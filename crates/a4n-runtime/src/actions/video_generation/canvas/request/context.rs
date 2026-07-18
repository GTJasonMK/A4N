use a4n_core::models::ai::video_gen_config;
use a4n_video::VideoGenerationRequest;

pub(in crate::actions::video_generation::canvas) struct CanvasVideoProviderRequestContext {
    pub(in crate::actions::video_generation::canvas) project_id: i64,
    pub(in crate::actions::video_generation::canvas) graph_id: i64,
    pub(in crate::actions::video_generation::canvas) node_id: i64,
    pub(in crate::actions::video_generation::canvas) workflow_run_id: Option<i64>,
    pub(in crate::actions::video_generation::canvas) source_node_ids: Vec<i64>,
    pub(in crate::actions::video_generation::canvas) cfg: video_gen_config::Model,
    pub(in crate::actions::video_generation::canvas) request: VideoGenerationRequest,
    pub(in crate::actions::video_generation::canvas) provider_input_note: Option<String>,
    pub(in crate::actions::video_generation::canvas) first_frame_image_id: Option<i64>,
    pub(in crate::actions::video_generation::canvas) last_frame_image_id: Option<i64>,
    pub(in crate::actions::video_generation::canvas) reference_image_ids: Vec<i64>,
}
