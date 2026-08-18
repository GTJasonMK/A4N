use tokio_util::sync::CancellationToken;

pub struct RagIngestProgress<'a> {
    current: u32,
    total: u32,
    reporter: Option<&'a (dyn Fn(u32, u32, &str) + Send + Sync)>,
    cancel_token: Option<&'a CancellationToken>,
}

impl<'a> RagIngestProgress<'a> {
    pub fn new(
        total: u32,
        reporter: Option<&'a (dyn Fn(u32, u32, &str) + Send + Sync)>,
        cancel_token: Option<&'a CancellationToken>,
    ) -> Self {
        Self {
            current: 0,
            total,
            reporter,
            cancel_token,
        }
    }

    pub(crate) fn stage(&self, message: impl AsRef<str>) -> Result<(), String> {
        self.check_cancelled()?;
        self.report(message.as_ref());
        Ok(())
    }

    pub(crate) fn advance(&mut self, message: impl AsRef<str>) -> Result<(), String> {
        self.check_cancelled()?;
        let current = self
            .current
            .checked_add(1)
            .ok_or_else(|| format!("RAG 同步进度超过 u32 上限：{} + 1", self.current))?;
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

    fn check_cancelled(&self) -> Result<(), String> {
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
