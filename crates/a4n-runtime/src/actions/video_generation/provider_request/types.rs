use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::actions::video_generation) enum CliproxyGrokReferenceMode {
    Reference,
    FirstFrame,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::actions::video_generation) enum ProviderGenerationWorkflow {
    BoundaryFrames,
    FirstFrameChain,
    Storyboard,
}

impl ProviderGenerationWorkflow {
    pub(in crate::actions::video_generation) fn as_str(self) -> &'static str {
        match self {
            Self::BoundaryFrames => "boundary_frames",
            Self::FirstFrameChain => "first_frame_chain",
            Self::Storyboard => "storyboard",
        }
    }

    pub(in crate::actions::video_generation) fn uses_first_frame_chain(self) -> bool {
        matches!(self, Self::FirstFrameChain)
    }

    pub(in crate::actions::video_generation) fn uses_storyboard(self) -> bool {
        matches!(self, Self::Storyboard)
    }
}

pub(in crate::actions::video_generation) fn cliproxy_grok_reference_mode(
    cfg: &video_gen_config::Model,
) -> Result<CliproxyGrokReferenceMode, String> {
    let Some(value) = cfg
        .provider_options_json
        .as_ref()
        .filter(|value| !value.is_null())
    else {
        return Ok(CliproxyGrokReferenceMode::Reference);
    };
    let Value::Object(map) = value else {
        return Err("视频 provider_options_json 必须是 JSON 对象".to_string());
    };
    let Some(mode) = map
        .get("grok_reference_mode")
        .filter(|value| !value.is_null())
    else {
        return Ok(CliproxyGrokReferenceMode::Reference);
    };
    let mode = mode
        .as_str()
        .map(str::trim)
        .ok_or_else(|| "视频 provider_options_json.grok_reference_mode 必须是字符串".to_string())?;
    match mode {
        "reference" => Ok(CliproxyGrokReferenceMode::Reference),
        "first_frame" => Ok(CliproxyGrokReferenceMode::FirstFrame),
        other => Err(format!(
            "视频 provider_options_json.grok_reference_mode 无效：{other}"
        )),
    }
}

#[cfg(feature = "embedded-media-providers")]
pub(in crate::actions::video_generation) fn cliproxy_grok_reference_mode_label(
    mode: CliproxyGrokReferenceMode,
) -> &'static str {
    match mode {
        CliproxyGrokReferenceMode::Reference => "参考模式",
        CliproxyGrokReferenceMode::FirstFrame => "首帧模式",
    }
}
