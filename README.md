[![Rust](https://github.com/bornacvitanic/portside/actions/workflows/rust.yml/badge.svg)](https://github.com/bornacvitanic/portside/actions/workflows/rust.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Crates.io](https://img.shields.io/crates/v/portside.svg)](https://crates.io/crates/portside)

# portside

`portside` is the shared core for the **freight** suite of `cargo-*` developer tools ([`cargo-bay`](https://github.com/bornacvitanic/cargo-bay), [`cargo-crane`](https://github.com/bornacvitanic/cargo-crane), …). It turns a single `cargo metadata` call into a small, tool-agnostic model of the workspace so each tool is a thin app on top rather than a re-implementation.

It's useful on its own too: if you're writing a Cargo subcommand and just want the workspace members, their targets, and their declared dependencies without hand-rolling the `cargo metadata` JSON, this is that.

## What v0.1 provides

Workspace discovery:

- **`load`** — run `cargo metadata` (optionally with the resolved dependency graph) and get a `Workspace`.
- **`parse`** — the JSON → model step on its own, so you can test against fixtures without invoking cargo.
- **`Workspace`** — the root, the real target directory, and the member `Package`s; plus `member(name)` and `linked_members(id)` (the transitive workspace-member closure, for per-crate staleness).
- **`Package`** — name, manifest dir, description, edition, license, `targets`, `dependencies`, and helpers: `crate_root()`, `src_dir()`, `bin_target()`, `lib_target()`, `manifest_path()`.
- **`Dependency`** — name, rename, version req, features, kind (normal / dev / build), and `extern_ident()`.

## Usage

```toml
[dependencies]
portside = "0.1"
```

```rust
let ws = portside::load(&portside::LoadOptions::default())?;
for member in &ws.members {
    if let Some(root) = member.crate_root() {
        println!("{:<20} {}", member.name, root.display());
    }
}
# Ok::<(), portside::Error>(())
```

Pass `LoadOptions { resolve: true, .. }` when you need `linked_members` (the dependency graph); it's omitted by default for speed.

## Scope

v0.1 is workspace discovery only. The freight suite's other shared pieces — freshness/staleness heuristics, the background-job runner, and the TUI chrome (palette, list/detail/log layout) — are candidates for later releases as they prove out across more than one tool.

## License

Licensed under the MIT License — see [LICENSE](LICENSE.md).
