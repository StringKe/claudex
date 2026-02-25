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
