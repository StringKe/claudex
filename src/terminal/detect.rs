use std::io::IsTerminal;

/// Detect whether the current terminal supports OSC 8 hyperlinks.
pub fn terminal_supports_hyperlinks() -> bool {
    let env = EnvSnapshot::from_real();
    let is_tty = std::io::stdout().is_terminal();
    detect_from_env(&env, is_tty)
}

/// Environment snapshot for testability.
#[derive(Default)]
struct EnvSnapshot {
    force_hyperlinks: Option<String>,
    domterm: Option<String>,
    term_program: Option<String>,
    term: Option<String>,
    vte_version: Option<String>,
    wt_session: Option<String>,
}

impl EnvSnapshot {
    fn from_real() -> Self {
        Self {
            force_hyperlinks: std::env::var("FORCE_HYPERLINKS").ok(),
            domterm: std::env::var("DOMTERM").ok(),
            term_program: std::env::var("TERM_PROGRAM").ok(),
            term: std::env::var("TERM").ok(),
            vte_version: std::env::var("VTE_VERSION").ok(),
            wt_session: std::env::var("WT_SESSION").ok(),
        }
    }
}

/// Pure detection logic, testable without touching real environment.
fn detect_from_env(env: &EnvSnapshot, is_tty: bool) -> bool {
    // 1. Force override
    if env.force_hyperlinks.as_deref() == Some("1") {
        return true;
    }

    // 2. Not a TTY → no hyperlinks
    if !is_tty {
        return false;
    }

    // 3. DOMTERM
    if env.domterm.is_some() {
        return true;
    }

    // 4. Known terminal programs
    if let Some(ref term_program) = env.term_program {
        if matches!(
            term_program.as_str(),
            "iTerm.app" | "WezTerm" | "vscode" | "Tabby" | "Hyper" | "mintty" | "WarpTerminal"
        ) {
            return true;
        }
    }

    // 5. Known TERM values
    if let Some(ref term) = env.term {
        if term.starts_with("xterm-kitty") || term.starts_with("xterm-ghostty") {
            return true;
        }
    }

    // 6. VTE version >= 0.50 (GNOME Terminal, xfce4-terminal, etc.)
    if let Some(ref vte_ver) = env.vte_version {
        if let Ok(ver) = vte_ver.parse::<u32>() {
            return ver >= 5000;
        }
    }

    // 7. Windows Terminal
    if env.wt_session.is_some() {
        return true;
    }

    // Unknown terminal → safe default
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_env() -> EnvSnapshot {
        EnvSnapshot::default()
    }

    #[test]
    fn test_detect_returns_bool() {
        let _ = terminal_supports_hyperlinks();
    }

    #[test]
    fn test_force_hyperlinks_overrides_all() {
        let env = EnvSnapshot {
            force_hyperlinks: Some("1".to_string()),
            ..Default::default()
        };
        // Should return true even when not a TTY
        assert!(detect_from_env(&env, false));
    }

    #[test]
    fn test_force_hyperlinks_zero_no_effect() {
        let env = EnvSnapshot {
            force_hyperlinks: Some("0".to_string()),
            ..Default::default()
        };
        // Not "1", so falls through; not TTY → false
        assert!(!detect_from_env(&env, false));
    }

    #[test]
    fn test_not_tty_returns_false() {
        assert!(!detect_from_env(&empty_env(), false));
    }

    #[test]
    fn test_domterm_returns_true() {
        let env = EnvSnapshot {
            domterm: Some("1".to_string()),
            ..Default::default()
        };
        assert!(detect_from_env(&env, true));
    }

    #[test]
    fn test_iterm_detected() {
        let env = EnvSnapshot {
            term_program: Some("iTerm.app".to_string()),
            ..Default::default()
        };
        assert!(detect_from_env(&env, true));
    }

    #[test]
    fn test_wezterm_detected() {
        let env = EnvSnapshot {
            term_program: Some("WezTerm".to_string()),
            ..Default::default()
        };
        assert!(detect_from_env(&env, true));
    }

    #[test]
    fn test_vscode_detected() {
        let env = EnvSnapshot {
            term_program: Some("vscode".to_string()),
            ..Default::default()
        };
        assert!(detect_from_env(&env, true));
    }

    #[test]
    fn test_warp_detected() {
        let env = EnvSnapshot {
            term_program: Some("WarpTerminal".to_string()),
            ..Default::default()
        };
        assert!(detect_from_env(&env, true));
    }

    #[test]
    fn test_tabby_detected() {
        let env = EnvSnapshot {
            term_program: Some("Tabby".to_string()),
            ..Default::default()
        };
        assert!(detect_from_env(&env, true));
    }

    #[test]
    fn test_hyper_detected() {
        let env = EnvSnapshot {
            term_program: Some("Hyper".to_string()),
            ..Default::default()
        };
        assert!(detect_from_env(&env, true));
    }

    #[test]
    fn test_mintty_detected() {
        let env = EnvSnapshot {
            term_program: Some("mintty".to_string()),
            ..Default::default()
        };
        assert!(detect_from_env(&env, true));
    }

    #[test]
    fn test_unknown_term_program_not_detected() {
        let env = EnvSnapshot {
            term_program: Some("SomeObscureTerminal".to_string()),
            ..Default::default()
        };
        assert!(!detect_from_env(&env, true));
    }

    #[test]
    fn test_kitty_term_detected() {
        let env = EnvSnapshot {
            term: Some("xterm-kitty".to_string()),
            ..Default::default()
        };
        assert!(detect_from_env(&env, true));
    }

    #[test]
    fn test_ghostty_term_detected() {
        let env = EnvSnapshot {
            term: Some("xterm-ghostty".to_string()),
            ..Default::default()
        };
        assert!(detect_from_env(&env, true));
    }

    #[test]
    fn test_plain_xterm_not_detected() {
        let env = EnvSnapshot {
            term: Some("xterm-256color".to_string()),
            ..Default::default()
        };
        assert!(!detect_from_env(&env, true));
    }

    #[test]
    fn test_vte_version_5000_detected() {
        let env = EnvSnapshot {
            vte_version: Some("5000".to_string()),
            ..Default::default()
        };
        assert!(detect_from_env(&env, true));
    }

    #[test]
    fn test_vte_version_above_5000_detected() {
        let env = EnvSnapshot {
            vte_version: Some("7200".to_string()),
            ..Default::default()
        };
        assert!(detect_from_env(&env, true));
    }

    #[test]
    fn test_vte_version_below_5000_not_detected() {
        let env = EnvSnapshot {
            vte_version: Some("4999".to_string()),
            ..Default::default()
        };
        assert!(!detect_from_env(&env, true));
    }

    #[test]
    fn test_vte_version_non_numeric_not_detected() {
        let env = EnvSnapshot {
            vte_version: Some("abc".to_string()),
            ..Default::default()
        };
        assert!(!detect_from_env(&env, true));
    }

    #[test]
    fn test_windows_terminal_detected() {
        let env = EnvSnapshot {
            wt_session: Some("some-session-id".to_string()),
            ..Default::default()
        };
        assert!(detect_from_env(&env, true));
    }

    #[test]
    fn test_unknown_tty_returns_false() {
        // TTY but no known terminal identifiers → safe default false
        assert!(!detect_from_env(&empty_env(), true));
    }

    #[test]
    fn test_priority_force_over_not_tty() {
        // FORCE_HYPERLINKS=1 should win even if not a TTY
        let env = EnvSnapshot {
            force_hyperlinks: Some("1".to_string()),
            ..Default::default()
        };
        assert!(detect_from_env(&env, false));
    }
}
