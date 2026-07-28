use a4n_core::video_execution::{
    ChapterVideoWorkflowMode, VIDEO_PROMPT_MEDIA_INPUT_CONTRACT_V1,
    VideoPromptMediaInputContractV1, VideoPromptMediaInputV1, VideoPromptMediaRoleV1,
    VideoPromptMediaSourceV1, VideoPromptTransportV1,
};

pub(super) fn boundary_frame_media_contract(
    start_frame_number: i32,
    end_frame_number: i32,
) -> VideoPromptMediaInputContractV1 {
    VideoPromptMediaInputContractV1 {
        schema_version: VIDEO_PROMPT_MEDIA_INPUT_CONTRACT_V1.to_string(),
        transport: VideoPromptTransportV1::AppProvider,
        inputs: vec![
            prompt_image_input(
                VideoPromptMediaRoleV1::StartFrame,
                ChapterVideoWorkflowMode::BoundaryFrames,
                "frame_prompt",
                start_frame_number,
                "video_boundary_frame_prompt",
            ),
            prompt_image_input(
                VideoPromptMediaRoleV1::EndFrame,
                ChapterVideoWorkflowMode::BoundaryFrames,
                "frame_prompt",
                end_frame_number,
                "video_boundary_frame_prompt",
            ),
        ],
        capture_tail_frame: false,
    }
}

pub(super) fn first_frame_chain_media_contract(
    shot_number: i32,
    capture_tail_frame: bool,
) -> VideoPromptMediaInputContractV1 {
    let source = if shot_number == 1 {
        VideoPromptMediaSourceV1::PromptImage {
            workflow_mode: ChapterVideoWorkflowMode::FirstFrameChain,
            prompt_type: "first_frame_prompt".to_string(),
            shot_number: 1,
            generation_source: "video_first_frame_prompt".to_string(),
        }
    } else {
        VideoPromptMediaSourceV1::PreviousVideoTail {
            workflow_mode: ChapterVideoWorkflowMode::FirstFrameChain,
            previous_shot_number: shot_number - 1,
        }
    };
    VideoPromptMediaInputContractV1 {
        schema_version: VIDEO_PROMPT_MEDIA_INPUT_CONTRACT_V1.to_string(),
        transport: VideoPromptTransportV1::AppProvider,
        inputs: vec![VideoPromptMediaInputV1 {
            role: VideoPromptMediaRoleV1::StartFrame,
            source,
            required: true,
        }],
        capture_tail_frame,
    }
}

pub(super) fn storyboard_media_contract(shot_number: i32) -> VideoPromptMediaInputContractV1 {
    VideoPromptMediaInputContractV1 {
        schema_version: VIDEO_PROMPT_MEDIA_INPUT_CONTRACT_V1.to_string(),
        transport: VideoPromptTransportV1::AppProvider,
        inputs: vec![prompt_image_input(
            VideoPromptMediaRoleV1::StoryboardReference,
            ChapterVideoWorkflowMode::Storyboard,
            "storyboard_prompt",
            shot_number,
            "video_storyboard_prompt",
        )],
        capture_tail_frame: false,
    }
}

pub(super) fn chat_video_media_contract() -> VideoPromptMediaInputContractV1 {
    VideoPromptMediaInputContractV1 {
        schema_version: VIDEO_PROMPT_MEDIA_INPUT_CONTRACT_V1.to_string(),
        transport: VideoPromptTransportV1::ExternalChat,
        inputs: Vec::new(),
        capture_tail_frame: false,
    }
}

fn prompt_image_input(
    role: VideoPromptMediaRoleV1,
    workflow_mode: ChapterVideoWorkflowMode,
    prompt_type: &str,
    shot_number: i32,
    generation_source: &str,
) -> VideoPromptMediaInputV1 {
    VideoPromptMediaInputV1 {
        role,
        source: VideoPromptMediaSourceV1::PromptImage {
            workflow_mode,
            prompt_type: prompt_type.to_string(),
            shot_number,
            generation_source: generation_source.to_string(),
        },
        required: true,
    }
}
