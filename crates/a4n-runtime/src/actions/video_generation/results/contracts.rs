pub(in crate::actions::video_generation) struct VideoProviderResultHandling<'a> {
    pub(in crate::actions::video_generation) prompt_id: Option<i64>,
    pub(in crate::actions::video_generation) completed_cancel_message: Option<&'a str>,
    pub(in crate::actions::video_generation) async_job_progress_message: &'a str,
    pub(in crate::actions::video_generation) missing_result_message: &'a str,
}
