//! Template-based renderer for BI indicator cards.
//!
//! CSS and card HTML are loaded from external files via `include_str!` at compile time.
//! Reference pages (classic.html / modern.html) remain unchanged for developer preview.
//! Template files:
//!   - assets/dashboards/classic.css      — CSS for classic style
//!   - assets/dashboards/classic_card.html — single card with {{placeholders}}
//!   - assets/dashboards/modern.css       — CSS for modern style
//!   - assets/dashboards/modern_card.html  — single card with {{placeholders}}

use super::spark::{demo_spark_points, points_to_svg_path};
use super::IndicatorCardParams;

// ── Embedded templates (compile-time) ────────────────────────────────────────

const CLASSIC_CSS: &str = include_str!("../../../assets/dashboards/classic.css");
const CLASSIC_CARD: &str = include_str!("../../../assets/dashboards/classic_card.html");
const MODERN_CSS: &str = include_str!("../../../assets/dashboards/modern.css");
const MODERN_CARD: &str = include_str!("../../../assets/dashboards/modern_card.html");

// ── Arrow SVGs ────────────────────────────────────────────────────────────────

const ARROW_UP: &str = r#"<svg viewBox="0 0 24 24" fill="none" aria-hidden="true" style="width:14px;height:14px"><path d="M7 14l5-5 5 5" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"/></svg>"#;
const ARROW_DOWN: &str = r#"<svg viewBox="0 0 24 24" fill="none" aria-hidden="true" style="width:14px;height:14px"><path d="M7 10l5 5 5-5" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"/></svg>"#;
const ARROW_FLAT: &str = r#"<svg viewBox="0 0 24 24" fill="none" aria-hidden="true" style="width:14px;height:14px"><path d="M6 12h12" stroke="currentColor" stroke-width="2.2" stroke-linecap="round"/></svg>"#;

// ── Public render entry point ─────────────────────────────────────────────────

/// Render a full HTML document (srcdoc) for a single indicator card.
pub fn render_srcdoc(params: &IndicatorCardParams) -> String {
    match params.style_name.as_str() {
        "modern" => render_modern(params),
        "custom" => render_custom(params),
        _ => render_classic(params),
    }
}

// ── Classic ───────────────────────────────────────────────────────────────────

fn render_classic(p: &IndicatorCardParams) -> String {
    let spark_pts = if p.spark_points.is_empty() {
        demo_spark_points()
    } else {
        p.spark_points.clone()
    };
    let (spark_line, spark_fill) = points_to_svg_path(&spark_pts);

    let delta_arrow = match p.delta_dir.as_str() {
        "down" => ARROW_DOWN,
        "flat" => ARROW_FLAT,
        _ => ARROW_UP,
    };
    let delta_neutral_class = if p.delta_dir == "flat" { "neutral" } else { "" };

    let card = CLASSIC_CARD
        .replace("{{col_class}}", &p.col_class)
        .replace("{{status}}", &p.status)
        .replace("{{name}}", &p.name)
        .replace("{{meta_1}}", &p.meta_1)
        .replace("{{meta_2}}", &p.meta_2)
        .replace("{{chip}}", &p.chip)
        .replace("{{value}}", &p.value)
        .replace("{{unit}}", &p.unit)
        .replace("{{delta}}", &p.delta)
        .replace("{{delta_arrow_svg}}", delta_arrow)
        .replace("{{delta_neutral_class}}", delta_neutral_class)
        .replace("{{spark_line}}", &spark_line)
        .replace("{{spark_fill}}", &spark_fill);

    wrap_in_page(CLASSIC_CSS, &card, &p.theme)
}

// ── Modern ────────────────────────────────────────────────────────────────────

fn render_modern(p: &IndicatorCardParams) -> String {
    let progress = p.progress.to_string();

    let card = MODERN_CARD
        .replace("{{col_class}}", &p.col_class)
        .replace("{{status}}", &p.status)
        .replace("{{progress}}", &progress)
        .replace("{{name}}", &p.name)
        .replace("{{meta_1}}", &p.meta_1)
        .replace("{{meta_2}}", &p.meta_2)
        .replace("{{chip}}", &p.chip)
        .replace("{{value}}", &p.value)
        .replace("{{delta}}", &p.delta)
        .replace("{{hint}}", &p.hint)
        .replace("{{footer_1}}", &p.footer_1)
        .replace("{{footer_2}}", &p.footer_2);

    wrap_in_page(MODERN_CSS, &card, &p.theme)
}

// ── Custom (backward compat) ──────────────────────────────────────────────────

fn render_custom(p: &IndicatorCardParams) -> String {
    let rendered = p
        .custom_html
        .as_deref()
        .unwrap_or("")
        .replace("{{title}}", &p.name)
        .replace("{{name}}", &p.name)
        .replace("{{value}}", &p.value)
        .replace("{{delta}}", &p.delta)
        .replace("{{unit}}", &p.unit)
        .replace("{{status}}", &p.status)
        .replace("{{chip}}", &p.chip);

    let user_css = p.custom_css.as_deref().unwrap_or("");
    let theme = if p.theme.is_empty() { "dark" } else { &p.theme };

    format!(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="UTF-8">
<style>
*,*::before,*::after{{box-sizing:border-box;margin:0;padding:0;}}
body{{
  font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif;
  background:var(--bi-bg,#ffffff);color:var(--bi-text,#1a1a2e);padding:16px;min-height:100%;
}}
:root{{
  --bi-primary:#3b82f6;--bi-success:#22c55e;--bi-danger:#ef4444;--bi-warning:#f59e0b;
  --bi-text:#1e293b;--bi-text-secondary:#64748b;--bi-bg:#ffffff;--bi-bg-secondary:#f8fafc;
  --bi-border:#e2e8f0;
}}
body[data-theme="dark"]{{
  --bi-text:#e5e7eb;--bi-text-secondary:#9aa4b2;--bi-bg:#0b1220;--bi-bg-secondary:#0f1a2e;
  --bi-border:rgba(255,255,255,.12);
}}
{user_css}
</style>
</head>
<body data-theme="{theme}"><div class="indicator-cell">{rendered}</div></body>
</html>"#,
        user_css = user_css,
        theme = theme,
        rendered = rendered,
    )
}

// ── Shared page wrapper ───────────────────────────────────────────────────────

/// Wraps a single card in a minimal centering page.
/// Uses a 280px-wide wrapper (≈ col-3 in 1200px grid) to preserve natural card size.
fn wrap_in_page(css: &str, card: &str, theme: &str) -> String {
    let theme = if theme.is_empty() { "dark" } else { theme };
    format!(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="UTF-8">
<style>{css}</style>
</head>
<body data-theme="{theme}">
<div style="width:min(280px,100%)">
{card}
</div>
</body>
</html>"#,
        css = css,
        theme = theme,
        card = card,
    )
}
