//! **portside** — the shared core for the [freight] suite of `cargo-*`
//! developer tools.
//!
//! v0.1 provides workspace discovery: run [`load`] to turn a single
//! `cargo metadata` call into a small, tool-agnostic model of the workspace,
//! its members, their targets, and their declared dependencies.
//!
//! ```no_run
//! let ws = portside::load(&portside::LoadOptions::default())?;
//! for member in &ws.members {
//!     if let Some(root) = member.crate_root() {
//!         println!("{} — {}", member.name, root.display());
//!     }
//! }
//! # Ok::<(), portside::Error>(())
//! ```
//!
//! [freight]: https://github.com/bornacvitanic/cargo-crane

mod error;
mod metadata;

pub use error::Error;
pub use metadata::{
    load, parse, DepKind, Dependency, LoadOptions, Package, Target, TargetKind, Workspace,
};
