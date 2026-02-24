use anyhow::Result;

const REPO_OWNER: &str = "StringKe";
const REPO_NAME: &str = "claudex";
const BIN_NAME: &str = "claudex";

/// Check if a newer version is available on GitHub Releases.
/// Returns Some(version) if newer, None if up-to-date.
pub async fn check_update() -> Result<Option<String>> {
    let current = env!("CARGO_PKG_VERSION");

    let updater = self_update::backends::github::Update::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .bin_name(BIN_NAME)
        .current_version(current)
        .build()?;

    let latest = updater.get_latest_release()?;
    let latest_version = latest.version.trim_start_matches('v');

    if latest_version != current {
        Ok(Some(latest_version.to_string()))
    } else {
        Ok(None)
    }
}

/// Download and install the latest version.
pub async fn self_update() -> Result<()> {
    let current = env!("CARGO_PKG_VERSION");
    println!("Current version: v{current}");
    println!("Checking for updates...");

    let status = self_update::backends::github::Update::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .bin_name(BIN_NAME)
        .current_version(current)
        .show_download_progress(true)
        .no_confirm(true)
        .build()?
        .update()?;

    if status.updated() {
        println!("Updated to v{}!", status.version());
    } else {
        println!("Already up to date (v{current})");
    }

    Ok(())
}
