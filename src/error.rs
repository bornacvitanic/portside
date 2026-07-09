//! Why workspace discovery couldn't proceed — each variant renders an
//! actionable message, so tools can surface it verbatim.

use std::fmt;

#[derive(Debug)]
pub enum Error {
    /// `cargo` was not found on `PATH`.
    CargoNotFound,
    /// `cargo metadata` failed — usually "not inside a workspace".
    NotWorkspace(String),
    /// `cargo metadata` couldn't be launched.
    Metadata(String),
    /// The `cargo metadata` JSON couldn't be parsed.
    Parse(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::CargoNotFound => write!(
                f,
                "cargo was not found on PATH.\n\
                 Install Rust from https://rustup.rs, then run again inside a workspace."
            ),
            Error::NotWorkspace(msg) => write!(
                f,
                "not inside a Cargo workspace.\n  cargo metadata: {msg}\n\
                 Run from a directory with a Cargo.toml, or pass a manifest path."
            ),
            Error::Metadata(e) => write!(f, "failed to run cargo metadata: {e}"),
            Error::Parse(e) => write!(f, "failed to parse cargo metadata: {e}"),
        }
    }
}

impl std::error::Error for Error {}
