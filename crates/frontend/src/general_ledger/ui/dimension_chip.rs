//! Бейджи измерений GL.
//!
//! Цвет определяется CSS-классом `gl-dim-chip--{key}` (layout.css).
//! Каждый `code_main` реестра измерений получает уникальный ключ и цвет.

use std::collections::HashSet;

use contracts::general_ledger::{GlDimensionCatalogItem, GlDimensionDef};
use leptos::prelude::*;

// ── Маппинг ───────────────────────────────────────────────────────────────

/// Возвращает CSS-суффикс для `gl-dim-chip--{key}` по `code_main` измерения.
pub fn color_key_for_code_main(code_main: &str) -> &'static str {
    match code_main {
        "Day" => "day",
        "Cab" => "cab",
        "RegType" => "regtype",
        "Layer" => "layer",
        "RegRef" => "regref",
        "Nom" => "nom",
        "uf" => "uf",
        "fulf" => "fulf",
        // Структурные («системные») измерения — единый системный цвет.
        "Turn" | "Dr" | "Cr" => "sys",
        _ => "default",
    }
}

pub fn label_for_code_main(code_main: &str) -> &'static str {
    match code_main {
        "Day" => "DAY",
        "Cab" => "CAB",
        "RegRef" => "DOC",
        "Nom" => "NOM",
        "RegType" => "TYPE",
        "Layer" => "LAYER",
        "uf" => "UF",
        "fulf" => "FULF",
        "Turn" => "TURN",
        "Dr" => "DT",
        "Cr" => "CT",
        _ => "?",
    }
}

/// Системное (структурное) измерение GL: оборот/счета проводки. Источник истины —
/// backend `is_structural_dimension`; здесь дублируем минимальный список id для
/// группировки в пикерах детализации без обращения к серверу.
pub fn is_system_dim_id(id: &str) -> bool {
    matches!(id, "turnover_code" | "debit_account" | "credit_account")
}

// ── Данные чипа ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct GlDimChip {
    pub label: String,
    pub color_key: &'static str,
    pub title: String,
}

pub fn chip_from_code_main(code_main: &str, title: &str) -> GlDimChip {
    GlDimChip {
        label: label_for_code_main(code_main).to_string(),
        color_key: color_key_for_code_main(code_main),
        title: title.to_string(),
    }
}

pub fn chip_from_dimension(d: &GlDimensionDef) -> GlDimChip {
    chip_from_code_main(&d.code_main, &d.label)
}

pub fn chip_from_catalog(item: &GlDimensionCatalogItem) -> GlDimChip {
    chip_from_code_main(&item.code_main, &item.label)
}

/// Уникальный чип на каждый `code_main` (порядок по первому появлению).
pub fn chips_from_dimensions(dimensions: &[GlDimensionDef]) -> Vec<GlDimChip> {
    let mut seen: HashSet<&str> = HashSet::new();
    let mut chips = Vec::new();
    for d in dimensions {
        if seen.insert(d.code_main.as_str()) {
            chips.push(chip_from_dimension(d));
        }
    }
    chips
}

// ── Компоненты ────────────────────────────────────────────────────────────

#[component]
pub fn GlDimensionChip(
    label: String,
    #[prop(into)] color_key: String,
    #[prop(default = String::new())] title: String,
    #[prop(default = false)] interactive: bool,
    #[prop(default = Callback::new(|_| ()))] on_click: Callback<()>,
) -> impl IntoView {
    let class = format!("gl-dim-chip gl-dim-chip--{color_key}");

    if interactive {
        let click = on_click;
        view! {
            <button type="button" class=class title=title on:click=move |_| click.run(())>
                {label}
            </button>
        }
        .into_any()
    } else {
        view! {
            <span class=class title=title>{label}</span>
        }
        .into_any()
    }
}

#[component]
pub fn GlDimensionChipList(
    chips: Vec<GlDimChip>,
    #[prop(default = false)] interactive: bool,
) -> impl IntoView {
    if chips.is_empty() {
        return view! {
            <span class="gl-dim-chip-empty">"—"</span>
        }
        .into_any();
    }

    view! {
        <div class="gl-dim-chip-list">
            <For
                each=move || chips.clone()
                key=|chip| chip.label.clone()
                children=move |chip| {
                    view! {
                        <GlDimensionChip
                            label=chip.label
                            color_key=chip.color_key
                            title=chip.title
                            interactive=interactive
                        />
                    }
                }
            />
        </div>
    }
    .into_any()
}
