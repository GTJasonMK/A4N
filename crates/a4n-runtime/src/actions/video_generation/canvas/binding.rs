use super::super::{
    CLIPROXY_OPENAI_VIDEO_BACKEND, CliproxyGrokReferenceMode, GenerateCanvasVideoInput,
    ProviderImageInputTransport, append_provider_input_note, cliproxy_grok_reference_mode,
    remote_temp_upload_configured, video_gen_config,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::actions::video_generation) struct CanvasVideoProviderImageBinding {
    pub(in crate::actions::video_generation) first_frame_image_id: Option<i64>,
    pub(in crate::actions::video_generation) last_frame_image_id: Option<i64>,
    pub(in crate::actions::video_generation) reference_image_ids: Vec<i64>,
}

pub(in crate::actions::video_generation) fn canvas_video_provider_image_binding(
    cfg: &video_gen_config::Model,
    input: &GenerateCanvasVideoInput,
    provider_input_note: &mut Option<String>,
) -> Result<CanvasVideoProviderImageBinding, String> {
    if cfg.backend != CLIPROXY_OPENAI_VIDEO_BACKEND {
        return Ok(CanvasVideoProviderImageBinding {
            first_frame_image_id: input.first_frame_image_id,
            last_frame_image_id: input.last_frame_image_id,
            reference_image_ids: input.reference_image_ids.clone(),
        });
    }

    match cliproxy_grok_reference_mode(cfg)? {
        CliproxyGrokReferenceMode::FirstFrame => Ok(CanvasVideoProviderImageBinding {
            first_frame_image_id: input.first_frame_image_id,
            last_frame_image_id: input.last_frame_image_id,
            reference_image_ids: input.reference_image_ids.clone(),
        }),
        CliproxyGrokReferenceMode::Reference => {
            let reference_image_ids = canvas_video_reference_ids_with_frame_images(
                input.first_frame_image_id,
                input.last_frame_image_id,
                &input.reference_image_ids,
            );
            if input.first_frame_image_id.is_some() || input.last_frame_image_id.is_some() {
                append_provider_input_note(
                    provider_input_note,
                    format!(
                        "CLIProxy/Grok 参考模式已将画布首帧/尾帧输入作为普通 reference_images 发送；实际参考图 ID 顺序：{}",
                        reference_image_ids
                            .iter()
                            .map(i64::to_string)
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                );
            }
            Ok(CanvasVideoProviderImageBinding {
                first_frame_image_id: None,
                last_frame_image_id: None,
                reference_image_ids,
            })
        }
    }
}

pub(super) fn default_canvas_video_image_transport() -> Result<ProviderImageInputTransport, String>
{
    let transport = if remote_temp_upload_configured()? {
        ProviderImageInputTransport::RemoteTempUpload
    } else {
        ProviderImageInputTransport::InlineDataUrl
    };
    Ok(transport)
}

fn canvas_video_reference_ids_with_frame_images(
    first_frame_image_id: Option<i64>,
    last_frame_image_id: Option<i64>,
    reference_image_ids: &[i64],
) -> Vec<i64> {
    let mut ids = Vec::new();
    for id in first_frame_image_id
        .into_iter()
        .chain(last_frame_image_id)
        .chain(reference_image_ids.iter().copied())
    {
        if id > 0 && !ids.contains(&id) {
            ids.push(id);
        }
    }
    ids
}
