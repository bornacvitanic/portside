# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0] - 2026-07-09

### Features

- Add shared TUI chrome behind the `tui` feature

A `chrome` module with the suite's palette, status glyphs, brandmark, spinner,
and render helpers (`brand_line`, `log_console`) — the guardrail against
palette/keybinding drift across the terminal tools. Gated behind the optional
`tui` feature (pulling `ratatui`), so the discovery core stays dependency-light
for non-TUI users.

## [0.1.0] - 2026-07-09

### Features

- Add portside: shared workspace-discovery core for the freight suite

Turn a single `cargo metadata` call into a small, tool-agnostic model: the
workspace root and target directory, the member packages with their targets
(bin/lib) and declared dependencies (name, rename, version req, features,
kind), and — optionally — the resolved dependency graph for computing the
transitive workspace-member closure. JSON parsing is exposed separately from
the cargo invocation so it can be tested against fixtures.
