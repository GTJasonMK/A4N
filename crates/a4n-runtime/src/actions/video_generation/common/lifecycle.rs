use super::super::job_lifecycle::has_active_provider_job as lifecycle_has_active_provider_job;

pub fn has_active_provider_job(provider_job_id: Option<&str>, status: &str) -> bool {
    lifecycle_has_active_provider_job(provider_job_id, status)
}
