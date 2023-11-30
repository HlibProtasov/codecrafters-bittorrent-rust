
use bittorrent_starter_rust::cli::Cli;
use clap::Parser;
use bittorrent_starter_rust::torrent_executor::TorrentExecutor;


#[tokio::main]
// Usage: yo
async fn main() -> anyhow::Result<()>
{
    let cli = Cli::parse();

    TorrentExecutor::execute(cli.command).await?;

    Ok(())
}



