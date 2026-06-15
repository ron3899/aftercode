mod client;
mod collect;
mod commands;
mod config;
mod credentials;
mod privacy;
mod session;

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
        /// Force a specific agent session source (claude-code|codex|cursor)
        #[arg(long)]
        agent: Option<String>,
        /// Read the session transcript from a file, or `-` for stdin. When set,
        /// this is the session source and on-disk auto-detection is skipped.
        #[arg(long)]
        transcript: Option<String>,
        /// Generate even when no session conversation was captured (diff only).
        #[arg(long)]
        allow_thin: bool,
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
            agent,
            transcript,
            allow_thin,
        } => commands::episode(language, from, length, agent, transcript, allow_thin).await,
        Cmd::Ignore { pattern } => commands::ignore(pattern),
        Cmd::Open => commands::open(),
    }
}
