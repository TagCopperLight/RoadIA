use tokio::io;
use server::api::runner::runner;

#[tokio::main]
async fn main() -> io::Result<()> {
    dotenvy::dotenv().ok();
    runner::run().await?;
    Ok(())
}
