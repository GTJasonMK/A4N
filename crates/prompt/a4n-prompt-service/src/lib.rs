mod app;
mod handlers;
mod protocol;

#[cfg(test)]
mod tests;

#[tokio::main]
pub async fn run() -> anyhow::Result<()> {
    app::run().await
}

#[tokio::main]
pub async fn run_from_args() -> anyhow::Result<()> {
    app::run_from_args().await
}
