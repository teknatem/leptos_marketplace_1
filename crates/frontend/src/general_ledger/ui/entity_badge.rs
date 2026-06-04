//! Прямоугольный бейдж субъекта учёта GL.
//!
//! По образцу `layer_badge`: фон одинаковый, различается цвет текста
//! (`gl-entity-badge--{key}`, layout.css). Единый источник состава субъектов —
//! `GL_ENTITY_CLASSES`.

use contracts::general_ledger::get_entity_class;
use leptos::prelude::*;

/// CSS-суффикс для `gl-entity-badge--{key}` по коду субъекта.
pub fn color_key_for_entity(code: &str) -> &'static str {
    get_entity_class(code)
        .map(|item| item.color_key)
        .unwrap_or("default")
}

/// Короткая подпись бейджа (код в верхнем регистре).
pub fn label_for_entity(code: &str) -> String {
    if code.trim().is_empty() {
        "—".to_string()
    } else {
        code.to_uppercase()
    }
}

/// Человекочитаемое имя субъекта (для подсказки), либо сам код.
pub fn name_for_entity(code: &str) -> String {
    get_entity_class(code)
        .map(|item| item.name.to_string())
        .unwrap_or_else(|| code.to_string())
}

#[component]
pub fn GlEntityBadge(#[prop(into)] entity: String) -> impl IntoView {
    let color_key = color_key_for_entity(&entity);
    let label = label_for_entity(&entity);
    let title = name_for_entity(&entity);
    let class = format!("gl-entity-badge gl-entity-badge--{color_key}");

    view! {
        <span class=class title=title>{label}</span>
    }
}
