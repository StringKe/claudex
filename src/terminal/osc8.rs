use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

// Pre-compiled regexes (compiled once, used many times)
static URL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(https?://|file://|mailto:)[^\s<>"'\x1b)\]]*[^\s<>"'\x1b)\].,:;!?]"#).unwrap()
});

static ABS_PATH_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"/[\w./_-]+\.\w+(?::\d+(?::\d+)?)?").unwrap());

static REL_PATH_DOTSLASH_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:\.\./|\./)([\w./_-]+)(?::\d+(?::\d+)?)?").unwrap());

static REL_PATH_DIR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[\w-]+/[\w./_-]+\.\w+(?::\d+(?::\d+)?)?").unwrap());

static ANSI_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\x1b(?:\[[0-9;]*[a-zA-Z]|\](?:[^;\x07\x1b]*;)*[^;\x07\x1b]*(?:\x07|\x1b\\))")
        .unwrap()
});

/// Segment of a terminal line: either an ANSI escape or plain text.
enum Segment {
    Escape(String),
    Text(String),
}

/// Split a line into ANSI escape and plain text segments.
fn split_ansi_segments(line: &str) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut last_end = 0;

    for m in ANSI_RE.find_iter(line) {
        if m.start() > last_end {
            segments.push(Segment::Text(line[last_end..m.start()].to_string()));
        }
        segments.push(Segment::Escape(m.as_str().to_string()));
        last_end = m.end();
    }

    if last_end < line.len() {
        segments.push(Segment::Text(line[last_end..].to_string()));
    }

    segments
}

/// Wrap text with OSC 8 hyperlink (using BEL as ST for best compatibility).
fn wrap_osc8(uri: &str, display_text: &str) -> String {
    format!("\x1b]8;;{uri}\x07{display_text}\x1b]8;;\x07")
}

/// Convert a file path (possibly with :line:col suffix) to a file:// URI.
fn file_path_to_uri(path: &str, cwd: &Path) -> String {
    let file_part = path.split(':').next().unwrap_or(path);
    let abs = if file_part.starts_with('/') {
        PathBuf::from(file_part)
    } else {
        cwd.join(file_part)
    };
    format!("file://{}", abs.display())
}

/// Link detection engine. Processes lines of terminal output and wraps
/// detected links with OSC 8 hyperlink sequences.
pub struct LinkDetector {
    cwd: PathBuf,
    file_cache: HashMap<String, bool>,
}

impl LinkDetector {
    pub fn new(cwd: PathBuf) -> Self {
        Self {
            cwd,
            file_cache: HashMap::new(),
        }
    }

    /// Check if a file path exists (with caching).
    fn check_file_exists(&mut self, file_part: &str) -> bool {
        if let Some(&cached) = self.file_cache.get(file_part) {
            return cached;
        }
        let abs = if file_part.starts_with('/') {
            PathBuf::from(file_part)
        } else {
            self.cwd.join(file_part)
        };
        let exists = abs.exists();
        self.file_cache.insert(file_part.to_string(), exists);
        exists
    }

    /// Enhance a single line by wrapping detected links with OSC 8.
    pub fn enhance_line(&mut self, line: &str) -> String {
        // Skip lines that already contain OSC 8 sequences
        if line.contains("\x1b]8;") {
            return line.to_string();
        }

        // Quick check: does the line contain anything link-like?
        if !line.contains("://")
            && !line.contains('/')
            && !line.contains('.')
            && !line.contains("mailto:")
        {
            return line.to_string();
        }

        // Split into ANSI escape vs. text segments
        let segments = split_ansi_segments(line);
        let mut result = String::with_capacity(line.len() + 128);

        for segment in segments {
            match segment {
                Segment::Escape(raw) => result.push_str(&raw),
                Segment::Text(text) => {
                    let enhanced = self.enhance_text(&text);
                    result.push_str(&enhanced);
                }
            }
        }

        result
    }

    /// Enhance a plain text segment (no ANSI escapes inside).
    fn enhance_text(&mut self, text: &str) -> String {
        // Collect all matches with their positions, types, and replacement strings
        let mut replacements: Vec<(usize, usize, String)> = Vec::new();

        // 1. Explicit protocol URLs (highest priority)
        for m in URL_RE.find_iter(text) {
            let url = m.as_str();
            replacements.push((m.start(), m.end(), wrap_osc8(url, url)));
        }

        // 2. Absolute paths
        for m in ABS_PATH_RE.find_iter(text) {
            if self.overlaps(&replacements, m.start(), m.end()) {
                continue;
            }
            let path_str = m.as_str();
            let file_part = path_str.split(':').next().unwrap_or(path_str);
            if self.check_file_exists(file_part) {
                let uri = file_path_to_uri(path_str, &self.cwd);
                replacements.push((m.start(), m.end(), wrap_osc8(&uri, path_str)));
            }
        }

        // 3. Relative paths with ./ or ../
        for m in REL_PATH_DOTSLASH_RE.find_iter(text) {
            if self.overlaps(&replacements, m.start(), m.end()) {
                continue;
            }
            let path_str = m.as_str();
            let file_part = path_str.split(':').next().unwrap_or(path_str);
            // Resolve ./ or ../ relative to cwd
            let resolved = self.cwd.join(file_part);
            let clean_part = resolved.to_string_lossy();
            if self.check_file_exists(&clean_part) {
                let uri = file_path_to_uri(path_str, &self.cwd);
                replacements.push((m.start(), m.end(), wrap_osc8(&uri, path_str)));
            }
        }

        // 4. Relative paths with dir/file.ext pattern (e.g. src/main.rs:42)
        for m in REL_PATH_DIR_RE.find_iter(text) {
            if self.overlaps(&replacements, m.start(), m.end()) {
                continue;
            }
            let path_str = m.as_str();
            let file_part = path_str.split(':').next().unwrap_or(path_str);
            if self.check_file_exists(file_part) {
                let uri = file_path_to_uri(path_str, &self.cwd);
                replacements.push((m.start(), m.end(), wrap_osc8(&uri, path_str)));
            }
        }

        if replacements.is_empty() {
            return text.to_string();
        }

        // Sort by start position
        replacements.sort_by_key(|r| r.0);

        // Build result by interleaving original text and replacements
        let mut result = String::with_capacity(text.len() + replacements.len() * 40);
        let mut last_end = 0;

        for (start, end, replacement) in &replacements {
            if *start >= last_end {
                result.push_str(&text[last_end..*start]);
                result.push_str(replacement);
                last_end = *end;
            }
        }

        result.push_str(&text[last_end..]);
        result
    }

    /// Check if a range overlaps with any existing replacement.
    fn overlaps(&self, replacements: &[(usize, usize, String)], start: usize, end: usize) -> bool {
        replacements
            .iter()
            .any(|(rs, re, _)| start < *re && end > *rs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn make_detector(cwd: &Path) -> LinkDetector {
        LinkDetector::new(cwd.to_path_buf())
    }

    #[test]
    fn test_http_url() {
        let mut d = make_detector(Path::new("/tmp"));
        let input = "Visit https://github.com/foo/bar for details";
        let result = d.enhance_line(input);
        assert!(result.contains("\x1b]8;;https://github.com/foo/bar\x07"));
        assert!(result.contains("https://github.com/foo/bar\x1b]8;;\x07"));
    }

    #[test]
    fn test_https_url() {
        let mut d = make_detector(Path::new("/tmp"));
        let input = "See https://example.com/path?q=1&b=2 for info";
        let result = d.enhance_line(input);
        assert!(result.contains("\x1b]8;;https://example.com/path?q=1&b=2\x07"));
    }

    #[test]
    fn test_http_url_basic() {
        let mut d = make_detector(Path::new("/tmp"));
        let input = "Go to http://localhost:8080/api endpoint";
        let result = d.enhance_line(input);
        assert!(result.contains("\x1b]8;;http://localhost:8080/api\x07"));
    }

    #[test]
    fn test_file_url_passthrough() {
        let mut d = make_detector(Path::new("/tmp"));
        let input = "Open file:///Users/chen/main.rs in editor";
        let result = d.enhance_line(input);
        assert!(result.contains("\x1b]8;;file:///Users/chen/main.rs\x07"));
    }

    #[test]
    fn test_mailto_link() {
        let mut d = make_detector(Path::new("/tmp"));
        let input = "Contact mailto:user@example.com for help";
        let result = d.enhance_line(input);
        assert!(result.contains("\x1b]8;;mailto:user@example.com\x07"));
    }

    #[test]
    fn test_absolute_path_existing_file() {
        let dir = std::env::temp_dir().join("claudex_test_osc8");
        fs::create_dir_all(&dir).unwrap();
        let test_file = dir.join("test.rs");
        fs::write(&test_file, "fn main() {}").unwrap();

        let mut d = make_detector(&dir);
        let path = test_file.to_str().unwrap();
        let input = format!("Error at {path}:42:10");
        let result = d.enhance_line(&input);
        assert!(result.contains("\x1b]8;;file://"));
        assert!(result.contains("test.rs"));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_absolute_path_nonexistent() {
        let mut d = make_detector(Path::new("/tmp"));
        let input = "/nonexistent/path/to/file.rs:42";
        let result = d.enhance_line(input);
        // Should NOT be wrapped (file doesn't exist)
        assert!(!result.contains("\x1b]8;"));
    }

    #[test]
    fn test_relative_path_dot_slash() {
        let dir = std::env::temp_dir().join("claudex_test_osc8_rel");
        fs::create_dir_all(dir.join("src")).unwrap();
        let test_file = dir.join("src/main.rs");
        fs::write(&test_file, "fn main() {}").unwrap();

        let mut d = make_detector(&dir);
        let input = "See ./src/main.rs:42 for details";
        let result = d.enhance_line(input);
        assert!(result.contains("\x1b]8;;file://"));
        assert!(result.contains("./src/main.rs:42"));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_relative_path_dir_file() {
        let dir = std::env::temp_dir().join("claudex_test_osc8_dir");
        fs::create_dir_all(dir.join("src")).unwrap();
        let test_file = dir.join("src/config.rs");
        fs::write(&test_file, "struct Config {}").unwrap();

        let mut d = make_detector(&dir);
        let input = "Modified src/config.rs";
        let result = d.enhance_line(input);
        assert!(result.contains("\x1b]8;;file://"));
        assert!(result.contains("src/config.rs"));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_path_with_line_col() {
        let dir = std::env::temp_dir().join("claudex_test_osc8_linecol");
        fs::create_dir_all(dir.join("src")).unwrap();
        let test_file = dir.join("src/main.rs");
        fs::write(&test_file, "fn main() {}").unwrap();

        let mut d = make_detector(&dir);
        let input = "Error at src/main.rs:42:10";
        let result = d.enhance_line(input);
        assert!(result.contains("\x1b]8;;file://"));
        assert!(result.contains("src/main.rs:42:10"));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_already_has_osc8() {
        let mut d = make_detector(Path::new("/tmp"));
        let input = "Link: \x1b]8;;https://example.com\x07example\x1b]8;;\x07 done";
        let result = d.enhance_line(input);
        assert_eq!(result, input);
    }

    #[test]
    fn test_ansi_colors_preserved() {
        let mut d = make_detector(Path::new("/tmp"));
        let input = "\x1b[32mhttps://github.com/repo\x1b[0m rest";
        let result = d.enhance_line(input);
        // ANSI codes should be preserved
        assert!(result.contains("\x1b[32m"));
        assert!(result.contains("\x1b[0m"));
        // URL should be wrapped
        assert!(result.contains("\x1b]8;;https://github.com/repo\x07"));
    }

    #[test]
    fn test_multiple_links_one_line() {
        let mut d = make_detector(Path::new("/tmp"));
        let input = "See https://a.com and https://b.com for info";
        let result = d.enhance_line(input);
        assert!(result.contains("\x1b]8;;https://a.com\x07"));
        assert!(result.contains("\x1b]8;;https://b.com\x07"));
    }

    #[test]
    fn test_plain_text_no_change() {
        let mut d = make_detector(Path::new("/tmp"));
        let input = "This is just plain text with no links";
        let result = d.enhance_line(input);
        assert_eq!(result, input);
    }

    #[test]
    fn test_empty_line() {
        let mut d = make_detector(Path::new("/tmp"));
        let result = d.enhance_line("");
        assert_eq!(result, "");
    }

    #[test]
    fn test_pure_ansi_escape_line() {
        let mut d = make_detector(Path::new("/tmp"));
        let input = "\x1b[2J\x1b[H";
        let result = d.enhance_line(input);
        assert_eq!(result, input);
    }

    #[test]
    fn test_wrap_osc8_format() {
        let result = wrap_osc8("https://example.com", "example");
        assert_eq!(result, "\x1b]8;;https://example.com\x07example\x1b]8;;\x07");
    }

    #[test]
    fn test_file_path_to_uri_absolute() {
        let uri = file_path_to_uri("/Users/chen/main.rs", Path::new("/tmp"));
        assert_eq!(uri, "file:///Users/chen/main.rs");
    }

    #[test]
    fn test_file_path_to_uri_relative() {
        let uri = file_path_to_uri("src/main.rs", Path::new("/project"));
        assert_eq!(uri, "file:///project/src/main.rs");
    }

    #[test]
    fn test_file_path_to_uri_with_line() {
        let uri = file_path_to_uri("src/main.rs:42:10", Path::new("/project"));
        assert_eq!(uri, "file:///project/src/main.rs");
    }

    #[test]
    fn test_file_cache_works() {
        let dir = std::env::temp_dir().join("claudex_test_osc8_cache");
        fs::create_dir_all(&dir).unwrap();
        let test_file = dir.join("cached.rs");
        fs::write(&test_file, "").unwrap();

        let mut d = make_detector(&dir);
        assert!(d.check_file_exists("cached.rs"));
        // Second call should use cache
        assert!(d.check_file_exists("cached.rs"));
        assert_eq!(d.file_cache.len(), 1);

        fs::remove_dir_all(&dir).ok();
    }

    // ── Additional test cases from the plan ──

    #[test]
    fn test_url_with_trailing_punctuation() {
        let mut d = make_detector(Path::new("/tmp"));
        // Trailing period should not be part of the URL
        let input = "Visit https://example.com.";
        let result = d.enhance_line(input);
        assert!(result.contains("\x1b]8;;https://example.com\x07"));
        assert!(result.contains("https://example.com\x1b]8;;\x07."));
    }

    #[test]
    fn test_url_with_parentheses() {
        let mut d = make_detector(Path::new("/tmp"));
        // URL inside parentheses: should not include closing paren
        let input = "(https://example.com/path)";
        let result = d.enhance_line(input);
        assert!(result.contains("\x1b]8;;https://example.com/path\x07"));
    }

    #[test]
    fn test_url_with_fragment() {
        let mut d = make_detector(Path::new("/tmp"));
        let input = "See https://example.com/page#section for details";
        let result = d.enhance_line(input);
        assert!(result.contains("\x1b]8;;https://example.com/page#section\x07"));
    }

    #[test]
    fn test_multiple_ansi_segments_with_url() {
        let mut d = make_detector(Path::new("/tmp"));
        // URL split across ANSI reset boundaries shouldn't break
        let input = "\x1b[1m\x1b[34mhttps://github.com/test\x1b[0m";
        let result = d.enhance_line(input);
        assert!(result.contains("\x1b[1m"));
        assert!(result.contains("\x1b[34m"));
        assert!(result.contains("\x1b]8;;https://github.com/test\x07"));
    }

    #[test]
    fn test_relative_path_nonexistent_not_wrapped() {
        let dir = std::env::temp_dir().join("claudex_test_osc8_norel");
        fs::create_dir_all(&dir).unwrap();

        let mut d = make_detector(&dir);
        let input = "See src/nonexistent_file.rs for details";
        let result = d.enhance_line(input);
        // File doesn't exist → should not be wrapped
        assert!(!result.contains("\x1b]8;"));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_line_with_only_line_number_path() {
        let dir = std::env::temp_dir().join("claudex_test_osc8_lineonly");
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::write(dir.join("src/lib.rs"), "").unwrap();

        let mut d = make_detector(&dir);
        let input = "  --> src/lib.rs:15";
        let result = d.enhance_line(input);
        assert!(result.contains("\x1b]8;;file://"));
        assert!(result.contains("src/lib.rs:15"));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_url_not_double_wrapped() {
        let mut d = make_detector(Path::new("/tmp"));
        // A URL that looks like it could also match an absolute path regex
        let input = "https://example.com/path/to/file.rs";
        let result = d.enhance_line(input);
        // Should only have exactly one OSC 8 open and one close
        let open_count = result.matches("\x1b]8;;").count();
        // open_count includes the close sequence (empty URI), so 2 means one link
        assert_eq!(open_count, 2, "Should have exactly one OSC 8 link pair");
    }

    #[test]
    fn test_path_and_url_on_same_line() {
        let dir = std::env::temp_dir().join("claudex_test_osc8_mixed");
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::write(dir.join("src/app.rs"), "").unwrap();

        let mut d = make_detector(&dir);
        let input = "See https://docs.rs and src/app.rs for info";
        let result = d.enhance_line(input);
        // Both should be wrapped
        assert!(result.contains("\x1b]8;;https://docs.rs\x07"));
        assert!(result.contains("\x1b]8;;file://"));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_split_ansi_segments_empty() {
        let segments = split_ansi_segments("");
        assert!(segments.is_empty());
    }

    #[test]
    fn test_split_ansi_segments_no_ansi() {
        let segments = split_ansi_segments("hello world");
        assert_eq!(segments.len(), 1);
        assert!(matches!(&segments[0], Segment::Text(t) if t == "hello world"));
    }

    #[test]
    fn test_split_ansi_segments_only_ansi() {
        let segments = split_ansi_segments("\x1b[31m\x1b[0m");
        assert_eq!(segments.len(), 2);
        assert!(matches!(&segments[0], Segment::Escape(e) if e == "\x1b[31m"));
        assert!(matches!(&segments[1], Segment::Escape(e) if e == "\x1b[0m"));
    }

    #[test]
    fn test_split_ansi_segments_mixed() {
        let segments = split_ansi_segments("before\x1b[32mgreen\x1b[0mafter");
        // "before" | ESC[32m | "green" | ESC[0m | "after"
        assert_eq!(segments.len(), 5);
        assert!(matches!(&segments[0], Segment::Text(t) if t == "before"));
        assert!(matches!(&segments[1], Segment::Escape(e) if e == "\x1b[32m"));
        assert!(matches!(&segments[2], Segment::Text(t) if t == "green"));
        assert!(matches!(&segments[3], Segment::Escape(e) if e == "\x1b[0m"));
        assert!(matches!(&segments[4], Segment::Text(t) if t == "after"));
    }

    #[test]
    fn test_overlaps_detection() {
        let d = make_detector(Path::new("/tmp"));
        let replacements = vec![(5usize, 10usize, String::new())];
        assert!(d.overlaps(&replacements, 7, 12)); // overlaps
        assert!(d.overlaps(&replacements, 3, 7)); // overlaps
        assert!(d.overlaps(&replacements, 5, 10)); // exact match
        assert!(!d.overlaps(&replacements, 0, 5)); // adjacent, no overlap
        assert!(!d.overlaps(&replacements, 10, 15)); // adjacent, no overlap
        assert!(!d.overlaps(&replacements, 15, 20)); // far away
    }

    #[test]
    fn test_line_with_only_dots_and_slashes() {
        let mut d = make_detector(Path::new("/tmp"));
        // Should not crash or produce invalid output
        let input = "../../../";
        let result = d.enhance_line(input);
        // May or may not be wrapped depending on regex matching,
        // but must not panic or produce malformed output
        assert!(!result.is_empty());
    }

    #[test]
    fn test_enhance_preserves_non_link_text() {
        let mut d = make_detector(Path::new("/tmp"));
        let input = "Hello https://example.com World";
        let result = d.enhance_line(input);
        assert!(result.starts_with("Hello "));
        assert!(result.ends_with(" World"));
    }

    #[test]
    fn test_link_detector_new_cwd() {
        let d = LinkDetector::new(PathBuf::from("/test/dir"));
        assert_eq!(d.cwd, PathBuf::from("/test/dir"));
        assert!(d.file_cache.is_empty());
    }

    #[test]
    fn test_dotdot_path() {
        let dir = std::env::temp_dir().join("claudex_test_osc8_dotdot/sub");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            std::env::temp_dir().join("claudex_test_osc8_dotdot/parent.rs"),
            "",
        )
        .unwrap();

        let mut d = make_detector(&dir);
        let input = "See ../parent.rs for reference";
        let result = d.enhance_line(input);
        assert!(result.contains("\x1b]8;;file://"));
        assert!(result.contains("../parent.rs"));

        fs::remove_dir_all(std::env::temp_dir().join("claudex_test_osc8_dotdot")).ok();
    }

    #[test]
    fn test_url_with_encoded_chars() {
        let mut d = make_detector(Path::new("/tmp"));
        let input = "https://example.com/path%20with%20spaces?q=hello+world";
        let result = d.enhance_line(input);
        assert!(
            result.contains("\x1b]8;;https://example.com/path%20with%20spaces?q=hello+world\x07")
        );
    }

    #[test]
    fn test_multiple_urls_with_text_between() {
        let mut d = make_detector(Path::new("/tmp"));
        let input = "First: https://a.com, Second: https://b.com, Third: https://c.com";
        let result = d.enhance_line(input);
        assert!(result.contains("\x1b]8;;https://a.com\x07"));
        assert!(result.contains("\x1b]8;;https://b.com\x07"));
        assert!(result.contains("\x1b]8;;https://c.com\x07"));
        // Text between should be preserved
        assert!(result.contains("First: "));
        assert!(result.contains(", Second: "));
        assert!(result.contains(", Third: "));
    }

    #[test]
    fn test_file_cache_negative_result() {
        let dir = std::env::temp_dir().join("claudex_test_osc8_neg_cache");
        fs::create_dir_all(&dir).unwrap();

        let mut d = make_detector(&dir);
        assert!(!d.check_file_exists("nonexistent.rs"));
        // Negative result should also be cached
        assert_eq!(d.file_cache.len(), 1);
        assert_eq!(d.file_cache.get("nonexistent.rs"), Some(&false));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_quick_skip_no_linklike_chars() {
        let mut d = make_detector(Path::new("/tmp"));
        // Line with no '/', '.', '://' or 'mailto:' → fast-path skip
        let input = "Hello world, nothing special here!";
        let result = d.enhance_line(input);
        assert_eq!(result, input);
    }
}
