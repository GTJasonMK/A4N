use a4n_error::AppError;
use a4n_llm::{provider::ChatRequest, trace::context_aware_chat_request_trace_json};
use serde_json::{Value, json};

#[derive(Debug, Clone)]
pub struct ChapterAnalysisRequestStep {
    pub key: String,
    pub title: String,
    pub prompt_name: String,
    pub request: ChatRequest,
}

pub fn chapter_analysis_stage_fingerprint(
    stage: &str,
    request: &ChatRequest,
) -> Result<Value, AppError> {
    Ok(json!({
        "schema_version": 1,
        "stage": stage,
        "request": context_aware_chat_request_trace_json(request)?,
    }))
}

#[cfg(test)]
mod tests {
    use a4n_llm::provider::ChatRequest;

    use super::chapter_analysis_stage_fingerprint;

    #[test]
    fn stage_fingerprint_keeps_stage_and_request_trace() {
        let request = ChatRequest::system_user("system", "user", Some(0.2), None);
        let fingerprint = chapter_analysis_stage_fingerprint("chapter_fact_extraction", &request)
            .expect("fingerprint");

        assert_eq!(fingerprint["schema_version"], 1);
        assert_eq!(fingerprint["stage"], "chapter_fact_extraction");
        assert_eq!(fingerprint["request"]["messages"][0]["content"], "system");
    }
}
