use crate::shared::icons::icon;
use contracts::shared::indicators::{IndicatorStatus, ValueFormat};
use leptos::prelude::*;

fn format_value(val: f64, fmt: &ValueFormat) -> String {
    match fmt {
        ValueFormat::Money { currency } => {
            let abs = val.abs();
            let formatted = if abs >= 1_000_000.0 {
                format!("{:.1}M", val / 1_000_000.0)
            } else if abs >= 1_000.0 {
                let int_part = val as i64;
                let frac = ((val.abs() - (int_part.abs() as f64)) * 100.0).round() as i64;
                let s = format_thousands(int_part);
                if frac == 0 {
                    s
                } else {
                    format!("{},{:02}", s, frac)
                }
            } else {
                format!("{:.2}", val).replace('.', ",")
            };
            format!("{} {}", formatted, currency)
        }
        ValueFormat::Number { decimals } => {
            format!("{:.prec$}", val, prec = *decimals as usize).replace('.', ",")
        }
        ValueFormat::Percent { decimals } => {
            format!("{:.prec$}%", val, prec = *decimals as usize).replace('.', ",")
        }
        ValueFormat::Integer => format_thousands(val as i64),
    }
}

fn format_thousands(n: i64) -> String {
    let s = n.abs().to_string();
    let mut result = String::new();
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push('\u{00a0}');
        }
        result.push(ch);
    }
    if n < 0 {
        result.push('-');
    }
    result.chars().rev().collect()
}

#[component]
pub fn StatCard(
    /// Label displayed above the value
    label: String,
    /// Icon name from the icon() helper
    icon_name: String,
    /// Primary numeric value (None = loading/error)
    #[prop(into)]
    value: Signal<Option<f64>>,
    /// How to format the value
    format: ValueFormat,
    /// Visual status
    #[prop(into)]
    status: Signal<IndicatorStatus>,
    /// Change % relative to previous period
    #[prop(into)]
    change_percent: Signal<Option<f64>>,
    /// Optional subtitle below the value
    #[prop(into, optional)]
    subtitle: Signal<Option<String>>,
) -> impl IntoView {
    let format_clone = format.clone();

    let status_class = move || match status.get() {
        IndicatorStatus::Good => "stat-card stat-card--success",
        IndicatorStatus::Bad => "stat-card stat-card--error",
        IndicatorStatus::Warning => "stat-card stat-card--warning",
        IndicatorStatus::Neutral => "stat-card",
    };

    let formatted = move || match value.get() {
        Some(v) => format_value(v, &format_clone),
        None => "—".to_string(),
    };

    let change_view = move || {
        change_percent.get().map(|pct| {
            let (arrow, cls) = if pct > 0.5 {
                ("\u{2191}", "stat-card__change stat-card__change--up")
            } else if pct < -0.5 {
                ("\u{2193}", "stat-card__change stat-card__change--down")
            } else {
                ("", "stat-card__change stat-card__change--flat")
            };
            let text = format!("{}{:.1}%", arrow, pct.abs());
            view! { <span class=cls>{text}</span> }
        })
    };

    let subtitle_view = move || {
        subtitle.get().map(|s| {
            view! { <div class="stat-card__subtitle">{s}</div> }
        })
    };

    view! {
        <div class=status_class>
            <div class="stat-card__icon">
                {icon(&icon_name)}
            </div>
            <div class="stat-card__content">
                <div class="stat-card__label">{label}</div>
                <div class="stat-card__value">
                    {formatted}
                    {change_view}
                </div>
                {subtitle_view}
            </div>
        </div>
    }
}
