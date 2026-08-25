mod generated_media;
mod orchestration;
mod project;

#[cfg(test)]
pub(super) use generated_media::{
    import_video_studio_generated_images, import_video_studio_generated_videos,
    update_imported_video_source_refs,
};

use super::types::{VideoStudioProjectImportIdMaps, VideoStudioProjectImportMediaFileWriter};
use sea_orm::ConnectionTrait;

pub use orchestration::import_video_studio_project_record;

pub(super) struct VideoStudioProjectImportMediaRowsContext<'a, C, Writer>
where
    C: ConnectionTrait,
    Writer: VideoStudioProjectImportMediaFileWriter,
{
    pub(super) db: &'a C,
    pub(super) project_id: i64,
    pub(super) maps: &'a mut VideoStudioProjectImportIdMaps,
    pub(super) media_file_writer: &'a mut Writer,
    pub(super) now: chrono::NaiveDateTime,
}
