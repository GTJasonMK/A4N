use super::{
    AppState, BoundaryFrameImageIds, ProviderGenerationWorkflow, VideoProviderRequestPurpose,
    append_provider_input_note, boundary_frame_image_ids_for_prompt,
    boundary_frame_missing_image_message, first_frame_chain_missing_start_image_message,
    first_frame_chain_start_image_id_for_prompt, reference_ids_with_boundary_frame_images,
    reference_ids_with_storyboard_image, storyboard_image_id_for_prompt,
    storyboard_missing_image_message,
};
use a4n_core::models::chapter::chapter_video_prompt;
use serde_json::{Value, json};

pub(super) struct ChapterProviderReferences {
    pub(super) boundary_frame_ids: BoundaryFrameImageIds,
    pub(super) storyboard_image_id: Option<i64>,
    pub(super) provider_reference_image_ids_json: Option<Value>,
    pub(super) provider_reference_image_count: usize,
    pub(super) provider_reference_image_ids: Vec<i64>,
    pub(super) provider_reference_ids_are_authoritative: bool,
}

pub(super) struct ChapterProviderReferenceRequest<'a> {
    pub(super) state: &'a AppState,
    pub(super) project_id: i64,
    pub(super) chapter_id: i64,
    pub(super) prompt: &'a chapter_video_prompt::Model,
    pub(super) prompt_reference_image_ids: &'a [i64],
    pub(super) prompt_reference_image_ids_json: Option<&'a Value>,
    pub(super) provider_workflow: ProviderGenerationWorkflow,
    pub(super) cliproxy_reference_mode: bool,
    pub(super) purpose: VideoProviderRequestPurpose,
}

pub(super) async fn resolve_chapter_provider_references(
    request: ChapterProviderReferenceRequest<'_>,
    provider_input_note: &mut Option<String>,
) -> Result<ChapterProviderReferences, String> {
    let ChapterProviderReferenceRequest {
        state,
        project_id,
        chapter_id,
        prompt,
        prompt_reference_image_ids,
        prompt_reference_image_ids_json,
        provider_workflow,
        cliproxy_reference_mode,
        purpose,
    } = request;
    let first_frame_chain = provider_workflow.uses_first_frame_chain();
    let storyboard_workflow = provider_workflow.uses_storyboard();
    let (boundary_frame_ids, storyboard_image_id) = if first_frame_chain {
        let start =
            first_frame_chain_start_image_id_for_prompt(state, project_id, chapter_id, prompt)
                .await?;
        if start.is_none() {
            let message = first_frame_chain_missing_start_image_message(prompt);
            if purpose == VideoProviderRequestPurpose::Generate {
                return Err(message);
            }
            append_provider_input_note(
                provider_input_note,
                format!(
                    "当前首帧串联起始帧缺失：{message}；此处仅展示 provider 请求结构，真实生成仍需补齐该输入"
                ),
            );
        }
        (
            BoundaryFrameImageIds {
                first: start,
                last: None,
            },
            None,
        )
    } else if storyboard_workflow {
        let storyboard_image_id =
            storyboard_image_id_for_prompt(state, project_id, chapter_id, prompt).await?;
        if storyboard_image_id.is_none() {
            let message = storyboard_missing_image_message(prompt);
            if purpose == VideoProviderRequestPurpose::Generate {
                return Err(message);
            }
            append_provider_input_note(
                provider_input_note,
                format!(
                    "当前分镜图缺失：{message}；此处仅展示 provider 请求结构，真实生成仍需补齐该输入"
                ),
            );
        }
        (BoundaryFrameImageIds::default(), storyboard_image_id)
    } else {
        let frame_ids =
            boundary_frame_image_ids_for_prompt(state, project_id, chapter_id, prompt).await?;
        if let Some(message) = boundary_frame_missing_image_message(prompt, &frame_ids) {
            if purpose == VideoProviderRequestPurpose::Generate {
                return Err(message);
            }
            append_provider_input_note(
                provider_input_note,
                format!(
                    "当前首尾帧图片不完整：{message}；此处仅展示 provider 请求结构，真实生成仍需补齐该输入"
                ),
            );
        }
        (frame_ids, None)
    };
    let provider_reference_image_ids = if storyboard_workflow {
        reference_ids_with_storyboard_image(storyboard_image_id, prompt_reference_image_ids)
    } else if cliproxy_reference_mode {
        reference_ids_with_boundary_frame_images(&boundary_frame_ids, prompt_reference_image_ids)
    } else {
        prompt_reference_image_ids.to_vec()
    };
    let provider_reference_ids_are_authoritative = storyboard_workflow || cliproxy_reference_mode;
    let provider_reference_image_count = if provider_reference_ids_are_authoritative {
        provider_reference_image_ids.len()
    } else {
        prompt_reference_image_ids.len()
    };
    if cliproxy_reference_mode
        && (boundary_frame_ids.first.is_some() || boundary_frame_ids.last.is_some())
    {
        append_provider_input_note(
            provider_input_note,
            format!(
                "CLIProxy/Grok 参考模式已将边界帧图片作为普通 reference_images 发送；实际参考图 ID 顺序：{}",
                joined_ids(&provider_reference_image_ids)
            ),
        );
    }
    if storyboard_workflow && storyboard_image_id.is_some() {
        append_provider_input_note(
            provider_input_note,
            format!(
                "分镜模式已将分镜图作为第 1 张 reference_images 发送，后续为素材参考图；实际参考图 ID 顺序：{}",
                joined_ids(&provider_reference_image_ids)
            ),
        );
    }
    let provider_reference_image_ids_json = if provider_reference_ids_are_authoritative {
        Some(json!(provider_reference_image_ids.clone()))
    } else {
        prompt_reference_image_ids_json.cloned()
    };
    Ok(ChapterProviderReferences {
        boundary_frame_ids,
        storyboard_image_id,
        provider_reference_image_ids_json,
        provider_reference_image_count,
        provider_reference_image_ids,
        provider_reference_ids_are_authoritative,
    })
}

fn joined_ids(ids: &[i64]) -> String {
    ids.iter()
        .map(i64::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}
