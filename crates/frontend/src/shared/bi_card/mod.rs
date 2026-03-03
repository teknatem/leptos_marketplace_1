//! BI indicator card rendering system.
//!
//! Developer-defined styles (classic, modern) are embedded as template strings.
//! The user selects a style; the system fills placeholders and renders an iframe srcdoc.

pub mod renderer;
pub mod spark;

pub use renderer::render_srcdoc;
pub use spark::{demo_spark_points, points_to_svg_path};

/// All data needed to render one indicator card.
#[derive(Debug, Clone, Default)]
pub struct IndicatorCardParams {
    /// Style: "classic" | "modern" | "custom"
    pub style_name: String,
    /// App theme: "dark" | "light" (maps to data-theme on <body> in iframe)
    pub theme: String,
    /// Indicator name / title
    pub name: String,
    /// Formatted value string (e.g. "₽2.40M")
    pub value: String,
    /// Unit suffix (e.g. "RUB", "MAU", "%") — may be empty
    pub unit: String,
    /// Delta text (e.g. "+12.5%")
    pub delta: String,
    /// Delta direction: "up" | "down" | "flat"
    pub delta_dir: String,
    /// Status: "ok" | "bad" | "warn" | "neutral"
    pub status: String,
    /// Badge / category label (e.g. "Выручка", "KPI")
    pub chip: String,
    /// Grid column class: "col-3" | "col-4" | "col-6" | "col-12"
    pub col_class: String,
    /// Progress 0–100 for ring indicator (modern style)
    pub progress: u8,
    /// Sparkline data points (classic style; empty → demo data)
    pub spark_points: Vec<f64>,
    /// First meta line (e.g. "Период: 30 дней")
    pub meta_1: String,
    /// Second meta line (e.g. "Обновлено: 5 мин")
    pub meta_2: String,
    /// Hint text below delta (modern)
    pub hint: String,
    /// Footer item 1 (modern)
    pub footer_1: String,
    /// Footer item 2 (modern)
    pub footer_2: String,
    /// Custom HTML (only used when style_name = "custom")
    pub custom_html: Option<String>,
    /// Custom CSS (only used when style_name = "custom")
    pub custom_css: Option<String>,
}
