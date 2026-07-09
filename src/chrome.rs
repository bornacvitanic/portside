//! Shared TUI chrome for the freight suite: the palette, status glyphs, the
//! brandmark, a spinner, and a couple of render helpers. This is the guardrail
//! against palette/keybinding drift across the tools — learn one, know them all.
//!
//! Behind the `tui` feature so portside's discovery core stays ratatui-free.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

// --- palette ------------------------------------------------------------

/// The suite accent (selections, keys, the brandmark).
pub const ACCENT: Color = Color::Cyan;
/// Secondary / label text.
pub const MUTED: Color = Color::DarkGray;
/// Good: passing, fresh.
pub const OK: Color = Color::Green;
/// Caution: warnings, stale.
pub const WARN: Color = Color::Yellow;
/// Bad: failure.
pub const BAD: Color = Color::Red;
/// Informational: not built yet, first-run.
pub const INFO: Color = Color::Blue;

// --- glyphs -------------------------------------------------------------

/// Freshness (shared with cargo-bay / cargo-deck).
pub const G_FRESH: &str = "[+]";
pub const G_STALE: &str = "[*]";
pub const G_NEW: &str = "[.]";

/// Status.
pub const G_OK: &str = "✓";
pub const G_BAD: &str = "✗";
pub const G_WARN: &str = "⚠";
pub const G_NA: &str = "—";

/// The brandmark; swap to `"#"` for strict-ASCII terminals.
pub const BRAND_MARK: &str = "⚓";

const SPINNER: [char; 4] = ['|', '/', '-', '\\'];

/// The spinner frame for a tick counter.
pub fn spinner(tick: u64) -> char {
    SPINNER[(tick as usize) % SPINNER.len()]
}

// --- helpers ------------------------------------------------------------

/// The `⚓ freight · <tool>` brand line for a header bar.
pub fn brand_line(tool: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{BRAND_MARK} freight"),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!(" · {tool}"), Style::default().fg(MUTED)),
    ])
}

/// A bordered, tail-following log console over `lines`, scrolled up by `scroll`
/// lines from the bottom, sized to `height` rows.
pub fn log_console(lines: &[String], scroll: u16, height: u16, title: &str) -> Paragraph<'static> {
    let visible = height.saturating_sub(2) as usize;
    let end = lines.len().saturating_sub(scroll as usize);
    let start = end.saturating_sub(visible);
    let text: Vec<Line> = lines[start..end.max(start)]
        .iter()
        .map(|l| Line::from(l.clone()))
        .collect();
    Paragraph::new(text).block(Block::bordered().title(title.to_string()))
}
