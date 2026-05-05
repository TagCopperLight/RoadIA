use tokio::io;
use server::api::runner::handlers;

#[tokio::main]
async fn main() -> io::Result<()> {
    dotenvy::dotenv().ok();
    handlers::run().await?;
    Ok(())
}
