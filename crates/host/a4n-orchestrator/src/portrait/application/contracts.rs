use a4n_api_contract::portrait::PortraitSummary;
use a4n_error::AppError;

#[cfg(test)]
pub(crate) use a4n_portrait_host_contract::DecodedPortraitImage;
pub(crate) use a4n_portrait_host_contract::{
    GeneratedPortraitImage, PortraitGenerationHostPort, PortraitImageProviderInput,
};

pub type PortraitProgress<'a> = &'a mut (dyn FnMut(u32, u32, &str) -> Result<(), AppError> + Send);

#[derive(Debug, Clone)]
pub struct PortraitBatchItem {
    pub item_key: String,
    pub item_index: u32,
    pub summary: PortraitSummary,
}

#[derive(Debug, Clone)]
pub struct PortraitBatchOutcome {
    pub total: u32,
    pub items: Vec<PortraitBatchItem>,
    pub failed_item_keys: Vec<String>,
    pub cancelled: u32,
    pub skipped: u32,
    pub interrupted: bool,
    pub first_error: Option<String>,
}

impl PortraitBatchOutcome {
    pub fn succeeded(&self) -> u32 {
        self.items.len() as u32
    }

    pub fn failed(&self) -> u32 {
        self.failed_item_keys.len() as u32
    }
}
