mod audio;
mod binding;

pub use self::audio::import_galgame_audio;
pub use self::binding::{bind_galgame_media, unbind_galgame_media};
pub use a4n_orchestrator::galgame::{
    BindGalgameMediaInput, ImportGalgameAudioInput, UnbindGalgameMediaInput,
};

#[cfg(test)]
use self::audio::persist_imported_galgame_audio;
#[cfg(test)]
mod tests;
