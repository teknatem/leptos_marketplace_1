//! Компонент ячейки таблицы для денежных значений
//!
//! # Примеры
//!
//! ```rust
//! // Базовое использование
//! <TableCellMoney value=amount />
//!
//! // С валютой
//! <TableCellMoney value=amount show_currency=true currency="$" />
//!
//! // С жирным шрифтом (для прибыли)
//! <TableCellMoney value=profit bold=true />
//!
//! // Без цветовой индикации
//! <TableCellMoney value=amount color_by_sign=false />
//! ```

use super::number_format::format_money;
use leptos::prelude::*;
use thaw::*;

/// Компонент ячейки таблицы для отображения денежных значений
///
/// Автоматически:
/// - Форматирует число с 2 знаками после запятой
/// - Добавляет разделитель тысяч (пробел)
/// - Выравнивает по правому краю
/// - Окрашивает положительные значения в зеленый, отрицательные в красный
#[component]
pub fn TableCellMoney(
    /// Значение для отображения
    #[prop(into)]
    value: Signal<Option<f64>>,
    
    /// Символ валюты (по умолчанию "₽")
    #[prop(optional, default = "₽")]
    currency: &'static str,
    
    /// Показывать ли суффикс валюты
    #[prop(optional, default = false)]
    show_currency: bool,
    
    /// Включить цветовую индикацию по знаку
    /// true: положительные - зеленый, отрицательные - красный
    #[prop(optional, default = true)]
    color_by_sign: bool,
    
    /// Жирный шрифт
    #[prop(optional, default = false)]
    bold: bool,
) -> impl IntoView {
    let formatted_text = move || {
        match value.get() {
            Some(v) => {
                let formatted = format_money(v);
                if show_currency {
                    format!("{} {}", formatted, currency)
                } else {
                    formatted
                }
            }
            None => "—".to_string(),
        }
    };
    
    let cell_style = move || {
        let mut styles = Vec::new();
        
        // Цветовая индикация
        if color_by_sign {
            if let Some(v) = value.get() {
                if v > 0.0 {
                    styles.push("color: var(--color-success-700)");
                } else if v < 0.0 {
                    styles.push("color: var(--color-error-700)");
                }
                // 0.0 остается нейтральным
            }
        }
        
        // Жирный шрифт
        if bold {
            styles.push("font-weight: 600");
        }
        
        styles.join("; ")
    };
    
    view! {
        <TableCell class="text-right">
            <span style=cell_style>
                {formatted_text}
            </span>
        </TableCell>
    }
}
