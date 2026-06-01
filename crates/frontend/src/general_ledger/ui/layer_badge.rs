//! Прямоугольный бейдж слоя GL.
//!
//! Форма — прямоугольная (как `.badge` в проводках GL), фон одинаковый для всех
//! слоёв; различается только цвет текста (`gl-layer-badge--{key}`, layout.css).
//! Единый источник состава слоёв — `GL_LAYER_CLASSES`.

use contracts::general_ledger::get_layer_class;
use leptos::prelude::*;

/// CSS-суффикс для `gl-layer-badge--{key}` по коду слоя.
pub fn color_key_for_layer(code: &str) -> &'static str {
    get_layer_class(code)
        .map(|item| item.color_key)
        .unwrap_or("default")
}

/// Короткая подпись бейджа (код в верхнем регистре).
pub fn label_for_layer(code: &str) -> String {
    if code.trim().is_empty() {
        "—".to_string()
    } else {
        code.to_uppercase()
    }
}

/// Человекочитаемое имя слоя (для подсказки), либо сам код.
pub fn name_for_layer(code: &str) -> String {
    get_layer_class(code)
        .map(|item| item.name.to_string())
        .unwrap_or_else(|| code.to_string())
}

#[component]
pub fn GlLayerBadge(#[prop(into)] layer: String) -> impl IntoView {
    let color_key = color_key_for_layer(&layer);
    let label = label_for_layer(&layer);
    let title = name_for_layer(&layer);
    let class = format!("gl-layer-badge gl-layer-badge--{color_key}");

    view! {
        <span class=class title=title>{label}</span>
    }
}
