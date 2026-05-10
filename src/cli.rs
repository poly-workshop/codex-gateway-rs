use std::path::PathBuf;

use anyhow::{Context, bail};
use clap::{Args as ClapArgs, Parser, Subcommand};

use crate::{auth, config::Config, db};

#[derive(Debug, Parser)]
#[command(
    version,
    about = "Codex-only gateway for member-managed OpenAI API keys"
)]
pub struct Args {
    #[arg(short, long, env = "CODEX_GATEWAY_CONFIG")]
    pub config: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Serve,
    #[command(subcommand)]
    Admin(AdminCommand),
}

#[derive(Debug, Subcommand)]
pub enum AdminCommand {
    Init,
    AddMember(AddMember),
    AddCodexKey(AddCodexKey),
    DisableCodexKey(DisableCodexKey),
    AddUpstreamKey(AddUpstreamKey),
    DisableUpstreamKey(DisableUpstreamKey),
    Usage,
    SampleConfig,
}

#[derive(Debug, ClapArgs)]
pub struct AddMember {
    pub name: String,
    #[arg(long, default_value_t = 1.0)]
    pub weight: f64,
    #[arg(long)]
    pub max_concurrent: Option<i64>,
    #[arg(long)]
    pub five_hour_quota: Option<i64>,
    #[arg(long)]
    pub weekly_quota: Option<i64>,
}

#[derive(Debug, ClapArgs)]
pub struct AddCodexKey {
    pub member: String,
}

#[derive(Debug, ClapArgs)]
pub struct DisableCodexKey {
    pub prefix: String,
}

#[derive(Debug, ClapArgs)]
pub struct AddUpstreamKey {
    pub name: String,
    pub secret: String,
    #[arg(long, default_value_t = true)]
    pub http: bool,
    #[arg(long, default_value_t = true)]
    pub ws: bool,
    #[arg(long, default_value_t = 1.0)]
    pub weight: f64,
    #[arg(long)]
    pub max_concurrent: Option<i64>,
}

#[derive(Debug, ClapArgs)]
pub struct DisableUpstreamKey {
    pub name: String,
}

pub async fn run_admin(
    command: AdminCommand,
    config: &Config,
    pool: &db::Db,
) -> anyhow::Result<()> {
    match command {
        AdminCommand::Init => {
            println!("database initialized: {}", config.database.url);
        }
        AdminCommand::AddMember(args) => {
            let max_concurrent = args
                .max_concurrent
                .unwrap_or(config.limits.default_member_concurrency);
            let five_hour_quota = args
                .five_hour_quota
                .unwrap_or(config.limits.default_member_5h_quota);
            let weekly_quota = args
                .weekly_quota
                .unwrap_or(config.limits.default_member_weekly_quota);
            let id = db::create_member(
                pool,
                &args.name,
                args.weight,
                max_concurrent,
                five_hour_quota,
                weekly_quota,
            )
            .await?;
            println!("member created: id={id} name={}", args.name);
        }
        AdminCommand::AddCodexKey(args) => {
            let member = db::find_member_by_name(pool, &args.member)
                .await?
                .with_context(|| format!("member not found: {}", args.member))?;
            let (key, prefix, hash) = auth::generate_codex_key();
            db::insert_codex_key(pool, member.id, &hash, &prefix).await?;
            println!("Codex key created for member {}", member.name);
            println!("prefix: {prefix}");
            println!("secret: {key}");
        }
        AdminCommand::DisableCodexKey(args) => {
            let rows = db::set_codex_key_status(pool, &args.prefix, "disabled").await?;
            if rows == 0 {
                bail!("Codex key prefix not found: {}", args.prefix);
            }
            println!("Codex key disabled: {}", args.prefix);
        }
        AdminCommand::AddUpstreamKey(args) => {
            let max_concurrent = args
                .max_concurrent
                .unwrap_or(config.limits.default_upstream_concurrency);
            let id = db::insert_upstream_key(
                pool,
                &args.name,
                &args.secret,
                args.http,
                args.ws,
                args.weight,
                max_concurrent,
            )
            .await?;
            println!("upstream key created: id={id} name={}", args.name);
        }
        AdminCommand::DisableUpstreamKey(args) => {
            let rows = db::set_upstream_key_status(pool, &args.name, "disabled").await?;
            if rows == 0 {
                bail!("upstream key not found: {}", args.name);
            }
            println!("upstream key disabled: {}", args.name);
        }
        AdminCommand::Usage => {
            let rows = db::usage_summary(pool).await?;
            if rows.is_empty() {
                println!("no usage recorded today");
            } else {
                for (name, usage) in rows {
                    println!(
                        "{name}: credits={:.2} weighted_tokens={:.2} total_tokens={} requests={} messages={} ws_connections={}",
                        usage.credits,
                        usage.weighted_tokens,
                        usage.total_tokens,
                        usage.request_count,
                        usage.message_count,
                        usage.ws_connection_count
                    );
                }
            }
        }
        AdminCommand::SampleConfig => {
            println!("{}", sample_config());
        }
    }

    Ok(())
}

fn sample_config() -> &'static str {
    r#"[server]
bind_addr = "127.0.0.1:8080"

[database]
url = "sqlite://codex-gateway.db"

[upstream]
http_base_url = "https://api.openai.com"
ws_base_url = "wss://api.openai.com"
timeout_secs = 120
retry_attempts = 2
cooldown_secs = 60
max_failures_before_cooldown = 3

[credit]
accounting = "token"
unknown_model_message_credits = 5.0
unknown_usage_credits = 5.0

[limits]
default_member_concurrency = 4
default_member_5h_quota = 0
default_member_weekly_quota = 0
default_upstream_concurrency = 8
ws_idle_timeout_secs = 120
ws_upstream_ping_interval_secs = 20
ws_max_connection_secs = 3600
ws_max_messages_per_connection = 5000
"#
}
