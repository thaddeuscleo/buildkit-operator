use kube::Client;

mod controller;
mod crd;
mod resources;
mod status;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::try_default().await?;

    controller::run(client).await;

    Ok(())
}
