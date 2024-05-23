/// Module for structured logging using the slog crate.
pub mod slog;

/// Trait defining the methods for logging.
pub trait Logger {
    /// Log a debug message.
    fn debug(&self, target: &str, message: &str, tags: &[&str]);

    /// Log an info message.
    fn info(&self, target: &str, message: &str, tags: &[&str]);

    /// Log a warning message.
    fn warn(&self, target: &str, message: &str, tags: &[&str]);

    /// Log an error message.
    fn error(&self, target: &str, message: &str, tags: &[&str]);
}
