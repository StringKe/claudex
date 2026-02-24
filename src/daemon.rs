use std::path::PathBuf;

use anyhow::{bail, Context, Result};

fn pid_file_path() -> Result<PathBuf> {
    let runtime_dir = dirs::runtime_dir()
        .or_else(dirs::cache_dir)
        .context("cannot determine runtime directory")?;
    let dir = runtime_dir.join("claudex");
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join("proxy.pid"))
}

pub fn write_pid(pid: u32) -> Result<()> {
    let path = pid_file_path()?;
    std::fs::write(&path, pid.to_string())?;
    tracing::info!(pid, path = %path.display(), "wrote PID file");
    Ok(())
}

pub fn read_pid() -> Result<Option<u32>> {
    let path = pid_file_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)?;
    let pid: u32 = content.trim().parse().context("invalid PID file content")?;
    Ok(Some(pid))
}

pub fn remove_pid() -> Result<()> {
    let path = pid_file_path()?;
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    Ok(())
}

pub fn is_proxy_running() -> Result<bool> {
    match read_pid()? {
        Some(pid) => {
            // Check if process exists via kill(pid, 0)
            let result = unsafe { libc::kill(pid as i32, 0) };
            Ok(result == 0)
        }
        None => Ok(false),
    }
}

pub fn stop_proxy() -> Result<()> {
    match read_pid()? {
        Some(pid) => {
            if is_proxy_running()? {
                unsafe {
                    libc::kill(pid as i32, libc::SIGTERM);
                }
                println!("Sent SIGTERM to proxy (PID {pid})");
            } else {
                println!("Proxy is not running (stale PID file)");
            }
            remove_pid()?;
            Ok(())
        }
        None => {
            bail!("no proxy PID file found — proxy is not running")
        }
    }
}

pub fn proxy_status() -> Result<()> {
    match read_pid()? {
        Some(pid) => {
            if is_proxy_running()? {
                println!("Proxy is running (PID {pid})");
            } else {
                println!("Proxy is NOT running (stale PID file for PID {pid})");
                remove_pid()?;
            }
        }
        None => {
            println!("Proxy is not running");
        }
    }
    Ok(())
}
