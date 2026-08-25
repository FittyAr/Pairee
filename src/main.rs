//! Thin binary entry: the application lives in the `pairee` library crate.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    pairee::run().await
}
