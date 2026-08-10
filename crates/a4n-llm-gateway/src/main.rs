mod app;
mod handlers;
mod protocol;
#[cfg(test)]
mod tests;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    app::run().await
}
