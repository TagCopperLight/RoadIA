use tokio::io;
use server::api::runner::runner;

#[tokio::main]
async fn main() -> io::Result<()> {
    runner::run().await?;
    Ok(())
}
