use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "claudex",
    version,
    about = "Multi-instance Claude Code manager with intelligent translation proxy"
)]
pub struct Cli {
    /// Override config file path
    #[arg(long, global = true, value_name = "PATH")]
    pub config: Option<PathBuf>,
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

    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: Option<ConfigAction>,
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
    /// Login to an OAuth provider (claude, chatgpt/openai, google, qwen, kimi, github/copilot)
    Login {
        /// Provider name
        provider: String,
        /// Profile name (defaults to provider name)
        #[arg(short, long)]
        profile: Option<String>,
        /// Skip existing credential detection, force interactive login
        #[arg(short, long)]
        force: bool,
        /// Use headless device code flow (for SSH/no-browser environments)
        #[arg(long)]
        headless: bool,
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
pub enum ConfigAction {
    /// Display configuration summary (default when no subcommand)
    Show {
        /// Print raw file contents
        #[arg(long)]
        raw: bool,
        /// Output merged config as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show config file paths and search order
    Path,
    /// Create config file in current directory
    Init {
        /// Use YAML format
        #[arg(long)]
        yaml: bool,
    },
    /// Recreate global config (backup original, preserve profiles)
    Recreate {
        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },
    /// Open config file in $EDITOR
    Edit {
        /// Open global config
        #[arg(long)]
        global: bool,
    },
    /// Validate config syntax and semantics
    Validate {
        /// Also test provider connectivity
        #[arg(long)]
        connectivity: bool,
    },
    /// Get a config value by dot-path (e.g. proxy_port, profiles.0.name)
    Get {
        /// Dot-separated key path
        key: String,
    },
    /// Set a config value by dot-path
    Set {
        /// Dot-separated key path
        key: String,
        /// Value to set (JSON or plain string)
        value: String,
    },
    /// Export config to another format
    Export {
        /// Output format: json, toml, yaml
        #[arg(long, default_value = "json")]
        format: String,
        /// Output file (stdout if omitted)
        #[arg(short, long)]
        output: Option<PathBuf>,
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
