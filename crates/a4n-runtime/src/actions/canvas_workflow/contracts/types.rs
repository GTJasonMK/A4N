use serde::{Deserialize, Serialize};

pub use a4n_api_contract::canvas::{CanvasNodeContract, CanvasPortContract};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GetCanvasNodeContractsInput {}
