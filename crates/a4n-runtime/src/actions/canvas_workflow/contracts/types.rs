use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GetCanvasNodeContractsInput {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasPortContract {
    pub id: String,
    pub label: String,
    pub data_type: String,
    pub direction: String,
    pub required: bool,
    pub multiple: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasNodeContract {
    pub kind: String,
    pub label: String,
    pub category: String,
    pub runnable: bool,
    pub inputs: Vec<CanvasPortContract>,
    pub outputs: Vec<CanvasPortContract>,
}
