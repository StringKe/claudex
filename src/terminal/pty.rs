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
/// Returns the detected resume session ID if Claude Code output a `claude --resume` line.
pub fn spawn_with_pty(mut cmd: Command, cwd: PathBuf) -> Result<Option<String>> {
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
            let (exit_code, resume_session_id) =
                run_proxy_loop(&master_fd, &stdin, &mut LinkDetector::new(cwd));

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

            Ok(resume_session_id)
        }
    }
}

/// Main proxy loop: shuttle data between stdin/PTY master and enhance output.
/// Returns (loop_result, detected_resume_session_id).
fn run_proxy_loop(
    master_fd: &OwnedFd,
    stdin_handle: &std::io::Stdin,
    detector: &mut LinkDetector,
) -> (Result<()>, Option<String>) {
    let mut resume_session_id: Option<String> = None;
    let result = run_proxy_loop_inner(master_fd, stdin_handle, detector, &mut resume_session_id);
    (result, resume_session_id)
}

/// Inner proxy loop with `?` operator support.
fn run_proxy_loop_inner(
    master_fd: &OwnedFd,
    stdin_handle: &std::io::Stdin,
    detector: &mut LinkDetector,
    resume_session_id: &mut Option<String>,
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

                            // 检测 `claude --resume <session-id>`
                            detect_resume_session(&line, resume_session_id);

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
                    detect_resume_session(&line_buf, resume_session_id);
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

/// 从输出行中检测 `claude --resume <session-id>` 模式，提取 session ID。
fn detect_resume_session(line: &str, session_id: &mut Option<String>) {
    // 匹配 ANSI 转义序列剥离后的纯文本，支持带/不带终端控制字符
    let stripped = strip_ansi_escapes(line);
    let trimmed = stripped.trim();
    if let Some(rest) = trimmed.strip_prefix("claude --resume ") {
        let id = rest.trim();
        if !id.is_empty() {
            *session_id = Some(id.to_string());
        }
    }
}

/// 剥离 ANSI 转义序列（CSI、OSC 等）
fn strip_ansi_escapes(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            match chars.peek() {
                Some('[') => {
                    chars.next();
                    // CSI: 消费到 0x40..0x7E
                    while let Some(&ch) = chars.peek() {
                        chars.next();
                        if ('\x40'..='\x7e').contains(&ch) {
                            break;
                        }
                    }
                }
                Some(']') => {
                    chars.next();
                    // OSC: 消费到 ST (ESC \ 或 BEL)
                    while let Some(&ch) = chars.peek() {
                        if ch == '\x07' {
                            chars.next();
                            break;
                        }
                        if ch == '\x1b' {
                            chars.next();
                            if chars.peek() == Some(&'\\') {
                                chars.next();
                            }
                            break;
                        }
                        chars.next();
                    }
                }
                _ => {
                    // 其他单字符转义
                    chars.next();
                }
            }
        } else {
            out.push(c);
        }
    }
    out
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── strip_ansi_escapes ──────────────────────────────────

    #[test]
    fn test_strip_ansi_plain_text() {
        assert_eq!(strip_ansi_escapes("hello world"), "hello world");
    }

    #[test]
    fn test_strip_ansi_empty() {
        assert_eq!(strip_ansi_escapes(""), "");
    }

    #[test]
    fn test_strip_ansi_csi_color() {
        // \x1b[32m = green, \x1b[0m = reset
        assert_eq!(
            strip_ansi_escapes("\x1b[32mclaude --resume abc\x1b[0m"),
            "claude --resume abc"
        );
    }

    #[test]
    fn test_strip_ansi_csi_cursor_movement() {
        // \x1b[2K = erase line, \x1b[1A = cursor up
        assert_eq!(strip_ansi_escapes("\x1b[2K\x1b[1Ahello"), "hello");
    }

    #[test]
    fn test_strip_ansi_osc_with_bel() {
        // OSC terminated by BEL (\x07)
        assert_eq!(strip_ansi_escapes("\x1b]0;title\x07text here"), "text here");
    }

    #[test]
    fn test_strip_ansi_osc_with_st() {
        // OSC terminated by ST (ESC \)
        assert_eq!(
            strip_ansi_escapes("\x1b]8;id=link;https://example.com\x1b\\click\x1b]8;;\x1b\\"),
            "click"
        );
    }

    #[test]
    fn test_strip_ansi_mixed_escapes() {
        let input = "\x1b[1m\x1b[36mclaude\x1b[0m --resume \x1b[33mabcdef\x1b[0m";
        assert_eq!(strip_ansi_escapes(input), "claude --resume abcdef");
    }

    #[test]
    fn test_strip_ansi_single_char_escape() {
        // ESC followed by a non-[ non-] char (e.g. ESC M = reverse line feed)
        assert_eq!(strip_ansi_escapes("\x1bMtext"), "text");
    }

    // ── detect_resume_session ───────────────────────────────

    #[test]
    fn test_detect_resume_plain() {
        let mut id = None;
        detect_resume_session("claude --resume abc-123-def", &mut id);
        assert_eq!(id.as_deref(), Some("abc-123-def"));
    }

    #[test]
    fn test_detect_resume_with_ansi() {
        let mut id = None;
        let line = "\x1b[32mclaude --resume \x1b[1mabc-123\x1b[0m";
        detect_resume_session(line, &mut id);
        assert_eq!(id.as_deref(), Some("abc-123"));
    }

    #[test]
    fn test_detect_resume_with_leading_whitespace() {
        let mut id = None;
        detect_resume_session("  claude --resume xyz-789  ", &mut id);
        assert_eq!(id.as_deref(), Some("xyz-789"));
    }

    #[test]
    fn test_detect_resume_not_matched() {
        let mut id = None;
        detect_resume_session("some other output line", &mut id);
        assert!(id.is_none());
    }

    #[test]
    fn test_detect_resume_partial_prefix() {
        let mut id = None;
        detect_resume_session("claude --resume", &mut id);
        // strip_prefix 成功但 rest 为空（trim 后为空），不应设置
        assert!(id.is_none());
    }

    #[test]
    fn test_detect_resume_uuid_format() {
        let mut id = None;
        detect_resume_session(
            "claude --resume 0130c158-76e0-4f95-b067-a6e171fa2f3a",
            &mut id,
        );
        assert_eq!(id.as_deref(), Some("0130c158-76e0-4f95-b067-a6e171fa2f3a"));
    }

    #[test]
    fn test_detect_resume_overwrites_previous() {
        let mut id = Some("old-id".to_string());
        detect_resume_session("claude --resume new-id", &mut id);
        assert_eq!(id.as_deref(), Some("new-id"));
    }

    #[test]
    fn test_detect_resume_empty_line() {
        let mut id = None;
        detect_resume_session("", &mut id);
        assert!(id.is_none());
    }

    #[test]
    fn test_detect_resume_osc8_wrapped() {
        // Claude Code 可能用 OSC 8 超链接包裹 resume 命令
        let mut id = None;
        let line = "\x1b]8;;https://example.com\x1b\\claude --resume abc-456\x1b]8;;\x1b\\";
        detect_resume_session(line, &mut id);
        assert_eq!(id.as_deref(), Some("abc-456"));
    }

    // ── find_utf8_safe_end ──────────────────────────────────

    #[test]
    fn test_utf8_safe_end_empty() {
        assert_eq!(find_utf8_safe_end(&[]), 0);
    }

    #[test]
    fn test_utf8_safe_end_ascii_only() {
        assert_eq!(find_utf8_safe_end(b"hello"), 5);
    }

    #[test]
    fn test_utf8_safe_end_complete_multibyte() {
        // "中" = 0xE4 0xB8 0xAD (3 bytes)
        let data = "中".as_bytes();
        assert_eq!(find_utf8_safe_end(data), 3);
    }

    #[test]
    fn test_utf8_safe_end_incomplete_multibyte() {
        // "中" leading byte + 1 continuation byte (missing last)
        let data = &[0xE4, 0xB8];
        assert_eq!(find_utf8_safe_end(data), 0);
    }

    #[test]
    fn test_utf8_safe_end_mixed_ascii_and_incomplete() {
        // "hi" + incomplete "中" (2 of 3 bytes)
        let data = &[b'h', b'i', 0xE4, 0xB8];
        assert_eq!(find_utf8_safe_end(data), 2);
    }
}
