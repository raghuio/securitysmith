//! Terminal color support with NO_COLOR and isatty detection.
//!
//! Colors are enabled only when:
//! 1. The `NO_COLOR` environment variable is NOT set, AND
//! 2. stdout is a TTY (not piped or redirected)
//!
//! See: https://no-color.org/

use std::io::IsTerminal;

/// ANSI color codes.
const RED: &str = "\x1b[31m";
const BRIGHT_RED: &str = "\x1b[31;1m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const BLUE: &str = "\x1b[34m";
const GRAY: &str = "\x1b[90m";
const RESET: &str = "\x1b[0m";

/// Returns true if colored output should be used.
pub fn should_color() -> bool {
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    std::io::stdout().is_terminal()
}

/// Wraps text in ANSI color codes if colors are enabled, otherwise returns as-is.
pub fn paint(text: &str, code: &str) -> String {
    if should_color() {
        format!("{code}{text}{RESET}")
    } else {
        text.to_string()
    }
}

/// Color a severity level string.
pub fn severity(text: &str) -> String {
    let code = match text {
        "critical" => BRIGHT_RED,
        "high" => RED,
        "medium" => YELLOW,
        "low" => BLUE,
        "informational" => GRAY,
        _ => return text.to_string(),
    };
    paint(text, code)
}

/// Color a status string.
pub fn status(text: &str) -> String {
    let code = match text {
        "open" => RED,
        "fixed" => GREEN,
        "false_positive" | "not_applicable" => GRAY,
        "risk_accepted" => YELLOW,
        _ => return text.to_string(),
    };
    paint(text, code)
}

/// Color an error prefix.
pub fn error_prefix(text: &str) -> String {
    paint(text, RED)
}

/// Color a warning prefix.
pub fn warn_prefix(text: &str) -> String {
    paint(text, YELLOW)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Mutex to serialize tests that modify environment variables.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn no_color_env_disables_colors() {
        let _guard = ENV_LOCK.lock().unwrap();
        // SAFETY: serialized by ENV_LOCK, single-threaded test
        unsafe { std::env::set_var("NO_COLOR", "1") };
        assert!(!should_color());
        assert_eq!(paint("hello", RED), "hello");
        assert_eq!(severity("critical"), "critical");
        assert_eq!(status("open"), "open");
        // SAFETY: serialized by ENV_LOCK, single-threaded test
        unsafe { std::env::remove_var("NO_COLOR") };
    }

    #[test]
    fn unknown_severity_not_colored() {
        let _guard = ENV_LOCK.lock().unwrap();
        // SAFETY: serialized by ENV_LOCK, single-threaded test
        unsafe { std::env::set_var("NO_COLOR", "1") };
        assert_eq!(severity("unknown"), "unknown");
        // SAFETY: serialized by ENV_LOCK, single-threaded test
        unsafe { std::env::remove_var("NO_COLOR") };
    }

    #[test]
    fn unknown_status_not_colored() {
        let _guard = ENV_LOCK.lock().unwrap();
        // SAFETY: serialized by ENV_LOCK, single-threaded test
        unsafe { std::env::set_var("NO_COLOR", "1") };
        assert_eq!(status("unknown"), "unknown");
        // SAFETY: serialized by ENV_LOCK, single-threaded test
        unsafe { std::env::remove_var("NO_COLOR") };
    }
}
