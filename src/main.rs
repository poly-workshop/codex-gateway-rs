mod app;
mod auth;
mod cli;
mod codex;
mod config;
mod db;
mod error;
mod meter;
mod monitor;
mod proxy;
mod scheduler;

use anyhow::Context;
use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "codex_gateway=info,tower_http=info".into()),
        )
        .init();

    let cli = cli::Args::parse();
    let config = config::Config::load(cli.config.as_deref())
        .with_context(|| "failed to load configuration")?;
    let pool = db::connect_and_migrate(&config.database.url)
        .await
        .with_context(|| "failed to initialize database")?;

    match cli.command {
        cli::Command::Serve => app::serve(config, pool).await,
        cli::Command::Admin(command) => cli::run_admin(command, &config, &pool).await,
    }
}
