use super::super::super::*;

mod chat;
mod cliproxy;
mod gemini;
mod newapi;
mod openai;
mod unsupported;

use chat::chat_preview_payload;
use cliproxy::cliproxy_preview_payload;
use gemini::gemini_preview_payload;
use newapi::newapi_preview_payload;
use openai::openai_compatible_preview_payload;
use unsupported::unsupported_backend_error;

pub(in crate::actions::video_generation) fn video_provider_preview_payload(
    context: &ChapterVideoProviderRequestContext,
) -> Result<(&'static str, &'static str, String, Value), String> {
    if context.cfg.backend == NEWAPI_VIDEO_BACKEND {
        return newapi_preview_payload(context);
    }

    if context.cfg.backend == OPENAI_COMPATIBLE_VIDEO_BACKEND {
        return Ok(openai_compatible_preview_payload(context));
    }

    if context.cfg.backend == CLIPROXY_OPENAI_VIDEO_BACKEND {
        return cliproxy_preview_payload(context);
    }

    if context.cfg.backend == GEMINI_VEO_VIDEO_BACKEND {
        return gemini_preview_payload(context);
    }

    if context.cfg.backend == OPENAI_CHAT_VIDEO_BACKEND {
        return Ok(chat_preview_payload(context));
    }

    Err(unsupported_backend_error(&context.cfg.backend))
}
