mod client;
mod collect;
mod commands;
mod config;
mod credentials;
mod privacy;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "aftercode",
    version,
    about = "Turn your AI coding sessions into learning podcasts"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Initialize Aftercode in the current project
    Init,
    /// Save a personal access token
    Login { token: String },
    /// Show project + collector status
    Status,
    /// Show what would be uploaded (no network)
    Preview,
    /// Generate a podcast episode from recent activity
    Episode {
        #[arg(long)]
        language: Option<String>,
        #[arg(long, default_value = "today")]
        from: String,
        #[arg(long)]
        length: Option<u8>,
    },
    /// Add a path/glob to the ignore list
    Ignore { pattern: String },
    /// Open the web UI
    Open,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Init => commands::init().await,
        Cmd::Login { token } => commands::login(token),
        Cmd::Status => commands::status().await,
        Cmd::Preview => commands::preview(),
        Cmd::Episode {
            language,
            from,
            length,
        } => commands::episode(language, from, length).await,
        Cmd::Ignore { pattern } => commands::ignore(pattern),
        Cmd::Open => commands::open(),
    }
}
