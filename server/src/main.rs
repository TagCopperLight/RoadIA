use tokio::io;
use server::api::server;

#[tokio::main]
async fn main() -> io::Result<()> {
    server::run().await?;
    Ok(())
}
