use std::io::Write;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd};
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{bail, Context, Result};
use nix::poll::{PollFd, PollFlags, PollTimeout};
use nix::pty::openpty;
use nix::sys::termios;
use nix::unistd::{self, ForkResult};

use super::osc8::LinkDetector;

/// Launch a child process in a PTY and proxy its output through the link detector.
pub fn spawn_with_pty(mut cmd: Command, cwd: PathBuf) -> Result<()> {
    // Open a PTY pair
    let pty = openpty(None, None).context("failed to open PTY")?;
    let master_fd = pty.master;
    let slave_fd = pty.slave;

    // Save original terminal settings for restoration
    let stdin = std::io::stdin();
    let orig_termios = termios::tcgetattr(&stdin).ok();

    // Set stdin to raw mode so keystrokes pass through immediately
    if let Some(ref orig) = orig_termios {
        let mut raw = orig.clone();
        termios::cfmakeraw(&mut raw);
        termios::tcsetattr(&stdin, termios::SetArg::TCSANOW, &raw)
            .context("failed to set raw mode")?;
    }

    // Set the PTY slave size to match the real terminal
    sync_winsize(stdin.as_raw_fd(), master_fd.as_raw_fd());

    // Fork: child runs in the PTY slave, parent proxies
    match unsafe { unistd::fork() }.context("fork failed")? {
        ForkResult::Child => {
            // Child: set up PTY slave as controlling terminal
            drop(master_fd);

            // Create a new session
            unistd::setsid().ok();

            // Set the slave as controlling terminal
            unsafe {
                libc::ioctl(slave_fd.as_raw_fd(), libc::TIOCSCTTY as _, 0);
            }

            // Redirect stdin/stdout/stderr to the PTY slave
            let slave_raw = slave_fd.as_raw_fd();
            unistd::dup2(slave_raw, 0).ok();
            unistd::dup2(slave_raw, 1).ok();
            unistd::dup2(slave_raw, 2).ok();

            drop(slave_fd);

            // Replace the process with the command (does not return on success)
            let err = cmd.exec();
            eprintln!("failed to execute command: {err}");
            std::process::exit(127);
        }
        ForkResult::Parent { child } => {
            drop(slave_fd);

            // Set up SIGWINCH handler to sync terminal size
            setup_sigwinch_handler(stdin.as_raw_fd(), master_fd.as_raw_fd());

            // Run the proxy loop
            let exit_code = run_proxy_loop(&master_fd, &stdin, &mut LinkDetector::new(cwd));

            // Restore terminal settings
            if let Some(ref orig) = orig_termios {
                termios::tcsetattr(&stdin, termios::SetArg::TCSANOW, orig).ok();
            }

            // Wait for child and propagate exit code
            match nix::sys::wait::waitpid(child, None) {
                Ok(nix::sys::wait::WaitStatus::Exited(_, code)) => {
                    if code != 0 {
                        bail!("claude exited with status: {code}");
                    }
                }
                Ok(nix::sys::wait::WaitStatus::Signaled(_, sig, _)) => {
                    bail!("claude killed by signal: {sig}");
                }
                _ => {
                    exit_code?;
                }
            }

            Ok(())
        }
    }
}

/// Main proxy loop: shuttle data between stdin/PTY master and enhance output.
fn run_proxy_loop(
    master_fd: &OwnedFd,
    stdin_handle: &std::io::Stdin,
    detector: &mut LinkDetector,
) -> Result<()> {
    let master_raw = master_fd.as_raw_fd();
    let stdin_raw = stdin_handle.as_raw_fd();
    let stdin_borrowed: BorrowedFd = stdin_handle.as_fd();
    let master_borrowed: BorrowedFd = master_fd.as_fd();

    let mut stdout = std::io::stdout().lock();
    let mut read_buf = [0u8; 4096];
    let mut line_buf = String::new();
    // 保留跨 read 边界的不完整 UTF-8 尾部字节（最多 3 字节）
    let mut utf8_residual = Vec::with_capacity(3);

    loop {
        let mut fds = [
            PollFd::new(stdin_borrowed, PollFlags::POLLIN),
            PollFd::new(master_borrowed, PollFlags::POLLIN),
        ];

        match nix::poll::poll(&mut fds, PollTimeout::from(50u16)) {
            Ok(0) => {
                // Timeout: flush any incomplete line buffer to avoid display lag
                if !line_buf.is_empty() {
                    let enhanced = detector.enhance_line(&line_buf);
                    write!(stdout, "{enhanced}")?;
                    stdout.flush()?;
                    line_buf.clear();
                }
                continue;
            }
            Ok(_) => {}
            Err(nix::errno::Errno::EINTR) => continue,
            Err(e) => return Err(e.into()),
        }

        // stdin → PTY master (user input, pass through unmodified)
        if let Some(revents) = fds[0].revents() {
            if revents.contains(PollFlags::POLLIN) {
                let n = nix::unistd::read(stdin_raw, &mut read_buf).context("read stdin failed")?;
                if n == 0 {
                    break;
                }
                nix::unistd::write(master_fd, &read_buf[..n]).context("write to PTY failed")?;
            }
        }

        // PTY master → stdout (claude output, enhance with hyperlinks)
        if let Some(revents) = fds[1].revents() {
            if revents.contains(PollFlags::POLLIN) {
                match nix::unistd::read(master_raw, &mut read_buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        // 将上次残留字节拼接到本次数据前面
                        let mut combined;
                        let data: &[u8] = if utf8_residual.is_empty() {
                            &read_buf[..n]
                        } else {
                            combined = std::mem::take(&mut utf8_residual);
                            combined.extend_from_slice(&read_buf[..n]);
                            &combined
                        };

                        // 找到最后一个完整 UTF-8 字符的边界
                        let valid_end = find_utf8_safe_end(data);

                        if let Ok(chunk) = std::str::from_utf8(&data[..valid_end]) {
                            line_buf.push_str(chunk);
                        }

                        // 保存不完整的 UTF-8 尾部字节
                        if valid_end < data.len() {
                            utf8_residual.extend_from_slice(&data[valid_end..]);
                        }

                        // Process complete lines
                        while let Some(pos) = line_buf.find('\n') {
                            let line = line_buf[..pos].to_string();
                            line_buf = line_buf[pos + 1..].to_string();

                            let enhanced = detector.enhance_line(&line);
                            writeln!(stdout, "{enhanced}")?;
                        }

                        stdout.flush()?;
                    }
                    Err(nix::errno::Errno::EIO) => break, // PTY closed
                    Err(e) => return Err(e.into()),
                }
            }

            if revents.contains(PollFlags::POLLHUP) {
                // Child exited: flush remaining buffer
                if !line_buf.is_empty() {
                    let enhanced = detector.enhance_line(&line_buf);
                    write!(stdout, "{enhanced}")?;
                    stdout.flush()?;
                }
                break;
            }
        }
    }

    Ok(())
}

/// 从字节切片末尾回溯，找到最后一个完整 UTF-8 字符的边界。
/// 返回可以安全转为 str 的字节长度；尾部不完整的字节留给下次 read。
fn find_utf8_safe_end(data: &[u8]) -> usize {
    if data.is_empty() {
        return 0;
    }
    // 从末尾往前扫描，最多回退 3 字节（UTF-8 最长 4 字节）
    let len = data.len();
    for i in 0..4.min(len) {
        let pos = len - 1 - i;
        let byte = data[pos];
        if byte < 0x80 {
            // ASCII：完整字符，pos+1 之前全部有效
            return len;
        }
        // 找到多字节序列的首字节（leading byte: 11xxxxxx）
        if byte >= 0xC0 {
            let expected_len = if byte < 0xE0 {
                2
            } else if byte < 0xF0 {
                3
            } else {
                4
            };
            let available = len - pos;
            return if available >= expected_len {
                len // 完整字符
            } else {
                pos // 不完整，截断到此处
            };
        }
        // 0x80..0xBF 是 continuation byte，继续往前找 leading byte
    }
    // 4 字节都是 continuation byte（不可能的合法 UTF-8），放弃全部
    0
}

/// Sync terminal window size from real terminal to PTY.
fn sync_winsize(stdin_fd: i32, master_fd: i32) {
    unsafe {
        let mut ws: libc::winsize = std::mem::zeroed();
        if libc::ioctl(stdin_fd, libc::TIOCGWINSZ, &mut ws) == 0 {
            libc::ioctl(master_fd, libc::TIOCSWINSZ, &ws);
        }
    }
}

/// Set up a SIGWINCH handler that syncs terminal size to the PTY.
fn setup_sigwinch_handler(stdin_fd: i32, master_fd: i32) {
    let _ = unsafe {
        signal_hook::low_level::register(signal_hook::consts::SIGWINCH, move || {
            sync_winsize(stdin_fd, master_fd);
        })
    };
}
