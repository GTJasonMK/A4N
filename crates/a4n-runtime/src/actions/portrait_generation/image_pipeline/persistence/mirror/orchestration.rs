use a4n_core::models::character::character_portrait;

use super::{
    PortraitImageMirror, mirror_portrait_to_generated_image, rollback_failed_portrait_creation,
};
use crate::AppState;

pub(in crate::actions::portrait_generation) async fn mirror_or_rollback(
    state: &AppState,
    portrait: &character_portrait::Model,
    image_path: &str,
    mirror: PortraitImageMirror<'_>,
) -> Result<(), String> {
    if let Err(err) = mirror_portrait_to_generated_image(state, mirror).await {
        return match rollback_failed_portrait_creation(state, portrait.id, image_path).await {
            Ok(()) => Err(err),
            Err(cleanup_err) => Err(format!("{err}；{cleanup_err}")),
        };
    }
    Ok(())
}
