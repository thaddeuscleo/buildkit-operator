use kube::Client;
use tracing_subscriber::EnvFilter;
use tracing::info;

mod controller;
mod crd;
mod resources;
mod status;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,buildkit_operator=debug")),
        )
        .init();

    info!("starting buildkit-operator");

    let client = Client::try_default().await?;

    controller::run(client).await;

    Ok(())
}
