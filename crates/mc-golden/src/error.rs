//! The crate's single error type.

use std::fmt;
use std::path::PathBuf;

use crate::compare::Mismatch;

/// Everything that can make a golden run fail.
#[derive(Debug)]
pub enum GoldenError {
    /// Reading or writing a file failed.
    Io {
        /// The file involved.
        path: PathBuf,
        /// The underlying error.
        source: std::io::Error,
    },
    /// A fixture file is malformed.
    Parse {
        /// The file involved.
        path: PathBuf,
        /// 1-based line number, for `.jsonl` files.
        line: Option<usize>,
        /// What was wrong.
        message: String,
    },
    /// The trace differs from `expected.jsonl` (PLAN_v4 §4.4 item 3).
    Mismatch {
        /// The first difference.
        mismatch: Box<Mismatch>,
        /// The fixture's `expected.jsonl`.
        expected: PathBuf,
        /// Where the Rust trace was written.
        actual: PathBuf,
    },
    /// Anything else, typically raised by a drive closure.
    Other(String),
}

impl GoldenError {
    /// Convenience for drive closures: `Err(GoldenError::other("..."))`.
    pub fn other(message: impl Into<String>) -> Self {
        Self::Other(message.into())
    }
}

impl fmt::Display for GoldenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "{}: {source}", path.display()),
            Self::Parse {
                path,
                line: Some(line),
                message,
            } => write!(f, "{}:{line}: {message}", path.display()),
            Self::Parse {
                path,
                line: None,
                message,
            } => write!(f, "{}: {message}", path.display()),
            Self::Mismatch {
                mismatch,
                expected,
                actual,
            } => write!(
                f,
                "{mismatch}\n  expected file: {}\n  actual file:   {}",
                expected.display(),
                actual.display()
            ),
            Self::Other(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for GoldenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}
