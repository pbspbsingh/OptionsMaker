use crawler::start_crawling;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    util::init::init_main();

    persist::init().await?;

    start_crawling().await?;

    Ok(())
}
