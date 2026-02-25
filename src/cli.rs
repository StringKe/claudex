use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "claudex",
    version,
    about = "Multi-instance Claude Code manager with intelligent translation proxy"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run Claude Code with a specific profile
    Run {
        /// Profile name to use
        profile: String,
        /// Override the model for this session
        #[arg(short, long)]
        model: Option<String>,
        /// Enable terminal hyperlinks (OSC 8) for clickable paths and URLs
        #[arg(long)]
        hyperlinks: bool,
        /// Extra arguments passed to claude
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Manage profiles
    Profile {
        #[command(subcommand)]
        action: ProfileAction,
    },

    /// Manage the translation proxy
    Proxy {
        #[command(subcommand)]
        action: ProxyAction,
    },

    /// Launch the TUI dashboard
    Dashboard,

    /// Show current configuration
    Config {
        /// Initialize claudex.toml in the current directory
        #[arg(long)]
        init: bool,
    },

    /// Self-update claudex binary
    Update {
        /// Only check for updates, don't install
        #[arg(long)]
        check: bool,
    },

    /// Manage OAuth authentication for subscription services
    Auth {
        #[command(subcommand)]
        action: AuthAction,
    },

    /// Manage Claude Code configuration sets
    Sets {
        #[command(subcommand)]
        action: SetsAction,
    },
}

#[derive(Subcommand)]
pub enum ProfileAction {
    /// List all profiles
    List,
    /// Add a new profile interactively
    Add,
    /// Remove a profile
    Remove {
        /// Profile name
        name: String,
    },
    /// Test connectivity of a profile
    Test {
        /// Profile name (or "all")
        name: String,
    },
    /// Show profile details
    Show {
        /// Profile name
        name: String,
    },
}

#[derive(Subcommand)]
pub enum AuthAction {
    /// Login to an OAuth provider (claude, openai, google, qwen, kimi, github)
    Login {
        /// Provider name
        provider: String,
        /// Profile name (defaults to provider name)
        #[arg(short, long)]
        profile: Option<String>,
    },
    /// Show OAuth token status
    Status {
        /// Profile name (show all if omitted)
        #[arg(short, long)]
        profile: Option<String>,
    },
    /// Remove OAuth token for a profile
    Logout {
        /// Profile name
        profile: String,
    },
    /// Force refresh OAuth token
    Refresh {
        /// Profile name
        profile: String,
    },
}

#[derive(Subcommand)]
pub enum SetsAction {
    /// Install a configuration set from git repo, local path, or URL
    Add {
        /// Source: git URL, local path, or HTTP URL
        source: String,
        /// Install globally (~/.claude/)
        #[arg(long)]
        global: bool,
        /// Pin to a specific git ref (tag/branch/commit)
        #[arg(long)]
        r#ref: Option<String>,
    },
    /// Remove an installed configuration set
    Remove {
        /// Set name
        name: String,
        /// Remove from global scope
        #[arg(long)]
        global: bool,
    },
    /// List installed configuration sets
    List {
        /// List global sets
        #[arg(long)]
        global: bool,
    },
    /// Update configuration sets to latest version
    Update {
        /// Set name (omit to update all)
        name: Option<String>,
        /// Update global sets
        #[arg(long)]
        global: bool,
    },
    /// Show details of an installed configuration set
    Show {
        /// Set name
        name: String,
        /// Show global set
        #[arg(long)]
        global: bool,
    },
}

#[derive(Subcommand)]
pub enum ProxyAction {
    /// Start the proxy server (foreground)
    Start {
        /// Port override
        #[arg(short, long)]
        port: Option<u16>,
        /// Run as daemon
        #[arg(short, long)]
        daemon: bool,
    },
    /// Stop the proxy daemon
    Stop,
    /// Show proxy status
    Status,
}
