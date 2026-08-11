use a4n_orchestrator::rag::RagRecordSyncOutcome;
use tokio_util::sync::CancellationToken;

pub type RagProgressReporter<'a> = Option<&'a (dyn Fn(u32, u32, &str) + Send + Sync)>;

pub struct RagIngestProgress<'a> {
    current: u32,
    total: u32,
    reporter: RagProgressReporter<'a>,
    cancel_token: Option<&'a CancellationToken>,
}

impl<'a> RagIngestProgress<'a> {
    pub fn new(
        total: u32,
        reporter: RagProgressReporter<'a>,
        cancel_token: Option<&'a CancellationToken>,
    ) -> Self {
        Self {
            current: 0,
            total,
            reporter,
            cancel_token,
        }
    }

    pub fn stage(&self, message: impl AsRef<str>) -> Result<(), String> {
        self.check_cancelled()?;
        self.report(message.as_ref());
        Ok(())
    }

    pub fn advance(&mut self, amount: u32, message: impl AsRef<str>) -> Result<(), String> {
        self.check_cancelled()?;
        let current = self
            .current
            .checked_add(amount)
            .ok_or_else(|| format!("RAG 同步进度超过 u32 上限：{} + {amount}", self.current))?;
        if current > self.total {
            return Err(format!(
                "RAG 同步进度超过声明总量：{current} > {}",
                self.total
            ));
        }
        self.current = current;
        self.report(message.as_ref());
        Ok(())
    }

    pub fn check_cancelled(&self) -> Result<(), String> {
        if self
            .cancel_token
            .is_some_and(CancellationToken::is_cancelled)
        {
            return Err("RAG 同步已停止".to_string());
        }
        Ok(())
    }

    fn report(&self, message: &str) {
        if let Some(report) = self.reporter {
            report(self.current, self.total, message);
        }
    }
}

pub(in crate::rag_ingest) fn progress_stage(
    progress: &mut Option<&mut RagIngestProgress<'_>>,
    message: impl AsRef<str>,
) -> Result<(), String> {
    if let Some(progress) = progress.as_deref_mut() {
        progress.stage(message)?;
    }
    Ok(())
}

pub(in crate::rag_ingest::sync) fn progress_advance(
    progress: &mut Option<&mut RagIngestProgress<'_>>,
    amount: u32,
    message: impl AsRef<str>,
) -> Result<(), String> {
    if let Some(progress) = progress.as_deref_mut() {
        progress.advance(amount, message)?;
    }
    Ok(())
}

pub(in crate::rag_ingest::sync) fn sync_progress_message(
    outcome: RagRecordSyncOutcome,
    label: impl AsRef<str>,
) -> String {
    let label = label.as_ref();
    match outcome {
        RagRecordSyncOutcome::Added => format!("已新增{label}"),
        RagRecordSyncOutcome::Updated => format!("已更新{label}"),
        RagRecordSyncOutcome::Skipped => format!("已跳过未变化：{label}"),
    }
}
