use a4n_db::canvas::CanvasGraphRecordDocument;
use a4n_domain::canvas::CanvasNodeRecord;

use crate::canvas::CanvasNodeRunHandle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanvasOutputDataType {
    Text,
    Image,
    Audio,
    Video,
}

impl CanvasOutputDataType {
    pub(crate) fn from_output_node_kind(kind: &str) -> Result<Self, String> {
        match kind {
            "TextOutputNode" => Ok(Self::Text),
            "ImageOutputNode" => Ok(Self::Image),
            "AudioOutputNode" => Ok(Self::Audio),
            "VideoOutputNode" => Ok(Self::Video),
            other => Err(format!("不支持的画布输出节点类型: {other}")),
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Image => "image",
            Self::Audio => "audio",
            Self::Video => "video",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasOutputValue {
    pub text: Option<String>,
    pub generated_image_id: Option<i64>,
    pub generated_video_id: Option<i64>,
    pub media_asset_id: Option<i64>,
    pub source_node_ids: Vec<i64>,
}

pub struct CanvasOutputNodeExecutionInput<'a> {
    pub project_id: i64,
    pub graph_id: i64,
    pub node: &'a CanvasNodeRecord,
    pub document: &'a CanvasGraphRecordDocument,
    pub shared_run: Option<CanvasNodeRunHandle>,
    pub mode: &'a str,
}
