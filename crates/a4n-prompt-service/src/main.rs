mod app;
mod contracts;
mod handlers;
mod protocol;

#[cfg(test)]
mod tests;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    app::run_from_args().await
}
